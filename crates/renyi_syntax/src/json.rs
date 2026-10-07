//! The syntax tree as JSON, for tools (`renyi parse --json`). The document
//! is the derived JSON of the types of `compiler/ast.ry`, the syntax tree of
//! the front end written in Renyi (decision W1): a record is an object whose
//! keys are its fields in declaration order, a variant is an object with
//! `kind` first and its fields after it, a `maybe` without a value is `null`,
//! and a span counts characters, as Renyi's text indices do. The Renyi front
//! end prints the same document with `json.render_indented`, so the two can
//! be compared byte for byte.

use crate::ast::*;
use crate::diagnostics::json_string;
use crate::span::{SourceFile, Span};

/// A JSON value, built from the tree and rendered with two-space indentation.
#[derive(Clone, Debug)]
pub enum Json {
    Null,
    Bool(bool),
    Number(usize),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(&'static str, Json)>),
}

impl Json {
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.render_into(&mut out, 0);
        out.push('\n');
        out
    }

    fn render_into(&self, out: &mut String, depth: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Json::Number(value) => out.push_str(&value.to_string()),
            Json::String(value) => out.push_str(&json_string(value)),
            Json::Array(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    newline(out, depth + 1);
                    item.render_into(out, depth + 1);
                }
                newline(out, depth);
                out.push(']');
            }
            Json::Object(fields) => {
                if fields.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                for (index, (key, value)) in fields.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    newline(out, depth + 1);
                    out.push_str(&json_string(key));
                    out.push_str(": ");
                    value.render_into(out, depth + 1);
                }
                newline(out, depth);
                out.push('}');
            }
        }
    }
}

fn newline(out: &mut String, depth: usize) {
    out.push('\n');
    for _ in 0..depth {
        out.push_str("  ");
    }
}

/// The whole module as a JSON document.
pub fn module_to_json(file: &SourceFile, module: &Module) -> String {
    Encoder::new(file).module(module).render()
}

struct Encoder<'a> {
    file: &'a SourceFile,
    /// The character offset at every byte offset of the text, and at its end.
    characters: Vec<usize>,
}

fn string(value: &str) -> Json {
    Json::String(value.to_string())
}

fn strings(values: &[String]) -> Json {
    Json::Array(values.iter().map(|value| string(value)).collect())
}

fn optional(value: Option<Json>) -> Json {
    value.unwrap_or(Json::Null)
}

fn array<T>(items: &[T], encode: impl Fn(&T) -> Json) -> Json {
    Json::Array(items.iter().map(encode).collect())
}

/// A variant of a sum type: `kind` first, then its fields.
fn variant(kind: &'static str, fields: Vec<(&'static str, Json)>) -> Json {
    let mut all = vec![("kind", string(kind))];
    all.extend(fields);
    Json::Object(all)
}

impl<'a> Encoder<'a> {
    fn new(file: &'a SourceFile) -> Self {
        let text = &file.text;
        let mut characters = vec![0; text.len() + 1];
        let mut count = 0;
        for (offset, c) in text.char_indices() {
            characters[offset..offset + c.len_utf8()].fill(count);
            count += 1;
        }
        characters[text.len()] = count;
        Encoder { file, characters }
    }

    fn offset(&self, byte: usize) -> Json {
        Json::Number(self.characters[byte.min(self.file.text.len())])
    }

    fn span(&self, span: Span) -> Json {
        Json::Object(vec![
            ("start", self.offset(span.start)),
            ("stop", self.offset(span.end)),
        ])
    }

    fn name(&self, name: &Name) -> Json {
        Json::Object(vec![
            ("text", string(&name.text)),
            ("span", self.span(name.span)),
        ])
    }

    fn names(&self, names: &[Name]) -> Json {
        array(names, |name| self.name(name))
    }

    fn type_name(&self, name: &TypeName) -> Json {
        Json::Object(vec![
            ("text", string(&name.text)),
            ("span", self.span(name.span)),
        ])
    }

    fn type_names(&self, names: &[TypeName]) -> Json {
        array(names, |name| self.type_name(name))
    }

