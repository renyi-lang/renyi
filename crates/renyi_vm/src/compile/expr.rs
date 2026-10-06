//! Expressions: every one leaves exactly one value on the operand stack, or
//! jumps away (`fail`, `return`, `crash`, `break`, `continue` as the outcome
//! of a branch).

use renyi_check::{NumberKind, Target};
use renyi_syntax::ast::{
    Arg, BinaryOp, Expr, ExprKind, MatchArm, Name, Outcome, TextPiece, TypeName,
};
use renyi_syntax::Span;

use super::Compiler;
use crate::bytecode::Op;
use crate::decimal::Decimal;
use crate::integer::Int;
use crate::value::Value;

impl Compiler<'_, '_> {
    pub fn expr(&mut self, expr: &Expr) {
        let span = expr.span;
        match &expr.kind {
            ExprKind::Integer(text) => self.number(text, span, false),
            ExprKind::Decimal(text) => self.number(text, span, true),
            ExprKind::Text { pieces, .. } => self.text(pieces, span),
            ExprKind::RawText(text) => self.constant(Value::text(text), span),
            ExprKind::Boolean(value) => self.constant(Value::Boolean(*value), span),
            ExprKind::Nothing => {
                self.emit(Op::Nothing, span);
            }
            ExprKind::SelfValue => match self.lookup("self") {
                Some(slot) => {
                    self.emit(Op::Load(slot), span);
                }
                None => self.unsupported("`self` here", span),
            },
            ExprKind::Name(name) => self.name(name),
            ExprKind::TypeName(name) => self.bare_type_name(name, span),
            ExprKind::Member { base, name } => self.member(base, name, span),
            ExprKind::Call { callee, args } => self.call(callee, args, span),
            ExprKind::Construct { name, args } => self.construct(name, args, span),
            ExprKind::List(items) => {
                for item in items {
                    self.expr(item);
                }
                self.emit(Op::MakeList(items.len() as u16), span);
            }
            ExprKind::Map(entries) => {
                for (key, value) in entries {
                    self.expr(key);
                    self.expr(value);
                }
                self.emit(Op::MakeMap(entries.len() as u16), span);
            }
            ExprKind::Range { from, to, by } => {
                self.expr(from);
                self.expr(to);
                if let Some(by) = by {
                    self.expr(by);
                }
                self.emit(
                    Op::MakeRange {
                        stepped: by.is_some(),
                    },
                    span,
                );
            }
            ExprKind::Not(inner) => {
                self.expr(inner);
                self.emit(Op::Not, span);
            }
            ExprKind::Binary { op, left, right } => self.binary(*op, left, right, span),
            ExprKind::With { base, updates } => {
                self.expr(base);
                for update in updates {
                    let name = update
                        .name
                        .as_ref()
                        .map(|n| n.text.clone())
                        .unwrap_or_default();
                    let index = self.name_constant(&name);
                    self.emit(Op::Const(index), update.span);
                    self.expr(&update.value);
                }
                self.emit(Op::With(updates.len() as u16), span);
            }
            ExprKind::Otherwise { value, fallback } => self.otherwise(value, fallback, span),
            ExprKind::If {
                branches,
                otherwise,
            } => {
                let mut ends = Vec::new();
                for (condition, outcome) in branches {
                    self.expr(condition);
                    let next = self.emit(Op::JumpIfFalse(0), condition.span);
                    self.outcome(outcome);
                    ends.push(self.emit(Op::Jump(0), outcome.span()));
                    self.patch(next);
                }
                self.outcome(otherwise);
                for end in ends {
                    self.patch(end);
                }
            }
            ExprKind::Match {
                subject,
                arms,
                otherwise,
            } => self.match_common(
                subject,
                arms,
                otherwise.as_deref(),
                span,
                &|compiler, outcome| compiler.outcome(outcome),
            ),
            ExprKind::Query(query) => self.query(query),
            ExprKind::Paren(inner) => self.expr(inner),
        }
    }

