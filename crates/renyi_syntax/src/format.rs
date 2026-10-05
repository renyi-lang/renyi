//! The formatter: a parsed module back to text in the canonical layout of
//! syntax sketch section 16. There are no options.
//!
//! Layout is decided with a small document model: text, line breaks that
//! become spaces when a group fits on one line, nesting, groups, and ordered
//! alternatives. A group is printed flat when it fits in the remaining width
//! and broken otherwise, so the outermost breakable construct breaks first:
//! `otherwise` before the arguments of the call it follows, the `is` of an
//! example before the arguments of its call.

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
    /// Words wrapped at the width, continuation lines at the current indent.
    Words(Vec<String>),
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
        Doc::Words(words) => Some(
            words.iter().map(|word| char_width(word)).sum::<usize>()
                + words.len().saturating_sub(1),
        ),
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
            Doc::Words(words) => {
                for (index, word) in words.iter().enumerate() {
                    if index > 0 {
                        if self.column + 1 + char_width(word) <= self.width {
                            self.push(" ");
                        } else {
                            self.newline(indent);
                        }
                    }
                    self.push(word);
                }
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
        let span = *self.comments.get(self.next_comment)?;
        if self.line_of(span.start) == self.line_of(end.saturating_sub(1)) && span.start >= end {
            self.next_comment += 1;
            return Some(text(format!("  {}", self.file.slice(span).trim_end())));
        }
        None
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

    fn import(&self, import: &Import) -> Doc {
        let path: Vec<&str> = import.path.iter().map(|name| name.text.as_str()).collect();
        let mut line = format!("import {}", path.join("."));
        if let Some(alias) = &import.alias {
            line.push_str(&format!(" as {}", alias.text));
        }
        if !import.exposing.is_empty() {
            let names: Vec<&str> = import
                .exposing
                .iter()
                .map(|name| name.text.as_str())
                .collect();
            line.push_str(&format!(" exposing {}", names.join(", ")));
        }
        text(line)
    }

    /// Documentation clauses, each on its own line; long prose wraps onto
    /// lines indented one level deeper than the clause word.
    fn docs(&self, docs: &Docs) -> Doc {
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

    fn example(&self, example: &Example) -> Doc {
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
                    self.type_(&constant.ty)
                );
                concat(vec![
                    text(head),
                    self.value_after(&constant.value),
                    nest(self.docs(&constant.docs)),
                ])
            }
            Item::Test(test) => {
                let mut head = format!("test {}", quote_text(&test.name));
                if !test.needs.is_empty() {
                    head.push_str(&format!(" needs {}", self.capabilities(&test.needs)));
                }
                concat(vec![
                    text(head),
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
                Some(ty) => text(format!("{}: {}", param.name.text, self.type_(ty))),
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
            clauses.push(text(format!("returns {}", self.type_(returns))));
        }
        if !function.fails.is_empty() {
            let names: Vec<String> = function.fails.iter().map(|ty| self.type_(ty)).collect();
            clauses.push(text(format!("or fails with {}", names.join(" or "))));
        }
        if !function.needs.is_empty() {
            clauses.push(text(format!(
                "needs {}",
                self.capabilities(&function.needs)
            )));
        }
        if let Some(for_any) = &function.type_params {
            clauses.push(text(self.for_any(for_any)));
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
                let mut parts = vec![text(format!("{head} is {}", self.type_(base)))];
                if let Some(refinement) = refinement {
                    parts.push(nest(self.refinement(refinement)));
                }
                concat(vec![group(concat(parts)), nest(self.docs(&type_def.docs))])
            }
            TypeKind::Record { fields, derives } => {
                let mut parts = vec![text(head), nest(self.docs(&type_def.docs))];
                for field in fields {
                    parts.push(nest(concat(vec![Doc::HardLine, self.field(field, true)])));
                }
                for derive in derives {
                    parts.push(nest(concat(vec![Doc::HardLine, self.derive(derive)])));
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
                    let mut variant_doc = vec![text(variant.name.text.clone())];
                    if !variant.fields.is_empty() {
                        let fields: Vec<Doc> = variant
                            .fields
                            .iter()
                            .map(|field| self.field(field, false))
                            .collect();
                        variant_doc.push(bracketed("(", fields, ")"));
                    }
                    parts.push(nest(concat(vec![Doc::HardLine, concat(variant_doc)])));
                }
                for derive in derives {
                    parts.push(nest(concat(vec![Doc::HardLine, self.derive(derive)])));
                }
                parts.push(Doc::HardLine);
                parts.push(text("end"));
                concat(parts)
            }
        }
    }

    fn field(&self, field: &Field, with_has: bool) -> Doc {
        let mut head = String::new();
        if with_has {
            head.push_str("has ");
        }
        head.push_str(&format!("{}: {}", field.name.text, self.type_(&field.ty)));
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
    fn refinement(&self, condition: &Expr) -> Doc {
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

    fn derive(&self, derive: &Derive) -> Doc {
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
                .map(|ty| format!("self can {}", self.type_(ty)))
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
            self.type_(&implementation.ability),
            self.type_(&implementation.target)
        ))];
        if let Some(for_any) = &implementation.type_params {
            parts.push(nest(concat(vec![
                Doc::HardLine,
                text(self.for_any(for_any)),
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

    fn for_any(&self, for_any: &ForAny) -> String {
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
                        self.type_(&constraint.ability)
                    )
                })
                .collect();
            out.push_str(&format!(" where {}", constraints.join(" and ")));
        }
        out
    }

    fn capabilities(&self, capabilities: &[Capability]) -> String {
        capabilities
            .iter()
            .map(|capability| {
                let path: Vec<&str> = capability
                    .path
                    .iter()
                    .map(|name| name.text.as_str())
                    .collect();
                match &capability.scope {
                    Some(scope) => format!("{}({})", path.join("."), quote_text(scope)),
                    None => path.join("."),
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn type_(&self, ty: &Type) -> String {
        match ty {
            Type::Named { name, args, .. } => {
                if args.is_empty() {
                    name.text.clone()
                } else if name.text == "Map" && args.len() == 2 {
                    format!(
                        "{} of {} to {}",
                        name.text,
                        self.type_(&args[0]),
                        self.type_(&args[1])
                    )
                } else {
                    let args: Vec<String> = args.iter().map(|arg| self.type_(arg)).collect();
                    format!("{} of {}", name.text, args.join(", "))
                }
            }
            Type::Maybe(inner, _) => format!("maybe {}", self.type_(inner)),
            Type::Function {
                params,
                returns,
                fails,
                needs,
                ..
            } => {
                let params: Vec<String> = params.iter().map(|param| self.type_(param)).collect();
                let mut out = format!("function({})", params.join(", "));
                if let Some(returns) = returns {
                    out.push_str(&format!(" returns {}", self.type_(returns)));
                }
                if !fails.is_empty() {
                    let fails: Vec<String> = fails.iter().map(|ty| self.type_(ty)).collect();
                    out.push_str(&format!(" or fails with {}", fails.join(" or ")));
                }
                if !needs.is_empty() {
                    out.push_str(&format!(" needs {}", self.capabilities(needs)));
                }
                out
            }
        }
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
                    head.push_str(&format!(": {}", self.type_(ty)));
                }
                head.push_str(" be");
                concat(vec![text(head), self.value_after(value)])
            }
            StmtKind::Set { name, value } => concat(vec![
                text(format!("set {} to", name.text)),
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
                    let mut head = vec![text("when "), self.pattern(&arm.pattern)];
                    if let Some(guard) = &arm.guard {
                        head.push(text(" where "));
                        head.push(self.expr(guard));
                    }
                    head.push(text(" then"));
                    parts.push(nest(concat(vec![
                        Doc::HardLine,
                        self.arm_body(concat(head), &arm.body),
                    ])));
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
            StmtKind::While { condition, body } => concat(vec![
                text("while "),
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
            StmtKind::Return(Some(value)) => concat(vec![text("return"), self.value_after(value)]),
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
        if block.statements.len() == 1
            && self
                .comments_before(block.statements[0].span.start)
                .is_empty()
        {
            let body = self.statement(&block.statements[0]);
            if flat_width(&body).is_some() {
                return group(concat(vec![head, nest(concat(vec![Doc::Line, body]))]));
            }
            return concat(vec![head, nest(concat(vec![Doc::HardLine, body]))]);
        }
        concat(vec![head, nest(self.block(block))])
    }

    /// The value after `be`, `to` or `return`: queries and conditionals may
    /// move to the next line; a `match` always does.
    fn value_after(&self, value: &Expr) -> Doc {
        match &value.kind {
            ExprKind::Query(_) | ExprKind::If { .. } => {
                group(nest(concat(vec![Doc::Line, self.expr(value)])))
            }
            ExprKind::Match { .. } => nest(concat(vec![Doc::HardLine, self.expr(value)])),
            _ => concat(vec![text(" "), self.expr(value)]),
        }
    }

    fn loop_source(&self, source: &Expr) -> Doc {
        match &source.kind {
            ExprKind::Range { .. } => self.expr(source),
            _ => concat(vec![text("in "), self.expr(source)]),
        }
    }

    fn ordering(&self, order: &Ordering) -> Doc {
        let mut parts = vec![text("sorted by "), self.expr(&order.key)];
        if order.descending {
            parts.push(text(" descending"));
        }
        concat(parts)
    }

    // -------------------------------------------------------------- expressions

    fn expr(&self, expr: &Expr) -> Doc {
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
                concat(vec![self.expr(callee), self.arguments(args)])
            }
            ExprKind::Construct { name, args } => {
                concat(vec![text(name.text.clone()), self.arguments(args)])
            }
            ExprKind::List(items) => {
                bracketed("[", items.iter().map(|item| self.expr(item)).collect(), "]")
            }
            ExprKind::Map(entries) => bracketed(
                "{",
                entries
                    .iter()
                    .map(|(key, value)| concat(vec![self.expr(key), text(": "), self.expr(value)]))
                    .collect(),
                "}",
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
                let updates: Vec<Doc> = updates.iter().map(|arg| self.argument(arg)).collect();
                group(concat(vec![
                    self.expr(base),
                    text(" with "),
                    nest(join(updates, concat(vec![text(","), Doc::Line]))),
                ]))
            }
            ExprKind::Otherwise { value, fallback } => group(concat(vec![
                self.expr(value),
                nest(concat(vec![
                    Doc::Line,
                    text("otherwise "),
                    self.outcome(fallback),
                ])),
            ])),
            ExprKind::If {
                branches,
                otherwise,
            } => {
                let mut parts = Vec::new();
                for (index, (condition, outcome)) in branches.iter().enumerate() {
                    if index > 0 {
                        parts.push(Doc::Line);
                        parts.push(text("otherwise "));
                    }
                    parts.push(text("if "));
                    parts.push(self.expr(condition));
                    parts.push(text(" then "));
                    parts.push(self.outcome(outcome));
                }
                parts.push(Doc::Line);
                parts.push(text("otherwise "));
                parts.push(self.outcome(otherwise));
                parts.push(Doc::Line);
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
                    let mut head = vec![text("when "), self.pattern(&arm.pattern)];
                    if let Some(guard) = &arm.guard {
                        head.push(text(" where "));
                        head.push(self.expr(guard));
                    }
                    head.push(text(" then"));
                    let body = self.outcome(&arm.body);
                    parts.push(nest(concat(vec![
                        Doc::HardLine,
                        group(concat(vec![
                            concat(head),
                            nest(concat(vec![Doc::Line, body])),
                        ])),
                    ])));
                }
                if let Some(outcome) = otherwise {
                    let body = self.outcome(outcome);
                    parts.push(nest(concat(vec![
                        Doc::HardLine,
                        group(concat(vec![
                            text("otherwise"),
                            nest(concat(vec![Doc::Line, body])),
                        ])),
                    ])));
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
    fn expr_with_broken_arguments(&self, expr: &Expr) -> Option<Doc> {
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

    fn arguments(&self, args: &[Arg]) -> Doc {
        bracketed(
            "(",
            args.iter().map(|arg| self.argument(arg)).collect(),
            ")",
        )
    }

    fn argument(&self, arg: &Arg) -> Doc {
        match &arg.name {
            Some(name) => concat(vec![
                text(format!("{}: ", name.text)),
                self.expr(&arg.value),
            ]),
            None => self.expr(&arg.value),
        }
    }

    fn outcome(&self, outcome: &Outcome) -> Doc {
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

    fn query(&self, query: &Query) -> Doc {
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
        if let Some(filter) = &query.filter {
            parts.push(Doc::Line);
            parts.push(text("where "));
            parts.push(self.expr(filter));
        }
        if let Some(order) = &query.order {
            parts.push(Doc::Line);
            parts.push(self.ordering(order));
        }
        if let Some(key) = &query.group_by {
            parts.push(Doc::Line);
            parts.push(text("group by "));
            parts.push(self.expr(key));
        }
        match &query.terminal {
            QueryTerminal::Collect(value) => {
                parts.push(Doc::Line);
                parts.push(text("collect "));
                parts.push(self.expr(value));
            }
            QueryTerminal::Sum(value) => {
                parts.push(Doc::Line);
                parts.push(text("sum "));
                parts.push(self.expr(value));
            }
            QueryTerminal::Count => {
                parts.push(Doc::Line);
                parts.push(text("count"));
            }
            QueryTerminal::First => {
                parts.push(Doc::Line);
                parts.push(text("first"));
            }
            QueryTerminal::Any(value) => {
                parts.push(Doc::Line);
                parts.push(text("any "));
                parts.push(self.expr(value));
            }
            QueryTerminal::All(value) => {
                parts.push(Doc::Line);
                parts.push(text("all "));
                parts.push(self.expr(value));
            }
            QueryTerminal::None => {}
        }
        concat(parts)
    }

    fn pattern(&self, pattern: &Pattern) -> Doc {
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
            Pattern::Typed { name, ty, .. } => text(format!("{}: {}", name.text, self.type_(ty))),
        }
    }

    fn text_literal(&self, pieces: &[TextPiece], block: bool) -> Doc {
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

/// `purpose: words ...` wrapped at the width.
fn clause_text(head: &str, body: &str) -> Doc {
    let mut words = vec![head.to_string()];
    words.extend(body.split_whitespace().map(str::to_string));
    nest(Doc::Words(words))
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
    fn breaks_before_otherwise_when_the_line_is_too_long() {
        let source = "module demo\n\nfunction load()\n  let configuration_text be filesystem.read_text(configuration_path) otherwise fail with Unreadable(path: configuration_path)\nend\n";
        let out = formatted(source);
        assert!(out.contains("  let configuration_text be filesystem.read_text(configuration_path)\n    otherwise fail with Unreadable(path: configuration_path)\n"), "{out}");
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
    fn long_purposes_wrap_one_level_deeper() {
        let source = "module demo\n\npublic function load()\n  purpose: This purpose clause is deliberately long so that the formatter has to wrap it onto a second line of text.\n  return 1\nend\n";
        let out = formatted(source);
        assert!(out.contains("  purpose: This purpose clause is deliberately long so that the formatter has to wrap it onto a\n    second line of text.\n"), "{out}");
    }
}