    // ------------------------------------------------------------ module

    fn module(&self, module: &Module) -> Json {
        Json::Object(vec![
            ("name", self.names(&module.name)),
            ("docs", self.docs(&module.docs)),
            (
                "imports",
                array(&module.imports, |import| self.import(import)),
            ),
            ("items", array(&module.items, |item| self.item(item))),
            (
                "comments",
                array(&module.comments, |span| {
                    Json::Object(vec![
                        ("text", string(self.file.slice(*span).trim_end())),
                        ("span", self.span(*span)),
                    ])
                }),
            ),
            ("span", self.span(module.span)),
        ])
    }

    fn import(&self, import: &Import) -> Json {
        Json::Object(vec![
            ("path", self.names(&import.path)),
            (
                "alias",
                optional(import.alias.as_ref().map(|alias| self.name(alias))),
            ),
            ("exposed", self.type_names(&import.exposing)),
            ("span", self.span(import.span)),
        ])
    }

    fn item(&self, item: &Item) -> Json {
        let (kind, value) = match item {
            Item::Function(function) => ("FunctionItem", self.function(function)),
            Item::Type(def) => ("TypeItem", self.type_def(def)),
            Item::Ability(ability) => ("AbilityItem", self.ability(ability)),
            Item::Implementation(implementation) => {
                ("ImplementationItem", self.implementation(implementation))
            }
            Item::Constant(constant) => ("ConstantItem", self.constant(constant)),
            Item::Test(test) => ("TestItem", self.test(test)),
        };
        variant(kind, vec![("value", value)])
    }

    fn docs(&self, docs: &Docs) -> Json {
        Json::Object(vec![
            ("summary", optional(docs.purpose.as_deref().map(string))),
            ("labels", strings(&docs.tags)),
            ("see_also", strings(&docs.see_also)),
            (
                "deprecation",
                optional(docs.deprecated.as_deref().map(string)),
            ),
            ("expose_as_tool", Json::Bool(docs.expose_as_tool)),
            (
                "examples",
                array(&docs.examples, |example| self.example(example)),
            ),
        ])
    }

    fn example(&self, example: &Example) -> Json {
        let outcome = match &example.outcome {
            ExampleOutcome::Is(value) => variant("IsValue", vec![("value", self.expr(value))]),
            ExampleOutcome::FailsWith(pattern) => {
                variant("FailsWith", vec![("pattern", self.pattern(pattern))])
            }
        };
        Json::Object(vec![
            ("expression", self.expr(&example.expression)),
            ("outcome", outcome),
            ("span", self.span(example.span)),
        ])
    }

    // ------------------------------------------------------------ items

    fn function(&self, function: &Function) -> Json {
        Json::Object(vec![
            ("is_public", Json::Bool(function.public)),
            ("name", self.name(&function.name)),
            ("params", array(&function.params, |param| self.param(param))),
            (
                "result",
                optional(function.returns.as_ref().map(|ty| self.ty(ty))),
            ),
            ("failures", self.types(&function.fails)),
            ("capabilities", self.capabilities(&function.needs)),
            (
                "type_params",
                optional(function.type_params.as_ref().map(|f| self.for_any(f))),
            ),
            ("docs", self.docs(&function.docs)),
            (
                "body",
                optional(function.body.as_ref().map(|body| self.block(body))),
            ),
            ("span", self.span(function.span)),
        ])
    }

    fn param(&self, param: &Param) -> Json {
        Json::Object(vec![
            ("name", self.name(&param.name)),
            (
                "annotation",
                optional(param.ty.as_ref().map(|ty| self.ty(ty))),
            ),
            ("span", self.span(param.span)),
        ])
    }

    fn capabilities(&self, capabilities: &[Capability]) -> Json {
        array(capabilities, |capability| self.capability(capability))
    }