    /// A branch of an `if` or `match` expression: a value, or a way out.
    pub fn outcome(&mut self, outcome: &Outcome) {
        match outcome {
            Outcome::Value(value) => self.expr(value),
            Outcome::Fail(None, span) => self.bare_fail(*span),
            Outcome::Fail(Some(error), span) => {
                self.expr(error);
                self.emit(Op::Fail, *span);
            }
            Outcome::Return(value, span) => match value {
                Some(value) => {
                    self.expr(value);
                    self.emit(Op::Return, *span);
                }
                None => {
                    self.emit(Op::ReturnNothing, *span);
                }
            },
            Outcome::Crash(message, span) => {
                self.expr(message);
                self.emit(Op::Crash, *span);
            }
            Outcome::Break(span) => self.emit_break(*span),
            Outcome::Continue(span) => self.emit_continue(*span),
        }
    }

    /// `fail` without an error: ends a test; elsewhere the checker has
    /// rejected it.
    pub fn bare_fail(&mut self, span: Span) {
        if self.in_test {
            self.constant(Value::text("the test called `fail`"), span);
            self.emit(Op::Fail, span);
        } else {
            self.unsupported("`fail` without an error outside `otherwise`", span);
        }
    }

    // ------------------------------------------------------------ literals

    fn number(&mut self, text: &str, span: Span, decimal_syntax: bool) {
        let digits: String = text.chars().filter(|c| *c != '_').collect();
        let kind = self.number_at(span).unwrap_or(if decimal_syntax {
            NumberKind::Decimal
        } else {
            NumberKind::Integer
        });
        let value = match kind {
            NumberKind::Integer => Int::parse(&digits).map(Value::Integer),
            NumberKind::Decimal => Decimal::parse(&digits).map(Value::Decimal),
            NumberKind::Float => digits.parse::<f64>().ok().map(Value::Float),
        };
        match value {
            Some(value) => self.constant(value, span),
            None => self.unsupported(&format!("the number `{text}`"), span),
        }
    }

    fn text(&mut self, pieces: &[TextPiece], span: Span) {
        match pieces {
            [] => self.constant(Value::text(""), span),
            [TextPiece::Text(text)] => self.constant(Value::text(text), span),
            _ => {
                for piece in pieces {
                    match piece {
                        TextPiece::Text(text) => self.constant(Value::text(text), span),
                        TextPiece::Hole(hole) => {
                            self.expr(hole);
                            self.emit(Op::ToText, hole.span);
                        }
                    }
                }
                self.emit(Op::Concat(pieces.len() as u16), span);
            }
        }
    }

    // ------------------------------------------------------------ names

    fn name(&mut self, name: &Name) {
        if let Some(slot) = self.lookup(&name.text) {
            let op = if self.move_receiver == Some(name.span) {
                Op::LoadMove(slot)
            } else {
                Op::Load(slot)
            };
            self.emit(op, name.span);
            return;
        }
        for target in self.targets(name.span).to_vec() {
            match target {
                Target::Constant(module, text) => {
                    match self.global(module, &text) {
                        Some(index) => {
                            self.emit(Op::Global(index), name.span);
                        }
                        None => self.unsupported(&format!("the constant `{text}`"), name.span),
                    }
                    return;
                }
                Target::Function(function) => {
                    self.constant(Value::Function(function), name.span);
                    return;
                }
                _ => {}
            }
        }
        self.unsupported(&format!("the name `{}`", name.text), name.span);
    }

    /// `Point`, `Red`: a variant without fields used as a value.
    fn bare_type_name(&mut self, name: &TypeName, span: Span) {
        for target in self.targets(name.span).to_vec() {
            match target {
                Target::Variant(ty, tag) => {
                    self.emit(
                        Op::ConstructVariant {
                            ty,
                            tag: tag as u16,
                            fields: 0,
                        },
                        span,
                    );
                    return;
                }
                Target::Type(ty) => {
                    self.emit(Op::Construct { ty, fields: 0 }, span);
                    return;
                }
                _ => {}
            }
        }
        self.unsupported(&format!("`{}` as a value", name.text), span);
    }

    fn function_target(&self, span: Span) -> Option<usize> {
        self.targets(span).iter().find_map(|t| match t {
            Target::Function(id) => Some(*id),
            _ => None,
        })
    }

    fn ability_target(&self, span: Span) -> Option<(usize, usize)> {
        self.targets(span).iter().find_map(|t| match t {
            Target::AbilityMethod(ability, index) => Some((*ability, *index)),
            _ => None,
        })
    }

