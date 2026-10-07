//! The formatter: a parsed module back to text in the canonical layout of
//! syntax sketch section 16. There are no options.
//!
//! Layout is decided with a small document model: text, line breaks that
//! become spaces when a group fits on one line, nesting, groups, and ordered
//! alternatives. A group is printed flat when it fits in the remaining width
//! and broken otherwise, so the outermost breakable construct breaks first:
//! the `is` of an example before the arguments of its call. `otherwise`
//! never starts a line (there it would be a branch, decision V2): a statement
//! that does not fit breaks inside the parentheses of the call before
//! `otherwise`, then inside the fallback's.

use crate::ast::*;
use crate::diagnostics::Diagnostic;
use crate::parser::parse;
use crate::span::{SourceFile, Span};

pub const WIDTH: usize = 100;
const INDENT: usize = 2;

/// Format a file. Fails with the parser's diagnostics when the file does not
/// parse: the formatter never guesses at broken code.
pub fn format(file: &SourceFile) -> Result<String, Vec<Diagnostic>> {
    let parsed = parse(&file.text);
    if parsed
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.is_error())
    {
        return Err(parsed.diagnostics);
    }
    let mut formatter = Formatter {
        file,
        comments: parsed.module.comments.clone(),
        next_comment: 0,
    };
    let doc = formatter.module(&parsed.module);
    Ok(render(&doc, WIDTH))
}

// ------------------------------------------------------------------ documents

#[derive(Clone, Debug)]
enum Doc {
    Text(String),
    /// A space when flat, a line break when broken.
    Line,
    /// Nothing when flat, a line break when broken.
    SoftLine,
    /// Always a line break; forces every enclosing group to break.
    HardLine,
    Nest(Box<Doc>),
    Group(Box<Doc>),
    Concat(Vec<Doc>),
    /// Tried in order; the first whose lines all fit is used, else the last.
    Alternatives(Vec<Doc>),
    /// Printed flat whatever the width; an alternative that must stay on one line.
    Flat(Box<Doc>),
}

fn text(value: impl Into<String>) -> Doc {
    Doc::Text(value.into())
}

fn concat(docs: Vec<Doc>) -> Doc {
    Doc::Concat(docs)
}

fn group(doc: Doc) -> Doc {
    Doc::Group(Box::new(doc))
}

fn nest(doc: Doc) -> Doc {
    Doc::Nest(Box::new(doc))
}

fn join(docs: Vec<Doc>, separator: Doc) -> Doc {
    let mut out = Vec::new();
    for (index, doc) in docs.into_iter().enumerate() {
        if index > 0 {
            out.push(separator.clone());
        }
        out.push(doc);
    }
    concat(out)
}

/// `open` items `close`, one per line with the closer at the outer indent
/// when the group breaks.
fn bracketed(open: &str, items: Vec<Doc>, close: &str) -> Doc {
    if items.is_empty() {
        return text(format!("{open}{close}"));
    }
    group(concat(vec![
        text(open),
        nest(concat(vec![
            Doc::SoftLine,
            join(items, concat(vec![text(","), Doc::Line])),
        ])),
        Doc::SoftLine,
        text(close),
    ]))
}

fn char_width(value: &str) -> usize {
    value.chars().count()
}

/// Width when printed flat, or `None` when the document contains a hard break.
fn flat_width(doc: &Doc) -> Option<usize> {
    match doc {
        Doc::Text(value) => Some(char_width(value)),
        Doc::Line => Some(1),
        Doc::SoftLine => Some(0),
        Doc::HardLine => None,
        Doc::Nest(inner) | Doc::Group(inner) | Doc::Flat(inner) => flat_width(inner),
        Doc::Concat(docs) => docs.iter().map(flat_width).sum(),
        Doc::Alternatives(alternatives) => flat_width(&alternatives[0]),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Flat,
    Break,
}

struct Printer {
    width: usize,
    out: String,
    column: usize,
}

fn render(doc: &Doc, width: usize) -> String {
    let mut printer = Printer {
        width,
        out: String::new(),
        column: 0,
    };
    printer.print(doc, 0, Mode::Break);
    let mut out = printer.out;
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    out
}

impl Printer {
    fn print(&mut self, doc: &Doc, indent: usize, mode: Mode) {
        match doc {
            Doc::Text(value) => self.push(value),
            Doc::Line => match mode {
                Mode::Flat => self.push(" "),
                Mode::Break => self.newline(indent),
            },
            Doc::SoftLine => {
                if mode == Mode::Break {
                    self.newline(indent);
                }
            }
            Doc::HardLine => self.newline(indent),
            Doc::Nest(inner) => self.print(inner, indent + INDENT, mode),
            Doc::Flat(inner) => self.print(inner, indent, Mode::Flat),
            Doc::Concat(docs) => {
                for inner in docs {
                    self.print(inner, indent, mode);
                }
            }
            Doc::Group(inner) => {
                let fits = mode == Mode::Flat
                    || flat_width(inner).is_some_and(|width| self.column + width <= self.width);
                self.print(inner, indent, if fits { Mode::Flat } else { Mode::Break });
            }
            Doc::Alternatives(alternatives) => {
                if mode == Mode::Flat {
                    return self.print(&alternatives[0], indent, Mode::Flat);
                }
                let (last, candidates) = alternatives
                    .split_last()
                    .expect("alternatives are never empty");
                for candidate in candidates {
                    let mut trial = Printer {
                        width: self.width,
                        out: String::new(),
                        column: self.column,
                    };
                    trial.print(candidate, indent, Mode::Break);
                    if trial.all_lines_fit(self.column) {
                        self.out.push_str(&trial.out);
                        self.column = trial.column;
                        return;
                    }
                }
                self.print(last, indent, Mode::Break);
            }
        }
    }

    fn push(&mut self, value: &str) {
        self.out.push_str(value);
        self.column += char_width(value);
    }

    fn newline(&mut self, indent: usize) {
        let trimmed = self.out.trim_end_matches(' ').len();
        self.out.truncate(trimmed);
        self.out.push('\n');
        for _ in 0..indent {
            self.out.push(' ');
        }
        self.column = indent;
    }

    fn all_lines_fit(&self, first_line_offset: usize) -> bool {
        self.out.split('\n').enumerate().all(|(index, line)| {
            let offset = if index == 0 { first_line_offset } else { 0 };
            offset + char_width(line) <= self.width
        })
    }
}

// ------------------------------------------------------------------ formatter

struct Formatter<'a> {
    file: &'a SourceFile,
    comments: Vec<Span>,
    next_comment: usize,
}

fn blank() -> Doc {
    concat(vec![Doc::HardLine, Doc::HardLine])
}

