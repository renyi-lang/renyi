//! The formal grammar (`docs/grammar.ebnf`) and the parser agree.
//!
//! An interpreter for the grammar file reads its rules (W3C EBNF over the
//! lexer's tokens) and accepts a token stream when some derivation covers
//! it, with the full meaning of `|`, `*` and `-`: every alternative and
//! every length, not the first that fits. It runs over the corpus, the
//! conformance programs and the library declarations and must give the
//! parser's verdict on each; two lists of corner programs cover the
//! spellings the corpus does not use, in both directions.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::rc::Rc;

use renyi_syntax::parser::CONTINUATION_WORDS;
use renyi_syntax::{lex, parse, parse_declarations, SourceFile, Span, TextPart, Token, TokenKind};

// ------------------------------------------------------------ the grammar

/// The token classes: the names the grammar uses without defining them.
const TOKEN_CLASSES: &[&str] = &[
    "Identifier",
    "TypeName",
    "Member",
    "ReservedWord",
    "Integer",
    "Decimal",
    "Text",
    "PlainText",
    "RawText",
    "ClauseText",
    "Newline",
];

#[derive(Clone, Debug)]
enum Node {
    Literal(String),
    Class(&'static str),
    Rule(usize),
    Sequence(Vec<Node>),
    Choice(Vec<Node>),
    Optional(Box<Node>),
    Repeated(Box<Node>),
    Except(Box<Node>, Box<Node>),
}

struct Grammar {
    names: Vec<String>,
    bodies: Vec<Node>,
}

impl Grammar {
    fn rule(&self, name: &str) -> usize {
        self.names
            .iter()
            .position(|n| n == name)
            .unwrap_or_else(|| panic!("the grammar has no rule `{name}`"))
    }

    /// The rules reachable from a start rule.
    fn reachable(&self, start: &str) -> BTreeSet<usize> {
        fn walk(node: &Node, seen: &mut BTreeSet<usize>, grammar: &Grammar) {
            match node {
                Node::Literal(_) | Node::Class(_) => {}
                Node::Rule(index) => {
                    if seen.insert(*index) {
                        walk(&grammar.bodies[*index], seen, grammar);
                    }
                }
                Node::Sequence(items) | Node::Choice(items) => {
                    for item in items {
                        walk(item, seen, grammar);
                    }
                }
                Node::Optional(inner) | Node::Repeated(inner) => walk(inner, seen, grammar),
                Node::Except(left, right) => {
                    walk(left, seen, grammar);
                    walk(right, seen, grammar);
                }
            }
        }
        let mut seen = BTreeSet::new();
        walk(&Node::Rule(self.rule(start)), &mut seen, self);
        seen
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Piece {
    Name(String),
    Literal(String),
    Defines,
    Bar,
    Open,
    Close,
    Question,
    Star,
    Plus,
    Minus,
}

/// The pieces of the grammar file, comments removed.
fn pieces(text: &str) -> Vec<Piece> {
    let mut source = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        source.push_str(&rest[..start]);
        let close = rest[start..].find("*/").expect("a comment closes");
        rest = &rest[start + close + 2..];
    }
    source.push_str(rest);
    let chars: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        match c {
            ' ' | '\n' | '\t' | '\r' => index += 1,
            ':' => {
                assert!(
                    chars[index..].starts_with(&[':', ':', '=']),
                    "`::=` expected in the grammar file"
                );
                out.push(Piece::Defines);
                index += 3;
            }
            '|' => {
                out.push(Piece::Bar);
                index += 1;
            }
            '(' => {
                out.push(Piece::Open);
                index += 1;
            }
            ')' => {
                out.push(Piece::Close);
                index += 1;
            }
            '?' => {
                out.push(Piece::Question);
                index += 1;
            }
            '*' => {
                out.push(Piece::Star);
                index += 1;
            }
            '+' => {
                out.push(Piece::Plus);
                index += 1;
            }
            '-' => {
                out.push(Piece::Minus);
                index += 1;
            }
            '\'' | '"' => {
                let close = chars[index + 1..]
                    .iter()
                    .position(|&d| d == c)
                    .expect("a literal closes");
                out.push(Piece::Literal(
                    chars[index + 1..index + 1 + close].iter().collect(),
                ));
                index += close + 2;
            }
            _ if c.is_ascii_alphabetic() => {
                let mut end = index;
                while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_')
                {
                    end += 1;
                }
                out.push(Piece::Name(chars[index..end].iter().collect()));
                index = end;
            }
            _ => panic!("unexpected `{c}` in the grammar file"),
        }
    }
    out
}

struct Reader<'a> {
    pieces: &'a [Piece],
    pos: usize,
    names: &'a [String],
}

