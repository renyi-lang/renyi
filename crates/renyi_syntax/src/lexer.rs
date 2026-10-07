//! The lexer: source text to tokens (syntax sketch, section 1).
//!
//! Rules it implements: multi-word phrases are single tokens matched longest
//! first with exactly one space between words; any word after a dot is a
//! member name; `"..."` interpolates `{expression}` and a hole may not hold a
//! string literal; `"""` blocks start on the line after the opening quotes,
//! end before the line of the closing quotes and lose their common
//! indentation; `raw "..."` has no holes and no escapes; the text after
//! `purpose:`, `tags:`, `see also:` and `deprecated:` is one free-text token
//! that runs to the end of the line (indentation carries no meaning,
//! decision V2).

use crate::diagnostics::Diagnostic;
use crate::span::Span;
use crate::token::{TextPart, Token, TokenKind, Word};

#[derive(Debug, Default)]
pub struct Lexed {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Lex a whole source text. The token stream ends with `Eof`.
pub fn lex(source: &str) -> Lexed {
    let mut lexer = Lexer::new(source, 0, source.len());
    lexer.run();
    lexer.push(TokenKind::Eof, source.len(), source.len());
    Lexed {
        tokens: lexer.tokens,
        diagnostics: lexer.diagnostics,
    }
}

struct Lexer<'s> {
    src: &'s str,
    bytes: &'s [u8],
    pos: usize,
    limit: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
    raw_pending: bool,
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
}