impl Formatter<'_> {
    fn line_of(&self, offset: usize) -> usize {
        self.file.position(offset).line
    }

    /// Full-line comments that come before `offset`, each on its own line.
    fn comments_before(&mut self, offset: usize) -> Vec<Doc> {
        let mut docs = Vec::new();
        while self.next_comment < self.comments.len()
            && self.comments[self.next_comment].start < offset
        {
            let span = self.comments[self.next_comment];
            docs.push(text(self.file.slice(span).trim_end()));
            docs.push(Doc::HardLine);
            self.next_comment += 1;
        }
        docs
    }

    /// A comment on the same line as the construct that ends at `end`.
    fn trailing_comment(&mut self, end: usize) -> Option<Doc> {
        self.trailing_comment_before(end, usize::MAX)
    }

    /// A comment on the same line as the construct that ends at `end` and
    /// before `limit`: the trailing comment of an element inside a bracket,
    /// a clause or an arm, never the comment that closes the whole statement.
    fn trailing_comment_before(&mut self, end: usize, limit: usize) -> Option<Doc> {
        let span = *self.comments.get(self.next_comment)?;
        if self.line_of(span.start) == self.line_of(end.saturating_sub(1))
            && span.start >= end
            && span.start < limit
        {
            self.next_comment += 1;
            return Some(text(format!("  {}", self.file.slice(span).trim_end())));
        }
        None
    }

    fn has_comments_before(&self, offset: usize) -> bool {
        self.comments
            .get(self.next_comment)
            .is_some_and(|span| span.start < offset)
    }

    /// The break before a clause or an arm that starts at `offset`: a soft line
    /// when nothing precedes it, otherwise a hard line with the full-line
    /// comments that stand before it, each on its own line. `after_trailing`
    /// makes the break hard because the previous element ended in a comment.
    fn clause_break(&mut self, offset: usize, after_trailing: bool) -> Doc {
        let comments = self.comments_before(offset);
        if comments.is_empty() && !after_trailing {
            return Doc::Line;
        }
        let mut parts = vec![Doc::HardLine];
        parts.extend(comments);
        concat(parts)
    }

    /// A bracketed list whose elements keep the comments written around them.
    /// Without comments it is the ordinary `bracketed` group; with any, the
    /// list breaks one element per line and each comment stays on its line.
    fn list_with_comments<E>(
        &mut self,
        open: &str,
        elements: &[E],
        span_of: impl Fn(&E) -> Span,
        mut print: impl FnMut(&mut Self, &E) -> Doc,
        close: &str,
        limit: usize,
    ) -> Doc {
        let mut items = Vec::new();
        let mut any_comment = false;
        for element in elements {
            let span = span_of(element);
            let leading = self.comments_before(span.start);
            let doc = print(self, element);
            let trailing = self.trailing_comment_before(span.end, limit);
            any_comment |= !leading.is_empty() || trailing.is_some();
            items.push((leading, doc, trailing));
        }
        if !any_comment {
            let docs = items.into_iter().map(|(_, doc, _)| doc).collect();
            return bracketed(open, docs, close);
        }
        let last = items.len() - 1;
        let mut inner = Vec::new();
        for (index, (leading, doc, trailing)) in items.into_iter().enumerate() {
            inner.push(Doc::HardLine);
            inner.extend(leading);
            inner.push(doc);
            if index < last {
                inner.push(text(","));
            }
            if let Some(trailing) = trailing {
                inner.push(trailing);
            }
        }
        concat(vec![
            text(open),
            nest(concat(inner)),
            Doc::HardLine,
            text(close),
        ])
    }

    fn module(&mut self, module: &Module) -> Doc {
        let mut parts = self.comments_before(module.span.start.max(1));
        let path: Vec<&str> = module.name.iter().map(|name| name.text.as_str()).collect();
        parts.push(text(format!("module {}", path.join("."))));
        parts.push(nest(self.docs(&module.docs)));
        if !module.imports.is_empty() {
            parts.push(blank());
            for (index, import) in module.imports.iter().enumerate() {
                if index > 0 {
                    parts.push(Doc::HardLine);
                }
                parts.extend(self.comments_before(import.span.start));
                parts.push(self.import(import));
            }
        }
        for item in &module.items {
            parts.push(blank());
            parts.extend(self.comments_before(item.span().start));
            parts.push(self.item(item));
        }
        parts.push(Doc::HardLine);
        let leftover: Vec<Span> = self.comments[self.next_comment..].to_vec();
        for span in leftover {
            parts.push(text(self.file.slice(span).trim_end()));
            parts.push(Doc::HardLine);
        }
        concat(parts)
    }

    /// An import on one line; an `exposing` list wider than the line breaks
    /// after its commas, one name per line indented once, since a line break
    /// after a comma carries no meaning (decision V12, as `needs_doc`).
    fn import(&self, import: &Import) -> Doc {
        let path: Vec<&str> = import.path.iter().map(|name| name.text.as_str()).collect();
        let mut head = format!("import {}", path.join("."));
        if let Some(alias) = &import.alias {
            head.push_str(&format!(" as {}", alias.text));
        }
        if import.exposing.is_empty() {
            return text(head);
        }
        let names: Vec<Doc> = import
            .exposing
            .iter()
            .map(|name| text(name.text.clone()))
            .collect();
        group(concat(vec![
            text(head),
            text(" exposing "),
            nest(join(names, concat(vec![text(","), Doc::Line]))),
        ]))
    }

    /// Documentation clauses, each on its own line; prose is never wrapped,
    /// since a clause's text runs to the end of its line (decision V2).
    fn docs(&mut self, docs: &Docs) -> Doc {
        let mut parts = Vec::new();
        if let Some(purpose) = &docs.purpose {
            parts.push(Doc::HardLine);
            parts.push(clause_text("purpose:", purpose));
        }
        if !docs.tags.is_empty() {
            parts.push(Doc::HardLine);
            parts.push(clause_text("tags:", &docs.tags.join(", ")));
        }
        if !docs.see_also.is_empty() {
            parts.push(Doc::HardLine);
            parts.push(clause_text("see also:", &docs.see_also.join(", ")));
        }
        if let Some(deprecated) = &docs.deprecated {
            parts.push(Doc::HardLine);
            parts.push(clause_text("deprecated:", deprecated));
        }
        if docs.expose_as_tool {
            parts.push(Doc::HardLine);
            parts.push(text("expose as tool"));
        }
        for example in &docs.examples {
            parts.push(Doc::HardLine);
            parts.push(self.example(example));
        }
        concat(parts)
    }

    fn example(&mut self, example: &Example) -> Doc {
        let mark = self.next_comment;
        let expression = self.expr(&example.expression);
        let outcome = match &example.outcome {
            ExampleOutcome::Is(value) => concat(vec![text("is "), self.expr(value)]),
            ExampleOutcome::FailsWith(pattern) => {
                concat(vec![text("fails with "), self.pattern(pattern)])
            }
        };
        // one line; or break before `is`; or break the call's arguments and
        // keep `) is value` on the closing line
        let flat = Doc::Flat(Box::new(concat(vec![
            text("example: "),
            expression.clone(),
            text(" "),
            outcome.clone(),
        ])));
        let before_is = concat(vec![
            text("example: "),
            Doc::Flat(Box::new(expression.clone())),
            nest(concat(vec![Doc::HardLine, outcome.clone()])),
        ]);
        let mut alternatives = vec![flat, before_is];
        self.next_comment = mark;
        if let Some(broken_call) = self.expr_with_broken_arguments(&example.expression) {
            alternatives.push(concat(vec![
                text("example: "),
                broken_call,
                text(" "),
                outcome,
            ]));
        }
        Doc::Alternatives(alternatives)
    }

    // -------------------------------------------------------------- items

    fn item(&mut self, item: &Item) -> Doc {
        match item {
            Item::Function(function) => self.function(function, 0),
            Item::Type(type_def) => self.type_def(type_def),
            Item::Ability(ability) => self.ability(ability),
            Item::Implementation(implementation) => self.implementation(implementation),
            Item::Constant(constant) => {
                let head = format!(
                    "{}let {}: {} be",
                    if constant.public { "public " } else { "" },
                    constant.name.text,
                    type_text(&constant.ty)
                );
                concat(vec![
                    text(head),
                    self.value_after(&constant.value),
                    nest(self.docs(&constant.docs)),
                ])
            }
            Item::Test(test) => {
                let mut head = vec![text(format!("test {}", quote_text(&test.name)))];
                if !test.needs.is_empty() {
                    head.push(text(" "));
                    head.push(needs_doc(&test.needs));
                }
                if let Some(recording) = &test.replays {
                    head.push(text(format!(" replays {}", quote_text(recording))));
                }
                concat(vec![
                    concat(head),
                    nest(self.block(&test.body)),
                    Doc::HardLine,
                    text("end"),
                ])
            }
        }
    }

    /// `indent` is the column of the `function` keyword, used to decide
    /// whether the signature clauses share the head line.
    fn function(&mut self, function: &Function, indent: usize) -> Doc {
        let mut head = String::new();
        if function.public {
            head.push_str("public ");
        }
        head.push_str(&format!("function {}", function.name.text));
        let params: Vec<Doc> = function
            .params
            .iter()
            .map(|param| match &param.ty {
                Some(ty) => text(format!("{}: {}", param.name.text, type_text(ty))),
                None => text(param.name.text.clone()),
            })
            .collect();
        let params_doc = if params.is_empty() {
            text("()")
        } else {
            bracketed("(", params, ")")
        };
        let mut clauses = Vec::new();
        if let Some(returns) = &function.returns {
            clauses.push(text(format!("returns {}", type_text(returns))));
        }
        if !function.fails.is_empty() {
            let names: Vec<String> = function.fails.iter().map(type_text).collect();
            clauses.push(text(format!("or fails with {}", names.join(" or "))));
        }
        if !function.needs.is_empty() {
            clauses.push(needs_doc(&function.needs));
        }
        if let Some(for_any) = &function.type_params {
            clauses.push(text(for_any_text(for_any)));
        }
        let has_clauses = !clauses.is_empty();
        let mut clause_docs = Vec::new();
        for clause in clauses {
            clause_docs.push(Doc::Line);
            clause_docs.push(clause);
        }
        let head_doc = group(concat(vec![
            text(head),
            params_doc,
            nest(concat(clause_docs)),
        ]));
        let clauses_on_head = flat_width(&head_doc).is_some_and(|width| indent + width <= WIDTH);
        let mut parts = vec![head_doc, nest(self.docs(&function.docs))];
        if let Some(body) = &function.body {
            if !docs_empty(&function.docs) || (has_clauses && !clauses_on_head) {
                parts.push(Doc::HardLine);
            }
            parts.push(nest(self.block(body)));
            parts.push(Doc::HardLine);
            parts.push(text("end"));
        }
        concat(parts)
    }

    fn type_def(&mut self, type_def: &TypeDef) -> Doc {
        let mut head = String::new();
        if type_def.public {
            head.push_str("public ");
        }
        head.push_str(&format!("type {}", type_def.name.text));
        if !type_def.type_params.is_empty() {
            let params: Vec<&str> = type_def
                .type_params
                .iter()
                .map(|param| param.text.as_str())
                .collect();
            head.push_str(&format!(" of {}", params.join(", ")));
        }
        match &type_def.kind {
            TypeKind::Subtype { base, refinement } => {
                let mut parts = vec![text(format!("{head} is {}", type_text(base)))];
                if let Some(refinement) = refinement {
                    parts.push(nest(self.refinement(refinement)));
                }
                concat(vec![group(concat(parts)), nest(self.docs(&type_def.docs))])
            }
            TypeKind::Record { fields, derives } => {
                let mut parts = vec![text(head), nest(self.docs(&type_def.docs))];
                for field in fields {
                    let mut line = vec![Doc::HardLine];
                    line.extend(self.comments_before(field.span.start));
                    line.push(self.field(field, true));
                    line.extend(self.trailing_comment_before(field.span.end, type_def.span.end));
                    parts.push(nest(concat(line)));
                }
                for derive in derives {
                    let mut line = vec![Doc::HardLine];
                    line.extend(self.comments_before(derive.span.start));
                    line.push(self.derive(derive));
                    line.extend(self.trailing_comment_before(derive.span.end, type_def.span.end));
                    parts.push(nest(concat(line)));
                }
                parts.push(Doc::HardLine);
                parts.push(text("end"));
                concat(parts)
            }
            TypeKind::Sum { variants, derives } => {
                let mut parts = vec![
                    text(format!("{head} is one of")),
                    nest(self.docs(&type_def.docs)),
                ];
                for variant in variants {
                    let mut line = vec![Doc::HardLine];
                    line.extend(self.comments_before(variant.span.start));
                    let mut variant_doc = vec![text(variant.name.text.clone())];
                    if !variant.fields.is_empty() {
                        variant_doc.push(self.list_with_comments(
                            "(",
                            &variant.fields,
                            |field| field.span,
                            |this, field| this.field(field, false),
                            ")",
                            variant.span.end,
                        ));
                    }
                    line.push(concat(variant_doc));
                    line.extend(self.trailing_comment_before(variant.span.end, type_def.span.end));
                    parts.push(nest(concat(line)));
                }
                for derive in derives {
                    let mut line = vec![Doc::HardLine];
                    line.extend(self.comments_before(derive.span.start));
                    line.push(self.derive(derive));
                    line.extend(self.trailing_comment_before(derive.span.end, type_def.span.end));
                    parts.push(nest(concat(line)));
                }
                parts.push(Doc::HardLine);
                parts.push(text("end"));
                concat(parts)
            }
        }
    }

    fn field(&mut self, field: &Field, with_has: bool) -> Doc {
        let mut head = String::new();
        if with_has {
            head.push_str("has ");
        }
        head.push_str(&format!("{}: {}", field.name.text, type_text(&field.ty)));
        let mut parts = vec![text(head)];
        if let Some(refinement) = &field.refinement {
            parts.push(nest(self.refinement(refinement)));
        }
        if let Some(external) = &field.external_name {
            parts.push(text(format!(" as {}", quote_text(external))));
        }
        group(concat(parts))
    }

    /// ` where a and b`, breaking before `where` and before each `and` / `or`
    /// together when the line is too long.
    fn refinement(&mut self, condition: &Expr) -> Doc {
        let mut operands = Vec::new();
        let mut operators = Vec::new();
        flatten_logic(condition, &mut operands, &mut operators);
        let mut parts = vec![Doc::Line, text("where "), self.expr(operands[0])];
        for (operand, op) in operands[1..].iter().zip(operators) {
            parts.push(Doc::Line);
            parts.push(text(format!("{} ", op.spelling())));
            parts.push(self.expr(operand));
        }
        concat(parts)
    }

    fn derive(&mut self, derive: &Derive) -> Doc {
        let mut line = format!("can {}", derive.ability.text);
        if !derive.by.is_empty() {
            let names: Vec<&str> = derive.by.iter().map(|name| name.text.as_str()).collect();
            line.push_str(&format!(" by {}", names.join(", ")));
        }
        text(line)
    }

    fn ability(&mut self, ability: &AbilityDecl) -> Doc {
        let mut head = String::new();
        if ability.public {
            head.push_str("public ");
        }
        head.push_str(&format!("ability {}", ability.name.text));
        if !ability.type_params.is_empty() {
            let params: Vec<&str> = ability
                .type_params
                .iter()
                .map(|param| param.text.as_str())
                .collect();
            head.push_str(&format!(" of {}", params.join(", ")));
        }
        if !ability.requirements.is_empty() {
            let requirements: Vec<String> = ability
                .requirements
                .iter()
                .map(|ty| format!("self can {}", type_text(ty)))
                .collect();
            head.push_str(&format!(" where {}", requirements.join(" and ")));
        }
        let mut parts = vec![text(head), nest(self.docs(&ability.docs))];
        for function in &ability.functions {
            parts.push(nest(concat(vec![
                Doc::HardLine,
                self.function(function, INDENT),
            ])));
        }
        parts.push(Doc::HardLine);
        parts.push(text("end"));
        concat(parts)
    }

    fn implementation(&mut self, implementation: &AbilityImpl) -> Doc {
        let mut parts = vec![text(format!(
            "ability {} for {}",
            type_text(&implementation.ability),
            type_text(&implementation.target)
        ))];
        if let Some(for_any) = &implementation.type_params {
            parts.push(nest(concat(vec![
                Doc::HardLine,
                text(for_any_text(for_any)),
            ])));
        }
        for function in &implementation.functions {
            parts.push(nest(concat(vec![
                Doc::HardLine,
                self.function(function, INDENT),
            ])));
        }
        parts.push(Doc::HardLine);
        parts.push(text("end"));
        concat(parts)
    }

    // -------------------------------------------------------------- statements

    /// The statements of a block, each on its own line, keeping one blank line
    /// where the source had any.
    fn block(&mut self, block: &Block) -> Doc {
        let mut parts = Vec::new();
        let mut previous_end: Option<usize> = None;
        for statement in &block.statements {
            let comments = self.comments_before(statement.span.start);
            if let Some(end) = previous_end {
                if self.line_of(end) + 1 < self.line_of(statement.span.start) && comments.is_empty()
                {
                    parts.push(Doc::HardLine);
                }
            }
            parts.push(Doc::HardLine);
            parts.extend(comments);
            parts.push(self.statement(statement));
            if let Some(trailing) = self.trailing_comment(statement.span.end) {
                parts.push(trailing);
            }
            previous_end = Some(statement.span.end);
        }
        concat(parts)
    }

    fn statement(&mut self, statement: &Stmt) -> Doc {
        match &statement.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                value,
            } => {
                let mut head = String::from("let ");
                if *mutable {
                    head.push_str("mutable ");
                }
                head.push_str(&name.text);
                if let Some(ty) = ty {
                    head.push_str(&format!(": {}", type_text(ty)));
                }
                head.push_str(" be");
                concat(vec![text(head), self.value_after(value)])
            }
            StmtKind::Change { name, value } => concat(vec![
                text(format!("change {} to", name.text)),
                self.value_after(value),
            ]),
            StmtKind::If {
                branches,
                otherwise,
            } => {
                if branches.len() == 1 && otherwise.is_none() && branches[0].1.statements.len() == 1
                {
                    let (condition, block) = &branches[0];
                    let body = self.statement(&block.statements[0]);
                    return group(concat(vec![
                        text("if "),
                        self.expr(condition),
                        text(" then"),
                        nest(concat(vec![Doc::Line, body])),
                        Doc::Line,
                        text("end"),
                    ]));
                }
                let mut parts = Vec::new();
                for (index, (condition, block)) in branches.iter().enumerate() {
                    if index > 0 {
                        parts.push(Doc::HardLine);
                        parts.extend(self.comments_before(condition.span.start));
                        parts.push(text("otherwise "));
                    }
                    parts.push(text("if "));
                    parts.push(self.expr(condition));
                    parts.push(text(" then"));
                    parts.push(nest(self.block(block)));
                }
                if let Some(block) = otherwise {
                    parts.push(Doc::HardLine);
                    parts.push(text("otherwise"));
                    parts.push(nest(self.block(block)));
                }
                parts.push(Doc::HardLine);
                parts.push(text("end"));
                concat(parts)
            }
            StmtKind::Match {
                subject,
                arms,
                otherwise,
            } => {
                let mut parts = vec![text("match "), self.expr(subject)];
                for arm in arms {
                    let comments = self.comments_before(arm.span.start);
                    let mut head = vec![text("when "), self.pattern(&arm.pattern)];
                    if let Some(guard) = &arm.guard {
                        head.push(text(" where "));
                        head.push(self.expr(guard));
                    }
                    head.push(text(" then"));
                    let mut arm_doc = vec![Doc::HardLine];
                    arm_doc.extend(comments);
                    arm_doc.push(self.arm_body(concat(head), &arm.body));
                    parts.push(nest(concat(arm_doc)));
                }
                if let Some(block) = otherwise {
                    parts.push(nest(concat(vec![
                        Doc::HardLine,
                        self.arm_body(text("otherwise"), block),
                    ])));
                }
                parts.push(Doc::HardLine);
                parts.push(text("end"));
                concat(parts)
            }
            StmtKind::ForEach {
                bindings,
                source,
                filter,
                order,
                body,
            } => {
                let mut head = vec![
                    text(format!("for each {} ", names(bindings))),
                    self.loop_source(source),
                ];
                let mut clauses = Vec::new();
                if let Some(filter) = filter {
                    clauses.push(Doc::Line);
                    clauses.push(text("where "));
                    clauses.push(self.expr(filter));
                }
                if let Some(order) = order {
                    clauses.push(Doc::Line);
                    clauses.push(self.ordering(order));
                }
                head.push(nest(concat(clauses)));
                concat(vec![
                    group(concat(head)),
                    nest(self.block(body)),
                    Doc::HardLine,
                    text("end"),
                ])
            }
            StmtKind::RepeatUntil { condition, body } => concat(vec![
                text("repeat until "),
                self.expr(condition),
                nest(self.block(body)),
                Doc::HardLine,
                text("end"),
            ]),
            StmtKind::RunConcurrently { within, body } => {
                let mut head = vec![text("run concurrently")];
                if let Some(within) = within {
                    head.push(text(" within "));
                    head.push(self.expr(within));
                }
                concat(vec![
                    concat(head),
                    nest(self.block(body)),
                    Doc::HardLine,
                    text("end"),
                ])
            }
            StmtKind::Return(None) => text("return"),
            StmtKind::Return(Some(value)) => {
                concat(vec![text("return"), self.value_on_line(value)])
            }
            StmtKind::Fail(None) => text("fail"),
            StmtKind::Fail(Some(value)) => concat(vec![text("fail with "), self.expr(value)]),
            StmtKind::Crash(value) => concat(vec![text("crash with "), self.expr(value)]),
            StmtKind::Break => text("break"),
            StmtKind::Continue => text("continue"),
            StmtKind::Ignore(value) => concat(vec![text("ignore "), self.expr(value)]),
            StmtKind::Check(value) => concat(vec![text("check "), self.expr(value)]),
            StmtKind::Expression(value) => self.expr(value),
        }
    }

    /// A `when ... then` or `otherwise` head with its block: one statement
    /// stays on the head line when it fits, otherwise the block follows.
    fn arm_body(&mut self, head: Doc, block: &Block) -> Doc {
        if block.statements.len() == 1 && !self.has_comments_before(block.statements[0].span.start)
        {
            let body = self.statement(&block.statements[0]);
            if flat_width(&body).is_some() {
                return group(concat(vec![head, nest(concat(vec![Doc::Line, body]))]));
            }
            return concat(vec![head, nest(concat(vec![Doc::HardLine, body]))]);
        }
        concat(vec![head, nest(self.block(block))])
    }

    /// The value after `return`, which stays on its line (decision V2: a
    /// `return` alone is a statement, so a value cannot start the next line).
    /// A query's clauses nest under it when they do not fit; an `if` or
    /// `match` expression spans lines as it does elsewhere.
    fn value_on_line(&mut self, value: &Expr) -> Doc {
        let inner = match &value.kind {
            ExprKind::Otherwise { value: inner, .. } => &inner.kind,
            other => other,
        };
        match inner {
            ExprKind::Query(_) => group(concat(vec![text(" "), nest(self.expr(value))])),
            _ => concat(vec![text(" "), self.expr(value)]),
        }
    }

    /// The value after `be` or `to`: queries and conditionals may move to the
    /// next line; a `match` always does.
    fn value_after(&mut self, value: &Expr) -> Doc {
        // an `otherwise` wrapped around a query or conditional lays out like the
        // value itself, so that the clauses stay nested under `be`
        let inner = match &value.kind {
            ExprKind::Otherwise { value: inner, .. } => &inner.kind,
            other => other,
        };
        match inner {
            ExprKind::Query(_) | ExprKind::If { .. } => {
                group(nest(concat(vec![Doc::Line, self.expr(value)])))
            }
            ExprKind::Match { .. } => nest(concat(vec![Doc::HardLine, self.expr(value)])),
            _ => concat(vec![text(" "), self.expr(value)]),
        }
    }

    fn loop_source(&mut self, source: &Expr) -> Doc {
        match &source.kind {
            ExprKind::Range { .. } => self.expr(source),
            _ => concat(vec![text("in "), self.expr(source)]),
        }
    }

    fn ordering(&mut self, order: &Ordering) -> Doc {
        let mut parts = vec![text("sorted by "), self.expr(&order.key)];
        if order.descending {
            parts.push(text(" descending"));
        }
        concat(parts)
    }

    // -------------------------------------------------------------- expressions

    fn expr(&mut self, expr: &Expr) -> Doc {
        match &expr.kind {
            ExprKind::Integer(value) | ExprKind::Decimal(value) => text(value.clone()),
            ExprKind::Text { pieces, block } => self.text_literal(pieces, *block),
            ExprKind::RawText(value) => text(format!("raw \"{value}\"")),
            ExprKind::Boolean(true) => text("true"),
            ExprKind::Boolean(false) => text("false"),
            ExprKind::Nothing => text("nothing"),
            ExprKind::SelfValue => text("self"),
            ExprKind::Name(name) => text(name.text.clone()),
            ExprKind::TypeName(name) => text(name.text.clone()),
            ExprKind::Member { base, name } => {
                concat(vec![self.expr(base), text(format!(".{}", name.text))])
            }
            ExprKind::Call { callee, args } => {
                let callee = self.expr(callee);
                concat(vec![callee, self.arguments(args, expr.span.end)])
            }
            ExprKind::Construct { name, args } => concat(vec![
                text(name.text.clone()),
                self.arguments(args, expr.span.end),
            ]),
            ExprKind::List(items) => self.list_with_comments(
                "[",
                items,
                |item| item.span,
                |this, item| this.expr(item),
                "]",
                expr.span.end,
            ),
            ExprKind::Map(entries) => self.list_with_comments(
                "{",
                entries,
                |(key, value)| key.span.join(value.span),
                |this, (key, value)| {
                    let key = this.expr(key);
                    concat(vec![key, text(": "), this.expr(value)])
                },
                "}",
                expr.span.end,
            ),
            ExprKind::Range { from, to, by } => {
                let mut parts = vec![text("from "), self.expr(from), text(" to "), self.expr(to)];
                if let Some(by) = by {
                    parts.push(text(" by "));
                    parts.push(self.expr(by));
                }
                concat(parts)
            }
            ExprKind::Not(inner) => concat(vec![text("not "), self.expr(inner)]),
            ExprKind::Binary {
                op: op @ (BinaryOp::And | BinaryOp::Or),
                ..
            } => {
                let mut operands = Vec::new();
                let mut operators = Vec::new();
                flatten_logic(expr, &mut operands, &mut operators);
                let mut parts = vec![self.expr(operands[0])];
                let mut rest = Vec::new();
                for (operand, operator) in operands[1..].iter().zip(operators) {
                    rest.push(Doc::Line);
                    rest.push(text(format!("{} ", operator.spelling())));
                    rest.push(self.expr(operand));
                }
                let _ = op;
                parts.push(nest(concat(rest)));
                group(concat(parts))
            }
            ExprKind::Binary { op, left, right } => concat(vec![
                self.expr(left),
                text(format!(" {} ", op.spelling())),
                self.expr(right),
            ]),
            ExprKind::With { base, updates } => {
                // an update starts with a field name, which could not continue
                // a line, so a record update breaks before `with` only
                let updates: Vec<Doc> = updates.iter().map(|arg| self.argument(arg)).collect();
                group(concat(vec![
                    self.expr(base),
                    nest(concat(vec![
                        Doc::Line,
                        text("with "),
                        join(updates, text(", ")),
                    ])),
                ]))
            }
            ExprKind::Otherwise { value, fallback } => {
                // `otherwise` never starts a line (decision V2). When the
                // statement does not fit, the longer of the two argument lists
                // (the call's before `otherwise`, the fallback's) breaks
                // first, then the other, then the value's own groups, then
                // both lists; the ordinary document is the last resort.
                let mark = self.next_comment;
                let value_flat = self.expr(value);
                let fallback_flat = self.outcome(fallback);
                let value_width = flat_width(&value_flat).unwrap_or(usize::MAX);
                let fallback_width = flat_width(&fallback_flat).unwrap_or(usize::MAX);
                let otherwise = || text(" otherwise ");
                let flat = Doc::Flat(Box::new(concat(vec![
                    value_flat.clone(),
                    otherwise(),
                    fallback_flat.clone(),
                ])));
                self.next_comment = mark;
                let value_broken = self.expr_with_broken_arguments(value).map(|broken| {
                    concat(vec![
                        broken,
                        otherwise(),
                        Doc::Flat(Box::new(fallback_flat.clone())),
                    ])
                });
                self.next_comment = mark;
                let fallback_broken = self.outcome_with_broken_arguments(fallback).map(|broken| {
                    concat(vec![
                        Doc::Flat(Box::new(value_flat.clone())),
                        otherwise(),
                        broken,
                    ])
                });
                self.next_comment = mark;
                let ordinary = concat(vec![value_flat, otherwise(), fallback_flat]);
                let both_broken = match self.expr_with_broken_arguments(value) {
                    Some(broken_value) => {
                        self.outcome_with_broken_arguments(fallback)
                            .map(|broken_fallback| {
                                concat(vec![broken_value, otherwise(), broken_fallback])
                            })
                    }
                    None => None,
                };
                // built last so that the comment bookkeeping ends after the
                // whole expression, whichever alternative is printed
                self.next_comment = mark;
                let last_resort =
                    concat(vec![self.expr(value), otherwise(), self.outcome(fallback)]);
                let mut alternatives = vec![flat];
                let (longer, shorter) = if fallback_width > value_width {
                    (fallback_broken, value_broken)
                } else {
                    (value_broken, fallback_broken)
                };
                alternatives.extend(longer);
                alternatives.extend(shorter);
                alternatives.push(ordinary);
                alternatives.extend(both_broken);
                alternatives.push(last_resort);
                Doc::Alternatives(alternatives)
            }
            ExprKind::If {
                branches,
                otherwise,
            } => {
                let mut parts = Vec::new();
                let mut after_trailing = false;
                for (index, (condition, outcome)) in branches.iter().enumerate() {
                    if index > 0 {
                        parts.push(self.clause_break(condition.span.start, after_trailing));
                        parts.push(text("otherwise "));
                    }
                    parts.push(text("if "));
                    parts.push(self.expr(condition));
                    parts.push(text(" then "));
                    parts.push(self.outcome(outcome));
                    let trailing = self.trailing_comment_before(outcome.span().end, expr.span.end);
                    after_trailing = trailing.is_some();
                    parts.extend(trailing);
                }
                parts.push(self.clause_break(otherwise.span().start, after_trailing));
                parts.push(text("otherwise "));
                parts.push(self.outcome(otherwise));
                let trailing = self.trailing_comment_before(otherwise.span().end, expr.span.end);
                after_trailing = trailing.is_some();
                parts.extend(trailing);
                parts.push(if after_trailing {
                    Doc::HardLine
                } else {
                    Doc::Line
                });
                parts.push(text("end"));
                group(concat(parts))
            }
            ExprKind::Match {
                subject,
                arms,
                otherwise,
            } => {
                let mut parts = vec![text("match "), self.expr(subject)];
                for arm in arms {
                    let comments = self.comments_before(arm.span.start);
                    let mut head = vec![text("when "), self.pattern(&arm.pattern)];
                    if let Some(guard) = &arm.guard {
                        head.push(text(" where "));
                        head.push(self.expr(guard));
                    }
                    head.push(text(" then"));
                    let body = self.outcome(&arm.body);
                    let mut arm_doc = vec![Doc::HardLine];
                    arm_doc.extend(comments);
                    arm_doc.push(group(concat(vec![
                        concat(head),
                        nest(concat(vec![Doc::Line, body])),
                    ])));
                    arm_doc.extend(self.trailing_comment_before(arm.span.end, expr.span.end));
                    parts.push(nest(concat(arm_doc)));
                }
                if let Some(outcome) = otherwise {
                    let comments = self.comments_before(outcome.span().start);
                    let body = self.outcome(outcome);
                    let mut arm_doc = vec![Doc::HardLine];
                    arm_doc.extend(comments);
                    arm_doc.push(group(concat(vec![
                        text("otherwise"),
                        nest(concat(vec![Doc::Line, body])),
                    ])));
                    arm_doc.extend(self.trailing_comment_before(outcome.span().end, expr.span.end));
                    parts.push(nest(concat(arm_doc)));
                }
                parts.push(Doc::HardLine);
                parts.push(text("end"));
                concat(parts)
            }
            ExprKind::Query(query) => self.query(query),
            ExprKind::Paren(inner) => concat(vec![text("("), self.expr(inner), text(")")]),
        }
    }

    /// The expression with the arguments of its outermost call broken one per
    /// line, for the third layout of an example clause.
    fn expr_with_broken_arguments(&mut self, expr: &Expr) -> Option<Doc> {
        let (head, args) = match &expr.kind {
            ExprKind::Call { callee, args } if !args.is_empty() => (self.expr(callee), args),
            ExprKind::Construct { name, args } if !args.is_empty() => {
                (text(name.text.clone()), args)
            }
            _ => return None,
        };
        let items: Vec<Doc> = args.iter().map(|arg| self.argument(arg)).collect();
        Some(concat(vec![
            head,
            text("("),
            nest(concat(vec![
                Doc::HardLine,
                join(items, concat(vec![text(","), Doc::HardLine])),
            ])),
            Doc::HardLine,
            text(")"),
        ]))
    }

    /// An outcome with the arguments of its outermost call broken one per
    /// line, for a fallback that does not fit after `otherwise`.
    fn outcome_with_broken_arguments(&mut self, outcome: &Outcome) -> Option<Doc> {
        let (head, expr) = match outcome {
            Outcome::Value(expr) => ("", expr),
            Outcome::Fail(Some(expr), _) => ("fail with ", expr),
            Outcome::Return(Some(expr), _) => ("return ", expr),
            Outcome::Crash(expr, _) => ("crash with ", expr),
            _ => return None,
        };
        let broken = self.expr_with_broken_arguments(expr)?;
        Some(concat(vec![text(head), broken]))
    }

    /// `(a, b)`; `limit` is where the closing parenthesis is, so that a comment
    /// after the last argument is only taken when the list already spans lines.
    fn arguments(&mut self, args: &[Arg], limit: usize) -> Doc {
        self.list_with_comments(
            "(",
            args,
            |arg| arg.span,
            |this, arg| this.argument(arg),
            ")",
            limit,
        )
    }

    fn argument(&mut self, arg: &Arg) -> Doc {
        match &arg.name {
            Some(name) => concat(vec![
                text(format!("{}: ", name.text)),
                self.expr(&arg.value),
            ]),
            None => self.expr(&arg.value),
        }
    }

    fn outcome(&mut self, outcome: &Outcome) -> Doc {
        match outcome {
            Outcome::Value(expr) => self.expr(expr),
            Outcome::Fail(None, _) => text("fail"),
            Outcome::Fail(Some(value), _) => concat(vec![text("fail with "), self.expr(value)]),
            Outcome::Return(None, _) => text("return"),
            Outcome::Return(Some(value), _) => concat(vec![text("return "), self.expr(value)]),
            Outcome::Crash(value, _) => concat(vec![text("crash with "), self.expr(value)]),
            Outcome::Break(_) => text("break"),
            Outcome::Continue(_) => text("continue"),
        }
    }

    fn query(&mut self, query: &Query) -> Doc {
        let sources: Vec<Doc> = query
            .sources
            .iter()
            .map(|source| {
                concat(vec![
                    text(format!("{} ", names(&source.bindings))),
                    self.loop_source(&source.source),
                ])
            })
            .collect();
        let mut parts = vec![text("for each "), join(sources, text(", "))];
        if query.concurrently {
            parts.push(text(" concurrently"));
            if let Some(within) = &query.within {
                parts.push(text(" within "));
                parts.push(self.expr(within));
            }
        }
        // the end of the sources line, for a trailing comment on it
        let sources_end = query
            .within
            .as_ref()
            .map(|within| within.span.end)
            .unwrap_or_else(|| {
                query
                    .sources
                    .last()
                    .map_or(query.span.start, |s| s.source.span.end)
            });
        let limit = query.span.end;
        let mut trailing = self.trailing_comment_before(sources_end, limit);
        let mut after_trailing = trailing.is_some();
        parts.extend(trailing.take());
        if let Some(filter) = &query.filter {
            self.query_clause(
                &mut parts,
                "where ",
                Some(filter),
                filter.span,
                limit,
                &mut after_trailing,
            );
        }
        if let Some(order) = &query.order {
            parts.push(self.clause_break(order.key.span.start, after_trailing));
            parts.push(text("sorted by "));
            parts.push(self.expr(&order.key));
            if order.descending {
                parts.push(text(" descending"));
            }
            let trailing = self.trailing_comment_before(order.key.span.end, limit);
            after_trailing = trailing.is_some();
            parts.extend(trailing);
        }
        if let Some(key) = &query.group_by {
            self.query_clause(
                &mut parts,
                "group by ",
                Some(key),
                key.span,
                limit,
                &mut after_trailing,
            );
        }
        let terminal_span = Span::new(limit, limit);
        match &query.terminal {
            QueryTerminal::Collect(value) => self.query_clause(
                &mut parts,
                "collect ",
                Some(value),
                value.span,
                limit,
                &mut after_trailing,
            ),
            QueryTerminal::Sum(value) => self.query_clause(
                &mut parts,
                "sum ",
                Some(value),
                value.span,
                limit,
                &mut after_trailing,
            ),
            QueryTerminal::Count => self.query_clause(
                &mut parts,
                "count",
                None,
                terminal_span,
                limit,
                &mut after_trailing,
            ),
            QueryTerminal::First => self.query_clause(
                &mut parts,
                "first",
                None,
                terminal_span,
                limit,
                &mut after_trailing,
            ),
            QueryTerminal::Any(value) => self.query_clause(
                &mut parts,
                "any ",
                Some(value),
                value.span,
                limit,
                &mut after_trailing,
            ),
            QueryTerminal::All(value) => self.query_clause(
                &mut parts,
                "all ",
                Some(value),
                value.span,
                limit,
                &mut after_trailing,
            ),
            QueryTerminal::None => {}
        }
        concat(parts)
    }

    /// One clause of a query: the break before it (with the comments that
    /// stand before the clause), the keyword and value, and a trailing comment.
    #[allow(clippy::too_many_arguments)]
    fn query_clause(
        &mut self,
        parts: &mut Vec<Doc>,
        keyword: &str,
        value: Option<&Expr>,
        span: Span,
        limit: usize,
        after_trailing: &mut bool,
    ) {
        parts.push(self.clause_break(span.start, *after_trailing));
        parts.push(text(keyword));
        if let Some(value) = value {
            parts.push(self.expr(value));
        }
        let trailing = self.trailing_comment_before(span.end, limit);
        *after_trailing = trailing.is_some();
        parts.extend(trailing);
    }

    fn pattern(&mut self, pattern: &Pattern) -> Doc {
        match pattern {
            Pattern::Variant { name, fields, .. } => {
                if fields.is_empty() {
                    return text(name.text.clone());
                }
                let fields: Vec<Doc> = fields
                    .iter()
                    .map(|field| match &field.pattern {
                        Some(inner) => concat(vec![
                            text(format!("{}: ", field.field.text)),
                            self.pattern(inner),
                        ]),
                        None => text(field.field.text.clone()),
                    })
                    .collect();
                concat(vec![text(name.text.clone()), bracketed("(", fields, ")")])
            }
            Pattern::Literal(expr) => self.expr(expr),
            Pattern::Nothing(_) => text("nothing"),
            Pattern::Some(inner, _) => concat(vec![text("some("), self.pattern(inner), text(")")]),
            Pattern::Success(inner, _) => {
                concat(vec![text("success("), self.pattern(inner), text(")")])
            }
            Pattern::Failure(inner, _) => {
                concat(vec![text("failure("), self.pattern(inner), text(")")])
            }
            Pattern::Binding(name) => text(name.text.clone()),
            Pattern::Typed { name, ty, .. } => text(format!("{}: {}", name.text, type_text(ty))),
        }
    }

    fn text_literal(&mut self, pieces: &[TextPiece], block: bool) -> Doc {
        if block {
            let mut rendered = String::new();
            for piece in pieces {
                match piece {
                    TextPiece::Text(value) => rendered.push_str(&escape_text(value, true)),
                    TextPiece::Hole(expr) => rendered.push_str(&format!(
                        "{{{}}}",
                        render(&self.expr(expr), usize::MAX).trim_end()
                    )),
                }
            }
            let mut parts = vec![text("\"\"\"")];
            let mut lines = Vec::new();
            for line in rendered.split('\n') {
                lines.push(Doc::HardLine);
                lines.push(text(line));
            }
            lines.push(Doc::HardLine);
            lines.push(text("\"\"\""));
            parts.push(nest(concat(lines)));
            return concat(parts);
        }
        let mut rendered = String::from("\"");
        for piece in pieces {
            match piece {
                TextPiece::Text(value) => rendered.push_str(&escape_text(value, false)),
                TextPiece::Hole(expr) => rendered.push_str(&format!(
                    "{{{}}}",
                    render(&self.expr(expr), usize::MAX).trim_end()
                )),
            }
        }
        rendered.push('"');
        text(rendered)
    }
}

