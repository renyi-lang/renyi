//! Tokens, reserved words and phrases (syntax sketch, sections 1 and 17).

use crate::span::Span;

macro_rules! words {
    ($( $variant:ident => $spelling:literal, )*) => {
        /// Every reserved word and every multi-word phrase. A phrase is one
        /// token: `is at least`, `or fails with`, `for each`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Word {
            $( $variant, )*
        }

        impl Word {
            pub const ALL: &'static [Word] = &[ $( Word::$variant, )* ];

            pub fn spelling(self) -> &'static str {
                match self { $( Word::$variant => $spelling, )* }
            }

            pub fn from_spelling(text: &str) -> Option<Word> {
                match text {
                    $( $spelling => Some(Word::$variant), )*
                    _ => None,
                }
            }
        }
    };
}

words! {
    Ability => "ability", All => "all", Also => "also", And => "and", Any => "any",
    As => "as", At => "at", Be => "be", Break => "break", By => "by", Can => "can",
    Check => "check", Collect => "collect", Concurrently => "concurrently",
    Continue => "continue", Count => "count", Crash => "crash", Deprecated => "deprecated",
    Descending => "descending", Each => "each", End => "end", Example => "example",
    Expose => "expose", Exposing => "exposing", Fail => "fail", Fails => "fails",
    Failure => "failure", False => "false", First => "first", For => "for", From => "from",
    Function => "function", Greater => "greater", Group => "group", Has => "has", If => "if",
    Ignore => "ignore", Import => "import", In => "in", Is => "is", Lazy => "lazy",
    Least => "least", Less => "less", Let => "let", Match => "match", Maybe => "maybe",
    Module => "module", Most => "most", Mutable => "mutable", Needs => "needs", Not => "not",
    Nothing => "nothing", Of => "of", One => "one", Only => "only", Or => "or",
    Otherwise => "otherwise", Per => "per",
    Power => "power", Public => "public", Purpose => "purpose", Raw => "raw",
    Remainder => "remainder", Repeat => "repeat", Replays => "replays", Return => "return",
    Returns => "returns",
    Run => "run",
    See => "see", SelfValue => "self", Set => "set", Some => "some", Sorted => "sorted",
    Success => "success", Sum => "sum", Tags => "tags", Test => "test", Than => "than",
    Then => "then", To => "to", Tool => "tool", True => "true", Type => "type", Until => "until",
    When => "when", Where => "where", With => "with", Within => "within",
    // phrases: single tokens, matched longest first
    IsNot => "is not", IsLessThan => "is less than", IsAtMost => "is at most",
    IsGreaterThan => "is greater than", IsAtLeast => "is at least",
    OrFailsWith => "or fails with", IsOneOf => "is one of", ForEach => "for each",
    ForAny => "for any", RunConcurrently => "run concurrently", SortedBy => "sorted by",
    GroupBy => "group by", SeeAlso => "see also", ExposeAsTool => "expose as tool",
    RepeatUntil => "repeat until", AtMost => "at most", OnlyTo => "only to",
}

impl Word {
    /// Phrases are reserved words with a space in them.
    pub fn is_phrase(self) -> bool {
        self.spelling().contains(' ')
    }

    /// Words that can begin a phrase; the lexer looks ahead after them.
    pub fn starts_phrase(self) -> bool {
        matches!(
            self,
            Word::Is
                | Word::Or
                | Word::For
                | Word::Run
                | Word::Repeat
                | Word::At
                | Word::Only
                | Word::Sorted
                | Word::Group
                | Word::See
                | Word::Expose
        )
    }

    /// Clause words whose remainder of the line is free text.
    pub fn takes_clause_text(self) -> bool {
        matches!(
            self,
            Word::Purpose | Word::Tags | Word::Deprecated | Word::SeeAlso
        )
    }
}

/// A piece of a text literal: literal characters, or the tokens of a `{hole}`.
#[derive(Clone, Debug, PartialEq)]
pub enum TextPart {
    Text(String),
    Hole(Vec<Token>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Word(Word),
    /// `[a-z][a-z0-9_]*`: a value, function, field, parameter or module name.
    Identifier,
    /// `[A-Z][A-Za-z0-9]*`: a type, ability or variant name.
    TypeName,
    /// Any word directly after a dot; reserved words are allowed here.
    Member,
    Integer,
    Decimal,
    /// `"..."` or `"""..."""`, with interpolation holes already lexed.
    Text {
        parts: Vec<TextPart>,
        block: bool,
    },
    /// `raw "..."`: no holes, no escapes.
    RawText(String),
    /// Free text after `purpose:`, `tags:`, `see also:` or `deprecated:`:
    /// the rest of the line.
    ClauseText(String),
    Comment,
    Newline,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Comma,
    Colon,
    Dot,
    Plus,
    Minus,
    Star,
    Slash,
    Eof,
    /// Something the lexer could not read; a diagnostic was emitted.
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Token {
        Token { kind, span }
    }

    /// The source text the token covers.
    pub fn text<'s>(&self, source: &'s str) -> &'s str {
        &source[self.span.start..self.span.end]
    }

    pub fn is_word(&self, word: Word) -> bool {
        self.kind == TokenKind::Word(word)
    }

    /// A short name for token dumps.
    pub fn kind_name(&self) -> String {
        match &self.kind {
            TokenKind::Word(word) => format!("word({})", word.spelling()),
            TokenKind::Identifier => "identifier".into(),
            TokenKind::TypeName => "type-name".into(),
            TokenKind::Member => "member".into(),
            TokenKind::Integer => "integer".into(),
            TokenKind::Decimal => "decimal".into(),
            TokenKind::Text { block: false, .. } => "text".into(),
            TokenKind::Text { block: true, .. } => "block-text".into(),
            TokenKind::RawText(_) => "raw-text".into(),
            TokenKind::ClauseText(_) => "clause-text".into(),
            TokenKind::Comment => "comment".into(),
            TokenKind::Newline => "newline".into(),
            TokenKind::LeftParen => "(".into(),
            TokenKind::RightParen => ")".into(),
            TokenKind::LeftBracket => "[".into(),
            TokenKind::RightBracket => "]".into(),
            TokenKind::LeftBrace => "{".into(),
            TokenKind::RightBrace => "}".into(),
            TokenKind::Comma => ",".into(),
            TokenKind::Colon => ":".into(),
            TokenKind::Dot => ".".into(),
            TokenKind::Plus => "+".into(),
            TokenKind::Minus => "-".into(),
            TokenKind::Star => "*".into(),
            TokenKind::Slash => "/".into(),
            TokenKind::Eof => "eof".into(),
            TokenKind::Error => "error".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eighty_eight_reserved_words_and_seventeen_phrases() {
        let words = Word::ALL.iter().filter(|w| !w.is_phrase()).count();
        let phrases = Word::ALL.iter().filter(|w| w.is_phrase()).count();
        assert_eq!(words, 88);
        assert_eq!(phrases, 17);
    }

    #[test]
    fn spellings_round_trip() {
        for word in Word::ALL {
            assert_eq!(Word::from_spelling(word.spelling()), Some(*word));
        }
        assert_eq!(Word::from_spelling("items"), None);
    }

    #[test]
    fn every_phrase_word_is_reserved_on_its_own() {
        for word in Word::ALL.iter().filter(|w| w.is_phrase()) {
            for part in word.spelling().split(' ') {
                assert!(
                    Word::from_spelling(part).is_some(),
                    "{part} is not reserved"
                );
            }
        }
    }
}