    fn capability(&self, capability: &Capability) -> Json {
        Json::Object(vec![
            ("path", self.names(&capability.path)),
            ("scope", optional(capability.scope.as_deref().map(string))),
            (
                "budget",
                optional(capability.budget.as_ref().map(|budget| {
                    Json::Object(vec![
                        ("limit", string(&budget.count)),
                        ("unit", self.name(&budget.per)),
                        ("span", self.span(budget.span)),
                    ])
                })),
            ),
            (
                "only_to",
                array(&capability.only_to, |sink| {
                    Json::Object(vec![
                        ("path", self.names(&sink.path)),
                        ("scope", optional(sink.scope.as_deref().map(string))),
                        ("span", self.span(sink.span)),
                    ])
                }),
            ),
            ("span", self.span(capability.span)),
        ])
    }

    fn for_any(&self, for_any: &ForAny) -> Json {
        Json::Object(vec![
            ("params", self.type_names(&for_any.params)),
            (
                "constraints",
                array(&for_any.constraints, |constraint| {
                    Json::Object(vec![
                        ("param", self.type_name(&constraint.param)),
                        ("requirement", self.ty(&constraint.ability)),
                    ])
                }),
            ),
            ("span", self.span(for_any.span)),
        ])
    }

    fn types(&self, types: &[Type]) -> Json {
        array(types, |ty| self.ty(ty))
    }

    fn ty(&self, ty: &Type) -> Json {
        match ty {
            Type::Named { name, args, span } => variant(
                "NamedType",
                vec![
                    ("name", self.type_name(name)),
                    ("args", self.types(args)),
                    ("span", self.span(*span)),
                ],
            ),
            Type::Maybe(inner, span) => variant(
                "MaybeType",
                vec![("inner", self.ty(inner)), ("span", self.span(*span))],
            ),
            Type::Function {
                params,
                returns,
                fails,
                needs,
                span,
            } => variant(
                "FunctionType",
                vec![
                    ("params", self.types(params)),
                    ("result", optional(returns.as_ref().map(|ty| self.ty(ty)))),
                    ("failures", self.types(fails)),
                    ("capabilities", self.capabilities(needs)),
                    ("span", self.span(*span)),
                ],
            ),
        }
    }

    fn type_def(&self, def: &TypeDef) -> Json {
        let definition = match &def.kind {
            TypeKind::Record { fields, derives } => variant(
                "RecordDefinition",
                vec![
                    ("fields", self.fields(fields)),
                    ("derives", self.derives(derives)),
                ],
            ),
            TypeKind::Sum { variants, derives } => variant(
                "SumDefinition",
                vec![
                    (
                        "variants",
                        array(variants, |variant| {
                            Json::Object(vec![
                                ("name", self.type_name(&variant.name)),
                                ("fields", self.fields(&variant.fields)),
                                ("span", self.span(variant.span)),
                            ])
                        }),
                    ),
                    ("derives", self.derives(derives)),
                ],
            ),
            TypeKind::Subtype { base, refinement } => variant(
                "SubtypeDefinition",
                vec![
                    ("base", self.ty(base)),
                    (
                        "refinement",
                        optional(refinement.as_ref().map(|expr| self.expr(expr))),
                    ),
                ],
            ),
        };
        Json::Object(vec![
            ("is_public", Json::Bool(def.public)),
            ("name", self.type_name(&def.name)),
            ("type_params", self.type_names(&def.type_params)),
            ("definition", definition),
            ("docs", self.docs(&def.docs)),
            ("span", self.span(def.span)),
        ])
    }

    fn fields(&self, fields: &[Field]) -> Json {
        array(fields, |field| {
            Json::Object(vec![
                ("name", self.name(&field.name)),
                ("annotation", self.ty(&field.ty)),
                (
                    "refinement",
                    optional(field.refinement.as_ref().map(|expr| self.expr(expr))),
                ),
                (
                    "external_name",
                    optional(field.external_name.as_deref().map(string)),
                ),
                ("span", self.span(field.span)),
            ])
        })
    }

    fn derives(&self, derives: &[Derive]) -> Json {
        array(derives, |derive| {
            Json::Object(vec![
                ("ability_name", self.type_name(&derive.ability)),
                ("fields", self.names(&derive.by)),
                ("span", self.span(derive.span)),
            ])
        })
    }