fn names(bindings: &[Name]) -> String {
    bindings
        .iter()
        .map(|name| name.text.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn docs_empty(docs: &Docs) -> bool {
    docs.purpose.is_none()
        && docs.tags.is_empty()
        && docs.see_also.is_empty()
        && docs.deprecated.is_none()
        && !docs.expose_as_tool
        && docs.examples.is_empty()
}

/// `purpose: words ...` on one line, with single spaces between the words.
fn clause_text(head: &str, body: &str) -> Doc {
    let words: Vec<&str> = body.split_whitespace().collect();
    text(format!("{head} {}", words.join(" ")))
}

/// Operands and operators of a chain of `and` / `or`, left to right.
fn flatten_logic<'e>(expr: &'e Expr, operands: &mut Vec<&'e Expr>, operators: &mut Vec<BinaryOp>) {
    match &expr.kind {
        ExprKind::Binary {
            op: op @ (BinaryOp::And | BinaryOp::Or),
            left,
            right,
        } => {
            flatten_logic(left, operands, operators);
            operators.push(*op);
            flatten_logic(right, operands, operators);
        }
        _ => operands.push(expr),
    }
}

fn escape_text(value: &str, block: bool) -> String {
    let mut out = String::new();
    for character in value.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '"' if !block => out.push_str("\\\""),
            '{' => out.push_str("\\{"),
            '\n' if !block => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn quote_text(value: &str) -> String {
    format!("\"{}\"", escape_text(value, false))
}

/// `for any T, U where T can Compare` as one line.
pub fn for_any_text(for_any: &ForAny) -> String {
    let params: Vec<&str> = for_any
        .params
        .iter()
        .map(|param| param.text.as_str())
        .collect();
    let mut out = format!("for any {}", params.join(", "));
    if !for_any.constraints.is_empty() {
        let constraints: Vec<String> = for_any
            .constraints
            .iter()
            .map(|constraint| {
                format!(
                    "{} can {}",
                    constraint.param.text,
                    type_text(&constraint.ability)
                )
            })
            .collect();
        out.push_str(&format!(" where {}", constraints.join(" and ")));
    }
    out
}

/// A `needs` list as one line, grant clauses included.
pub fn capabilities_text(capabilities: &[Capability]) -> String {
    capabilities
        .iter()
        .map(capability_text)
        .collect::<Vec<_>>()
        .join(", ")
}

/// A `needs` clause: one line when it fits, else one capability per line
/// indented once more, since a line break after a comma carries no meaning
/// (decision V12).
fn needs_doc(capabilities: &[Capability]) -> Doc {
    let items: Vec<Doc> = capabilities
        .iter()
        .map(|capability| text(capability_text(capability)))
        .collect();
    group(concat(vec![
        text("needs "),
        nest(join(items, concat(vec![text(","), Doc::Line]))),
    ]))
}

/// One capability with its scope, budget and guard, as the formatter spells it.
fn capability_text(capability: &Capability) -> String {
    let path: Vec<&str> = capability
        .path
        .iter()
        .map(|name| name.text.as_str())
        .collect();
    let mut out = match &capability.scope {
        Some(scope) => format!("{}({})", path.join("."), quote_text(scope)),
        None => path.join("."),
    };
    if let Some(budget) = &capability.budget {
        out.push_str(&format!(
            " at most {} per {}",
            budget.count, budget.per.text
        ));
    }
    if !capability.only_to.is_empty() {
        let sinks: Vec<String> = capability
            .only_to
            .iter()
            .map(|sink| {
                let path: Vec<&str> = sink.path.iter().map(|name| name.text.as_str()).collect();
                match &sink.scope {
                    Some(scope) => format!("{}({})", path.join("."), quote_text(scope)),
                    None => path.join("."),
                }
            })
            .collect();
        out.push_str(&format!(" only to {}", sinks.join(" or ")));
    }
    out
}

/// A type as one line, as the formatter spells it.
pub fn type_text(ty: &Type) -> String {
    match ty {
        Type::Named { name, args, .. } => {
            if args.is_empty() {
                name.text.clone()
            } else if name.text == "Map" && args.len() == 2 {
                format!(
                    "{} of {} to {}",
                    name.text,
                    type_text(&args[0]),
                    type_text(&args[1])
                )
            } else {
                let args: Vec<String> = args.iter().map(type_text).collect();
                format!("{} of {}", name.text, args.join(", "))
            }
        }
        Type::Maybe(inner, _) => format!("maybe {}", type_text(inner)),
        Type::Function {
            params,
            returns,
            fails,
            needs,
            ..
        } => {
            let params: Vec<String> = params.iter().map(type_text).collect();
            let mut out = format!("function({})", params.join(", "));
            if let Some(returns) = returns {
                out.push_str(&format!(" returns {}", type_text(returns)));
            }
            if !fails.is_empty() {
                let fails: Vec<String> = fails.iter().map(type_text).collect();
                out.push_str(&format!(" or fails with {}", fails.join(" or ")));
            }
            if !needs.is_empty() {
                out.push_str(&format!(" needs {}", capabilities_text(needs)));
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn formatted(source: &str) -> String {
        let file = SourceFile::new("t.ry", source);
        format(&file).unwrap_or_else(|diagnostics| panic!("{diagnostics:#?}"))
    }

    #[test]
    fn normalizes_spacing_and_blank_lines() {
        let source = "module   demo\n  purpose:   Show   formatting.\n\n\n\nimport std.console\npublic function main()   needs console\n  purpose: Say hi.\n  console.print(\"hi\")\nend\n";
        assert_eq!(
            formatted(source),
            "module demo\n  purpose: Show formatting.\n\nimport std.console\n\npublic function main() needs console\n  purpose: Say hi.\n\n  console.print(\"hi\")\nend\n"
        );
    }

    #[test]
    fn breaks_inside_the_call_before_otherwise_when_the_line_is_too_long() {
        // the longer argument list breaks first: here the call's
        let source = "module demo\n\nfunction load()\n  let configuration_text be filesystem.read_text(path: configuration_path, encoding: \"utf-8\") otherwise fail with Unreadable(path: configuration_path)\nend\n";
        let out = formatted(source);
        assert!(out.contains("  let configuration_text be filesystem.read_text(\n    path: configuration_path,\n    encoding: \"utf-8\"\n  ) otherwise fail with Unreadable(path: configuration_path)\n"), "{out}");
        assert_eq!(formatted(&out), out);
        // here the fallback's
        let source = "module demo\n\nfunction load()\n  let text be files.read(path) otherwise fail with Unreadable(path: configuration_path, detail: \"the configuration file cannot be read\")\nend\n";
        let out = formatted(source);
        assert!(out.contains("  let text be files.read(path) otherwise fail with Unreadable(\n    path: configuration_path,\n    detail: \"the configuration file cannot be read\"\n  )\n"), "{out}");
        assert_eq!(formatted(&out), out);
    }

    #[test]
    fn a_long_query_after_return_nests_its_clauses() {
        let source = "module demo\n\nfunction overdue(tasks: List of Task, today: Date) returns List of Task\n  return for each task in tasks where task.due is less than today and not task.done sorted by task.due collect task\nend\n";
        let out = formatted(source);
        assert!(out.contains("  return for each task in tasks\n    where task.due is less than today and not task.done\n    sorted by task.due\n    collect task\n"), "{out}");
        assert_eq!(formatted(&out), out);
    }

    #[test]
    fn a_record_update_breaks_before_with() {
        let source = "module demo\n\nfunction renamed(user: User) returns User\n  return user with name: \"a considerably longer name than before\", age: user.age + 1, email: user.email\nend\n";
        let out = formatted(source);
        assert!(out.contains("  return user\n    with name: \"a considerably longer name than before\", age: user.age + 1, email: user.email\n"), "{out}");
        assert_eq!(formatted(&out), out);
    }

    #[test]
    fn long_queries_take_one_clause_per_line() {
        let source = "module demo\n\nfunction load()\n  let emails be for each user in users where user.is_active and user.age is at least 18 sorted by user.created_at descending collect user.email\n  let paid be for each order in orders where order.is_paid count\nend\n";
        let out = formatted(source);
        assert!(out.contains("  let emails be\n    for each user in users\n    where user.is_active and user.age is at least 18\n    sorted by user.created_at descending\n    collect user.email\n"), "{out}");
        assert!(
            out.contains("  let paid be for each order in orders where order.is_paid count\n"),
            "{out}"
        );
    }

    #[test]
    fn signature_clauses_move_to_their_own_lines_with_a_blank_line_before_the_body() {
        let source = "module demo\n\npublic function active_adult_emails(path: Path) returns List of Email or fails with FileError or JsonError needs filesystem.read\n  purpose: Read users.\n  return []\nend\n\nfunction short(path: Path) returns Text\n  return \"\"\nend\n";
        let out = formatted(source);
        assert!(out.contains("public function active_adult_emails(path: Path)\n  returns List of Email\n  or fails with FileError or JsonError\n  needs filesystem.read\n  purpose: Read users.\n\n  return []\nend\n"), "{out}");
        assert!(
            out.contains("function short(path: Path) returns Text\n  return \"\"\nend\n"),
            "{out}"
        );
    }

    #[test]
    fn examples_break_before_is_or_inside_the_call() {
        let source = "module demo\n\npublic function statement_line(order: Invoice) returns Text\n  purpose: One row.\n  example: statement_line(Invoice(customer: \"ACME\", lines: [], discount_percent: 0)) is \"ACME          0 lines      0.00\"\n  example: missing(required: [Permission(\"orders:read\"), Permission(\"orders:write\")].to_set(), roles: [Role(name: \"viewer\", granted: [Permission(\"orders:read\")].to_set())]) is [Permission(\"orders:write\")].to_set()\n  return \"\"\nend\n";
        let out = formatted(source);
        assert!(out.contains("  example: statement_line(Invoice(customer: \"ACME\", lines: [], discount_percent: 0))\n    is \"ACME          0 lines      0.00\"\n"), "{out}");
        assert!(out.contains("  example: missing(\n    required: [Permission(\"orders:read\"), Permission(\"orders:write\")].to_set(),\n    roles: [Role(name: \"viewer\", granted: [Permission(\"orders:read\")].to_set())]\n  ) is [Permission(\"orders:write\")].to_set()\n"), "{out}");
    }

    #[test]
    fn comments_survive() {
        let source = "module demo\n\n# leading comment\nfunction load()\n  # why we retry\n  let total be 1 # trailing\n  return total\nend\n";
        let out = formatted(source);
        assert_eq!(out, "module demo\n\n# leading comment\nfunction load()\n  # why we retry\n  let total be 1  # trailing\n  return total\nend\n");
    }

    #[test]
    fn one_statement_ifs_stay_on_one_line_when_they_fit() {
        let source = "module demo\n\nfunction load()\n  if done then\n    break\n  end\n  if total is 0 then\n    console.print(\"none\")\n  otherwise\n    console.print(\"some\")\n  end\nend\n";
        let out = formatted(source);
        assert!(out.contains("  if done then break end\n"), "{out}");
        assert!(out.contains("  if total is 0 then\n    console.print(\"none\")\n  otherwise\n    console.print(\"some\")\n  end\n"), "{out}");
    }

    #[test]
    fn long_purposes_stay_on_their_line() {
        let source = "module demo\n\npublic function load()\n  purpose:   This purpose clause is deliberately long so that a wrapping formatter   would have to break it onto a second line.\n  return 1\nend\n";
        let out = formatted(source);
        assert!(out.contains("  purpose: This purpose clause is deliberately long so that a wrapping formatter would have to break it onto a second line.\n"), "{out}");
    }

    /// Formatting a canonical source with interior comments leaves it alone
    /// and is idempotent.
    fn stays(source: &str) {
        let out = formatted(source);
        assert_eq!(out, source);
        assert_eq!(formatted(&out), out);
    }

    #[test]
    fn a_query_with_otherwise_stays_nested_under_be() {
        // fits on the line after `be`: one line, nested
        let source = "module demo\n\nfunction find(words: List of Text, wanted: Text) returns Text\n  let found be for each word in words where word is wanted first otherwise fail with Missing(word: wanted)\n  return found\nend\n";
        let out = formatted(source);
        assert_eq!(out, "module demo\n\nfunction find(words: List of Text, wanted: Text) returns Text\n  let found be\n    for each word in words where word is wanted first otherwise fail with Missing(word: wanted)\n  return found\nend\n");
        assert!(parse(&out).diagnostics.is_empty());
        assert_eq!(formatted(&out), out);
        // too long for one line: one clause per line, `otherwise` on the last
        let long = "module demo\n\nfunction find(candidate_words: List of Text, wanted_word: Text) returns Text\n  let found be for each candidate_word in candidate_words where candidate_word is wanted_word first otherwise fail with MissingWord(wanted: wanted_word)\n  return found\nend\n";
        let out = formatted(long);
        assert_eq!(out, "module demo\n\nfunction find(candidate_words: List of Text, wanted_word: Text) returns Text\n  let found be\n    for each candidate_word in candidate_words\n    where candidate_word is wanted_word\n    first otherwise fail with MissingWord(wanted: wanted_word)\n  return found\nend\n");
        let reparsed = parse(&out);
        assert!(
            reparsed.diagnostics.is_empty(),
            "{:?}",
            reparsed.diagnostics
        );
        assert_eq!(formatted(&out), out);
    }

    #[test]
    fn grant_clauses_and_replays_round_trip() {
        stays("module demo\n\npublic function main()\n  or fails with AppError\n  needs console, network.http(\"api.example.com\") at most 60 per minute\n  purpose: Try the grant clauses.\n\n  console.print(\"hi\")\nend\n\ntest \"the forecast is read\" needs network.http replays \"fixtures/forecast.json\"\n  check true\nend\n");
        stays("module demo\n\npublic function main()\n  needs filesystem.read(\"secrets\") only to console or network.http(\"api.example.com\")\n  purpose: Guard the secrets.\n\n  console.print(\"hi\")\nend\n");
    }

    #[test]
    fn a_wide_needs_clause_breaks_after_its_commas() {
        // wider than the line: one capability per line, indented once more
        stays("module demo\n\npublic function main()\n  needs console,\n    network.http(\"api.example.com\") at most 60 per minute,\n    filesystem.read(\"secrets\") only to network.http(\"api.example.com\")\n  purpose: Guard the secrets.\n\n  console.print(\"hi\")\nend\n\ntest \"the secrets stay home\" needs console,\n  network.http(\"api.example.com\") at most 60 per minute,\n  filesystem.read(\"secrets\") only to network.http(\"api.example.com\") replays \"fixtures/a.json\"\n  check true\nend\n");
        // a line break after a comma carries no meaning: the source joins
        let out = formatted("module demo\n\npublic function main()\n  needs console,\n    time\n  purpose: Two.\n\n  console.print(\"hi\")\nend\n");
        assert!(
            out.contains("public function main() needs console, time\n"),
            "{out}"
        );
    }

    #[test]
    fn a_wide_exposing_list_breaks_after_its_commas() {
        // wider than the line: one name per line, indented once
        stays("module demo\n\nimport ast exposing Span,\n  Name,\n  TypeName,\n  Module,\n  Comment,\n  Import,\n  Item,\n  Docs,\n  Example,\n  ExampleOutcome,\n  Function,\n  Param,\n  Capability\n");
        // a line break after a comma carries no meaning: the source joins
        let out = formatted("module demo\n\nimport shapes exposing Shape,\n  Wrapper\n");
        assert!(
            out.contains("import shapes exposing Shape, Wrapper\n"),
            "{out}"
        );
    }

    #[test]
    fn comments_stay_inside_lists_and_arguments() {
        stays("module demo\n\nfunction items() returns List of Integer\n  return [\n    1,\n    # the second one\n    2,  # trailing\n    3\n  ]\nend\n");
        stays("module demo\n\nfunction run_it() returns Integer\n  return compute(\n    # the first operand\n    left: 1,\n    right: 2  # the second\n  )\nend\n");
        // a comment after a one-line list closes the statement, not the list
        stays("module demo\n\nfunction items() returns List of Integer\n  return [1, 2, 3]  # all of them\nend\n");
    }

    #[test]
    fn comments_stay_inside_queries() {
        stays("module demo\n\nfunction emails(users: List of User) returns List of Text\n  let result be\n    for each user in users\n    # only grown-ups\n    where user.age is at least 18  # inclusive\n    sorted by user.name\n    collect user.email\n  return result\nend\n");
        stays("module demo\n\nfunction total(orders: List of Order) returns Integer\n  return for each order in orders  # every order\n    where order.is_paid\n    count\nend\n");
    }

    #[test]
    fn comments_stay_inside_match_and_if_expressions() {
        stays("module demo\n\nfunction label(shape: Shape) returns Text\n  let kind be\n    match shape\n      # round things\n      when Circle then \"circle\"\n      when Point then \"point\"  # degenerate\n      otherwise \"other\"\n    end\n  return kind\nend\n");
        stays("module demo\n\nfunction label(done: Boolean) returns Text\n  let kind be\n    if done then \"yes\"\n    # the other case\n    otherwise \"no\"\n    end\n  return kind\nend\n");
    }

    #[test]
    fn comments_stay_between_fields_and_arms() {
        stays("module demo\n\npublic type User\n  purpose: A person.\n  # identity\n  has name: Text\n  has age: Integer  # in years\n  can Compare by name\nend\n");
        stays("module demo\n\npublic type Shape is one of\n  purpose: A figure.\n  # the round one\n  Circle(radius: Decimal)\n  Point  # no size\nend\n");
        stays("module demo\n\nfunction pick(flag: Boolean) returns Integer\n  match flag\n    when true then\n      # the common case\n      return 1\n    otherwise return 2\n  end\nend\n");
        stays("module demo\n\nfunction pick(flag: Boolean) returns Integer\n  if flag then\n    return 1\n  # the rare case\n  otherwise if not flag then\n    return 2\n  otherwise\n    return 3\n  end\nend\n");
    }
}
