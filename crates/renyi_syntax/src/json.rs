//! The syntax tree as JSON, for tools (`renyi parse --json`). Every node is
//! an object with a `node` field naming its kind and a `span` with byte
//! offsets and the line and column where it starts; the other fields mirror
//! the `ast` module one to one.

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
    Encoder { file }.module(module).render()
}

struct Encoder<'a> {
    file: &'a SourceFile,
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

impl Encoder<'_> {
    fn span(&self, span: Span) -> Json {
        let position = self.file.position(span.start);
        Json::Object(vec![
            ("start", Json::Number(span.start)),
            ("end", Json::Number(span.end)),
            ("line", Json::Number(position.line)),
            ("column", Json::Number(position.column)),
        ])
    }

    fn node(&self, kind: &'static str, span: Span, fields: Vec<(&'static str, Json)>) -> Json {
        let mut all = vec![("node", string(kind)), ("span", self.span(span))];
        all.extend(fields);
        Json::Object(all)
    }

    fn name(&self, name: &Name) -> Json {
        Json::Object(vec![
            ("text", string(&name.text)),
            ("span", self.span(name.span)),
        ])
    }

    fn names(&self, names: &[Name]) -> Json {
        Json::Array(names.iter().map(|name| self.name(name)).collect())
    }

    fn type_name(&self, name: &TypeName) -> Json {
        Json::Object(vec![
            ("text", string(&name.text)),
            ("span", self.span(name.span)),
        ])
    }

    fn type_names(&self, names: &[TypeName]) -> Json {
        Json::Array(names.iter().map(|name| self.type_name(name)).collect())
    }

    fn module(&self, module: &Module) -> Json {
        self.node(
            "Module",
            module.span,
            vec![
                ("name", self.names(&module.name)),
                ("docs", self.docs(&module.docs)),
                (
                    "imports",
                    Json::Array(module.imports.iter().map(|i| self.import(i)).collect()),
                ),
                (
                    "items",
                    Json::Array(module.items.iter().map(|i| self.item(i)).collect()),
                ),
                (
                    "comments",
                    Json::Array(
                        module
                            .comments
                            .iter()
                            .map(|span| {
                                Json::Object(vec![
                                    ("text", string(self.file.slice(*span).trim_end())),
                                    ("span", self.span(*span)),
                                ])
                            })
                            .collect(),
                    ),
                ),
            ],
        )
    }

    fn import(&self, import: &Import) -> Json {
        self.node(
            "Import",
            import.span,
            vec![
                ("path", self.names(&import.path)),
                (
                    "alias",
                    optional(import.alias.as_ref().map(|a| self.name(a))),
                ),
                ("exposing", self.type_names(&import.exposing)),
            ],
        )
    }

    fn docs(&self, docs: &Docs) -> Json {
        Json::Object(vec![
            ("purpose", optional(docs.purpose.as_deref().map(string))),
            ("tags", strings(&docs.tags)),
            ("see_also", strings(&docs.see_also)),
            (
                "deprecated",
                optional(docs.deprecated.as_deref().map(string)),
            ),
            ("expose_as_tool", Json::Bool(docs.expose_as_tool)),
            (
                "examples",
                Json::Array(docs.examples.iter().map(|e| self.example(e)).collect()),
            ),
        ])
    }

    fn example(&self, example: &Example) -> Json {
        let outcome = match &example.outcome {
            ExampleOutcome::Is(value) => {
                Json::Object(vec![("kind", string("is")), ("value", self.expr(value))])
            }
            ExampleOutcome::FailsWith(pattern) => Json::Object(vec![
                ("kind", string("fails with")),
                ("pattern", self.pattern(pattern)),
            ]),
        };
        self.node(
            "Example",
            example.span,
            vec![
                ("expression", self.expr(&example.expression)),
                ("outcome", outcome),
            ],
        )
    }

    fn item(&self, item: &Item) -> Json {
        match item {
            Item::Function(function) => self.function(function),
            Item::Type(def) => self.type_def(def),
            Item::Ability(ability) => self.ability(ability),
            Item::Implementation(implementation) => self.implementation(implementation),
            Item::Constant(constant) => self.constant(constant),
            Item::Test(test) => self.test(test),
        }
    }

    fn function(&self, function: &Function) -> Json {
        self.node(
            "Function",
            function.span,
            vec![
                ("public", Json::Bool(function.public)),
                ("name", self.name(&function.name)),
                (
                    "params",
                    Json::Array(function.params.iter().map(|p| self.param(p)).collect()),
                ),
                (
                    "returns",
                    optional(function.returns.as_ref().map(|t| self.ty(t))),
                ),
                ("fails", self.types(&function.fails)),
                ("needs", self.capabilities(&function.needs)),
                (
                    "type_params",
                    optional(function.type_params.as_ref().map(|f| self.for_any(f))),
                ),
                ("docs", self.docs(&function.docs)),
                (
                    "body",
                    optional(function.body.as_ref().map(|b| self.block(b))),
                ),
            ],
        )
    }

    fn param(&self, param: &Param) -> Json {
        self.node(
            "Param",
            param.span,
            vec![
                ("name", self.name(&param.name)),
                ("type", optional(param.ty.as_ref().map(|t| self.ty(t)))),
            ],
        )
    }

    fn capabilities(&self, capabilities: &[Capability]) -> Json {
        Json::Array(
            capabilities
                .iter()
                .map(|capability| {
                    self.node(
                        "Capability",
                        capability.span,
                        vec![
                            ("path", self.names(&capability.path)),
                            ("scope", optional(capability.scope.as_deref().map(string))),
                            (
                                "budget",
                                optional(capability.budget.as_ref().map(|budget| {
                                    Json::Object(vec![
                                        ("count", string(&budget.count)),
                                        ("per", string(&budget.per.text)),
                                    ])
                                })),
                            ),
                            (
                                "only_to",
                                Json::Array(
                                    capability
                                        .only_to
                                        .iter()
                                        .map(|sink| {
                                            Json::Object(vec![
                                                ("path", self.names(&sink.path)),
                                                (
                                                    "scope",
                                                    optional(sink.scope.as_deref().map(string)),
                                                ),
                                            ])
                                        })
                                        .collect(),
                                ),
                            ),
                        ],
                    )
                })
                .collect(),
        )
    }

    fn for_any(&self, for_any: &ForAny) -> Json {
        self.node(
            "ForAny",
            for_any.span,
            vec![
                ("params", self.type_names(&for_any.params)),
                (
                    "constraints",
                    Json::Array(
                        for_any
                            .constraints
                            .iter()
                            .map(|constraint| {
                                Json::Object(vec![
                                    ("param", self.type_name(&constraint.param)),
                                    ("ability", self.ty(&constraint.ability)),
                                ])
                            })
                            .collect(),
                    ),
                ),
            ],
        )
    }

    fn types(&self, types: &[Type]) -> Json {
        Json::Array(types.iter().map(|t| self.ty(t)).collect())
    }

    fn ty(&self, ty: &Type) -> Json {
        match ty {
            Type::Named { name, args, span } => self.node(
                "NamedType",
                *span,
                vec![("name", self.type_name(name)), ("args", self.types(args))],
            ),
            Type::Maybe(inner, span) => {
                self.node("MaybeType", *span, vec![("inner", self.ty(inner))])
            }
            Type::Function {
                params,
                returns,
                fails,
                needs,
                span,
            } => self.node(
                "FunctionType",
                *span,
                vec![
                    ("params", self.types(params)),
                    ("returns", optional(returns.as_ref().map(|t| self.ty(t)))),
                    ("fails", self.types(fails)),
                    ("needs", self.capabilities(needs)),
                ],
            ),
        }
    }

    fn type_def(&self, def: &TypeDef) -> Json {
        let kind = match &def.kind {
            TypeKind::Record { fields, derives } => Json::Object(vec![
                ("kind", string("record")),
                ("fields", self.fields(fields)),
                ("derives", self.derives(derives)),
            ]),
            TypeKind::Sum { variants, derives } => Json::Object(vec![
                ("kind", string("sum")),
                (
                    "variants",
                    Json::Array(
                        variants
                            .iter()
                            .map(|variant| {
                                self.node(
                                    "Variant",
                                    variant.span,
                                    vec![
                                        ("name", self.type_name(&variant.name)),
                                        ("fields", self.fields(&variant.fields)),
                                    ],
                                )
                            })
                            .collect(),
                    ),
                ),
                ("derives", self.derives(derives)),
            ]),
            TypeKind::Subtype { base, refinement } => Json::Object(vec![
                ("kind", string("subtype")),
                ("base", self.ty(base)),
                (
                    "refinement",
                    optional(refinement.as_ref().map(|e| self.expr(e))),
                ),
            ]),
        };
        self.node(
            "Type",
            def.span,
            vec![
                ("public", Json::Bool(def.public)),
                ("name", self.type_name(&def.name)),
                ("type_params", self.type_names(&def.type_params)),
                ("definition", kind),
                ("docs", self.docs(&def.docs)),
            ],
        )
    }

    fn fields(&self, fields: &[Field]) -> Json {
        Json::Array(
            fields
                .iter()
                .map(|field| {
                    self.node(
                        "Field",
                        field.span,
                        vec![
                            ("name", self.name(&field.name)),
                            ("type", self.ty(&field.ty)),
                            (
                                "refinement",
                                optional(field.refinement.as_ref().map(|e| self.expr(e))),
                            ),
                            (
                                "external_name",
                                optional(field.external_name.as_deref().map(string)),
                            ),
                        ],
                    )
                })
                .collect(),
        )
    }

    fn derives(&self, derives: &[Derive]) -> Json {
        Json::Array(
            derives
                .iter()
                .map(|derive| {
                    self.node(
                        "Derive",
                        derive.span,
                        vec![
                            ("ability", self.type_name(&derive.ability)),
                            ("by", self.names(&derive.by)),
                        ],
                    )
                })
                .collect(),
        )
    }

    fn ability(&self, ability: &AbilityDecl) -> Json {
        self.node(
            "Ability",
            ability.span,
            vec![
                ("public", Json::Bool(ability.public)),
                ("name", self.type_name(&ability.name)),
                ("type_params", self.type_names(&ability.type_params)),
                ("requirements", self.types(&ability.requirements)),
                ("docs", self.docs(&ability.docs)),
                (
                    "functions",
                    Json::Array(ability.functions.iter().map(|f| self.function(f)).collect()),
                ),
            ],
        )
    }

    fn implementation(&self, implementation: &AbilityImpl) -> Json {
        self.node(
            "Implementation",
            implementation.span,
            vec![
                ("ability", self.ty(&implementation.ability)),
                ("target", self.ty(&implementation.target)),
                (
                    "type_params",
                    optional(implementation.type_params.as_ref().map(|f| self.for_any(f))),
                ),
                (
                    "functions",
                    Json::Array(
                        implementation
                            .functions
                            .iter()
                            .map(|f| self.function(f))
                            .collect(),
                    ),
                ),
            ],
        )
    }

    fn constant(&self, constant: &Constant) -> Json {
        self.node(
            "Constant",
            constant.span,
            vec![
                ("public", Json::Bool(constant.public)),
                ("name", self.name(&constant.name)),
                ("type", self.ty(&constant.ty)),
                ("value", self.expr(&constant.value)),
                ("docs", self.docs(&constant.docs)),
            ],
        )
    }

    fn test(&self, test: &Test) -> Json {
        self.node(
            "Test",
            test.span,
            vec![
                ("name", string(&test.name)),
                ("needs", self.capabilities(&test.needs)),
                ("replays", optional(test.replays.as_deref().map(string))),
                ("body", self.block(&test.body)),
            ],
        )
    }

    fn block(&self, block: &Block) -> Json {
        self.node(
            "Block",
            block.span,
            vec![(
                "statements",
                Json::Array(block.statements.iter().map(|s| self.statement(s)).collect()),
            )],
        )
    }

    fn statement(&self, statement: &Stmt) -> Json {
        let span = statement.span;
        match &statement.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                value,
            } => self.node(
                "Let",
                span,
                vec![
                    ("mutable", Json::Bool(*mutable)),
                    ("name", self.name(name)),
                    ("type", optional(ty.as_ref().map(|t| self.ty(t)))),
                    ("value", self.expr(value)),
                ],
            ),
            StmtKind::Set { name, value } => self.node(
                "Set",
                span,
                vec![("name", self.name(name)), ("value", self.expr(value))],
            ),
            StmtKind::If {
                branches,
                otherwise,
            } => self.node(
                "If",
                span,
                vec![
                    (
                        "branches",
                        Json::Array(
                            branches
                                .iter()
                                .map(|(condition, block)| {
                                    Json::Object(vec![
                                        ("condition", self.expr(condition)),
                                        ("body", self.block(block)),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                    (
                        "otherwise",
                        optional(otherwise.as_ref().map(|b| self.block(b))),
                    ),
                ],
            ),
            StmtKind::Match {
                subject,
                arms,
                otherwise,
            } => self.node(
                "Match",
                span,
                vec![
                    ("subject", self.expr(subject)),
                    (
                        "arms",
                        Json::Array(
                            arms.iter()
                                .map(|arm| self.arm(arm, |body| self.block(body)))
                                .collect(),
                        ),
                    ),
                    (
                        "otherwise",
                        optional(otherwise.as_ref().map(|b| self.block(b))),
                    ),
                ],
            ),
            StmtKind::ForEach {
                bindings,
                source,
                filter,
                order,
                body,
            } => self.node(
                "ForEach",
                span,
                vec![
                    ("bindings", self.names(bindings)),
                    ("source", self.expr(source)),
                    ("where", optional(filter.as_ref().map(|e| self.expr(e)))),
                    (
                        "sorted_by",
                        optional(order.as_ref().map(|o| self.ordering(o))),
                    ),
                    ("body", self.block(body)),
                ],
            ),
            StmtKind::RepeatUntil { condition, body } => self.node(
                "RepeatUntil",
                span,
                vec![
                    ("condition", self.expr(condition)),
                    ("body", self.block(body)),
                ],
            ),
            StmtKind::RunConcurrently { within, body } => self.node(
                "RunConcurrently",
                span,
                vec![
                    ("within", optional(within.as_ref().map(|e| self.expr(e)))),
                    ("body", self.block(body)),
                ],
            ),
            StmtKind::Return(value) => self.node(
                "Return",
                span,
                vec![("value", optional(value.as_ref().map(|e| self.expr(e))))],
            ),
            StmtKind::Fail(value) => self.node(
                "Fail",
                span,
                vec![("value", optional(value.as_ref().map(|e| self.expr(e))))],
            ),
            StmtKind::Crash(message) => {
                self.node("Crash", span, vec![("message", self.expr(message))])
            }
            StmtKind::Break => self.node("Break", span, vec![]),
            StmtKind::Continue => self.node("Continue", span, vec![]),
            StmtKind::Ignore(value) => self.node("Ignore", span, vec![("value", self.expr(value))]),
            StmtKind::Check(value) => self.node("Check", span, vec![("value", self.expr(value))]),
            StmtKind::Expression(value) => {
                self.node("Expression", span, vec![("value", self.expr(value))])
            }
        }
    }

    fn arm<Body>(&self, arm: &MatchArm<Body>, body: impl Fn(&Body) -> Json) -> Json {
        self.node(
            "Arm",
            arm.span,
            vec![
                ("pattern", self.pattern(&arm.pattern)),
                ("where", optional(arm.guard.as_ref().map(|e| self.expr(e)))),
                ("body", body(&arm.body)),
            ],
        )
    }

    fn ordering(&self, ordering: &Ordering) -> Json {
        Json::Object(vec![
            ("key", self.expr(&ordering.key)),
            ("descending", Json::Bool(ordering.descending)),
        ])
    }

    fn pattern(&self, pattern: &Pattern) -> Json {
        let span = pattern.span();
        match pattern {
            Pattern::Variant { name, fields, .. } => self.node(
                "VariantPattern",
                span,
                vec![
                    ("name", self.type_name(name)),
                    (
                        "fields",
                        Json::Array(
                            fields
                                .iter()
                                .map(|field| {
                                    Json::Object(vec![
                                        ("field", self.name(&field.field)),
                                        (
                                            "pattern",
                                            optional(
                                                field.pattern.as_ref().map(|p| self.pattern(p)),
                                            ),
                                        ),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                ],
            ),
            Pattern::Literal(value) => {
                self.node("LiteralPattern", span, vec![("value", self.expr(value))])
            }
            Pattern::Nothing(_) => self.node("NothingPattern", span, vec![]),
            Pattern::Some(inner, _) => {
                self.node("SomePattern", span, vec![("inner", self.pattern(inner))])
            }
            Pattern::Success(inner, _) => {
                self.node("SuccessPattern", span, vec![("inner", self.pattern(inner))])
            }
            Pattern::Failure(inner, _) => {
                self.node("FailurePattern", span, vec![("inner", self.pattern(inner))])
            }
            Pattern::Binding(name) => {
                self.node("BindingPattern", span, vec![("name", self.name(name))])
            }
            Pattern::Typed { name, ty, .. } => self.node(
                "TypedPattern",
                span,
                vec![("name", self.name(name)), ("type", self.ty(ty))],
            ),
        }
    }

    fn outcome(&self, outcome: &Outcome) -> Json {
        let span = outcome.span();
        match outcome {
            Outcome::Value(value) => self.node("Value", span, vec![("value", self.expr(value))]),
            Outcome::Fail(value, _) => self.node(
                "Fail",
                span,
                vec![("value", optional(value.as_ref().map(|e| self.expr(e))))],
            ),
            Outcome::Return(value, _) => self.node(
                "Return",
                span,
                vec![("value", optional(value.as_ref().map(|e| self.expr(e))))],
            ),
            Outcome::Crash(message, _) => {
                self.node("Crash", span, vec![("message", self.expr(message))])
            }
            Outcome::Break(_) => self.node("Break", span, vec![]),
            Outcome::Continue(_) => self.node("Continue", span, vec![]),
        }
    }

    fn args(&self, args: &[Arg]) -> Json {
        Json::Array(
            args.iter()
                .map(|arg| {
                    self.node(
                        "Arg",
                        arg.span,
                        vec![
                            ("name", optional(arg.name.as_ref().map(|n| self.name(n)))),
                            ("value", self.expr(&arg.value)),
                        ],
                    )
                })
                .collect(),
        )
    }

    fn expr(&self, expr: &Expr) -> Json {
        let span = expr.span;
        match &expr.kind {
            ExprKind::Integer(digits) => {
                self.node("Integer", span, vec![("value", string(digits))])
            }
            ExprKind::Decimal(digits) => {
                self.node("Decimal", span, vec![("value", string(digits))])
            }
            ExprKind::Text { pieces, block } => self.node(
                "Text",
                span,
                vec![
                    ("block", Json::Bool(*block)),
                    (
                        "pieces",
                        Json::Array(
                            pieces
                                .iter()
                                .map(|piece| match piece {
                                    TextPiece::Text(value) => Json::Object(vec![
                                        ("kind", string("text")),
                                        ("value", string(value)),
                                    ]),
                                    TextPiece::Hole(hole) => Json::Object(vec![
                                        ("kind", string("hole")),
                                        ("value", self.expr(hole)),
                                    ]),
                                })
                                .collect(),
                        ),
                    ),
                ],
            ),
            ExprKind::RawText(value) => self.node("RawText", span, vec![("value", string(value))]),
            ExprKind::Boolean(value) => {
                self.node("Boolean", span, vec![("value", Json::Bool(*value))])
            }
            ExprKind::Nothing => self.node("Nothing", span, vec![]),
            ExprKind::SelfValue => self.node("Self", span, vec![]),
            ExprKind::Name(name) => self.node("Name", span, vec![("name", self.name(name))]),
            ExprKind::TypeName(name) => {
                self.node("TypeName", span, vec![("name", self.type_name(name))])
            }
            ExprKind::Member { base, name } => self.node(
                "Member",
                span,
                vec![("base", self.expr(base)), ("name", self.name(name))],
            ),
            ExprKind::Call { callee, args } => self.node(
                "Call",
                span,
                vec![("callee", self.expr(callee)), ("args", self.args(args))],
            ),
            ExprKind::Construct { name, args } => self.node(
                "Construct",
                span,
                vec![("name", self.type_name(name)), ("args", self.args(args))],
            ),
            ExprKind::List(items) => self.node(
                "List",
                span,
                vec![(
                    "items",
                    Json::Array(items.iter().map(|item| self.expr(item)).collect()),
                )],
            ),
            ExprKind::Map(entries) => self.node(
                "Map",
                span,
                vec![(
                    "entries",
                    Json::Array(
                        entries
                            .iter()
                            .map(|(key, value)| {
                                Json::Object(vec![
                                    ("key", self.expr(key)),
                                    ("value", self.expr(value)),
                                ])
                            })
                            .collect(),
                    ),
                )],
            ),
            ExprKind::Range { from, to, by } => self.node(
                "Range",
                span,
                vec![
                    ("from", self.expr(from)),
                    ("to", self.expr(to)),
                    ("by", optional(by.as_ref().map(|e| self.expr(e)))),
                ],
            ),
            ExprKind::Not(inner) => self.node("Not", span, vec![("value", self.expr(inner))]),
            ExprKind::Binary { op, left, right } => self.node(
                "Binary",
                span,
                vec![
                    ("op", string(op.spelling())),
                    ("left", self.expr(left)),
                    ("right", self.expr(right)),
                ],
            ),
            ExprKind::With { base, updates } => self.node(
                "With",
                span,
                vec![("base", self.expr(base)), ("updates", self.args(updates))],
            ),
            ExprKind::Otherwise { value, fallback } => self.node(
                "Otherwise",
                span,
                vec![
                    ("value", self.expr(value)),
                    ("fallback", self.outcome(fallback)),
                ],
            ),
            ExprKind::If {
                branches,
                otherwise,
            } => self.node(
                "IfExpr",
                span,
                vec![
                    (
                        "branches",
                        Json::Array(
                            branches
                                .iter()
                                .map(|(condition, outcome)| {
                                    Json::Object(vec![
                                        ("condition", self.expr(condition)),
                                        ("outcome", self.outcome(outcome)),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                    ("otherwise", self.outcome(otherwise)),
                ],
            ),
            ExprKind::Match {
                subject,
                arms,
                otherwise,
            } => self.node(
                "MatchExpr",
                span,
                vec![
                    ("subject", self.expr(subject)),
                    (
                        "arms",
                        Json::Array(
                            arms.iter()
                                .map(|arm| self.arm(arm, |outcome| self.outcome(outcome)))
                                .collect(),
                        ),
                    ),
                    (
                        "otherwise",
                        optional(otherwise.as_ref().map(|o| self.outcome(o))),
                    ),
                ],
            ),
            ExprKind::Query(query) => self.query(query),
            ExprKind::Paren(inner) => self.node("Paren", span, vec![("value", self.expr(inner))]),
        }
    }

    fn query(&self, query: &Query) -> Json {
        let (terminal, argument) = match &query.terminal {
            QueryTerminal::Collect(value) => ("collect", Some(self.expr(value))),
            QueryTerminal::Sum(value) => ("sum", Some(self.expr(value))),
            QueryTerminal::Count => ("count", None),
            QueryTerminal::First => ("first", None),
            QueryTerminal::Any(value) => ("any", Some(self.expr(value))),
            QueryTerminal::All(value) => ("all", Some(self.expr(value))),
            QueryTerminal::None => ("none", None),
        };
        self.node(
            "Query",
            query.span,
            vec![
                (
                    "sources",
                    Json::Array(
                        query
                            .sources
                            .iter()
                            .map(|source| {
                                Json::Object(vec![
                                    ("bindings", self.names(&source.bindings)),
                                    ("source", self.expr(&source.source)),
                                ])
                            })
                            .collect(),
                    ),
                ),
                ("concurrently", Json::Bool(query.concurrently)),
                (
                    "within",
                    optional(query.within.as_ref().map(|e| self.expr(e))),
                ),
                (
                    "where",
                    optional(query.filter.as_ref().map(|e| self.expr(e))),
                ),
                (
                    "sorted_by",
                    optional(query.order.as_ref().map(|o| self.ordering(o))),
                ),
                (
                    "group_by",
                    optional(query.group_by.as_ref().map(|e| self.expr(e))),
                ),
                ("terminal", string(terminal)),
                ("argument", optional(argument)),
            ],
        )
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
        assert!(json.contains("\"node\": \"Module\""));
        assert!(json.contains("\"node\": \"Function\""));
        assert!(json.contains("\"node\": \"IfExpr\""));
        assert!(json.contains("\"op\": \"*\""));
        assert!(json.contains("\"purpose\": \"Twice the value.\""));
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
}