impl Reader<'_> {
    fn choice(&mut self) -> Node {
        let mut alternatives = vec![self.sequence()];
        while self.pieces.get(self.pos) == Some(&Piece::Bar) {
            self.pos += 1;
            alternatives.push(self.sequence());
        }
        if alternatives.len() == 1 {
            alternatives.pop().unwrap()
        } else {
            Node::Choice(alternatives)
        }
    }

    fn sequence(&mut self) -> Node {
        let mut items = Vec::new();
        while let Some(piece) = self.pieces.get(self.pos) {
            if matches!(piece, Piece::Bar | Piece::Close) {
                break;
            }
            items.push(self.item());
        }
        if items.len() == 1 {
            items.pop().unwrap()
        } else {
            Node::Sequence(items)
        }
    }

    /// An item, with `A - B` binding tighter than a sequence.
    fn item(&mut self) -> Node {
        let node = self.postfixed();
        if self.pieces.get(self.pos) == Some(&Piece::Minus) {
            self.pos += 1;
            let right = self.postfixed();
            return Node::Except(Box::new(node), Box::new(right));
        }
        node
    }

    fn postfixed(&mut self) -> Node {
        let mut node = self.atom();
        loop {
            match self.pieces.get(self.pos) {
                Some(Piece::Question) => {
                    self.pos += 1;
                    node = Node::Optional(Box::new(node));
                }
                Some(Piece::Star) => {
                    self.pos += 1;
                    node = Node::Repeated(Box::new(node));
                }
                Some(Piece::Plus) => {
                    self.pos += 1;
                    node = Node::Sequence(vec![node.clone(), Node::Repeated(Box::new(node))]);
                }
                _ => return node,
            }
        }
    }

    fn atom(&mut self) -> Node {
        let piece = self
            .pieces
            .get(self.pos)
            .cloned()
            .expect("an atom in the grammar file");
        self.pos += 1;
        match piece {
            Piece::Name(name) => {
                if let Some(index) = self.names.iter().position(|n| *n == name) {
                    Node::Rule(index)
                } else if let Some(class) = TOKEN_CLASSES.iter().find(|class| **class == name) {
                    Node::Class(class)
                } else {
                    panic!("`{name}` is neither a rule nor a token class")
                }
            }
            Piece::Literal(text) => Node::Literal(text),
            Piece::Open => {
                let node = self.choice();
                assert_eq!(
                    self.pieces.get(self.pos),
                    Some(&Piece::Close),
                    "`)` expected in the grammar file"
                );
                self.pos += 1;
                node
            }
            other => panic!("unexpected {other:?} in a rule"),
        }
    }
}

fn read_grammar(text: &str) -> Grammar {
    let pieces = pieces(text);
    let mut rules: Vec<(String, Vec<Piece>)> = Vec::new();
    let mut index = 0;
    while index < pieces.len() {
        let Piece::Name(name) = &pieces[index] else {
            panic!("a rule starts with a name, found {:?}", pieces[index]);
        };
        assert_eq!(
            pieces.get(index + 1),
            Some(&Piece::Defines),
            "`::=` after `{name}`"
        );
        let mut end = index + 2;
        while end < pieces.len()
            && !(matches!(pieces[end], Piece::Name(_))
                && pieces.get(end + 1) == Some(&Piece::Defines))
        {
            end += 1;
        }
        rules.push((name.clone(), pieces[index + 2..end].to_vec()));
        index = end;
    }
    let names: Vec<String> = rules.iter().map(|(name, _)| name.clone()).collect();
    for (position, name) in names.iter().enumerate() {
        assert!(
            !names[..position].contains(name),
            "rule `{name}` is defined twice"
        );
        assert!(
            !TOKEN_CLASSES.contains(&name.as_str()),
            "`{name}` is a token class and cannot be a rule"
        );
    }
    let mut bodies = Vec::new();
    for (name, body) in &rules {
        let mut reader = Reader {
            pieces: body,
            pos: 0,
            names: &names,
        };
        let node = reader.choice();
        assert_eq!(reader.pos, body.len(), "rule `{name}` has pieces left over");
        bodies.push(node);
    }
    Grammar { names, bodies }
}

// ------------------------------------------------------------ the tokens

#[derive(Clone, Debug, PartialEq)]
enum Class {
    Word,
    Punctuation,
    Identifier,
    TypeName,
    Member,
    Integer,
    Decimal,
    Text { plain: bool },
    RawText,
    ClauseText,
    Newline,
    Broken,
}

#[derive(Clone, Debug)]
struct Tok {
    class: Class,
    text: String,
    span: Span,
}

