//! The parser: tokens to the AST of `ast.rs`, following the clause grammar of
//! the syntax sketch.
//!
//! Line structure matters. A statement ends at a line break unless a bracket
//! is open, or the next line is indented deeper than the line that started the
//! statement and begins with a continuation word (`otherwise`, `where`,
//! `sorted by`, `group by`, `collect`, `sum`, `count`, `first`, `any`, `all`,
//! `returns`, `or fails with`, `needs`, `for any`, `with`, `and`, `or`). The
//! expression after `be` or `return` may start on the next, deeper line, and
//! an `example:` clause continues on any deeper line.

use crate::ast::*;
use crate::diagnostics::Diagnostic;
use crate::lexer::lex;
use crate::span::Span;
use crate::token::{TextPart, Token, TokenKind, Word};

#[derive(Debug)]
pub struct Parsed {
    pub module: Module,
    pub diagnostics: Vec<Diagnostic>,
}

/// Lex and parse one module.
pub fn parse(source: &str) -> Parsed {
    parse_with(source, false)
}

/// Lex and parse a declaration file: a module whose functions have no
/// bodies, as the standard library declares its types and signatures.
pub fn parse_declarations(source: &str) -> Parsed {
    parse_with(source, true)
}

fn parse_with(source: &str, declarations: bool) -> Parsed {
    let lexed = lex(source);
    let mut comments = Vec::new();
    let tokens: Vec<Token> = lexed
        .tokens
        .into_iter()
        .filter(|token| {
            if token.kind == TokenKind::Comment {
                comments.push(token.span);
                false
            } else {
                true
            }
        })
        .collect();
    let mut parser = Parser {
        src: source,
        declarations,
        tokens,
        pos: 0,
        diagnostics: lexed.diagnostics,
        nesting: 0,
        continuation: Vec::new(),
    };
    let mut module = parser.module();
    module.comments = comments;
    Parsed {
        module,
        diagnostics: parser.diagnostics,
    }
}

/// Parse a run of tokens as one expression (used for interpolation holes).
fn parse_hole(src: &str, tokens: Vec<Token>, diagnostics: &mut Vec<Diagnostic>) -> Expr {
    let span = tokens
        .first()
        .map(|first| first.span.join(tokens.last().unwrap().span))
        .unwrap_or_default();
    let mut tokens = tokens;
    tokens.push(Token::new(TokenKind::Eof, Span::new(span.end, span.end)));
    let mut parser = Parser {
        src,
        declarations: false,
        tokens,
        pos: 0,
        diagnostics: Vec::new(),
        nesting: 1,
        continuation: Vec::new(),
    };
    let expr = parser.expr().unwrap_or(Expr {
        kind: ExprKind::Nothing,
        span,
    });
    if !parser.at(&TokenKind::Eof) {
        let span = parser.peek().span;
        parser.error("hole-syntax", "a hole holds exactly one expression", span);
    }
    diagnostics.extend(parser.diagnostics);
    expr
}

struct Continuation {
    column: usize,
    any_word: bool,
}

struct Parser<'s> {
    src: &'s str,
    /// Functions have no bodies (a library declaration file).
    declarations: bool,
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    nesting: usize,
    continuation: Vec<Continuation>,
}

const CONTINUATION_WORDS: &[Word] = &[
    Word::Otherwise,
    Word::Where,
    Word::SortedBy,
    Word::GroupBy,
    Word::Collect,
    Word::Sum,
    Word::Count,
    Word::First,
    Word::Any,
    Word::All,
    Word::Returns,
    Word::OrFailsWith,
    Word::Needs,
    Word::ForAny,
    Word::With,
    Word::And,
    Word::Or,
];

type ParseResult<T> = Result<T, ()>;

impl<'s> Parser<'s> {
    // ------------------------------------------------------------ cursor

    fn column(&self, offset: usize) -> usize {
        let line_start = self.src[..offset].rfind('\n').map_or(0, |index| index + 1);
        self.src[line_start..offset].chars().count()
    }

    fn raw(&self) -> &Token {
        &self.tokens[self.pos]
    }

    /// The next significant token, skipping line breaks that the layout rules
    /// make insignificant (open brackets, continuation lines).
    fn peek(&mut self) -> &Token {
        loop {
            if self.tokens[self.pos].kind != TokenKind::Newline {
                return &self.tokens[self.pos];
            }
            let mut next = self.pos;
            while self.tokens[next].kind == TokenKind::Newline {
                next += 1;
            }
            let token = &self.tokens[next];
            if self.nesting > 0 || self.continues(token) {
                self.pos = next;
            } else {
                return &self.tokens[self.pos];
            }
        }
    }

    fn continues(&self, token: &Token) -> bool {
        let Some(context) = self.continuation.last() else {
            return false;
        };
        if token.kind == TokenKind::Eof {
            return false;
        }
        let deeper = self.column(token.span.start) > context.column;
        let continuation_word =
            matches!(&token.kind, TokenKind::Word(word) if CONTINUATION_WORDS.contains(word));
        deeper && (context.any_word || continuation_word)
    }

    /// Look past the next significant token without moving.
    fn peek_second(&mut self) -> &Token {
        self.peek();
        let mut index = self.pos + 1;
        if self.nesting > 0 {
            while self.tokens[index].kind == TokenKind::Newline {
                index += 1;
            }
        }
        &self.tokens[index]
    }

    fn at(&mut self, kind: &TokenKind) -> bool {
        &self.peek().kind == kind
    }

    fn at_word(&mut self, word: Word) -> bool {
        self.peek().kind == TokenKind::Word(word)
    }