/// `userName` as `user_name`, `HTTPClient` as `http_client`: the spelling
/// of a value name (decision C5), for a fix.
pub(crate) fn snake_case(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    for (index, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() {
            let after_lower = index > 0
                && (chars[index - 1].is_ascii_lowercase() || chars[index - 1].is_ascii_digit());
            let after_upper = index > 0 && chars[index - 1].is_ascii_uppercase();
            let before_lower = chars.get(index + 1).is_some_and(|n| n.is_ascii_lowercase());
            if (after_lower || (after_upper && before_lower)) && !out.ends_with('_') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// `user_record` as `UserRecord`: the spelling of a type name (decision
/// C5), for a fix.
pub(crate) fn pascal_case(text: &str) -> String {
    text.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

impl<'s> Lexer<'s> {
    fn new(src: &'s str, start: usize, limit: usize) -> Lexer<'s> {
        Lexer {
            src,
            bytes: src.as_bytes(),
            pos: start,
            limit,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
            raw_pending: false,
        }
    }

    fn run(&mut self) {
        while self.pos < self.limit {
            let start = self.pos;
            match self.bytes[start] {
                b' ' => self.pos += 1,
                b'\t' => {
                    self.error("tab", "tab character; Renyi indents with two spaces", start, start + 1)
                        .with_fix_in_place("replace the tab with spaces");
                    self.pos += 1;
                }
                b'\r' => {
                    self.error("crlf", "carriage return; Renyi files use LF line endings", start, start + 1);
                    self.pos += 1;
                }
                b'\n' => {
                    self.push(TokenKind::Newline, start, start + 1);
                    self.pos += 1;
                }
                b'#' => self.comment(),
                b'"' => self.text(),
                b'0'..=b'9' => self.number(),
                b'a'..=b'z' => self.word(),
                b'A'..=b'Z' => self.type_name(),
                b'(' => self.single(TokenKind::LeftParen),
                b')' => self.single(TokenKind::RightParen),
                b'[' => self.single(TokenKind::LeftBracket),
                b']' => self.single(TokenKind::RightBracket),
                b'{' => self.single(TokenKind::LeftBrace),
                b'}' => self.single(TokenKind::RightBrace),
                b',' => self.single(TokenKind::Comma),
                b':' => self.single(TokenKind::Colon),
                b'.' => self.single(TokenKind::Dot),
                b'+' => self.single(TokenKind::Plus),
                b'-' => self.single(TokenKind::Minus),
                b'*' => self.single(TokenKind::Star),
                b'/' => self.single(TokenKind::Slash),
                b'=' => self.symbol_error(
                    "equals-sign",
                    "`=` is not Renyi",
                    "write `let name be value` to define, `change name to value` to change, `is` to compare",
                ),
                b'<' | b'>' => self.symbol_error(
                    "angle-comparison",
                    "`<` and `>` are not Renyi",
                    "write `is less than`, `is at most`, `is greater than` or `is at least`",
                ),
                b';' => self.symbol_error("semicolon", "`;` is not Renyi", "one statement per line, no terminator"),
                b'!' | b'&' | b'|' | b'%' | b'^' | b'~' | b'?' | b'@' | b'$' | b'`' | b'\\' | b'\'' => {
                    let symbol = self.bytes[start] as char;
                    self.symbol_error(
                        "symbolic-operator",
                        &format!("`{symbol}` is not Renyi"),
                        "use the English word for the operation (`and`, `or`, `not`, `remainder`, `power`)",
                    )
                }
                b'_' => {
                    let end = self.scan_while(start, is_word_byte);
                    self.error("identifier-shape", "a name cannot start with an underscore", start, end)
                        .with_fix_in_place("start the name with a lowercase letter");
                    self.push(TokenKind::Error, start, end);
                    self.pos = end;
                }
                _ => {
                    let character = self.src[start..].chars().next().unwrap_or('\u{fffd}');
                    let end = start + character.len_utf8();
                    self.error(
                        "unknown-character",
                        format!("unexpected character `{character}`"),
                        start,
                        end,
                    )
                    .with_fix_in_place("remove it, or put it inside a text literal");
                    self.push(TokenKind::Error, start, end);
                    self.pos = end;
                }
            }
        }
    }

    // ----------------------------------------------------------- helpers

    fn push(&mut self, kind: TokenKind, start: usize, end: usize) {
        self.raw_pending = kind == TokenKind::Word(Word::Raw);
        self.tokens.push(Token::new(kind, Span::new(start, end)));
    }

    fn single(&mut self, kind: TokenKind) {
        let start = self.pos;
        self.push(kind, start, start + 1);
        self.pos += 1;
    }

    fn error(
        &mut self,
        code: &'static str,
        message: impl Into<String>,
        start: usize,
        end: usize,
    ) -> LastDiagnostic<'_, 's> {
        self.diagnostics
            .push(Diagnostic::error(code, message, Span::new(start, end)));
        LastDiagnostic { lexer: self }
    }

    fn symbol_error(&mut self, code: &'static str, message: &str, fix: &str) {
        let start = self.pos;
        let mut end = start + 1;
        while end < self.limit && matches!(self.bytes[end], b'=' | b'<' | b'>' | b'!' | b'&' | b'|')
        {
            end += 1;
        }
        let found = &self.src[start..end];
        let message = if found.len() > 1 {
            format!("`{found}` is not Renyi")
        } else {
            message.to_string()
        };
        self.error(code, message, start, end).with_fix_in_place(fix);
        self.push(TokenKind::Error, start, end);
        self.pos = end;
    }

    fn scan_while(&self, mut pos: usize, predicate: impl Fn(u8) -> bool) -> usize {
        while pos < self.limit && predicate(self.bytes[pos]) {
            pos += 1;
        }
        pos
    }

    /// Only spaces between the start of the line and `offset`.
    fn at_line_start(&self, offset: usize) -> bool {
        let line_start = self.src[..offset].rfind('\n').map_or(0, |index| index + 1);
        self.bytes[line_start..offset]
            .iter()
            .all(|&byte| byte == b' ')
    }

    fn line_end(&self, mut pos: usize) -> usize {
        while pos < self.limit && self.bytes[pos] != b'\n' {
            pos += 1;
        }
        pos
    }

    fn previous_is_adjacent_dot(&self, start: usize) -> bool {
        matches!(self.tokens.last(), Some(token) if token.kind == TokenKind::Dot && token.span.end == start)
    }

    // ----------------------------------------------------------- words

    fn word(&mut self) {
        let start = self.pos;
        let end = self.scan_while(start, is_word_byte);
        let text = &self.src[start..end];
        if end < self.limit && self.bytes[end].is_ascii_uppercase() {
            // `userName`: one name in another language's casing (decision C5)
            let full_end =
                self.scan_while(end, |byte| byte.is_ascii_alphanumeric() || byte == b'_');
            let full = &self.src[start..full_end];
            let message = format!("name `{full}` is not snake_case");
            let fix = format!("write `{}`", snake_case(full));
            self.error("identifier-shape", message, start, full_end)
                .with_fix_in_place(&fix);
            let kind = if self.previous_is_adjacent_dot(start) {
                TokenKind::Member
            } else {
                TokenKind::Identifier
            };
            self.push(kind, start, full_end);
            self.pos = full_end;
            return;
        }
        if self.previous_is_adjacent_dot(start) {
            self.push(TokenKind::Member, start, end);
            self.pos = end;
            return;
        }
        if let Some(word) = Word::from_spelling(text) {
            let (word, end) = if word.starts_phrase() {
                self.longest_phrase(word, end)
            } else {
                (word, end)
            };
            self.push(TokenKind::Word(word), start, end);
            self.pos = end;
            if word.takes_clause_text()
                && self.at_line_start(start)
                && self.pos < self.limit
                && self.bytes[self.pos] == b':'
            {
                self.single(TokenKind::Colon);
                self.clause_text();
            }
            return;
        }
        if text.len() == 1 {
            self.error(
                "single-letter-identifier",
                format!("single-letter name `{text}`"),
                start,
                end,
            )
            .with_fix_in_place("use a word that says what the value is");
        } else if text.ends_with('_') || text.contains("__") {
            self.error(
                "identifier-shape",
                format!("name `{text}` has a trailing or doubled underscore"),
                start,
                end,
            )
            .with_fix_in_place("separate words with single underscores");
        }
        self.push(TokenKind::Identifier, start, end);
        self.pos = end;
    }

    /// Extend a phrase starter over following words separated by single spaces.
    fn longest_phrase(&self, first: Word, mut end: usize) -> (Word, usize) {
        let mut spelling = first.spelling().to_string();
        let mut best = (first, end);
        for _ in 0..2 {
            if end + 1 < self.limit
                && self.bytes[end] == b' '
                && self.bytes[end + 1].is_ascii_lowercase()
            {
                let word_end = self.scan_while(end + 1, is_word_byte);
                spelling.push(' ');
                spelling.push_str(&self.src[end + 1..word_end]);
                end = word_end;
                if let Some(word) = Word::from_spelling(&spelling) {
                    best = (word, end);
                }
            } else {
                break;
            }
        }
        best
    }

    fn type_name(&mut self) {
        let start = self.pos;
        let end = self.scan_while(start, |byte| byte.is_ascii_alphanumeric());
        if end < self.limit && self.bytes[end] == b'_' {
            let full_end =
                self.scan_while(end, |byte| byte.is_ascii_alphanumeric() || byte == b'_');
            self.error(
                "type-name-shape",
                "type, ability and variant names are PascalCase without underscores",
                start,
                full_end,
            )
            .with_fix_in_place("remove the underscores and capitalize each word");
            self.push(TokenKind::Error, start, full_end);
            self.pos = full_end;
            return;
        }
        self.push(TokenKind::TypeName, start, end);
        self.pos = end;
    }

    /// The free text of a documentation clause: the rest of the line.
    fn clause_text(&mut self) {
        let text_start = self.scan_while(self.pos, |byte| byte == b' ');
        let end = self.line_end(text_start);
        let text = self.src[text_start..end].trim_end().to_string();
        self.push(TokenKind::ClauseText(text), text_start, end);
        self.pos = end;
    }

    // ----------------------------------------------------------- numbers

    fn number(&mut self) {
        let start = self.pos;
        let mut end = self.scan_while(start, |byte| byte.is_ascii_digit() || byte == b'_');
        let mut kind = TokenKind::Integer;
        if end + 1 < self.limit && self.bytes[end] == b'.' && self.bytes[end + 1].is_ascii_digit() {
            end = self.scan_while(end + 1, |byte| byte.is_ascii_digit());
            kind = TokenKind::Decimal;
        }
        if end < self.limit && (self.bytes[end].is_ascii_alphabetic() || self.bytes[end] == b'_') {
            let bad_end = self.scan_while(end, |byte| byte.is_ascii_alphanumeric() || byte == b'_');
            self.error(
                "number-shape",
                "a number cannot run into letters",
                start,
                bad_end,
            )
            .with_fix_in_place("separate the number and the name with a space or an operator");
            self.push(TokenKind::Error, start, bad_end);
            self.pos = bad_end;
            return;
        }
        if self.src[start..end].ends_with('_') {
            self.error(
                "number-shape",
                "a number cannot end with an underscore",
                start,
                end,
            );
        }
        self.push(kind, start, end);
        self.pos = end;
    }

    // ----------------------------------------------------------- comments

    fn comment(&mut self) {
        let start = self.pos;
        let end = self.line_end(start);
        self.push(TokenKind::Comment, start, end);
        self.pos = end;
    }

    // ----------------------------------------------------------- text

    fn text(&mut self) {
        let start = self.pos;
        if self.src[start..].starts_with("\"\"\"") {
            self.block_text();
        } else if self.raw_pending {
            self.raw_text();
        } else {
            self.simple_text();
        }
    }

    fn raw_text(&mut self) {
        let start = self.pos;
        let mut end = start + 1;
        while end < self.limit && self.bytes[end] != b'"' && self.bytes[end] != b'\n' {
            end += 1;
        }
        if end >= self.limit || self.bytes[end] != b'"' {
            self.error(
                "unterminated-text",
                "raw text without a closing quote",
                start,
                end,
            );
            self.push(TokenKind::Error, start, end);
            self.pos = end;
            return;
        }
        let content = self.src[start + 1..end].to_string();
        self.push(TokenKind::RawText(content), start, end + 1);
        self.pos = end + 1;
    }

    fn simple_text(&mut self) {
        let start = self.pos;
        let mut end = start + 1;
        while end < self.limit {
            match self.bytes[end] {
                b'"' => break,
                b'\n' => break,
                b'\\' => end += 2,
                b'{' => {
                    // a hole may not hold a text literal, so its quotes do not close the text
                    let line_end = self.line_end(end);
                    end = match self.src[end..line_end].find('}') {
                        Some(offset) => end + offset + 1,
                        None => end + 1,
                    };
                }
                _ => end += 1,
            }
        }
        if end >= self.limit || self.bytes[end] != b'"' {
            let end = end.min(self.limit);
            self.error(
                "unterminated-text",
                "text without a closing quote on the same line",
                start,
                end,
            )
            .with_fix_in_place("close the quote, or use `\"\"\"` for text that spans lines");
            self.push(TokenKind::Error, start, end);
            self.pos = end;
            return;
        }
        let mut parts = Vec::new();
        self.scan_text_segment(start + 1, end, &mut parts);
        self.push(
            TokenKind::Text {
                parts,
                block: false,
            },
            start,
            end + 1,
        );
        self.pos = end + 1;
    }

    fn block_text(&mut self) {
        let start = self.pos;
        let after_quotes = start + 3;
        let first_line_end = self.line_end(after_quotes);
        if !self.src[after_quotes..first_line_end].trim().is_empty() {
            self.error(
                "block-text-start",
                "block text starts on the line after the opening quotes",
                after_quotes,
                first_line_end,
            )
            .with_fix_in_place("move the text to the next line");
        }
        // collect content lines up to the line holding the closing quotes
        let mut lines: Vec<(usize, usize)> = Vec::new();
        let mut cursor = first_line_end;
        let closing: Option<(usize, usize)> = loop {
            if cursor >= self.limit {
                break None;
            }
            let line_start = cursor + 1; // skip the newline
            let line_end = self.line_end(line_start);
            if self.src[line_start..line_end].trim() == "\"\"\"" {
                let quote_start =
                    line_start + self.src[line_start..line_end].find('"').unwrap_or(0);
                break Some((quote_start, quote_start + 3));
            }
            lines.push((line_start, line_end));
            cursor = line_end;
        };
        let Some((closing_start, closing_end)) = closing else {
            self.error(
                "unterminated-block-text",
                "block text without closing `\"\"\"` on its own line",
                start,
                self.limit,
            );
            self.push(TokenKind::Error, start, self.limit);
            self.pos = self.limit;
            return;
        };
        let common_indent = lines
            .iter()
            .filter(|(line_start, line_end)| !self.src[*line_start..*line_end].trim().is_empty())
            .map(|(line_start, line_end)| {
                self.src[*line_start..*line_end]
                    .bytes()
                    .take_while(|&byte| byte == b' ')
                    .count()
            })
            .min()
            .unwrap_or(0);
        let mut parts = Vec::new();
        for (index, (line_start, line_end)) in lines.iter().enumerate() {
            if index > 0 {
                push_text(&mut parts, "\n");
            }
            let content_start = (*line_start + common_indent).min(*line_end);
            self.scan_text_segment(content_start, *line_end, &mut parts);
        }
        let _ = closing_start;
        self.push(TokenKind::Text { parts, block: true }, start, closing_end);
        self.pos = closing_end;
    }

    /// Escapes and `{holes}` within one line of text content.
    fn scan_text_segment(&mut self, start: usize, end: usize, parts: &mut Vec<TextPart>) {
        let mut buffer = String::new();
        let mut pos = start;
        while pos < end {
            match self.bytes[pos] {
                b'\\' => {
                    let escape = self.bytes.get(pos + 1).copied();
                    match escape {
                        Some(b'n') => buffer.push('\n'),
                        Some(b't') => buffer.push('\t'),
                        Some(b'"') => buffer.push('"'),
                        Some(b'\\') => buffer.push('\\'),
                        Some(b'{') => buffer.push('{'),
                        _ => {
                            let bad_end = (pos + 2).min(end);
                            self.error(
                                "unknown-escape",
                                format!("unknown escape `{}`", &self.src[pos..bad_end]),
                                pos,
                                bad_end,
                            )
                            .with_fix_in_place("the escapes are \\n \\t \\\" \\\\ and \\{");
                        }
                    }
                    pos = (pos + 2).min(end);
                }
                b'{' => {
                    let hole_start = pos + 1;
                    let mut hole_end = hole_start;
                    while hole_end < end && self.bytes[hole_end] != b'}' {
                        hole_end += 1;
                    }
                    if hole_end >= end {
                        self.error(
                            "unterminated-hole",
                            "`{` without a closing `}` in text",
                            pos,
                            end,
                        )
                        .with_fix_in_place("close the hole, or write `\\{` for a literal brace");
                        pos = end;
                        continue;
                    }
                    if self.src[hole_start..hole_end].trim().is_empty() {
                        self.error("empty-hole", "empty `{}` in text", pos, hole_end + 1)
                            .with_fix_in_place("put an expression in the hole, or write `\\{}` for a literal brace");
                    } else if self.src[hole_start..hole_end].contains('"') {
                        self.error(
                            "text-in-hole",
                            "a text literal inside a hole",
                            hole_start,
                            hole_end,
                        )
                        .with_fix_in_place(
                            "bind the text to a name first and use the name in the hole",
                        );
                    }
                    if !buffer.is_empty() {
                        push_text(parts, &buffer);
                        buffer.clear();
                    }
                    let mut inner = Lexer::new(self.src, hole_start, hole_end);
                    inner.run();
                    self.diagnostics.extend(inner.diagnostics);
                    parts.push(TextPart::Hole(inner.tokens));
                    pos = hole_end + 1;
                }
                _ => {
                    let character = self.src[pos..].chars().next().unwrap_or('\u{fffd}');
                    buffer.push(character);
                    pos += character.len_utf8();
                }
            }
        }
        if !buffer.is_empty() {
            push_text(parts, &buffer);
        }
    }
}

fn push_text(parts: &mut Vec<TextPart>, text: &str) {
    if let Some(TextPart::Text(last)) = parts.last_mut() {
        last.push_str(text);
    } else {
        parts.push(TextPart::Text(text.to_string()));
    }
}

/// Lets `self.error(...)` be followed by `.with_fix_in_place(...)`.
struct LastDiagnostic<'a, 's> {
    lexer: &'a mut Lexer<'s>,
}

impl LastDiagnostic<'_, '_> {
    fn with_fix_in_place(self, fix: &str) {
        if let Some(diagnostic) = self.lexer.diagnostics.last_mut() {
            diagnostic.fix = Some(fix.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<String> {
        let lexed = lex(source);
        assert!(lexed.diagnostics.is_empty(), "{:?}", lexed.diagnostics);
        lexed.tokens.iter().map(|token| token.kind_name()).collect()
    }

    fn texts(source: &str) -> Vec<String> {
        lex(source)
            .tokens
            .iter()
            .map(|token| token.text(source).to_string())
            .collect()
    }

    #[test]
    fn phrases_are_single_tokens_matched_longest_first() {
        assert_eq!(
            kinds("age is at least 18 and age is not 0"),
            [
                "identifier",
                "word(is at least)",
                "integer",
                "word(and)",
                "identifier",
                "word(is not)",
                "integer",
                "eof"
            ]
        );
        assert_eq!(
            texts("for each user in users"),
            ["for each", "user", "in", "users", ""]
        );
        assert_eq!(kinds("shape is one of")[1], "word(is one of)");
        assert_eq!(kinds("shape is at most")[1], "word(is at most)");
    }

    #[test]
    fn a_phrase_needs_exactly_one_space() {
        assert_eq!(kinds("count is  less")[1], "word(is)");
    }

    #[test]
    fn words_after_a_dot_are_members() {
        assert_eq!(
            kinds("event.type.count()"),
            ["identifier", ".", "member", ".", "member", "(", ")", "eof"]
        );
        assert_eq!(kinds("items.sorted()")[2], "member");
    }

    #[test]
    fn interpolation_holes_are_lexed() {
        let lexed = lex("\"Hello {user.name}, total {total}!\"");
        assert!(lexed.diagnostics.is_empty());
        match &lexed.tokens[0].kind {
            TokenKind::Text {
                parts,
                block: false,
            } => {
                assert_eq!(parts.len(), 5);
                assert_eq!(parts[0], TextPart::Text("Hello ".into()));
                assert!(matches!(&parts[1], TextPart::Hole(tokens) if tokens.len() == 3));
                assert_eq!(parts[4], TextPart::Text("!".into()));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn escapes_and_errors_in_text() {
        let lexed = lex("\"a\\{b\\n\"");
        assert!(lexed.diagnostics.is_empty(), "{:?}", lexed.diagnostics);
        assert_eq!(
            lexed.tokens[0].kind,
            TokenKind::Text {
                parts: vec![TextPart::Text("a{b\n".into())],
                block: false
            }
        );
        assert_eq!(lex("\"{}\"").diagnostics[0].code, "empty-hole");
        assert_eq!(
            lex("\"{items.join(\", \")}\"").diagnostics[0].code,
            "text-in-hole"
        );
        assert_eq!(lex("\"open").diagnostics[0].code, "unterminated-text");
        assert_eq!(lex("\"\\q\"").diagnostics[0].code, "unknown-escape");
    }

    #[test]
    fn raw_text_has_no_escapes_or_holes() {
        let lexed = lex("raw \"^[0-9]{4}\\d$\"");
        assert!(lexed.diagnostics.is_empty());
        assert_eq!(
            lexed.tokens[1].kind,
            TokenKind::RawText("^[0-9]{4}\\d$".into())
        );
    }

    #[test]
    fn block_text_is_dedented_and_joined() {
        let source = "let schema be \"\"\"\n    CREATE {kind}\n      x\n    \"\"\"\nlet";
        let lexed = lex(source);
        assert!(lexed.diagnostics.is_empty(), "{:?}", lexed.diagnostics);
        match &lexed.tokens[3].kind {
            TokenKind::Text { parts, block: true } => {
                assert_eq!(parts[0], TextPart::Text("CREATE ".into()));
                assert!(matches!(parts[1], TextPart::Hole(_)));
                assert_eq!(parts[2], TextPart::Text("\n  x".into()));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(lexed.tokens[4].kind, TokenKind::Newline);
        assert!(lexed.tokens[5].is_word(Word::Let));
    }

    #[test]
    fn clause_text_runs_to_the_end_of_its_line_whatever_the_indentation() {
        let source = "  purpose: Read users; keep the adults.  \ntags: a, b\n  let";
        let lexed = lex(source);
        assert!(lexed.diagnostics.is_empty(), "{:?}", lexed.diagnostics);
        assert_eq!(
            lexed.tokens[2].kind,
            TokenKind::ClauseText("Read users; keep the adults.".into())
        );
        assert_eq!(lexed.tokens[3].kind, TokenKind::Newline);
        assert!(lexed.tokens[4].is_word(Word::Tags));
        assert_eq!(lexed.tokens[6].kind, TokenKind::ClauseText("a, b".into()));
        // a deeper second line is code, not more prose
        let lexed = lex("  purpose: Read users.\n    Second line.\n");
        assert_eq!(lexed.tokens[3].kind, TokenKind::Newline);
        assert_eq!(lexed.tokens[4].kind, TokenKind::TypeName);
    }

    #[test]
    fn see_also_is_a_clause_too() {
        let lexed = lex("  see also: total, subtotal\n");
        assert_eq!(lexed.tokens[0].kind, TokenKind::Word(Word::SeeAlso));
        assert_eq!(
            lexed.tokens[2].kind,
            TokenKind::ClauseText("total, subtotal".into())
        );
    }

    #[test]
    fn numbers() {
        assert_eq!(
            kinds("1_000 19.99 3.to_text()"),
            ["integer", "decimal", "integer", ".", "member", "(", ")", "eof"]
        );
        assert_eq!(lex("12abc").diagnostics[0].code, "number-shape");
    }

    #[test]
    fn symbols_get_fix_suggestions() {
        let lexed = lex("let total = 1");
        let codes: Vec<&str> = lexed.diagnostics.iter().map(|d| d.code).collect();
        assert_eq!(codes, ["equals-sign"]);
        assert_eq!(
            lex("left == right").diagnostics[0].message,
            "`==` is not Renyi"
        );
        assert_eq!(lex("left <= right").diagnostics[0].code, "angle-comparison");
        assert_eq!(lex("tiny").diagnostics.len(), 0);
        assert_eq!(lex("x").diagnostics[0].code, "single-letter-identifier");
    }

    #[test]
    fn identifier_shapes() {
        assert_eq!(lex("foo_").diagnostics[0].code, "identifier-shape");
        assert_eq!(lex("Foo_Bar").diagnostics[0].code, "type-name-shape");
        assert_eq!(lex("_x").diagnostics[0].code, "identifier-shape");
    }

    #[test]
    fn comments_and_newlines() {
        assert_eq!(
            kinds("let a_b be 1 # why\n"),
            [
                "word(let)",
                "identifier",
                "word(be)",
                "integer",
                "comment",
                "newline",
                "eof"
            ]
        );
    }
}