fn classify(token: &Token, source: &str) -> Tok {
    let class = match &token.kind {
        TokenKind::Word(_) => Class::Word,
        TokenKind::Identifier => Class::Identifier,
        TokenKind::TypeName => Class::TypeName,
        TokenKind::Member => Class::Member,
        TokenKind::Integer => Class::Integer,
        TokenKind::Decimal => Class::Decimal,
        TokenKind::Text { parts, .. } => Class::Text {
            plain: !parts.iter().any(|part| matches!(part, TextPart::Hole(_))),
        },
        TokenKind::RawText(_) => Class::RawText,
        TokenKind::ClauseText(_) => Class::ClauseText,
        TokenKind::Newline => Class::Newline,
        TokenKind::Comment | TokenKind::Eof | TokenKind::Error => Class::Broken,
        _ => Class::Punctuation,
    };
    Tok {
        class,
        text: token.text(source).to_string(),
        span: token.span,
    }
}

/// The token stream the grammar sees: comments gone, the line breaks that
/// carry no meaning gone (inside brackets, after a comma, before a
/// continuation word), a run of line breaks one Newline, and one Newline at
/// the end.
fn stream(source: &str, tokens: &[Token]) -> Vec<Tok> {
    let significant: Vec<&Token> = tokens
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Comment | TokenKind::Eof))
        .collect();
    let mut out: Vec<Tok> = Vec::new();
    let mut depth: usize = 0;
    let mut index = 0;
    while index < significant.len() {
        let token = significant[index];
        if token.kind == TokenKind::Newline {
            let mut next = index;
            while next < significant.len() && significant[next].kind == TokenKind::Newline {
                next += 1;
            }
            let continues = next < significant.len()
                && matches!(&significant[next].kind, TokenKind::Word(word) if CONTINUATION_WORDS.contains(word));
            let after_comma = index > 0 && significant[index - 1].kind == TokenKind::Comma;
            let folded = matches!(
                out.last(),
                Some(Tok {
                    class: Class::Newline,
                    ..
                })
            );
            if depth == 0 && !after_comma && !continues && !folded {
                out.push(Tok {
                    class: Class::Newline,
                    text: "\n".to_string(),
                    span: token.span,
                });
            }
            index = next;
            continue;
        }
        match token.kind {
            TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::LeftBrace => depth += 1,
            TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace => {
                depth = depth.saturating_sub(1)
            }
            _ => {}
        }
        out.push(classify(token, source));
        index += 1;
    }
    if !matches!(
        out.last(),
        Some(Tok {
            class: Class::Newline,
            ..
        })
    ) {
        out.push(Tok {
            class: Class::Newline,
            text: "\n".to_string(),
            span: Span::new(source.len(), source.len()),
        });
    }
    out
}

fn terminal_matches(tok: &Tok, node: &Node) -> bool {
    match node {
        Node::Literal(text) => {
            tok.text == *text
                && matches!(
                    tok.class,
                    Class::Word | Class::Punctuation | Class::Identifier
                )
        }
        Node::Class(class) => match *class {
            "Identifier" => tok.class == Class::Identifier,
            "TypeName" => tok.class == Class::TypeName,
            "Member" => tok.class == Class::Member,
            "ReservedWord" => tok.class == Class::Word && !tok.text.contains(' '),
            "Integer" => tok.class == Class::Integer,
            "Decimal" => tok.class == Class::Decimal,
            "Text" => matches!(tok.class, Class::Text { .. }),
            "PlainText" => tok.class == Class::Text { plain: true },
            "RawText" => tok.class == Class::RawText,
            "ClauseText" => tok.class == Class::ClauseText,
            "Newline" => tok.class == Class::Newline,
            other => panic!("unknown token class {other}"),
        },
        _ => unreachable!("only terminals are matched"),
    }
}

// ------------------------------------------------------------ the machine

/// Computes, for a rule at a position, every position a derivation of the
/// rule can end at; memoised per (rule, position), so that the full meaning
/// of alternation and repetition costs polynomial time.
struct Machine<'g> {
    grammar: &'g Grammar,
    tokens: Vec<Tok>,
    memo: HashMap<(usize, usize), Rc<Vec<usize>>>,
    active: Vec<(usize, usize)>,
    furthest: usize,
    expected: BTreeSet<String>,
}