    fn advance(&mut self) -> Token {
        self.peek();
        let token = self.tokens[self.pos].clone();
        if token.kind != TokenKind::Eof {
            self.pos += 1;
        }
        token
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn eat_word(&mut self, word: Word) -> bool {
        self.eat(&TokenKind::Word(word))
    }

    fn expect(&mut self, kind: &TokenKind, what: &str) -> ParseResult<Token> {
        if self.at(kind) {
            Ok(self.advance())
        } else {
            let span = self.peek().span;
            let found = self.describe(span);
            self.error("expected", format!("expected {what}, found {found}"), span);
            Err(())
        }
    }

    fn expect_word(&mut self, word: Word) -> ParseResult<Token> {
        self.expect(&TokenKind::Word(word), &format!("`{}`", word.spelling()))
    }

    fn describe(&self, span: Span) -> String {
        let token = &self.tokens[self.pos];
        match &token.kind {
            TokenKind::Newline => "the end of the line".to_string(),
            TokenKind::Eof => "the end of the file".to_string(),
            _ => format!("`{}`", &self.src[span.start..span.end]),
        }
    }

    fn error(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span));
    }

    /// Skip to the start of the next line, after an error.
    fn recover(&mut self) {
        while !matches!(self.raw().kind, TokenKind::Newline | TokenKind::Eof) {
            self.pos += 1;
        }
        self.nesting = 0;
    }

    /// Consume line breaks at a point where statements may start.
    fn skip_newlines(&mut self) {
        while self.raw().kind == TokenKind::Newline {
            self.pos += 1;
        }
    }

    /// A statement or clause ends here: a line break, the end of the file, or a
    /// block delimiter on the same line (`if done then break end`).
    fn end_of_statement(&mut self) -> ParseResult<()> {
        match &self.raw().kind {
            TokenKind::Newline => {
                self.skip_newlines();
                Ok(())
            }
            TokenKind::Eof => Ok(()),
            TokenKind::Word(Word::End)
            | TokenKind::Word(Word::Otherwise)
            | TokenKind::Word(Word::When) => Ok(()),
            _ => {
                let span = self.raw().span;
                let found = self.describe(span);
                self.error(
                    "expected",
                    format!("expected the end of the line, found {found}"),
                    span,
                );
                Err(())
            }
        }
    }

    fn with_continuation<T>(
        &mut self,
        column: usize,
        any_word: bool,
        body: impl FnOnce(&mut Self) -> T,
    ) -> T {
        self.continuation.push(Continuation { column, any_word });
        let result = body(self);
        self.continuation.pop();
        result
    }

    /// After `be` or `return`: the expression may start on the next, deeper line.
    fn allow_next_line(&mut self, column: usize) {
        if self.raw().kind == TokenKind::Newline {
            let mut next = self.pos;
            while self.tokens[next].kind == TokenKind::Newline {
                next += 1;
            }
            if self.tokens[next].kind != TokenKind::Eof
                && self.column(self.tokens[next].span.start) > column
            {
                self.pos = next;
            }
        }
    }

    fn text_of(&self, token: &Token) -> String {
        token.text(self.src).to_string()
    }

    fn start_column(&mut self) -> usize {
        let start = self.peek().span.start;
        self.column(start)
    }

    // ------------------------------------------------------------ module

    fn module(&mut self) -> Module {
        let start = self.peek().span.start;
        let mut module = Module {
            name: Vec::new(),
            docs: Docs::default(),
            imports: Vec::new(),
            items: Vec::new(),
            comments: Vec::new(),
            span: Span::default(),
        };
        self.skip_newlines();
        if self.at_word(Word::Module) {
            self.advance();
            if let Ok(path) = self.dotted_name() {
                module.name = path;
            }
            let _ = self.end_of_statement();
            module.docs = self.doc_clauses(0);
        } else {
            let span = self.peek().span;
            self.error("module-header", "a file starts with `module name`", span);
        }
        loop {
            self.skip_newlines();
            if self.at(&TokenKind::Eof) {
                break;
            }
            if self.at_word(Word::Import) {
                if let Ok(import) = self.import() {
                    module.imports.push(import);
                } else {
                    self.recover();
                }
                continue;
            }
            match self.item() {
                Ok(item) => module.items.push(item),
                Err(()) => {
                    self.recover();
                    self.skip_newlines();
                    self.skip_to_next_item();
                }
            }
        }
        module.span = Span::new(start, self.src.len());
        module
    }

    /// After a failed item, skip lines until one starts a new top-level item.
    fn skip_to_next_item(&mut self) {
        loop {
            match &self.raw().kind {
                TokenKind::Eof => return,
                TokenKind::Word(word)
                    if matches!(
                        word,
                        Word::Public
                            | Word::Function
                            | Word::Type
                            | Word::Ability
                            | Word::Test
                            | Word::Let
                            | Word::Import
                    ) && self.column(self.raw().span.start) == 0 =>
                {
                    return
                }
                _ => {
                    self.recover();
                    self.skip_newlines();
                }
            }
        }
    }

    fn dotted_name(&mut self) -> ParseResult<Vec<Name>> {
        let mut path = vec![self.identifier("a module name")?];
        while self.eat(&TokenKind::Dot) {
            let token = self.advance();
            match token.kind {
                TokenKind::Member | TokenKind::Identifier => path.push(Name {
                    text: self.text_of(&token),
                    span: token.span,
                }),
                _ => {
                    self.error("expected", "expected a name after the dot", token.span);
                    return Err(());
                }
            }
        }
        Ok(path)
    }

    fn import(&mut self) -> ParseResult<Import> {
        let start = self.advance().span;
        let path = self.dotted_name()?;
        let mut alias = None;
        let mut exposing = Vec::new();
        if self.eat_word(Word::As) {
            alias = Some(self.identifier("an alias")?);
        }
        if self.eat_word(Word::Exposing) {
            exposing.push(self.type_name("a type or ability name")?);
            while self.eat(&TokenKind::Comma) {
                exposing.push(self.type_name("a type or ability name")?);
            }
        }
        let end = self.tokens[self.pos - 1].span;
        self.end_of_statement()?;
        Ok(Import {
            path,
            alias,
            exposing,
            span: start.join(end),
        })
    }

    /// Documentation clauses on the lines after a head, indented deeper than `column`.
    fn doc_clauses(&mut self, column: usize) -> Docs {
        let mut docs = Docs::default();
        loop {
            self.skip_newlines();
            let token = self.raw().clone();
            if self.column(token.span.start) <= column && column > 0 {
                break;
            }
            match token.kind {
                TokenKind::Word(
                    word @ (Word::Purpose | Word::Tags | Word::SeeAlso | Word::Deprecated),
                ) => {
                    if self.column(token.span.start) <= column && column == 0 && !docs_empty(&docs)
                    {
                        break;
                    }
                    self.advance();
                    let _ = self.expect(&TokenKind::Colon, "`:`");
                    let text = match self.raw().kind.clone() {
                        TokenKind::ClauseText(text) => {
                            self.advance();
                            text
                        }
                        _ => String::new(),
                    };
                    match word {
                        Word::Purpose => docs.purpose = Some(text),
                        Word::Tags => docs.tags = split_list(&text),
                        Word::SeeAlso => docs.see_also = split_list(&text),
                        _ => docs.deprecated = Some(text),
                    }
                    let _ = self.end_of_statement();
                }
                TokenKind::Word(Word::ExposeAsTool) => {
                    self.advance();
                    docs.expose_as_tool = true;
                    let _ = self.end_of_statement();
                }
                TokenKind::Word(Word::Example) => {
                    let example_column = self.column(token.span.start);
                    self.advance();
                    let _ = self.expect(&TokenKind::Colon, "`:`");
                    match self.with_continuation(example_column, true, |parser| {
                        parser.example(token.span.start)
                    }) {
                        Ok(example) => docs.examples.push(example),
                        Err(()) => self.recover(),
                    }
                    let _ = self.end_of_statement();
                }
                _ => break,
            }
        }
        docs
    }

    fn example(&mut self, start: usize) -> ParseResult<Example> {
        let expression = self.expr()?;
        let (expression, outcome) = match expression.kind {
            ExprKind::Binary {
                op: BinaryOp::Is,
                left,
                right,
            } => (*left, ExampleOutcome::Is(*right)),
            _ => {
                if self.eat_word(Word::Fails) {
                    self.expect_word(Word::With)?;
                    let pattern = self.pattern()?;
                    (expression, ExampleOutcome::FailsWith(pattern))
                } else {
                    let span = expression.span;
                    self.error(
                        "example-shape",
                        "an example is `expression is value` or `expression fails with Pattern`",
                        span,
                    );
                    return Err(());
                }
            }
        };
        let end = self.tokens[self.pos - 1].span;
        Ok(Example {
            expression,
            outcome,
            span: Span::new(start, end.end),
        })
    }

    // ------------------------------------------------------------ items

    fn item(&mut self) -> ParseResult<Item> {
        let start = self.peek().span.start;
        let public = self.eat_word(Word::Public);
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Word(Word::Function) => {
                let with_body = !self.declarations;
                self.function(public, start, with_body).map(Item::Function)
            }
            TokenKind::Word(Word::Type) => self.type_def(public, start).map(Item::Type),
            TokenKind::Word(Word::Ability) => self.ability(public, start),
            TokenKind::Word(Word::Let) => self.constant(public, start).map(Item::Constant),
            TokenKind::Word(Word::Test) if !public => self.test(start).map(Item::Test),
            _ => {
                let found = self.describe(token.span);
                self.error(
                    "expected",
                    format!("expected `function`, `type`, `ability`, `let` or `test` at the top level, found {found}"),
                    token.span,
                );
                Err(())
            }
        }
    }

    fn function(&mut self, public: bool, start: usize, with_body: bool) -> ParseResult<Function> {
        let head_column = self.column(start);
        self.expect_word(Word::Function)?;
        let name = self.method_or_function_name()?;
        let params = self.params()?;
        let (returns, fails, needs, type_params) =
            self.with_continuation(head_column, false, |parser| parser.signature_clauses())?;
        self.end_of_statement()?;
        let docs = self.doc_clauses(head_column);
        let body = if with_body {
            let block = self.block(&[Word::End])?;
            self.expect_word(Word::End)?;
            Some(block)
        } else {
            None
        };
        let end = self.tokens[self.pos - 1].span.end;
        if with_body {
            self.end_of_statement()?;
        }
        Ok(Function {
            public,
            name,
            params,
            returns,
            fails,
            needs,
            type_params,
            docs,
            body,
            span: Span::new(start, end),
        })
    }

    /// A function name. A method, whose first parameter is `self`, is only
    /// ever called after a dot, so like a member name it may be any word,
    /// reserved words included (`first`, `at`, `set`, `sum`, `repeat`).
    fn method_or_function_name(&mut self) -> ParseResult<Name> {
        let token = self.peek().clone();
        let is_method = matches!(token.kind, TokenKind::Word(_))
            && self.tokens.get(self.pos + 1).map(|t| &t.kind) == Some(&TokenKind::LeftParen)
            && self.tokens.get(self.pos + 2).map(|t| &t.kind)
                == Some(&TokenKind::Word(Word::SelfValue));
        if is_method {
            self.advance();
            return Ok(Name {
                text: self.src[token.span.start..token.span.end].to_string(),
                span: token.span,
            });
        }
        self.identifier("a function name")
    }

    fn params(&mut self) -> ParseResult<Vec<Param>> {
        self.expect(&TokenKind::LeftParen, "`(`")?;
        self.nesting += 1;
        let mut params = Vec::new();
        while !self.at(&TokenKind::RightParen) {
            let token = self.peek().clone();
            let param = match token.kind {
                TokenKind::Word(Word::SelfValue) => {
                    self.advance();
                    let name = Name {
                        text: "self".into(),
                        span: token.span,
                    };
                    let ty = if self.eat(&TokenKind::Colon) {
                        Some(self.type_()?)
                    } else {
                        None
                    };
                    let end = self.tokens[self.pos - 1].span;
                    Param {
                        name,
                        ty,
                        span: token.span.join(end),
                    }
                }
                _ => {
                    let name = self.identifier("a parameter name")?;
                    self.expect(&TokenKind::Colon, "`:` and the parameter type")?;
                    let ty = self.type_()?;
                    let span = name.span.join(ty.span());
                    Param {
                        name,
                        ty: Some(ty),
                        span,
                    }
                }
            };
            params.push(param);
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.nesting -= 1;
        self.expect(&TokenKind::RightParen, "`)`")?;
        Ok(params)
    }

    #[allow(clippy::type_complexity)]
    fn signature_clauses(
        &mut self,
    ) -> ParseResult<(Option<Type>, Vec<Type>, Vec<Capability>, Option<ForAny>)> {
        let mut returns = None;
        let mut fails = Vec::new();
        let mut needs = Vec::new();
        let mut type_params = None;
        if self.eat_word(Word::Returns) {
            returns = Some(self.type_()?);
        }
        if self.eat_word(Word::OrFailsWith) {
            fails.push(self.type_()?);
            while self.eat_word(Word::Or) {
                fails.push(self.type_()?);
            }
        }
        if self.eat_word(Word::Needs) {
            needs = self.capabilities()?;
        }
        if self.at_word(Word::ForAny) {
            type_params = Some(self.for_any()?);
        }
        Ok((returns, fails, needs, type_params))
    }

    fn capabilities(&mut self) -> ParseResult<Vec<Capability>> {
        let mut capabilities = Vec::new();
        loop {
            let start = self.peek().span;
            let (path, scope) = self.capability_path()?;
            let budget = if self.at_word(Word::AtMost) {
                Some(self.budget()?)
            } else {
                None
            };
            let mut only_to = Vec::new();
            if self.eat_word(Word::OnlyTo) {
                loop {
                    let sink_start = self.peek().span;
                    let (sink_path, sink_scope) = self.capability_path()?;
                    let sink_end = self.tokens[self.pos - 1].span;
                    only_to.push(Sink {
                        path: sink_path,
                        scope: sink_scope,
                        span: sink_start.join(sink_end),
                    });
                    if !self.eat_word(Word::Or) {
                        break;
                    }
                }
            }
            let end = self.tokens[self.pos - 1].span;
            capabilities.push(Capability {
                path,
                scope,
                budget,
                only_to,
                span: start.join(end),
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        Ok(capabilities)
    }

    /// `filesystem.read("data")`: a capability path with an optional scope.
    fn capability_path(&mut self) -> ParseResult<(Vec<Name>, Option<String>)> {
        let path = self.dotted_name()?;
        let mut scope = None;
        if self.eat(&TokenKind::LeftParen) {
            let token = self.advance();
            match token.kind {
                TokenKind::Text { parts, .. } => scope = Some(literal_text(&parts)),
                TokenKind::RawText(text) => scope = Some(text),
                _ => {
                    self.error(
                        "capability-scope",
                        "a capability scope is a text literal",
                        token.span,
                    );
                    return Err(());
                }
            }
            self.expect(&TokenKind::RightParen, "`)`")?;
        }
        Ok((path, scope))
    }

    /// `at most 60 per minute` (decision P2).
    fn budget(&mut self) -> ParseResult<Budget> {
        let start = self.expect_word(Word::AtMost)?.span;
        let count_token = self.advance();
        if count_token.kind != TokenKind::Integer {
            self.error(
                "expected",
                "expected a whole number after `at most`",
                count_token.span,
            );
            return Err(());
        }
        let count = self.text_of(&count_token);
        self.expect_word(Word::Per)?;
        let unit_token = self.advance();
        let unit = self.text_of(&unit_token);
        if !matches!(unit.as_str(), "second" | "minute" | "hour" | "day" | "run") {
            self.error(
                "expected",
                format!(
                    "a budget is per `second`, `minute`, `hour`, `day` or `run`, found `{unit}`"
                ),
                unit_token.span,
            );
            return Err(());
        }
        Ok(Budget {
            count,
            per: Name {
                text: unit,
                span: unit_token.span,
            },
            span: start.join(unit_token.span),
        })
    }

    fn for_any(&mut self) -> ParseResult<ForAny> {
        let start = self.expect_word(Word::ForAny)?.span;
        let mut params = vec![self.type_name("a type parameter")?];
        while self.eat(&TokenKind::Comma) {
            params.push(self.type_name("a type parameter")?);
        }
        let mut constraints = Vec::new();
        if self.eat_word(Word::Where) {
            loop {
                let param = self.type_name("a type parameter")?;
                self.expect_word(Word::Can)?;
                let ability = self.type_()?;
                constraints.push(Constraint { param, ability });
                if !self.eat_word(Word::And) {
                    break;
                }
            }
        }
        let end = self.tokens[self.pos - 1].span;
        Ok(ForAny {
            params,
            constraints,
            span: start.join(end),
        })
    }

    fn type_def(&mut self, public: bool, start: usize) -> ParseResult<TypeDef> {
        let head_column = self.column(start);
        self.expect_word(Word::Type)?;
        let name = self.type_name("a type name")?;
        let mut type_params = Vec::new();
        if self.eat_word(Word::Of) {
            type_params.push(self.type_name("a type parameter")?);
            while self.eat(&TokenKind::Comma) {
                type_params.push(self.type_name("a type parameter")?);
            }
        }
        if self.eat_word(Word::IsOneOf) {
            self.end_of_statement()?;
            let docs = self.doc_clauses(head_column);
            let mut variants = Vec::new();
            let mut derives = Vec::new();
            loop {
                self.skip_newlines();
                if self.at_word(Word::End) {
                    break;
                }
                if self.at_word(Word::Can) {
                    derives.push(self.derive()?);
                    continue;
                }
                let variant_name = self.type_name("a variant name")?;
                let fields = if self.at(&TokenKind::LeftParen) {
                    self.variant_fields()?
                } else {
                    Vec::new()
                };
                let end = self.tokens[self.pos - 1].span;
                variants.push(Variant {
                    name: variant_name.clone(),
                    fields,
                    span: variant_name.span.join(end),
                });
                self.end_of_statement()?;
            }
            self.expect_word(Word::End)?;
            let end = self.tokens[self.pos - 1].span.end;
            self.end_of_statement()?;
            return Ok(TypeDef {
                public,
                name,
                type_params,
                kind: TypeKind::Sum { variants, derives },
                docs,
                span: Span::new(start, end),
            });
        }
        if self.eat_word(Word::Is) {
            let (base, refinement) =
                self.with_continuation(head_column, false, |parser| -> ParseResult<_> {
                    let base = parser.type_()?;
                    let refinement = if parser.eat_word(Word::Where) {
                        Some(parser.expr()?)
                    } else {
                        None
                    };
                    Ok((base, refinement))
                })?;
            let end = self.tokens[self.pos - 1].span.end;
            self.end_of_statement()?;
            let docs = self.doc_clauses(head_column);
            return Ok(TypeDef {
                public,
                name,
                type_params,
                kind: TypeKind::Subtype { base, refinement },
                docs,
                span: Span::new(start, end),
            });
        }
        self.end_of_statement()?;
        let docs = self.doc_clauses(head_column);
        let mut fields = Vec::new();
        let mut derives = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_word(Word::End) {
                break;
            }
            if self.at_word(Word::Can) {
                derives.push(self.derive()?);
            } else if self.at_word(Word::Has) {
                let has_column = self.start_column();
                self.advance();
                let field =
                    self.with_continuation(has_column, false, |parser| parser.field(true))?;
                fields.push(field);
                self.end_of_statement()?;
            } else {
                let span = self.peek().span;
                let found = self.describe(span);
                self.error(
                    "expected",
                    format!("expected `has`, `can` or `end` in a type, found {found}"),
                    span,
                );
                return Err(());
            }
        }
        self.expect_word(Word::End)?;
        let end = self.tokens[self.pos - 1].span.end;
        self.end_of_statement()?;
        Ok(TypeDef {
            public,
            name,
            type_params,
            kind: TypeKind::Record { fields, derives },
            docs,
            span: Span::new(start, end),
        })
    }

    fn field(&mut self, allow_external_name: bool) -> ParseResult<Field> {
        let name = self.identifier("a field name")?;
        self.expect(&TokenKind::Colon, "`:` and the field type")?;
        let ty = self.type_()?;
        let refinement = if self.eat_word(Word::Where) {
            Some(self.expr()?)
        } else {
            None
        };
        let mut external_name = None;
        if allow_external_name && self.at_word(Word::As) {
            self.advance();
            let token = self.advance();
            match token.kind {
                TokenKind::Text { parts, .. } => external_name = Some(literal_text(&parts)),
                _ => {
                    self.error(
                        "external-name",
                        "the external name after `as` is a text literal",
                        token.span,
                    );
                    return Err(());
                }
            }
        }
        let end = self.tokens[self.pos - 1].span;
        Ok(Field {
            span: name.span.join(end),
            name,
            ty,
            refinement,
            external_name,
        })
    }

    fn variant_fields(&mut self) -> ParseResult<Vec<Field>> {
        self.expect(&TokenKind::LeftParen, "`(`")?;
        self.nesting += 1;
        let mut fields = Vec::new();
        while !self.at(&TokenKind::RightParen) {
            fields.push(self.field(false)?);
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.nesting -= 1;
        self.expect(&TokenKind::RightParen, "`)`")?;
        Ok(fields)
    }

    fn derive(&mut self) -> ParseResult<Derive> {
        let start = self.expect_word(Word::Can)?.span;
        let ability = self.type_name("an ability name")?;
        let mut by = Vec::new();
        if self.eat_word(Word::By) {
            by.push(self.identifier("a field name")?);
            while self.eat(&TokenKind::Comma) {
                by.push(self.identifier("a field name")?);
            }
        }
        let end = self.tokens[self.pos - 1].span;
        self.end_of_statement()?;
        Ok(Derive {
            ability,
            by,
            span: start.join(end),
        })
    }

    fn ability(&mut self, public: bool, start: usize) -> ParseResult<Item> {
        let head_column = self.column(start);
        self.expect_word(Word::Ability)?;
        let name = self.type_name("an ability name")?;
        let mut type_params = Vec::new();
        if self.eat_word(Word::Of) {
            type_params.push(self.type_name("a type parameter")?);
            while self.eat(&TokenKind::Comma) {
                type_params.push(self.type_name("a type parameter")?);
            }
        }
        if self.eat_word(Word::For) {
            // implementation: `ability Sized for Stack of Item`
            let target = self.type_()?;
            let ability_span = name.span;
            let ability = Type::Named {
                args: type_params
                    .iter()
                    .map(|param| Type::Named {
                        name: param.clone(),
                        args: Vec::new(),
                        span: param.span,
                    })
                    .collect(),
                name,
                span: ability_span,
            };
            let end_of_head = self.tokens[self.pos - 1].span;
            self.end_of_statement()?;
            self.skip_newlines();
            let for_any = if self.at_word(Word::ForAny) {
                let clause = self.for_any()?;
                self.end_of_statement()?;
                Some(clause)
            } else {
                None
            };
            let functions = self.ability_functions(true)?;
            self.expect_word(Word::End)?;
            let end = self.tokens[self.pos - 1].span.end;
            self.end_of_statement()?;
            if public {
                self.error(
                    "public-implementation",
                    "an implementation is never `public`; the ability and the type are",
                    Span::new(start, end_of_head.end),
                );
            }
            return Ok(Item::Implementation(AbilityImpl {
                ability,
                target,
                type_params: for_any,
                functions,
                span: Span::new(start, end),
            }));
        }
        let mut requirements = Vec::new();
        if self.eat_word(Word::Where) {
            loop {
                self.expect_word(Word::SelfValue)?;
                self.expect_word(Word::Can)?;
                requirements.push(self.type_()?);
                if !self.eat_word(Word::And) {
                    break;
                }
            }
        }
        self.end_of_statement()?;
        let docs = self.doc_clauses(head_column);
        let functions = self.ability_functions(false)?;
        self.expect_word(Word::End)?;
        let end = self.tokens[self.pos - 1].span.end;
        self.end_of_statement()?;
        Ok(Item::Ability(AbilityDecl {
            public,
            name,
            type_params,
            requirements,
            docs,
            functions,
            span: Span::new(start, end),
        }))
    }

    fn ability_functions(&mut self, with_bodies: bool) -> ParseResult<Vec<Function>> {
        let mut functions = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_word(Word::End) || self.at(&TokenKind::Eof) {
                break;
            }
            let start = self.peek().span.start;
            let public = self.eat_word(Word::Public);
            if !self.at_word(Word::Function) {
                let span = self.peek().span;
                let found = self.describe(span);
                self.error(
                    "expected",
                    format!("expected `function` or `end` in an ability, found {found}"),
                    span,
                );
                return Err(());
            }
            functions.push(self.function(public, start, with_bodies)?);
        }
        Ok(functions)
    }

    fn constant(&mut self, public: bool, start: usize) -> ParseResult<Constant> {
        let head_column = self.column(start);
        self.expect_word(Word::Let)?;
        let name = self.identifier("a constant name")?;
        self.expect(&TokenKind::Colon, "`:` and the type of the constant")?;
        let ty = self.type_()?;
        self.expect_word(Word::Be)?;
        let value = self.with_continuation(head_column, false, |parser| parser.expr())?;
        let end = self.tokens[self.pos - 1].span.end;
        self.end_of_statement()?;
        let docs = self.doc_clauses(head_column);
        Ok(Constant {
            public,
            name,
            ty,
            value,
            docs,
            span: Span::new(start, end),
        })
    }

    fn test(&mut self, start: usize) -> ParseResult<Test> {
        self.expect_word(Word::Test)?;
        let token = self.advance();
        let name = match token.kind {
            TokenKind::Text { parts, .. } => literal_text(&parts),
            _ => {
                self.error(
                    "expected",
                    "expected the test name as a text literal",
                    token.span,
                );
                return Err(());
            }
        };
        let needs = if self.eat_word(Word::Needs) {
            self.capabilities()?
        } else {
            Vec::new()
        };
        let replays = if self.eat_word(Word::Replays) {
            let token = self.advance();
            match token.kind {
                TokenKind::Text { parts, .. } => Some(literal_text(&parts)),
                _ => {
                    self.error(
                        "expected",
                        "expected the recording's path as a text literal after `replays`",
                        token.span,
                    );
                    return Err(());
                }
            }
        } else {
            None
        };
        self.end_of_statement()?;
        let body = self.block(&[Word::End])?;
        self.expect_word(Word::End)?;
        let end = self.tokens[self.pos - 1].span.end;
        self.end_of_statement()?;
        Ok(Test {
            name,
            needs,
            replays,
            body,
            span: Span::new(start, end),
        })
    }

    // ------------------------------------------------------------ types

    fn type_(&mut self) -> ParseResult<Type> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Word(Word::Maybe) => {
                self.advance();
                let inner = self.type_()?;
                let span = token.span.join(inner.span());
                Ok(Type::Maybe(Box::new(inner), span))
            }
            TokenKind::Word(Word::Function) => {
                self.advance();
                self.expect(&TokenKind::LeftParen, "`(`")?;
                self.nesting += 1;
                let mut params = Vec::new();
                while !self.at(&TokenKind::RightParen) {
                    params.push(self.type_()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.nesting -= 1;
                self.expect(&TokenKind::RightParen, "`)`")?;
                let returns = if self.eat_word(Word::Returns) {
                    Some(Box::new(self.type_()?))
                } else {
                    None
                };
                let mut fails = Vec::new();
                if self.eat_word(Word::OrFailsWith) {
                    fails.push(self.type_()?);
                    while self.eat_word(Word::Or) {
                        fails.push(self.type_()?);
                    }
                }
                let needs = if self.eat_word(Word::Needs) {
                    self.capabilities()?
                } else {
                    Vec::new()
                };
                let end = self.tokens[self.pos - 1].span;
                Ok(Type::Function {
                    params,
                    returns,
                    fails,
                    needs,
                    span: token.span.join(end),
                })
            }
            TokenKind::TypeName => {
                let name = self.type_name("a type")?;
                let mut args = Vec::new();
                if self.eat_word(Word::Of) {
                    args.push(self.type_()?);
                    if self.eat_word(Word::To) {
                        args.push(self.type_()?);
                    } else {
                        while self.at(&TokenKind::Comma) && self.type_follows_comma() {
                            self.advance();
                            args.push(self.type_()?);
                        }
                    }
                }
                let end = self.tokens[self.pos - 1].span;
                Ok(Type::Named {
                    span: name.span.join(end),
                    name,
                    args,
                })
            }
            _ => {
                let found = self.describe(token.span);
                self.error(
                    "expected",
                    format!("expected a type, found {found}"),
                    token.span,
                );
                Err(())
            }
        }
    }

    /// After a comma inside a type argument list: another type argument, or
    /// the next parameter or field of the enclosing list?
    fn type_follows_comma(&mut self) -> bool {
        matches!(
            self.peek_second().kind,
            TokenKind::TypeName | TokenKind::Word(Word::Maybe) | TokenKind::Word(Word::Function)
        )
    }

    fn identifier(&mut self, what: &str) -> ParseResult<Name> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Identifier => {
                self.advance();
                Ok(Name {
                    text: self.text_of(&token),
                    span: token.span,
                })
            }
            TokenKind::Word(word) if !word.is_phrase() => {
                self.advance();
                let spelling = word.spelling();
                self.diagnostics.push(
                    Diagnostic::error(
                        "reserved-word",
                        format!("`{spelling}` is a reserved word and cannot name {what}"),
                        token.span,
                    )
                    .with_fix(format!("rename it, for example to `{}_value`", spelling)),
                );
                Ok(Name {
                    text: spelling.to_string(),
                    span: token.span,
                })
            }
            _ => {
                let found = self.describe(token.span);
                self.error(
                    "expected",
                    format!("expected {what}, found {found}"),
                    token.span,
                );
                Err(())
            }
        }
    }

    fn type_name(&mut self, what: &str) -> ParseResult<TypeName> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::TypeName => {
                self.advance();
                Ok(TypeName {
                    text: self.text_of(&token),
                    span: token.span,
                })
            }
            _ => {
                let found = self.describe(token.span);
                self.error(
                    "expected",
                    format!("expected {what}, found {found}"),
                    token.span,
                );
                Err(())
            }
        }
    }

    // ------------------------------------------------------------ statements

    fn block(&mut self, terminators: &[Word]) -> ParseResult<Block> {
        let mut block = Block::default();
        let start = self.peek().span.start;
        loop {
            self.skip_newlines();
            let token = self.raw().clone();
            match token.kind {
                TokenKind::Eof => {
                    self.error(
                        "missing-end",
                        "the file ended inside a block; `end` is missing",
                        token.span,
                    );
                    return Err(());
                }
                TokenKind::Word(word) if terminators.contains(&word) => break,
                _ => {}
            }
            match self.statement() {
                Ok(statement) => block.statements.push(statement),
                Err(()) => self.recover(),
            }
        }
        let end = self.tokens[self.pos.saturating_sub(1)].span.end.max(start);
        block.span = Span::new(start, end);
        Ok(block)
    }

    fn statement(&mut self) -> ParseResult<Stmt> {
        let token = self.peek().clone();
        let start = token.span.start;
        let column = self.column(start);
        let kind = self.with_continuation(column, false, |parser| {
            parser.statement_kind(&token, column)
        })?;
        let end = self.tokens[self.pos - 1].span.end;
        self.end_of_statement()?;
        Ok(Stmt {
            kind,
            span: Span::new(start, end),
        })
    }

    fn statement_kind(&mut self, token: &Token, column: usize) -> ParseResult<StmtKind> {
        match &token.kind {
            TokenKind::Word(Word::Let) => {
                self.advance();
                let mutable = self.eat_word(Word::Mutable);
                let name = self.identifier("a name to bind")?;
                let ty = if self.eat(&TokenKind::Colon) {
                    Some(self.type_()?)
                } else {
                    None
                };
                self.expect_word(Word::Be)?;
                self.allow_next_line(column);
                let value = self.expr()?;
                Ok(StmtKind::Let {
                    mutable,
                    name,
                    ty,
                    value,
                })
            }
            TokenKind::Word(Word::Set) => {
                self.advance();
                let name = self.identifier("the name of a mutable binding")?;
                self.expect_word(Word::To)?;
                self.allow_next_line(column);
                let value = self.expr()?;
                Ok(StmtKind::Set { name, value })
            }
            TokenKind::Word(Word::If) => self.if_statement(),
            TokenKind::Word(Word::Match) => self.match_statement(),
            TokenKind::Word(Word::ForEach) => self.for_each(),
            TokenKind::Word(Word::RepeatUntil) => {
                self.advance();
                let condition = self.expr()?;
                self.end_of_statement()?;
                let body = self.block(&[Word::End])?;
                self.expect_word(Word::End)?;
                Ok(StmtKind::RepeatUntil { condition, body })
            }
            TokenKind::Word(Word::RunConcurrently) => {
                self.advance();
                let within = if self.eat_word(Word::Within) {
                    Some(self.expr()?)
                } else {
                    None
                };
                self.end_of_statement()?;
                let body = self.block(&[Word::End])?;
                self.expect_word(Word::End)?;
                Ok(StmtKind::RunConcurrently { within, body })
            }
            TokenKind::Word(Word::Return) => {
                self.advance();
                // `return` alone ends the line; a deeper next line holds the value
                self.allow_next_line(column);
                if self.statement_ends_here() {
                    return Ok(StmtKind::Return(None));
                }
                Ok(StmtKind::Return(Some(self.expr()?)))
            }
            TokenKind::Word(Word::Fail) => {
                self.advance();
                if self.eat_word(Word::With) {
                    Ok(StmtKind::Fail(Some(self.expr()?)))
                } else {
                    Ok(StmtKind::Fail(None))
                }
            }
            TokenKind::Word(Word::Crash) => {
                self.advance();
                self.expect_word(Word::With)?;
                Ok(StmtKind::Crash(self.expr()?))
            }
            TokenKind::Word(Word::Break) => {
                self.advance();
                Ok(StmtKind::Break)
            }
            TokenKind::Word(Word::Continue) => {
                self.advance();
                Ok(StmtKind::Continue)
            }
            TokenKind::Word(Word::Ignore) => {
                self.advance();
                Ok(StmtKind::Ignore(self.expr()?))
            }
            TokenKind::Word(Word::Check) => {
                self.advance();
                Ok(StmtKind::Check(self.expr()?))
            }
            TokenKind::Word(Word::Until) | TokenKind::Word(Word::Repeat) => {
                self.diagnostics.push(
                    Diagnostic::error(
                        "expected",
                        "a loop starts with the phrase `repeat until`, then its condition",
                        token.span,
                    )
                    .with_fix("write `repeat until condition` on one line"),
                );
                Err(())
            }
            TokenKind::Identifier if self.foreign_statement_word(token).is_some() => {
                let (word, fix) = self.foreign_statement_word(token).expect("checked above");
                self.diagnostics.push(
                    Diagnostic::error(
                        "foreign-keyword",
                        format!("`{word}` is not Renyi"),
                        token.span,
                    )
                    .with_fix(fix),
                );
                Err(())
            }
            _ => {
                let expr = self.expr()?;
                if !matches!(
                    expr.kind,
                    ExprKind::Call { .. } | ExprKind::Otherwise { .. }
                ) {
                    self.error(
                        "statement-shape",
                        "a statement is a call, or starts with `let`, `set`, `if`, `match`, `for each`, `repeat until`, `return`, `fail`, `crash`, `ignore` or `check`",
                        expr.span,
                    );
                    return Err(());
                }
                Ok(StmtKind::Expression(expr))
            }
        }
    }

    /// A keyword of another language at the start of a statement, with the
    /// Renyi form to write instead (decision C4). An identifier followed by `(`
    /// or `.` is a call and never foreign.
    fn foreign_statement_word(&self, token: &Token) -> Option<(String, &'static str)> {
        let word = &self.src[token.span.start..token.span.end];
        let next = self.tokens.get(self.pos + 1).map(|t| &t.kind);
        if matches!(next, Some(TokenKind::LeftParen) | Some(TokenKind::Dot)) {
            return None;
        }
        let fix = match word {
            "while" => "write `repeat until` with the opposite condition",
            "loop" | "do" => "write `repeat until condition`",
            "else" | "elif" | "elsif" => "write `otherwise`, or `otherwise if condition then`",
            "def" | "fn" | "func" => "write `function`",
            "var" | "const" => "write `let`, or `let mutable` for a value that changes",
            "switch" => "write `match`",
            "case" => "write `when pattern then`",
            "foreach" => "write `for each`",
            "throw" | "raise" => "write `fail with`",
            _ => return None,
        };
        Some((word.to_string(), fix))
    }

    fn statement_ends_here(&self) -> bool {
        matches!(
            self.raw().kind,
            TokenKind::Newline
                | TokenKind::Eof
                | TokenKind::Word(Word::End)
                | TokenKind::Word(Word::Otherwise)
                | TokenKind::Word(Word::When)
        )
    }

    fn if_statement(&mut self) -> ParseResult<StmtKind> {
        self.expect_word(Word::If)?;
        let mut branches = Vec::new();
        let mut otherwise = None;
        loop {
            let condition = self.expr()?;
            self.expect_word(Word::Then)?;
            let block = self.block(&[Word::Otherwise, Word::End])?;
            branches.push((condition, block));
            if self.eat_word(Word::Otherwise) {
                if self.eat_word(Word::If) {
                    continue;
                }
                otherwise = Some(self.block(&[Word::End])?);
            }
            break;
        }
        self.expect_word(Word::End)?;
        Ok(StmtKind::If {
            branches,
            otherwise,
        })
    }

    fn match_statement(&mut self) -> ParseResult<StmtKind> {
        self.expect_word(Word::Match)?;
        let subject = self.expr()?;
        self.end_of_statement()?;
        let mut arms = Vec::new();
        let mut otherwise = None;
        loop {
            self.skip_newlines();
            if self.at_word(Word::End) {
                break;
            }
            if self.eat_word(Word::Otherwise) {
                otherwise = Some(self.block(&[Word::End])?);
                break;
            }
            let start = self.expect_word(Word::When)?.span.start;
            let pattern = self.pattern()?;
            let guard = if self.eat_word(Word::Where) {
                Some(self.expr()?)
            } else {
                None
            };
            self.expect_word(Word::Then)?;
            let body = self.block(&[Word::When, Word::Otherwise, Word::End])?;
            let span = Span::new(start, body.span.end);
            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span,
            });
        }
        self.expect_word(Word::End)?;
        Ok(StmtKind::Match {
            subject,
            arms,
            otherwise,
        })
    }

    fn for_each(&mut self) -> ParseResult<StmtKind> {
        self.expect_word(Word::ForEach)?;
        let (bindings, source) = self.loop_source()?;
        let filter = if self.eat_word(Word::Where) {
            Some(self.expr()?)
        } else {
            None
        };
        let order = self.ordering()?;
        self.end_of_statement()?;
        let body = self.block(&[Word::End])?;
        self.expect_word(Word::End)?;
        Ok(StmtKind::ForEach {
            bindings,
            source,
            filter,
            order,
            body,
        })
    }

    /// `x in xs`, `key, value in map` or `index from 1 to 10 by 2`.
    fn loop_source(&mut self) -> ParseResult<(Vec<Name>, Expr)> {
        let mut bindings = vec![self.identifier("a loop variable")?];
        while self.eat(&TokenKind::Comma) {
            bindings.push(self.identifier("a loop variable")?);
        }
        if self.at_word(Word::From) {
            let range = self.range()?;
            return Ok((bindings, range));
        }
        self.expect_word(Word::In)?;
        let source = self.or_expr()?;
        Ok((bindings, source))
    }

    fn ordering(&mut self) -> ParseResult<Option<Ordering>> {
        if self.eat_word(Word::SortedBy) {
            let key = self.or_expr()?;
            let descending = self.eat_word(Word::Descending);
            Ok(Some(Ordering { key, descending }))
        } else {
            Ok(None)
        }
    }

    // ------------------------------------------------------------ patterns

    fn pattern(&mut self) -> ParseResult<Pattern> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::TypeName => {
                let name = self.type_name("a variant")?;
                let mut fields = Vec::new();
                let mut end = name.span;
                if self.eat(&TokenKind::LeftParen) {
                    self.nesting += 1;
                    while !self.at(&TokenKind::RightParen) {
                        let field = self.identifier("a field name")?;
                        let pattern = if self.eat(&TokenKind::Colon) {
                            Some(self.pattern()?)
                        } else {
                            None
                        };
                        fields.push(FieldPattern { field, pattern });
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.nesting -= 1;
                    end = self.expect(&TokenKind::RightParen, "`)`")?.span;
                }
                Ok(Pattern::Variant {
                    span: name.span.join(end),
                    name,
                    fields,
                })
            }
            TokenKind::Word(Word::Nothing) => {
                self.advance();
                Ok(Pattern::Nothing(token.span))
            }
            TokenKind::Word(wrapper @ (Word::Some | Word::Success | Word::Failure)) => {
                self.advance();
                self.expect(&TokenKind::LeftParen, "`(`")?;
                self.nesting += 1;
                let inner = self.pattern()?;
                self.nesting -= 1;
                let end = self.expect(&TokenKind::RightParen, "`)`")?.span;
                let span = token.span.join(end);
                Ok(match wrapper {
                    Word::Some => Pattern::Some(Box::new(inner), span),
                    Word::Success => Pattern::Success(Box::new(inner), span),
                    _ => Pattern::Failure(Box::new(inner), span),
                })
            }
            TokenKind::Identifier => {
                let name = self.identifier("a name")?;
                if self.at(&TokenKind::Colon)
                    && matches!(
                        self.peek_second().kind,
                        TokenKind::TypeName | TokenKind::Word(Word::Maybe)
                    )
                {
                    self.advance();
                    let ty = self.type_()?;
                    let span = name.span.join(ty.span());
                    return Ok(Pattern::Typed { name, ty, span });
                }
                Ok(Pattern::Binding(name))
            }
            TokenKind::Integer
            | TokenKind::Decimal
            | TokenKind::Text { .. }
            | TokenKind::RawText(_)
            | TokenKind::Minus
            | TokenKind::LeftBracket
            | TokenKind::LeftBrace
            | TokenKind::Word(Word::True)
            | TokenKind::Word(Word::False) => {
                let literal = self.primary()?;
                Ok(Pattern::Literal(literal))
            }
            _ => {
                let found = self.describe(token.span);
                self.error(
                    "expected",
                    format!("expected a pattern, found {found}"),
                    token.span,
                );
                Err(())
            }
        }
    }

    // ------------------------------------------------------------ expressions

    /// A full expression, including a trailing `otherwise`.
    fn expr(&mut self) -> ParseResult<Expr> {
        let mut value = self.or_expr()?;
        while self.at_word(Word::Otherwise) {
            self.advance();
            let fallback = self.outcome()?;
            let span = value.span.join(fallback.span());
            value = Expr {
                kind: ExprKind::Otherwise {
                    value: Box::new(value),
                    fallback: Box::new(fallback),
                },
                span,
            };
        }
        Ok(value)
    }

    /// A value or a way out, as in a branch or after `otherwise`.
    fn outcome(&mut self) -> ParseResult<Outcome> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Word(Word::Fail) => {
                self.advance();
                if self.eat_word(Word::With) {
                    let value = self.or_expr()?;
                    let span = token.span.join(value.span);
                    Ok(Outcome::Fail(Some(value), span))
                } else {
                    Ok(Outcome::Fail(None, token.span))
                }
            }
            TokenKind::Word(Word::Return) => {
                self.advance();
                if self.statement_ends_here() || self.at_word(Word::End) {
                    Ok(Outcome::Return(None, token.span))
                } else {
                    let value = self.or_expr()?;
                    let span = token.span.join(value.span);
                    Ok(Outcome::Return(Some(value), span))
                }
            }
            TokenKind::Word(Word::Crash) => {
                self.advance();
                self.expect_word(Word::With)?;
                let value = self.or_expr()?;
                let span = token.span.join(value.span);
                Ok(Outcome::Crash(value, span))
            }
            TokenKind::Word(Word::Break) => {
                self.advance();
                Ok(Outcome::Break(token.span))
            }
            TokenKind::Word(Word::Continue) => {
                self.advance();
                Ok(Outcome::Continue(token.span))
            }
            _ => Ok(Outcome::Value(self.or_expr()?)),
        }
    }

    fn or_expr(&mut self) -> ParseResult<Expr> {
        let mut left = self.and_expr()?;
        while self.at_word(Word::Or) {
            self.advance();
            let right = self.and_expr()?;
            left = binary(BinaryOp::Or, left, right);
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> ParseResult<Expr> {
        let mut left = self.not_expr()?;
        while self.at_word(Word::And) {
            self.advance();
            let right = self.not_expr()?;
            left = binary(BinaryOp::And, left, right);
        }
        Ok(left)
    }

    fn not_expr(&mut self) -> ParseResult<Expr> {
        if self.at_word(Word::Not) {
            let start = self.advance().span;
            let inner = self.not_expr()?;
            let span = start.join(inner.span);
            return Ok(Expr {
                kind: ExprKind::Not(Box::new(inner)),
                span,
            });
        }
        self.comparison()
    }

    fn comparison(&mut self) -> ParseResult<Expr> {
        let left = self.with_expr()?;
        let op = match &self.peek().kind {
            TokenKind::Word(Word::Is) => BinaryOp::Is,
            TokenKind::Word(Word::IsNot) => BinaryOp::IsNot,
            TokenKind::Word(Word::IsLessThan) => BinaryOp::IsLessThan,
            TokenKind::Word(Word::IsAtMost) => BinaryOp::IsAtMost,
            TokenKind::Word(Word::IsGreaterThan) => BinaryOp::IsGreaterThan,
            TokenKind::Word(Word::IsAtLeast) => BinaryOp::IsAtLeast,
            _ => return Ok(left),
        };
        self.advance();
        let right = self.with_expr()?;
        Ok(binary(op, left, right))
    }

    fn with_expr(&mut self) -> ParseResult<Expr> {
        let base = self.additive()?;
        if self.at_word(Word::With) {
            self.advance();
            let mut updates = Vec::new();
            loop {
                let name = self.identifier("a field name")?;
                self.expect(&TokenKind::Colon, "`:`")?;
                let value = self.additive()?;
                let span = name.span.join(value.span);
                updates.push(Arg {
                    name: Some(name),
                    value,
                    span,
                });
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            let span = base
                .span
                .join(updates.last().map(|arg| arg.span).unwrap_or(base.span));
            return Ok(Expr {
                kind: ExprKind::With {
                    base: Box::new(base),
                    updates,
                },
                span,
            });
        }
        Ok(base)
    }

    fn additive(&mut self) -> ParseResult<Expr> {
        let mut left = self.multiplicative()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Subtract,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.multiplicative()?;
            left = binary(op, left, right);
        }
    }

    fn multiplicative(&mut self) -> ParseResult<Expr> {
        let mut left = self.power()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::Star => BinaryOp::Multiply,
                TokenKind::Slash => BinaryOp::Divide,
                TokenKind::Word(Word::Remainder) => BinaryOp::Remainder,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.power()?;
            left = binary(op, left, right);
        }
    }

    fn power(&mut self) -> ParseResult<Expr> {
        let base = self.postfix()?;
        if self.at_word(Word::Power) {
            self.advance();
            let exponent = self.power()?;
            return Ok(binary(BinaryOp::Power, base, exponent));
        }
        Ok(base)
    }

    fn postfix(&mut self) -> ParseResult<Expr> {
        let mut expr = self.primary()?;
        loop {
            if self.raw().kind == TokenKind::Dot {
                self.advance();
                let token = self.advance();
                let name = match token.kind {
                    TokenKind::Member | TokenKind::Identifier => Name {
                        text: self.text_of(&token),
                        span: token.span,
                    },
                    _ => {
                        self.error(
                            "expected",
                            "expected a member name after the dot",
                            token.span,
                        );
                        return Err(());
                    }
                };
                let span = expr.span.join(name.span);
                expr = Expr {
                    kind: ExprKind::Member {
                        base: Box::new(expr),
                        name,
                    },
                    span,
                };
                continue;
            }
            if self.raw().kind == TokenKind::LeftParen
                && matches!(expr.kind, ExprKind::Member { .. } | ExprKind::Name(_))
            {
                let args = self.arguments()?;
                let span = expr.span.join(self.tokens[self.pos - 1].span);
                expr = Expr {
                    kind: ExprKind::Call {
                        callee: Box::new(expr),
                        args,
                    },
                    span,
                };
                continue;
            }
            return Ok(expr);
        }
    }

    fn arguments(&mut self) -> ParseResult<Vec<Arg>> {
        self.expect(&TokenKind::LeftParen, "`(`")?;
        self.nesting += 1;
        let mut args = Vec::new();
        while !self.at(&TokenKind::RightParen) {
            let named =
                self.at(&TokenKind::Identifier) && self.peek_second().kind == TokenKind::Colon;
            let name = if named {
                let name = self.identifier("an argument name")?;
                self.advance();
                Some(name)
            } else {
                None
            };
            let value = self.expr()?;
            let span = name
                .as_ref()
                .map_or(value.span, |name| name.span.join(value.span));
            args.push(Arg { name, value, span });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.nesting -= 1;
        self.expect(&TokenKind::RightParen, "`)`")?;
        Ok(args)
    }

    fn primary(&mut self) -> ParseResult<Expr> {
        let token = self.peek().clone();
        let span = token.span;
        match token.kind {
            TokenKind::Integer => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Integer(self.text_of(&token)),
                    span,
                })
            }
            TokenKind::Decimal => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Decimal(self.text_of(&token)),
                    span,
                })
            }
            TokenKind::Minus => {
                self.advance();
                let number = self.advance();
                match number.kind {
                    TokenKind::Integer => Ok(Expr {
                        kind: ExprKind::Integer(format!("-{}", self.text_of(&number))),
                        span: span.join(number.span),
                    }),
                    TokenKind::Decimal => Ok(Expr {
                        kind: ExprKind::Decimal(format!("-{}", self.text_of(&number))),
                        span: span.join(number.span),
                    }),
                    _ => {
                        self.error("unary-minus", "`-` negates only a number literal; write `0 - value` for a computed value", span.join(number.span));
                        Err(())
                    }
                }
            }
            TokenKind::Text { parts, block } => {
                self.advance();
                let mut pieces = Vec::new();
                for part in parts {
                    match part {
                        TextPart::Text(text) => pieces.push(TextPiece::Text(text)),
                        TextPart::Hole(tokens) => {
                            let expr = parse_hole(self.src, tokens, &mut self.diagnostics);
                            pieces.push(TextPiece::Hole(expr));
                        }
                    }
                }
                Ok(Expr {
                    kind: ExprKind::Text { pieces, block },
                    span,
                })
            }
            TokenKind::RawText(text) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::RawText(text),
                    span,
                })
            }
            TokenKind::Word(Word::Raw) => {
                self.advance();
                let token = self.advance();
                match token.kind {
                    TokenKind::RawText(text) => Ok(Expr {
                        kind: ExprKind::RawText(text),
                        span: span.join(token.span),
                    }),
                    _ => {
                        self.error(
                            "expected",
                            "expected a text literal after `raw`",
                            token.span,
                        );
                        Err(())
                    }
                }
            }
            TokenKind::Word(Word::True) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Boolean(true),
                    span,
                })
            }
            TokenKind::Word(Word::False) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Boolean(false),
                    span,
                })
            }
            TokenKind::Word(Word::Nothing) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Nothing,
                    span,
                })
            }
            TokenKind::Word(Word::SelfValue) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::SelfValue,
                    span,
                })
            }
            TokenKind::Identifier => {
                let name = self.identifier("a name")?;
                Ok(Expr {
                    kind: ExprKind::Name(name),
                    span,
                })
            }
            TokenKind::TypeName => {
                let name = self.type_name("a type")?;
                if self.raw().kind == TokenKind::LeftParen {
                    let args = self.arguments()?;
                    let span = span.join(self.tokens[self.pos - 1].span);
                    return Ok(Expr {
                        kind: ExprKind::Construct { name, args },
                        span,
                    });
                }
                Ok(Expr {
                    kind: ExprKind::TypeName(name),
                    span,
                })
            }
            TokenKind::LeftParen => {
                self.advance();
                self.nesting += 1;
                let inner = self.expr()?;
                self.nesting -= 1;
                let end = self.expect(&TokenKind::RightParen, "`)`")?.span;
                Ok(Expr {
                    kind: ExprKind::Paren(Box::new(inner)),
                    span: span.join(end),
                })
            }
            TokenKind::LeftBracket => {
                self.advance();
                self.nesting += 1;
                let mut items = Vec::new();
                while !self.at(&TokenKind::RightBracket) {
                    items.push(self.expr()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.nesting -= 1;
                let end = self.expect(&TokenKind::RightBracket, "`]`")?.span;
                Ok(Expr {
                    kind: ExprKind::List(items),
                    span: span.join(end),
                })
            }
            TokenKind::LeftBrace => {
                self.advance();
                self.nesting += 1;
                let mut entries = Vec::new();
                while !self.at(&TokenKind::RightBrace) {
                    let key = self.or_expr()?;
                    self.expect(&TokenKind::Colon, "`:` between a map key and its value")?;
                    let value = self.expr()?;
                    entries.push((key, value));
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.nesting -= 1;
                let end = self.expect(&TokenKind::RightBrace, "`}`")?.span;
                Ok(Expr {
                    kind: ExprKind::Map(entries),
                    span: span.join(end),
                })
            }
            TokenKind::Word(Word::From) => self.range(),
            TokenKind::Word(Word::If) => self.if_expr(),
            TokenKind::Word(Word::Match) => self.match_expr(),
            TokenKind::Word(Word::ForEach) => self.query(),
            _ => {
                let found = self.describe(span);
                self.error(
                    "expected",
                    format!("expected an expression, found {found}"),
                    span,
                );
                Err(())
            }
        }
    }

    fn range(&mut self) -> ParseResult<Expr> {
        let start = self.expect_word(Word::From)?.span;
        let from = self.additive()?;
        self.expect_word(Word::To)?;
        let to = self.additive()?;
        let by = if self.eat_word(Word::By) {
            Some(Box::new(self.additive()?))
        } else {
            None
        };
        let end = self.tokens[self.pos - 1].span;
        Ok(Expr {
            kind: ExprKind::Range {
                from: Box::new(from),
                to: Box::new(to),
                by,
            },
            span: start.join(end),
        })
    }

    fn if_expr(&mut self) -> ParseResult<Expr> {
        let start = self.expect_word(Word::If)?.span;
        let mut branches = Vec::new();
        loop {
            let condition = self.expr()?;
            self.expect_word(Word::Then)?;
            self.skip_newlines_in_expr();
            let outcome = self.outcome()?;
            branches.push((condition, outcome));
            self.skip_newlines_in_expr();
            self.expect_word(Word::Otherwise)?;
            if self.eat_word(Word::If) {
                continue;
            }
            self.skip_newlines_in_expr();
            let otherwise = self.outcome()?;
            self.skip_newlines_in_expr();
            let end = self.expect_word(Word::End)?.span;
            return Ok(Expr {
                kind: ExprKind::If {
                    branches,
                    otherwise: Box::new(otherwise),
                },
                span: start.join(end),
            });
        }
    }

    /// Inside a multi-line `if` or `match` expression, line breaks between the
    /// parts carry no meaning.
    fn skip_newlines_in_expr(&mut self) {
        self.skip_newlines();
    }

    fn match_expr(&mut self) -> ParseResult<Expr> {
        let start = self.expect_word(Word::Match)?.span;
        let subject = self.expr()?;
        let mut arms = Vec::new();
        let mut otherwise = None;
        loop {
            self.skip_newlines();
            if self.at_word(Word::End) {
                break;
            }
            if self.eat_word(Word::Otherwise) {
                otherwise = Some(Box::new(self.outcome()?));
                continue;
            }
            let arm_start = self.expect_word(Word::When)?.span.start;
            let pattern = self.pattern()?;
            let guard = if self.eat_word(Word::Where) {
                Some(self.expr()?)
            } else {
                None
            };
            self.expect_word(Word::Then)?;
            let body =
                self.with_continuation(self.column(arm_start), false, |parser| parser.outcome())?;
            let span = Span::new(arm_start, body.span().end);
            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span,
            });
        }
        let end = self.expect_word(Word::End)?.span;
        Ok(Expr {
            kind: ExprKind::Match {
                subject: Box::new(subject),
                arms,
                otherwise,
            },
            span: start.join(end),
        })
    }

    fn query(&mut self) -> ParseResult<Expr> {
        let start = self.expect_word(Word::ForEach)?.span;
        let mut sources = Vec::new();
        loop {
            let (bindings, source) = self.loop_source()?;
            sources.push(QuerySource { bindings, source });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        let concurrently = self.eat_word(Word::Concurrently);
        let within = if concurrently && self.eat_word(Word::Within) {
            Some(self.or_expr()?)
        } else {
            None
        };
        let filter = if self.eat_word(Word::Where) {
            Some(self.or_expr()?)
        } else {
            None
        };
        let order = self.ordering()?;
        let group_by = if self.eat_word(Word::GroupBy) {
            Some(self.or_expr()?)
        } else {
            None
        };
        let terminal = match &self.peek().kind {
            TokenKind::Word(Word::Collect) => {
                self.advance();
                QueryTerminal::Collect(self.expr()?)
            }
            TokenKind::Word(Word::Sum) => {
                self.advance();
                QueryTerminal::Sum(self.expr()?)
            }
            TokenKind::Word(Word::Count) => {
                self.advance();
                QueryTerminal::Count
            }
            TokenKind::Word(Word::First) => {
                self.advance();
                QueryTerminal::First
            }
            TokenKind::Word(Word::Any) => {
                self.advance();
                QueryTerminal::Any(self.expr()?)
            }
            TokenKind::Word(Word::All) => {
                self.advance();
                QueryTerminal::All(self.expr()?)
            }
            _ if group_by.is_some() => QueryTerminal::None,
            _ => {
                let span = self.peek().span;
                let found = self.describe(span);
                self.error(
                    "query-shape",
                    format!("a query ends with `collect`, `sum`, `count`, `first`, `any`, `all` or `group by`, found {found}"),
                    span,
                );
                return Err(());
            }
        };
        let end = self.tokens[self.pos - 1].span;
        let span = start.join(end);
        Ok(Expr {
            kind: ExprKind::Query(Box::new(Query {
                sources,
                concurrently,
                within,
                filter,
                order,
                group_by,
                terminal,
                span,
            })),
            span,
        })
    }
}