    fn constant_target(&self, span: Span) -> Option<(usize, String)> {
        self.targets(span).iter().find_map(|t| match t {
            Target::Constant(module, name) => Some((*module, name.clone())),
            _ => None,
        })
    }

    fn member(&mut self, base: &Expr, name: &Name, span: Span) {
        // `module.function` as a value, or `module.constant`
        if let ExprKind::Name(namespace) = &base.kind {
            if self.lookup(&namespace.text).is_none() {
                if let Some(function) = self.function_target(name.span) {
                    self.constant(Value::Function(function), span);
                    return;
                }
                if let Some((module, text)) = self.constant_target(name.span) {
                    match self.global(module, &text) {
                        Some(index) => {
                            self.emit(Op::Global(index), span);
                        }
                        None => self.unsupported(&format!("the constant `{text}`"), span),
                    }
                    return;
                }
            }
        }
        self.expr(base);
        let index = self.name_constant(&name.text);
        self.emit(Op::Field(index), span);
    }

    // ------------------------------------------------------------ calls

    fn call(&mut self, callee: &Expr, args: &[Arg], span: Span) {
        match &callee.kind {
            ExprKind::Name(name) => {
                if let Some(function) = self.function_target(name.span) {
                    let params = self.param_names(function);
                    self.args(args, &params);
                    self.emit_call(function, args.len(), span);
                    return;
                }
            }
            ExprKind::Member { base, name } => {
                if let Some(function) = self.function_target(name.span) {
                    let params = self.param_names(function);
                    if self.is_method(function) {
                        self.expr(base);
                        self.args(args, params.get(1..).unwrap_or(&[]));
                        self.emit_call(function, args.len() + 1, span);
                    } else {
                        // `module.function(...)`: the base names a module
                        self.args(args, &params);
                        self.emit_call(function, args.len(), span);
                    }
                    return;
                }
                if let Some((ability, method)) = self.ability_target(name.span) {
                    self.expr(base);
                    self.args(args, &[]);
                    self.emit(
                        Op::CallAbility {
                            ability,
                            method: method as u16,
                            args: (args.len() + 1) as u16,
                        },
                        span,
                    );
                    return;
                }
            }
            _ => {}
        }
        // a function value
        self.expr(callee);
        self.args(args, &[]);
        self.emit(Op::CallValue(args.len() as u16), span);
    }

    fn emit_call(&mut self, function: usize, argc: usize, span: Span) {
        if let Some(index) = self.result_type_at(span) {
            self.emit(Op::ResultType(index), span);
        }
        self.emit(
            Op::Call {
                function,
                args: argc as u16,
            },
            span,
        );
    }

    /// The arguments in parameter order: named arguments are matched to the
    /// names given, positional ones stay where they are.
    fn args(&mut self, args: &[Arg], params: &[String]) {
        let mut order: Vec<&Arg> = args.iter().collect();
        if args.len() == params.len() && args.iter().all(|a| a.name.is_some()) {
            let by_name: Vec<&Arg> = params
                .iter()
                .filter_map(|param| {
                    args.iter()
                        .find(|a| a.name.as_ref().is_some_and(|n| n.text == *param))
                })
                .collect();
            if by_name.len() == args.len() {
                order = by_name;
            }
        }
        for arg in order {
            self.expr(&arg.value);
        }
    }

    fn construct(&mut self, name: &TypeName, args: &[Arg], span: Span) {
        for target in self.targets(name.span).to_vec() {
            match target {
                Target::Variant(ty, tag) => {
                    let fields = self.variant_fields(ty, tag);
                    self.args(args, &fields);
                    self.emit(
                        Op::ConstructVariant {
                            ty,
                            tag: tag as u16,
                            fields: args.len() as u16,
                        },
                        span,
                    );
                    return;
                }
                Target::Type(ty) => {
                    if ty == self.world.builtins.pair {
                        self.args(args, &["left".to_string(), "right".to_string()]);
                        self.emit(Op::MakePair, span);
                        return;
                    }
                    let fields = self.record_fields(ty);
                    self.args(args, &fields);
                    self.emit(
                        Op::Construct {
                            ty,
                            fields: args.len() as u16,
                        },
                        span,
                    );
                    return;
                }
                _ => {}
            }
        }
        self.unsupported(&format!("constructing `{}`", name.text), span);
    }