impl<'g> Machine<'g> {
    fn new(grammar: &'g Grammar, tokens: Vec<Tok>) -> Machine<'g> {
        Machine {
            grammar,
            tokens,
            memo: HashMap::new(),
            active: Vec::new(),
            furthest: 0,
            expected: BTreeSet::new(),
        }
    }

    fn accepts(&mut self, start: &str) -> bool {
        let rule = self.grammar.rule(start);
        let ends = self.ends(&Node::Rule(rule), 0);
        ends.contains(&self.tokens.len())
    }

    fn ends(&mut self, node: &Node, pos: usize) -> Vec<usize> {
        match node {
            Node::Literal(_) | Node::Class(_) => {
                if pos < self.tokens.len() && terminal_matches(&self.tokens[pos], node) {
                    vec![pos + 1]
                } else {
                    self.miss(pos, node);
                    Vec::new()
                }
            }
            Node::Rule(index) => {
                if let Some(found) = self.memo.get(&(*index, pos)) {
                    return found.to_vec();
                }
                assert!(
                    !self.active.contains(&(*index, pos)),
                    "rule `{}` is left-recursive",
                    self.grammar.names[*index]
                );
                self.active.push((*index, pos));
                let grammar = self.grammar;
                let result = self.ends(&grammar.bodies[*index], pos);
                self.active.pop();
                self.memo.insert((*index, pos), Rc::new(result.clone()));
                result
            }
            Node::Sequence(items) => {
                let mut current = vec![pos];
                for item in items {
                    let mut next = Vec::new();
                    for &at in &current {
                        for end in self.ends(item, at) {
                            if !next.contains(&end) {
                                next.push(end);
                            }
                        }
                    }
                    if next.is_empty() {
                        return next;
                    }
                    next.sort_unstable();
                    current = next;
                }
                current
            }
            Node::Choice(alternatives) => {
                let mut all = Vec::new();
                for alternative in alternatives {
                    for end in self.ends(alternative, pos) {
                        if !all.contains(&end) {
                            all.push(end);
                        }
                    }
                }
                all.sort_unstable();
                all
            }
            Node::Optional(inner) => {
                let mut all = self.ends(inner, pos);
                if !all.contains(&pos) {
                    all.push(pos);
                    all.sort_unstable();
                }
                all
            }
            Node::Repeated(inner) => {
                let mut all = vec![pos];
                let mut frontier = vec![pos];
                while !frontier.is_empty() {
                    let mut next = Vec::new();
                    for &at in &frontier {
                        for end in self.ends(inner, at) {
                            if !all.contains(&end) && !next.contains(&end) {
                                next.push(end);
                            }
                        }
                    }
                    all.extend(next.iter().copied());
                    frontier = next;
                }
                all.sort_unstable();
                all
            }
            Node::Except(left, right) => {
                let excluded = self.ends(right, pos);
                self.ends(left, pos)
                    .into_iter()
                    .filter(|end| !excluded.contains(end))
                    .collect()
            }
        }
    }

    /// Remembers the furthest token a terminal failed on, and what was
    /// expected there, for the report of a rejected program.
    fn miss(&mut self, pos: usize, node: &Node) {
        if pos > self.furthest {
            self.furthest = pos;
            self.expected.clear();
        }
        if pos == self.furthest {
            self.expected.insert(match node {
                Node::Literal(text) => format!("`{text}`"),
                Node::Class(class) => class.to_string(),
                _ => unreachable!(),
            });
        }
    }

    fn report(&self, file: &SourceFile) -> String {
        let (where_, offset) = match self.tokens.get(self.furthest) {
            Some(tok) if tok.class == Class::Newline => {
                ("the end of the line".to_string(), tok.span.start)
            }
            Some(tok) => (format!("`{}`", tok.text), tok.span.start),
            None => ("the end of the file".to_string(), file.text.len()),
        };
        let position = file.position(offset);
        let expected: Vec<String> = self.expected.iter().cloned().collect();
        format!(
            "{}:{}:{}: the grammar stops at {where_}; it expected one of {}",
            file.name,
            position.line,
            position.column,
            expected.join(", ")
        )
    }
}

// ------------------------------------------------------------ the verdicts

#[derive(Debug, PartialEq)]
enum Verdict {
    Accepted,
    Rejected,
    /// The lexer reported something but produced whole tokens: a matter of
    /// the lexical level, outside the grammar.
    Lexical,
}

fn parser_verdict(source: &str, declarations: bool) -> Verdict {
    let lexed = lex(source);
    let parsed = if declarations {
        parse_declarations(source)
    } else {
        parse(source)
    };
    let broken = lexed
        .tokens
        .iter()
        .any(|token| token.kind == TokenKind::Error);
    if broken || parsed.diagnostics.len() > lexed.diagnostics.len() {
        Verdict::Rejected
    } else if lexed.diagnostics.is_empty() {
        Verdict::Accepted
    } else {
        Verdict::Lexical
    }
}

/// Whether the grammar accepts the program from a start rule, with the
/// report of where it stopped when it does not.
fn grammar_verdict(grammar: &Grammar, file: &SourceFile, start: &str) -> (bool, String) {
    let tokens = stream(&file.text, &lex(&file.text).tokens);
    let mut machine = Machine::new(grammar, tokens);
    let accepted = machine.accepts(start);
    let report = if accepted {
        String::new()
    } else {
        machine.report(file)
    };
    (accepted, report)
}

/// Every hole of every text literal holds one `Expression`.
fn holes_are_expressions(grammar: &Grammar, file: &SourceFile) -> Vec<String> {
    let mut problems = Vec::new();
    for token in lex(&file.text).tokens {
        let TokenKind::Text { parts, .. } = token.kind else {
            continue;
        };
        for part in parts {
            let TextPart::Hole(hole) = part else {
                continue;
            };
            let tokens: Vec<Tok> = hole
                .iter()
                .map(|token| classify(token, &file.text))
                .collect();
            let mut machine = Machine::new(grammar, tokens);
            if !machine.accepts("Expression") {
                problems.push(machine.report(file));
            }
        }
    }
    problems
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn grammar() -> Grammar {
    read_grammar(
        &std::fs::read_to_string(root().join("docs/grammar.ebnf")).expect("docs/grammar.ebnf"),
    )
}

fn programs(directory: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(root().join(directory))
        .unwrap_or_else(|_| panic!("{directory}"))
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("ry"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "{directory} is empty");
    files
}

fn source_file(path: &PathBuf) -> SourceFile {
    let text = std::fs::read_to_string(path).expect("read");
    SourceFile::new(
        path.strip_prefix(root())
            .unwrap_or(path)
            .display()
            .to_string()
            .replace('\\', "/"),
        text,
    )
}

// ------------------------------------------------------------ the tests

#[test]
fn the_grammar_file_is_well_formed() {
    let grammar = grammar();
    assert!(grammar.names.len() >= 60, "{} rules", grammar.names.len());
    let reachable: BTreeSet<usize> = grammar
        .reachable("Module")
        .union(&grammar.reachable("Declarations"))
        .copied()
        .collect();
    let unreachable: Vec<&String> = grammar
        .names
        .iter()
        .enumerate()
        .filter(|(index, _)| !reachable.contains(index))
        .map(|(_, name)| name)
        .collect();
    assert!(
        unreachable.is_empty(),
        "rules no start rule reaches: {unreachable:?}"
    );
    // the parser's continuation words are the ones the sketch lists
    assert_eq!(CONTINUATION_WORDS.len(), 18);
}

#[test]
fn every_corpus_program_is_in_the_grammar() {
    let grammar = grammar();
    let mut report = String::new();
    for path in programs("examples") {
        let file = source_file(&path);
        assert_eq!(
            parser_verdict(&file.text, false),
            Verdict::Accepted,
            "{}",
            file.name
        );
        let (accepted, why) = grammar_verdict(&grammar, &file, "Module");
        if !accepted {
            report.push_str(&why);
            report.push('\n');
        }
        for problem in holes_are_expressions(&grammar, &file) {
            report.push_str(&problem);
            report.push('\n');
        }
    }
    assert!(report.is_empty(), "{report}");
}

#[test]
fn every_library_declaration_is_in_the_grammar() {
    let grammar = grammar();
    let mut report = String::new();
    for path in programs("library/std") {
        let file = source_file(&path);
        assert_eq!(
            parser_verdict(&file.text, true),
            Verdict::Accepted,
            "{}",
            file.name
        );
        let (accepted, why) = grammar_verdict(&grammar, &file, "Declarations");
        if !accepted {
            report.push_str(&why);
            report.push('\n');
        }
    }
    assert!(report.is_empty(), "{report}");
}

#[test]
fn every_conformance_program_gets_the_parser_verdict() {
    let grammar = grammar();
    let mut report = String::new();
    let mut rejected = 0;
    for path in programs("tests/conformance/programs") {
        let file = source_file(&path);
        let verdict = parser_verdict(&file.text, false);
        if verdict == Verdict::Lexical {
            continue;
        }
        let (accepted, why) = grammar_verdict(&grammar, &file, "Module");
        match verdict {
            Verdict::Accepted if !accepted => {
                report.push_str(&format!(
                    "the parser accepts, the grammar does not: {why}\n"
                ));
            }
            Verdict::Rejected if accepted => {
                report.push_str(&format!(
                    "{}: the parser rejects, the grammar accepts\n",
                    file.name
                ));
            }
            Verdict::Rejected => rejected += 1,
            _ => {}
        }
    }
    assert!(report.is_empty(), "{report}");
    assert!(rejected >= 2, "{rejected} programs with parse errors");
}

/// Spellings the corpus does not use and both the parser and the grammar
/// accept.
const ACCEPTED: &[(&str, &str)] = &[
    (
        "a line break inside brackets before a dot or a call's parenthesis",
        "module corner\n\nfunction go(items: List of Integer) returns Integer\n  let total be items.append(\n    1\n  ).length()\n  console.print(\n    items\n      .length()\n      .to_text()\n  )\n  return total\nend\n",
    ),
    (
        "a line break after a comma",
        "module corner\n\nimport accounts.models exposing User,\n  UserId\n\npublic function main()\n  needs console,\n    network.http(\"api.example.com\") at most 60 per minute,\n    filesystem.read(\"secrets\") only to network.http(\"api.example.com\")\n  for any Item,\n    Other where Item can Compare\n  purpose: Run.\n\n  let renamed be found with name: \"x\",\n    age: 2\n  let pairs be for each left in lefts,\n    right in rights collect left + right\n  console.print(renamed)\nend\n\npublic type User\n  purpose: An account.\n  has name: Text\n  has age: Integer\n  can Compare by name,\n    age\nend\n\npublic type Pair of Left,\n  Right\n  purpose: Two.\n  has left: Left\n  has right: Right\nend\n",
    ),
    (
        "a method named by a reserved word",
        "module corner\n\npublic type Stack\n  purpose: A stack.\n  has items: List of Integer\nend\n\npublic function first(self: Stack) returns maybe Integer\n  purpose: The top.\n  return self.items.first()\nend\n\npublic function set(self: Stack, item: Integer) returns Stack\n  purpose: Push.\n  return self with items: self.items.append(item)\nend\n",
    ),
    (
        "if and match as expressions, typed and raw patterns",
        "module corner\n\nfunction label(amount: Integer, handler: maybe Integer, text: Text) returns Text\n  let short be if amount is 0 then \"none\" otherwise \"some\" end\n  let long be\n    if amount is 0 then\n      \"none\"\n    otherwise if amount is 1 then\n      \"one\"\n    otherwise\n      \"many\"\n    end\n  let kind be match handler\n    when some(value: Integer) then \"int {value}\"\n    when nothing then \"none\"\n  end\n  let raw_kind be\n    match text\n      when raw \"a\" then \"a\"\n      when \"b\" then \"b\"\n      otherwise\n        \"other\"\n    end\n  match handler\n    when callback: function(Integer) returns Integer then return callback(1)\n    otherwise return 0\n  end\n  return short + long + kind + raw_kind\nend\n",
    ),
    (
        "examples with is, with fails with, and with both",
        "module corner\n\npublic function go(value: Integer) returns Integer or fails with Oops\n  purpose: Go.\n  example: go(1) is 2\n  example: go(0)\n    fails with Oops(detail: \"zero\")\n  example: go(2) is 3 fails with Oops\n\n  if value is 0 then fail with Oops(detail: \"zero\") end\n  return value + 1\nend\n\npublic type Oops\n  purpose: An error.\n  has detail: Text\nend\n",
    ),
    (
        "signature clauses on their own lines, budgets, guards and function types",
        "module corner\n\npublic function main()\n  or fails with AppError\n  needs console, network.http(\"api.example.com\") at most 60 per minute, filesystem.read(\"secrets\") only to network.http(\"api.example.com\") or console\n  purpose: Run.\n  tags: demo, corner\n  see also: retry\n  deprecated: since 2.0, replaced by main_two\n  expose as tool\n\n  retry(action: fetch, attempts: 3)\nend\n\nfunction retry(action: function(Text) returns Text or fails with HttpError needs network.http, attempts: Integer)\n  returns Text\n  or fails with HttpError\n  needs network.http\n  for any Item, Other where Item can Compare and Other can ToText\n  return action(\"x\")\nend\n",
    ),
    (
        "queries and loops in every form",
        "module corner\n\nfunction go(users: List of Integer, orders: List of Integer, urls: List of Text) returns Integer\n  let emails be\n    for each user in users\n    where user is at least 18\n    sorted by user descending\n    collect user\n  let by_country be for each user in users group by user collect user\n  let totals be for each user in users group by user\n  let revenue be for each order in orders where order is greater than 0 sum order\n  let paid_count be for each order in orders count\n  let oldest be for each order in orders sorted by order first\n  let all_shipped be for each order in orders all order is 1\n  let any_refund be for each order in orders any order is 2\n  let skus be for each order in orders, user in users collect order + user\n  let pages be for each url in urls concurrently within time.seconds(5) collect web.get(url) otherwise fail\n  for each user in users where user is 1 sorted by user\n    console.print(user)\n  end\n  for each index from 0 to users.length() - 1 by 2\n    console.print(index)\n  end\n  for each key, value in {\"a\": 1}\n    console.print(key)\n  end\n  for each step in (from 1 to 3)\n    console.print(step)\n  end\n  return emails.length() + by_country.length() + totals.length() + revenue + paid_count\nend\n",
    ),
    (
        "statements, outcomes and continuation lines",
        "module corner\n\nfunction go(items: List of Integer, flag: Boolean) returns Integer\n  let mutable total be 0\n  change total to\n    total + 1\n  let found be items.first() otherwise return 0\n  let other be items.last() otherwise fail\n  let third be items.at(2) otherwise fail with Oops(detail: \"x\")\n  let fourth be items.at(3) otherwise crash with \"unreachable\"\n  repeat until total is at least 3\n    change total to total + 1\n    let next be items.at(total) otherwise break\n    let after be items.at(next) otherwise continue\n    if flag then break end\n  end\n  run concurrently within time.seconds(5)\n    let users be accounts.fetch_all() otherwise fail\n  end\n  run concurrently\n    ignore console.print(\"x\")\n  end\n  check total is 3\n  if flag\n    and total is 3\n    or not flag then\n    return\n  end\n  let renamed be found\n    with name: \"x\", age: 2\n  fail\nend\n",
    ),
    (
        "types, abilities, constants and tests",
        "module corner\n\npublic type User\n  purpose: An account.\n  has name: Text\n  has age: Integer where age is at least 0\n  has kind: Text as \"type\"\n  has email: maybe Email\n  can Compare by name, age\n  can ToJson\nend\n\npublic type Shape is one of\n  purpose: A figure.\n  Circle(radius: Decimal where radius is greater than 0)\n  Rectangle(\n    width: Decimal,\n    height: Decimal\n  )\n  Point\n  can ToText\nend\n\npublic type Email is Text where value.matches(raw \"^[^@]+@[^@]+$\")\n  purpose: An address.\n\npublic type Pair of Left, Right\n  purpose: Two values.\n  has left: Left\n  has right: Right\nend\n\npublic ability Printable where self can ToText and self can Hash\n  purpose: Prints.\n  function print(self) returns Text\n  function size(self, other: Self) returns Integer\nend\n\npublic ability Iterable of Item\n  purpose: Walks.\n  function to_list(self) returns List of Item\nend\n\nability Iterable of User for Pair of User, User\n  for any Left\n  function to_list(self) returns List of User\n    return [self.left, self.right]\n  end\nend\n\npublic let limit: Integer be\n  3\n  purpose: The limit.\n\ntest \"a test\" needs filesystem.read(\"data\") replays \"recordings/a.json\"\n  check limit is 3\nend\n",
    ),
];

/// Spellings both reject: the parser with the fix named here, the grammar
/// by having no derivation.
const REJECTED: &[(&str, &str, &str)] = &[
    (
        "a trailing comma in a list",
        "module corner\n\nfunction go() returns Integer\n  let items be [1, 2,]\n  return items.length()\nend\n",
        "drop the comma",
    ),
    (
        "a trailing comma across lines",
        "module corner\n\nfunction go() returns Integer\n  let items be [\n    1,\n    2,\n  ]\n  return items.length()\nend\n",
        "drop the comma",
    ),
    (
        "a trailing comma in arguments",
        "module corner\n\nfunction go() returns Integer\n  return add(left: 1, right: 2,)\nend\n",
        "drop the comma",
    ),
    (
        "a trailing comma in parameters",
        "module corner\n\nfunction go(left: Integer,) returns Integer\n  return left\nend\n",
        "drop the comma",
    ),
    (
        "a trailing comma in a map",
        "module corner\n\nfunction go() returns Integer\n  let map be {\"a\": 1,}\n  return map.length()\nend\n",
        "drop the comma",
    ),
    (
        "a trailing comma in a pattern",
        "module corner\n\nfunction go(shape: Shape) returns Integer\n  match shape\n    when Circle(radius,) then return radius\n    otherwise return 0\n  end\nend\n",
        "drop the comma",
    ),
    (
        "a trailing comma in a function type",
        "module corner\n\nfunction go(action: function(Integer,) returns Integer) returns Integer\n  return action(1)\nend\n",
        "drop the comma",
    ),
    (
        "a variant with empty parentheses",
        "module corner\n\ntype Shape is one of\n  Circle(radius: Decimal)\n  Point()\nend\n",
        "write `Point` without parentheses",
    ),
    (
        "a pattern with empty parentheses",
        "module corner\n\nfunction go(shape: Shape) returns Integer\n  match shape\n    when Circle() then return 1\n    otherwise return 0\n  end\nend\n",
        "when Circle then",
    ),
    (
        "a space between a dot and a member",
        "module corner\n\nfunction go(user: User) returns Text\n  return user. name\nend\n",
        "directly after the dot",
    ),
    (
        "a space between a dot and a module path segment",
        "module corner\n\nimport std. console\n\nfunction go()\n  console.print(\"x\")\nend\n",
        "directly after the dot",
    ),
    (
        "otherwise before the last arm of a match expression",
        "module corner\n\nfunction go(items: List of Integer) returns Text\n  let label be match items.first()\n    otherwise \"none\"\n    when some(value) then \"some\"\n  end\n  return label\nend\n",
        "write the `otherwise` arm last",
    ),
    (
        "a head and end on one line",
        "module corner\n\nfunction go() returns Integer end\n",
        "write `end` on its own line",
    ),
    (
        "a variant and end on one line",
        "module corner\n\ntype Shape is one of\n  Circle(radius: Decimal)\n  Point end\n",
        "write `end` on its own line",
    ),
    (
        "a statement after a head on its line",
        "module corner\n\nfunction go() returns Integer\n  repeat until done change total to 1\n  end\n  return 1\nend\n",
        "one statement per line",
    ),
    (
        "a hole in a test name",
        "module corner\n\ntest \"a {name}\"\n  check true\nend\n",
        "without a hole",
    ),
    (
        "a hole in a capability scope",
        "module corner\n\nfunction go() needs filesystem.read(\"data/{year}\")\n  console.print(\"x\")\nend\n",
        "without a hole",
    ),
    (
        "a hole in an external name",
        "module corner\n\ntype User\n  has kind: Text as \"ty{pe}\"\nend\n",
        "without a hole",
    ),
    (
        "a hole in a recording's path",
        "module corner\n\ntest \"a\" replays \"recordings/{name}.json\"\n  check true\nend\n",
        "without a hole",
    ),
    (
        "public on a method",
        "module corner\n\npublic ability Sized\n  purpose: Has a size.\n  public function size(self) returns Integer\nend\n",
        "drop `public`",
    ),
    (
        "a range after in",
        "module corner\n\nfunction go() returns Integer\n  for each step in from 1 to 3\n    console.print(step)\n  end\n  return 1\nend\n",
        "drop `in`",
    ),
    (
        "a clause with a space before its colon",
        "module corner\n  purpose : A module.\n\nfunction go()\n  console.print(\"x\")\nend\n",
        "directly after the word",
    ),
    (
        "a phrase as a method name",
        "module corner\n\nfunction is not(self: Text) returns Boolean\n  return true\nend\n",
        "function name",
    ),
];

#[test]
fn the_corners_agree_with_the_parser() {
    let grammar = grammar();
    let mut report = String::new();
    for (label, source) in ACCEPTED {
        let file = SourceFile::new(*label, *source);
        let parsed = parse(source);
        if !parsed.diagnostics.is_empty() {
            report.push_str(&format!(
                "{label}: the parser rejects:\n{}",
                renyi_syntax::diagnostics::render_text(&file, &parsed.diagnostics)
            ));
        }
        let (accepted, why) = grammar_verdict(&grammar, &file, "Module");
        if !accepted {
            report.push_str(&format!("{why}\n"));
        }
    }
    for (label, source, fix) in REJECTED {
        let file = SourceFile::new(*label, *source);
        let parsed = parse(source);
        let fixes: Vec<&str> = parsed
            .diagnostics
            .iter()
            .map(|d| d.fix.as_deref().unwrap_or(""))
            .collect();
        if parsed.diagnostics.is_empty() {
            report.push_str(&format!("{label}: the parser accepts\n"));
        } else if !fixes.iter().any(|f| f.contains(fix)) {
            report.push_str(&format!(
                "{label}: no fix says {fix:?}:\n{}",
                renyi_syntax::diagnostics::render_text(&file, &parsed.diagnostics)
            ));
        } else if parsed.diagnostics.iter().any(|d| d.fix.is_none()) {
            report.push_str(&format!("{label}: a diagnostic without a fix\n"));
        }
        let (accepted, _) = grammar_verdict(&grammar, &file, "Module");
        if accepted {
            report.push_str(&format!("{label}: the grammar accepts\n"));
        }
    }
    assert!(report.is_empty(), "{report}");
}