    fn ability(&self, ability: &AbilityDecl) -> Json {
        Json::Object(vec![
            ("is_public", Json::Bool(ability.public)),
            ("name", self.type_name(&ability.name)),
            ("type_params", self.type_names(&ability.type_params)),
            ("requirements", self.types(&ability.requirements)),
            ("docs", self.docs(&ability.docs)),
            ("functions", array(&ability.functions, |f| self.function(f))),
            ("span", self.span(ability.span)),
        ])
    }

    fn implementation(&self, implementation: &AbilityImpl) -> Json {
        Json::Object(vec![
            ("ability_type", self.ty(&implementation.ability)),
            ("target", self.ty(&implementation.target)),
            (
                "type_params",
                optional(implementation.type_params.as_ref().map(|f| self.for_any(f))),
            ),
            (
                "functions",
                array(&implementation.functions, |f| self.function(f)),
            ),
            ("span", self.span(implementation.span)),
        ])
    }

    fn constant(&self, constant: &Constant) -> Json {
        Json::Object(vec![
            ("is_public", Json::Bool(constant.public)),
            ("name", self.name(&constant.name)),
            ("annotation", self.ty(&constant.ty)),
            ("value", self.expr(&constant.value)),
            ("docs", self.docs(&constant.docs)),
            ("span", self.span(constant.span)),
        ])
    }

    fn test(&self, test: &Test) -> Json {
        Json::Object(vec![
            ("name", string(&test.name)),
            ("capabilities", self.capabilities(&test.needs)),
            ("recording", optional(test.replays.as_deref().map(string))),
            ("body", self.block(&test.body)),
            ("span", self.span(test.span)),
        ])
    }

    // ------------------------------------------------------------ statements

    fn block(&self, block: &Block) -> Json {
        Json::Object(vec![
            (
                "statements",
                array(&block.statements, |statement| self.statement(statement)),
            ),
            ("span", self.span(block.span)),
        ])
    }