fn binary(op: BinaryOp, left: Expr, right: Expr) -> Expr {
    let span = left.span.join(right.span);
    Expr {
        kind: ExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
        span,
    }
}

fn docs_empty(docs: &Docs) -> bool {
    docs.purpose.is_none()
        && docs.tags.is_empty()
        && docs.see_also.is_empty()
        && docs.deprecated.is_none()
        && !docs.expose_as_tool
        && docs.examples.is_empty()
}

fn split_list(text: &str) -> Vec<String> {
    text.split(',')
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

/// The literal characters of a text token that has no holes.
fn literal_text(parts: &[TextPart]) -> String {
    parts
        .iter()
        .map(|part| match part {
            TextPart::Text(text) => text.clone(),
            TextPart::Hole(_) => String::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(source: &str) -> Module {
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        parsed.module
    }

    fn function_body(source: &str) -> Vec<Stmt> {
        let module = parse_ok(&format!("module tests\n\nfunction body()\n{source}\nend\n"));
        match &module.items[0] {
            Item::Function(function) => function.body.clone().unwrap().statements,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn module_header_imports_and_docs() {
        let module = parse_ok("module billing.invoices\n  purpose: Compute invoices.\n  tags: billing, money\n\nimport std.json\nimport std.http as web\nimport accounts.models exposing User, UserId\n");
        assert_eq!(
            module
                .name
                .iter()
                .map(|n| n.text.as_str())
                .collect::<Vec<_>>(),
            ["billing", "invoices"]
        );
        assert_eq!(module.docs.purpose.as_deref(), Some("Compute invoices."));
        assert_eq!(module.docs.tags, ["billing", "money"]);
        assert_eq!(module.imports.len(), 3);
        assert_eq!(module.imports[1].alias.as_ref().unwrap().text, "web");
        assert_eq!(module.imports[2].exposing.len(), 2);
    }

    #[test]
    fn function_with_clauses_and_examples() {
        let module = parse_ok(
            "module tests\n\npublic function total(items: List of Item, rate: TaxRate)\n  returns Money\n  or fails with PricingError or Other\n  needs network.http(\"api.example.com\"), console\n  for any Item where Item can Priced\n  purpose: Add up.\n  tags: billing\n  see also: tax_for\n  expose as tool\n  example: total(items: [], rate: TaxRate(0.1)) is 0\n  example: total(items: [], rate: TaxRate(2))\n    fails with Invalid(detail: \"too high\")\n\n  return 0\nend\n",
        );
        let Item::Function(function) = &module.items[0] else {
            panic!()
        };
        assert!(function.public);
        assert_eq!(function.params.len(), 2);
        assert!(function.returns.is_some());
        assert_eq!(function.fails.len(), 2);
        assert_eq!(function.needs.len(), 2);
        assert_eq!(function.needs[0].scope.as_deref(), Some("api.example.com"));
        assert_eq!(function.type_params.as_ref().unwrap().constraints.len(), 1);
        assert_eq!(function.docs.purpose.as_deref(), Some("Add up."));
        assert!(function.docs.expose_as_tool);
        assert_eq!(function.docs.examples.len(), 2);
        assert!(matches!(
            function.docs.examples[1].outcome,
            ExampleOutcome::FailsWith(Pattern::Variant { .. })
        ));
        assert_eq!(function.body.as_ref().unwrap().statements.len(), 1);
    }

    #[test]
    fn statements_and_continuation_lines() {
        let statements = function_body(
            "  let text be files.read_text(path)\n    otherwise fail\n  let users: List of User be json.parse(text) otherwise fail with Bad(detail: \"x\")\n  let mutable total be 0\n  set total to total + 1\n  if total is at least 18 then\n    set total to 1\n  otherwise if total is 2 then\n    set total to 2\n  otherwise\n    set total to 3\n  end\n  for each user in users where user.is_active sorted by user.name descending\n    console.print(user.name)\n  end\n  repeat until total is at least 3\n    set total to total + 1\n    if done then break end\n  end\n  return total",
        );
        assert_eq!(statements.len(), 8);
        assert!(matches!(
            &statements[0].kind,
            StmtKind::Let {
                value: Expr {
                    kind: ExprKind::Otherwise { .. },
                    ..
                },
                ..
            }
        ));
        assert!(
            matches!(&statements[4].kind, StmtKind::If { branches, otherwise: Some(_) } if branches.len() == 2)
        );
        assert!(matches!(
            &statements[5].kind,
            StmtKind::ForEach {
                filter: Some(_),
                order: Some(_),
                ..
            }
        ));
        assert!(matches!(&statements[6].kind, StmtKind::RepeatUntil { .. }));
        assert!(matches!(&statements[7].kind, StmtKind::Return(Some(_))));
    }

    #[test]
    fn grant_clauses_and_replays_parse() {
        let parsed = parse(
            "module demo\n\npublic function main() or fails with AppError\n  needs console, network.http(\"api.example.com\") at most 60 per minute, filesystem.read(\"secrets\") only to console or network.http(\"api.example.com\")\n  purpose: Try the grant clauses.\n\n  console.print(\"hi\")\nend\n\ntest \"the forecast is read\" needs network.http replays \"fixtures/forecast.json\"\n  check true\nend\n",
        );
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let Item::Function(main) = &parsed.module.items[0] else {
            panic!()
        };
        assert_eq!(main.needs.len(), 3);
        let budget = main.needs[1].budget.as_ref().expect("budget");
        assert_eq!(
            (budget.count.as_str(), budget.per.text.as_str()),
            ("60", "minute")
        );
        assert_eq!(main.needs[2].only_to.len(), 2);
        assert_eq!(
            main.needs[2].only_to[1].scope.as_deref(),
            Some("api.example.com")
        );
        let Item::Test(test) = &parsed.module.items[1] else {
            panic!()
        };
        assert_eq!(test.replays.as_deref(), Some("fixtures/forecast.json"));
        // a budget is per one of five units
        let bad =
            parse("module demo\n\nfunction main() needs network.http at most 3 per week\nend\n");
        assert!(bad
            .diagnostics
            .iter()
            .any(|d| d.message.contains("per `second`")));
    }

    #[test]
    fn foreign_loop_keywords_get_the_renyi_form() {
        let parsed = parse(
            "module demo\n\nfunction run_all() returns Integer\n  let mutable total be 0\n  while total is less than 3\n    set total to total + 1\n  end\n  return total\nend\n",
        );
        let foreign: Vec<_> = parsed
            .diagnostics
            .iter()
            .filter(|d| d.code == "foreign-keyword")
            .collect();
        assert_eq!(foreign.len(), 1, "{:?}", parsed.diagnostics);
        assert!(foreign[0].message.contains("`while`"));
        assert!(foreign[0].fix.as_deref().unwrap().contains("repeat until"));

        let parsed = parse("module demo\n\nfunction wait()\n  until done\n  end\nend\n");
        assert!(parsed.diagnostics.iter().any(|d| d
            .fix
            .as_deref()
            .is_some_and(|f| f.contains("repeat until condition"))));

        // an own-module function may be called `loop` or `print`: a call is never foreign
        let parsed = parse("module demo\n\nfunction go()\n  loop(3)\nend\n\nfunction loop(times: Integer)\n  ignore times\nend\n");
        assert!(
            parsed
                .diagnostics
                .iter()
                .all(|d| d.code != "foreign-keyword"),
            "{:?}",
            parsed.diagnostics
        );
    }

    #[test]
    fn otherwise_at_the_if_column_is_not_a_continuation() {
        let statements = function_body("  if allowed then\n    console.print(\"yes\")\n  otherwise\n    console.print(\"no\")\n  end");
        assert_eq!(statements.len(), 1);
        let StmtKind::If {
            branches,
            otherwise,
        } = &statements[0].kind
        else {
            panic!()
        };
        assert_eq!(branches[0].1.statements.len(), 1);
        assert!(otherwise.is_some());
    }

    #[test]
    fn queries_match_and_if_expressions() {
        let statements = function_body(
            "  let emails be\n    for each user in users\n    where user.is_active and user.age is at least 18\n    sorted by user.created_at descending\n    collect user.email\n  let paid be for each order in orders where order.is_paid count\n  let kind be\n    match shape\n      when Circle(radius) then \"circle {radius}\"\n      when Rectangle(width: wide, height) then \"rect\"\n      when Point then \"point\"\n      otherwise \"other\"\n    end\n  let label be if done then \"yes\" otherwise \"no\" end\n  let grouped be for each word in words group by word",
        );
        assert_eq!(statements.len(), 5);
        let StmtKind::Let { value, .. } = &statements[0].kind else {
            panic!()
        };
        let ExprKind::Query(query) = &value.kind else {
            panic!("{value:?}")
        };
        assert!(query.filter.is_some() && query.order.is_some());
        assert!(matches!(query.terminal, QueryTerminal::Collect(_)));
        let StmtKind::Let { value, .. } = &statements[2].kind else {
            panic!()
        };
        let ExprKind::Match {
            arms, otherwise, ..
        } = &value.kind
        else {
            panic!("{value:?}")
        };
        assert_eq!(arms.len(), 3);
        assert!(otherwise.is_some());
        let StmtKind::Let { value, .. } = &statements[4].kind else {
            panic!()
        };
        let ExprKind::Query(query) = &value.kind else {
            panic!()
        };
        assert!(query.group_by.is_some() && matches!(query.terminal, QueryTerminal::None));
    }

    #[test]
    fn types_abilities_tests_and_constants() {
        let module = parse_ok(
            "module tests\n\npublic type User\n  purpose: An account.\n  has name: Text\n  has age: Integer where age is at least 0\n  has kind: Text as \"type\"\n  has email: maybe Email\n  can Compare by name, age\n  can ToJson\nend\n\npublic type Shape is one of\n  purpose: A figure.\n  Circle(radius: Decimal where radius is greater than 0)\n  Rectangle(\n    width: Decimal,\n    height: Decimal\n  )\n  Point\n  can ToText\nend\n\npublic type Email is Text where value.matches(raw \"^[^@]+@[^@]+$\")\n  purpose: An address.\n\npublic type Pair of Left, Right\n  has left: Left\n  has right: Right\nend\n\npublic ability Sized\n  purpose: Has a size.\n  function size(self) returns Integer\n  function is_empty(self) returns Boolean\nend\n\nability Sized for Stack of Item\n  for any Item\n  function size(self) returns Integer\n    return self.items.length()\n  end\n  function is_empty(self) returns Boolean\n    return self.items.is_empty()\n  end\nend\n\npublic let limit: Integer be 3\n  purpose: The limit.\n\ntest \"a test\" needs filesystem.read\n  check limit is 3\nend\n",
        );
        assert_eq!(module.items.len(), 8);
        let Item::Type(user) = &module.items[0] else {
            panic!()
        };
        let TypeKind::Record { fields, derives } = &user.kind else {
            panic!()
        };
        assert_eq!(fields.len(), 4);
        assert_eq!(fields[2].external_name.as_deref(), Some("type"));
        assert_eq!(derives[0].by.len(), 2);
        let Item::Type(shape) = &module.items[1] else {
            panic!()
        };
        let TypeKind::Sum { variants, derives } = &shape.kind else {
            panic!()
        };
        assert_eq!(variants.len(), 3);
        assert_eq!(variants[1].fields.len(), 2);
        assert_eq!(derives.len(), 1);
        let Item::Type(email) = &module.items[2] else {
            panic!()
        };
        assert!(matches!(
            email.kind,
            TypeKind::Subtype {
                refinement: Some(_),
                ..
            }
        ));
        assert_eq!(email.docs.purpose.as_deref(), Some("An address."));
        let Item::Type(pair) = &module.items[3] else {
            panic!()
        };
        assert_eq!(pair.type_params.len(), 2);
        let Item::Ability(sized) = &module.items[4] else {
            panic!()
        };
        assert_eq!(sized.functions.len(), 2);
        assert!(sized.functions[0].body.is_none());
        let Item::Implementation(implementation) = &module.items[5] else {
            panic!()
        };
        assert!(implementation.type_params.is_some());
        assert_eq!(implementation.functions.len(), 2);
        let Item::Constant(constant) = &module.items[6] else {
            panic!()
        };
        assert!(constant.public);
        let Item::Test(test) = &module.items[7] else {
            panic!()
        };
        assert_eq!(test.name, "a test");
        assert_eq!(test.needs.len(), 1);
    }

    #[test]
    fn interpolation_holes_become_expressions() {
        let statements = function_body("  console.print(\"Hello {user.name}, {total + 1}!\")");
        let StmtKind::Expression(expr) = &statements[0].kind else {
            panic!()
        };
        let ExprKind::Call { args, .. } = &expr.kind else {
            panic!()
        };
        let ExprKind::Text { pieces, .. } = &args[0].value.kind else {
            panic!()
        };
        assert_eq!(pieces.len(), 5);
        assert!(matches!(
            &pieces[3],
            TextPiece::Hole(Expr {
                kind: ExprKind::Binary { .. },
                ..
            })
        ));
    }

    #[test]
    fn errors_have_positions_and_recover() {
        let parsed = parse("module tests\n\nfunction body()\n  let total be 1 +\n  set total to 2\nend\n\nfunction other()\n  return 1\nend\n");
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(parsed.diagnostics[0].code, "expected");
        assert_eq!(parsed.module.items.len(), 2);
    }

    #[test]
    fn reserved_words_as_names_are_reported_with_a_fix() {
        let parsed =
            parse("module tests\n\nfunction body()\n  let count be 1\n  return count\nend\n");
        assert_eq!(parsed.diagnostics[0].code, "reserved-word");
        assert!(parsed.diagnostics[0].fix.is_some());
    }
}