    // ------------------------------------------------------------ operators

    fn binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr, span: Span) {
        match op {
            BinaryOp::And => {
                self.expr(left);
                self.emit(Op::Dup, span);
                let skip = self.emit(Op::JumpIfFalse(0), span);
                self.emit(Op::Pop, span);
                self.expr(right);
                self.patch(skip);
            }
            BinaryOp::Or => {
                self.expr(left);
                self.emit(Op::Dup, span);
                let skip = self.emit(Op::JumpIfTrue(0), span);
                self.emit(Op::Pop, span);
                self.expr(right);
                self.patch(skip);
            }
            _ => {
                self.expr(left);
                self.expr(right);
                self.emit(Op::Binary(op), span);
            }
        }
    }

    // ------------------------------------------------------------ otherwise

    /// `value otherwise fallback`: the value is evaluated in a handled
    /// region; a failure or a `nothing` lands in the fallback with the
    /// absent value on top of the stack.
    fn otherwise(&mut self, value: &Expr, fallback: &Outcome, span: Span) {
        // the checker says whether this `otherwise` handles a failure or an
        // absence: a fallible call that returns nothing succeeds with `Nothing`
        let fallible = self
            .targets(span)
            .iter()
            .any(|target| matches!(target, Target::Otherwise { fallible: true }));
        let handler = self.push_handler(span);
        self.expr(value);
        self.pop_handler(span);
        self.patch(handler);
        let to_fallback = if fallible {
            self.emit(Op::JumpIfFailure(0), span)
        } else {
            self.emit(Op::JumpIfAbsent(0), span)
        };
        let to_end = self.emit(Op::Jump(0), span);
        self.patch(to_fallback);
        match fallback {
            Outcome::Value(default) => {
                self.emit(Op::Pop, span);
                self.expr(default);
            }
            Outcome::Fail(None, fail_span) => {
                // pass the failure on as it is
                self.emit(Op::Fail, *fail_span);
            }
            Outcome::Fail(Some(error), fail_span) => {
                self.emit(Op::Pop, span);
                self.expr(error);
                self.emit(Op::Fail, *fail_span);
            }
            other => {
                self.emit(Op::Pop, span);
                self.outcome(other);
            }
        }
        self.patch(to_end);
    }

    // ------------------------------------------------------------ match

    /// A `match` as a statement or an expression: the subject goes into a
    /// temporary slot, every arm tests its pattern and guard, and the first
    /// fit runs its body. A `success`/`failure` arm makes the subject a
    /// handled region, so that a failure lands in the slot as a `Failure`.
    pub fn match_common<B>(
        &mut self,
        subject: &Expr,
        arms: &[MatchArm<B>],
        otherwise: Option<&B>,
        span: Span,
        body: &dyn Fn(&mut Self, &B),
    ) {
        let slot = self.temp();
        let fallible = arms.iter().any(|arm| {
            matches!(
                arm.pattern,
                renyi_syntax::ast::Pattern::Success(..) | renyi_syntax::ast::Pattern::Failure(..)
            )
        });
        if fallible {
            let handler = self.push_handler(span);
            self.expr(subject);
            self.pop_handler(span);
            self.patch(handler);
        } else {
            self.expr(subject);
        }
        self.emit(Op::Store(slot), span);
        let mut ends = Vec::new();
        for arm in arms {
            self.push_scope();
            let mut next = self.pattern(&arm.pattern, slot);
            if let Some(guard) = &arm.guard {
                self.expr(guard);
                next.push(self.emit(Op::JumpIfFalse(0), guard.span));
            }
            body(self, &arm.body);
            ends.push(self.emit(Op::Jump(0), arm.span));
            self.pop_scope();
            for at in next {
                self.patch(at);
            }
        }
        match otherwise {
            Some(fallback) => body(self, fallback),
            None => {
                self.constant(Value::text("no arm of the match fits the value"), span);
                self.emit(Op::Crash, span);
            }
        }
        for end in ends {
            self.patch(end);
        }
    }
}