    fn statement(&self, statement: &Stmt) -> Json {
        let span = ("span", self.span(statement.span));
        match &statement.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                value,
            } => variant(
                "Let",
                vec![
                    ("is_mutable", Json::Bool(*mutable)),
                    ("name", self.name(name)),
                    ("annotation", optional(ty.as_ref().map(|ty| self.ty(ty)))),
                    ("value", self.expr(value)),
                    span,
                ],
            ),
            StmtKind::Change { name, value } => variant(
                "Change",
                vec![("name", self.name(name)), ("value", self.expr(value)), span],
            ),
            StmtKind::If {
                branches,
                otherwise,
            } => variant(
                "If",
                vec![
                    (
                        "branches",
                        array(branches, |(condition, body)| {
                            Json::Object(vec![
                                ("condition", self.expr(condition)),
                                ("body", self.block(body)),
                            ])
                        }),
                    ),
                    (
                        "fallback",
                        optional(otherwise.as_ref().map(|body| self.block(body))),
                    ),
                    span,
                ],
            ),
            StmtKind::Match {
                subject,
                arms,
                otherwise,
            } => variant(
                "Match",
                vec![
                    ("subject", self.expr(subject)),
                    (
                        "arms",
                        array(arms, |arm| self.arm(arm, |body| self.block(body))),
                    ),
                    (
                        "fallback",
                        optional(otherwise.as_ref().map(|body| self.block(body))),
                    ),
                    span,
                ],
            ),
            StmtKind::ForEach {
                bindings,
                source,
                filter,
                order,
                body,
            } => variant(
                "ForEach",
                vec![
                    ("bindings", self.names(bindings)),
                    ("source", self.expr(source)),
                    (
                        "filter",
                        optional(filter.as_ref().map(|expr| self.expr(expr))),
                    ),
                    (
                        "order",
                        optional(order.as_ref().map(|order| self.sort_order(order))),
                    ),
                    ("body", self.block(body)),
                    span,
                ],
            ),
            StmtKind::RepeatUntil { condition, body } => variant(
                "RepeatUntil",
                vec![
                    ("condition", self.expr(condition)),
                    ("body", self.block(body)),
                    span,
                ],
            ),
            StmtKind::RunConcurrently { within, body } => variant(
                "RunConcurrently",
                vec![
                    (
                        "deadline",
                        optional(within.as_ref().map(|expr| self.expr(expr))),
                    ),
                    ("body", self.block(body)),
                    span,
                ],
            ),
            StmtKind::Return(value) => variant(
                "Return",
                vec![
                    ("value", optional(value.as_ref().map(|e| self.expr(e)))),
                    span,
                ],
            ),
            StmtKind::Fail(value) => variant(
                "Fail",
                vec![
                    ("value", optional(value.as_ref().map(|e| self.expr(e)))),
                    span,
                ],
            ),
            StmtKind::Crash(message) => {
                variant("Crash", vec![("message", self.expr(message)), span])
            }
            StmtKind::Break => variant("Break", vec![span]),
            StmtKind::Continue => variant("Continue", vec![span]),
            StmtKind::Ignore(value) => variant("Ignore", vec![("value", self.expr(value)), span]),
            StmtKind::Check(value) => variant("Check", vec![("value", self.expr(value)), span]),
            StmtKind::Expression(value) => {
                variant("Expression", vec![("value", self.expr(value)), span])
            }
        }
    }

    fn arm<Body>(&self, arm: &MatchArm<Body>, body: impl Fn(&Body) -> Json) -> Json {
        Json::Object(vec![
            ("pattern", self.pattern(&arm.pattern)),
            (
                "guard",
                optional(arm.guard.as_ref().map(|expr| self.expr(expr))),
            ),
            ("body", body(&arm.body)),
            ("span", self.span(arm.span)),
        ])
    }

    fn sort_order(&self, order: &Ordering) -> Json {
        Json::Object(vec![
            ("key", self.expr(&order.key)),
            ("is_descending", Json::Bool(order.descending)),
        ])
    }

    // ------------------------------------------------------------ patterns

    fn pattern(&self, pattern: &Pattern) -> Json {
        let span = ("span", self.span(pattern.span()));
        match pattern {
            Pattern::Variant { name, fields, .. } => variant(
                "VariantPattern",
                vec![
                    ("name", self.type_name(name)),
                    (
                        "fields",
                        array(fields, |field| {
                            Json::Object(vec![
                                ("field", self.name(&field.field)),
                                (
                                    "pattern",
                                    optional(field.pattern.as_ref().map(|p| self.pattern(p))),
                                ),
                            ])
                        }),
                    ),
                    span,
                ],
            ),
            Pattern::Literal(value) => {
                variant("LiteralPattern", vec![("value", self.expr(value)), span])
            }
            Pattern::Nothing(_) => variant("NothingPattern", vec![span]),
            Pattern::Some(inner, _) => {
                variant("SomePattern", vec![("inner", self.pattern(inner)), span])
            }
            Pattern::Success(inner, _) => {
                variant("SuccessPattern", vec![("inner", self.pattern(inner)), span])
            }
            Pattern::Failure(inner, _) => {
                variant("FailurePattern", vec![("inner", self.pattern(inner)), span])
            }
            Pattern::Binding(name) => {
                variant("BindingPattern", vec![("name", self.name(name)), span])
            }
            Pattern::Typed { name, ty, .. } => variant(
                "TypedPattern",
                vec![("name", self.name(name)), ("annotation", self.ty(ty)), span],
            ),
        }
    }

    // ------------------------------------------------------------ expressions

    fn outcome(&self, outcome: &Outcome) -> Json {
        let span = ("span", self.span(outcome.span()));
        match outcome {
            Outcome::Value(value) => {
                variant("ValueOutcome", vec![("value", self.expr(value)), span])
            }
            Outcome::Fail(value, _) => variant(
                "FailOutcome",
                vec![
                    ("value", optional(value.as_ref().map(|e| self.expr(e)))),
                    span,
                ],
            ),
            Outcome::Return(value, _) => variant(
                "ReturnOutcome",
                vec![
                    ("value", optional(value.as_ref().map(|e| self.expr(e)))),
                    span,
                ],
            ),
            Outcome::Crash(message, _) => {
                variant("CrashOutcome", vec![("message", self.expr(message)), span])
            }
            Outcome::Break(_) => variant("BreakOutcome", vec![span]),
            Outcome::Continue(_) => variant("ContinueOutcome", vec![span]),
        }
    }

    fn args(&self, args: &[Arg]) -> Json {
        array(args, |arg| {
            Json::Object(vec![
                (
                    "name",
                    optional(arg.name.as_ref().map(|name| self.name(name))),
                ),
                ("value", self.expr(&arg.value)),
                ("span", self.span(arg.span)),
            ])
        })
    }

    fn exprs(&self, exprs: &[Expr]) -> Json {
        array(exprs, |expr| self.expr(expr))
    }

    fn expr(&self, expr: &Expr) -> Json {
        let span = ("span", self.span(expr.span));
        match &expr.kind {
            ExprKind::Integer(digits) => {
                variant("IntegerLiteral", vec![("text", string(digits)), span])
            }
            ExprKind::Decimal(digits) => {
                variant("DecimalLiteral", vec![("text", string(digits)), span])
            }
            ExprKind::Text { pieces, block } => variant(
                "TextLiteral",
                vec![
                    (
                        "pieces",
                        array(pieces, |piece| match piece {
                            TextPiece::Text(text) => {
                                variant("Literal", vec![("text", string(text))])
                            }
                            TextPiece::Hole(hole) => {
                                variant("Hole", vec![("expression", self.expr(hole))])
                            }
                        }),
                    ),
                    ("block", Json::Bool(*block)),
                    span,
                ],
            ),
            ExprKind::RawText(text) => {
                variant("RawTextLiteral", vec![("text", string(text)), span])
            }
            ExprKind::Boolean(value) => {
                variant("BooleanLiteral", vec![("value", Json::Bool(*value)), span])
            }
            ExprKind::Nothing => variant("NothingLiteral", vec![span]),
            ExprKind::SelfValue => variant("SelfValue", vec![span]),
            ExprKind::Name(name) => variant("NameExpr", vec![("name", self.name(name)), span]),
            ExprKind::TypeName(name) => {
                variant("TypeNameExpr", vec![("name", self.type_name(name)), span])
            }
            ExprKind::Member { base, name } => variant(
                "Member",
                vec![("base", self.expr(base)), ("name", self.name(name)), span],
            ),
            ExprKind::Call { callee, args } => variant(
                "Call",
                vec![
                    ("callee", self.expr(callee)),
                    ("args", self.args(args)),
                    span,
                ],
            ),
            ExprKind::Construct { name, args } => variant(
                "Construct",
                vec![
                    ("name", self.type_name(name)),
                    ("args", self.args(args)),
                    span,
                ],
            ),
            ExprKind::List(items) => {
                variant("ListLiteral", vec![("items", self.exprs(items)), span])
            }
            ExprKind::Map(entries) => variant(
                "MapLiteral",
                vec![
                    (
                        "entries",
                        array(entries, |(key, value)| {
                            Json::Object(vec![("key", self.expr(key)), ("value", self.expr(value))])
                        }),
                    ),
                    span,
                ],
            ),
            ExprKind::Range { from, to, by } => variant(
                "RangeLiteral",
                vec![
                    ("lower", self.expr(from)),
                    ("upper", self.expr(to)),
                    ("step", optional(by.as_ref().map(|e| self.expr(e)))),
                    span,
                ],
            ),
            ExprKind::Not(inner) => variant("Not", vec![("value", self.expr(inner)), span]),
            ExprKind::Binary { op, left, right } => variant(
                "Binary",
                vec![
                    ("operator", variant(operator_name(*op), Vec::new())),
                    ("left", self.expr(left)),
                    ("right", self.expr(right)),
                    span,
                ],
            ),
            ExprKind::With { base, updates } => variant(
                "With",
                vec![
                    ("base", self.expr(base)),
                    ("updates", self.args(updates)),
                    span,
                ],
            ),
            ExprKind::Otherwise { value, fallback } => variant(
                "Otherwise",
                vec![
                    ("value", self.expr(value)),
                    ("fallback", self.outcome(fallback)),
                    span,
                ],
            ),
            ExprKind::If {
                branches,
                otherwise,
            } => variant(
                "IfExpr",
                vec![
                    (
                        "branches",
                        array(branches, |(condition, outcome)| {
                            Json::Object(vec![
                                ("condition", self.expr(condition)),
                                ("outcome", self.outcome(outcome)),
                            ])
                        }),
                    ),
                    ("fallback", self.outcome(otherwise)),
                    span,
                ],
            ),
            ExprKind::Match {
                subject,
                arms,
                otherwise,
            } => variant(
                "MatchExpr",
                vec![
                    ("subject", self.expr(subject)),
                    (
                        "arms",
                        array(arms, |arm| self.arm(arm, |outcome| self.outcome(outcome))),
                    ),
                    (
                        "fallback",
                        optional(otherwise.as_ref().map(|outcome| self.outcome(outcome))),
                    ),
                    span,
                ],
            ),
            ExprKind::Query(query) => self.query(query, span),
            ExprKind::Paren(inner) => variant("Paren", vec![("value", self.expr(inner)), span]),
        }
    }

    fn query(&self, query: &Query, span: (&'static str, Json)) -> Json {
        let terminal = match &query.terminal {
            QueryTerminal::Collect(value) => variant("Collect", vec![("value", self.expr(value))]),
            QueryTerminal::Sum(value) => variant("Sum", vec![("value", self.expr(value))]),
            QueryTerminal::Count => variant("Count", Vec::new()),
            QueryTerminal::First => variant("First", Vec::new()),
            QueryTerminal::Any(condition) => {
                variant("Any", vec![("condition", self.expr(condition))])
            }
            QueryTerminal::All(condition) => {
                variant("All", vec![("condition", self.expr(condition))])
            }
            QueryTerminal::None => variant("GroupOnly", Vec::new()),
        };
        variant(
            "QueryExpr",
            vec![
                (
                    "sources",
                    array(&query.sources, |source| {
                        Json::Object(vec![
                            ("bindings", self.names(&source.bindings)),
                            ("source", self.expr(&source.source)),
                        ])
                    }),
                ),
                ("is_concurrent", Json::Bool(query.concurrently)),
                (
                    "deadline",
                    optional(query.within.as_ref().map(|e| self.expr(e))),
                ),
                (
                    "filter",
                    optional(query.filter.as_ref().map(|e| self.expr(e))),
                ),
                (
                    "order",
                    optional(query.order.as_ref().map(|o| self.sort_order(o))),
                ),
                (
                    "group_by",
                    optional(query.group_by.as_ref().map(|e| self.expr(e))),
                ),
                ("terminal", terminal),
                span,
            ],
        )
    }
}

/// The variant of `compiler/ast.ry`'s `BinaryOp` for an operator.
fn operator_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "Add",
        BinaryOp::Subtract => "Subtract",
        BinaryOp::Multiply => "Multiply",
        BinaryOp::Divide => "Divide",
        BinaryOp::Remainder => "Remainder",
        BinaryOp::Power => "Power",
        BinaryOp::Is => "Is",
        BinaryOp::IsNot => "IsNot",
        BinaryOp::IsLessThan => "IsLessThan",
        BinaryOp::IsAtMost => "IsAtMost",
        BinaryOp::IsGreaterThan => "IsGreaterThan",
        BinaryOp::IsAtLeast => "IsAtLeast",
        BinaryOp::And => "And",
        BinaryOp::Or => "Or",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    /// A small validator: enough JSON grammar to reject anything malformed.
    fn validate(text: &str) {
        let bytes = text.as_bytes();
        let mut pos = 0;
        value(bytes, &mut pos);
        skip(bytes, &mut pos);
        assert_eq!(pos, bytes.len(), "trailing text after the JSON value");
    }

    fn skip(bytes: &[u8], pos: &mut usize) {
        while *pos < bytes.len() && (bytes[*pos] as char).is_ascii_whitespace() {
            *pos += 1;
        }
    }

    fn value(bytes: &[u8], pos: &mut usize) {
        skip(bytes, pos);
        match bytes[*pos] {
            b'{' => {
                *pos += 1;
                skip(bytes, pos);
                if bytes[*pos] == b'}' {
                    *pos += 1;
                    return;
                }
                loop {
                    skip(bytes, pos);
                    assert_eq!(bytes[*pos], b'"', "object key at {pos}");
                    value(bytes, pos);
                    skip(bytes, pos);
                    assert_eq!(bytes[*pos], b':', "colon at {pos}");
                    *pos += 1;
                    value(bytes, pos);
                    skip(bytes, pos);
                    match bytes[*pos] {
                        b',' => *pos += 1,
                        b'}' => {
                            *pos += 1;
                            return;
                        }
                        other => panic!("unexpected {} at {pos}", other as char),
                    }
                }
            }
            b'[' => {
                *pos += 1;
                skip(bytes, pos);
                if bytes[*pos] == b']' {
                    *pos += 1;
                    return;
                }
                loop {
                    value(bytes, pos);
                    skip(bytes, pos);
                    match bytes[*pos] {
                        b',' => *pos += 1,
                        b']' => {
                            *pos += 1;
                            return;
                        }
                        other => panic!("unexpected {} at {pos}", other as char),
                    }
                }
            }
            b'"' => {
                *pos += 1;
                while bytes[*pos] != b'"' {
                    if bytes[*pos] == b'\\' {
                        *pos += 1;
                    }
                    *pos += 1;
                }
                *pos += 1;
            }
            b't' => *pos += 4,
            b'f' => *pos += 5,
            b'n' => *pos += 4,
            b'0'..=b'9' => {
                while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
                    *pos += 1;
                }
            }
            other => panic!("unexpected {} at {pos}", other as char),
        }
    }

    #[test]
    fn a_small_module_renders_as_valid_json() {
        let source = "module demo\n  purpose: Try the encoder.\n\npublic function double(value: Integer) returns Integer\n  purpose: Twice the value.\n  example: double(2) is 4\n\n  let label be if value is 0 then \"zero\" otherwise \"some {value}\" end\n  ignore label\n  return value * 2\nend\n";
        let file = SourceFile::new("demo.ry", source);
        let parsed = parse(&file.text);
        assert!(parsed.diagnostics.is_empty());
        let json = module_to_json(&file, &parsed.module);
        validate(&json);
        assert!(json.starts_with("{\n  \"name\": [\n    {\n      \"text\": \"demo\""));
        assert!(json.contains("\"kind\": \"FunctionItem\""));
        assert!(json.contains("\"kind\": \"IfExpr\""));
        assert!(json.contains("\"kind\": \"Multiply\""));
        assert!(json.contains("\"summary\": \"Twice the value.\""));
        assert!(json.contains("\"kind\": \"IsValue\""));
    }

    #[test]
    fn strings_are_escaped() {
        let source =
            "module demo\n\nfunction greet() returns Text\n  return \"say \\\"hi\\\"\\n\"\nend\n";
        let file = SourceFile::new("demo.ry", source);
        let parsed = parse(&file.text);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let json = module_to_json(&file, &parsed.module);
        validate(&json);
        assert!(json.contains("say \\\"hi\\\"\\n"));
    }

    #[test]
    fn spans_count_characters() {
        // `é` is two bytes and one character: the names after it shift by one
        let source = "module demo\n\nfunction greet() returns Text\n  return \"é\"\nend\n\nfunction other()\n  return\nend\n";
        let file = SourceFile::new("demo.ry", source);
        let parsed = parse(&file.text);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let json = module_to_json(&file, &parsed.module);
        let byte_offset = source.find("other").expect("the second function");
        let character_offset = source[..byte_offset].chars().count();
        assert_eq!(byte_offset, character_offset + 1);
        let expected = format!(
            "\"text\": \"other\",\n          \"span\": {{\n            \"start\": {character_offset},"
        );
        assert!(json.contains(&expected), "{json}");
    }
}
