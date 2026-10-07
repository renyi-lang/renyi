//! Statements and loops.

use renyi_syntax::ast::{Block, Expr, ExprKind, Name, Ordering, Stmt, StmtKind};
use renyi_syntax::Span;

use super::Compiler;
use crate::bytecode::Op;
use crate::value::Value;

impl Compiler<'_, '_> {
    pub fn statement(&mut self, statement: &Stmt) {
        let span = statement.span;
        match &statement.kind {
            StmtKind::Let { name, value, .. } => {
                self.expr(value);
                let slot = self.declare(&name.text);
                self.emit(Op::Store(slot), span);
            }
            StmtKind::Change { name, value } => {
                let Some(slot) = self.lookup(&name.text) else {
                    self.unsupported(&format!("changing `{}`", name.text), span);
                    return;
                };
                self.move_receiver = move_candidate(name, value);
                self.expr(value);
                self.move_receiver = None;
                self.emit(Op::Store(slot), span);
            }
            StmtKind::If {
                branches,
                otherwise,
            } => {
                let mut ends = Vec::new();
                for (condition, body) in branches {
                    self.expr(condition);
                    let next = self.emit(Op::JumpIfFalse(0), condition.span);
                    self.block(body);
                    ends.push(self.emit(Op::Jump(0), body.span));
                    self.patch(next);
                }
                if let Some(body) = otherwise {
                    self.block(body);
                }
                for end in ends {
                    self.patch(end);
                }
            }
            StmtKind::Match {
                subject,
                arms,
                otherwise,
            } => self.match_common(
                subject,
                arms,
                otherwise.as_ref(),
                span,
                &|compiler, body| compiler.block(body),
            ),
            StmtKind::ForEach {
                bindings,
                source,
                filter,
                order,
                body,
            } => self.for_each(
                bindings,
                source,
                filter.as_ref(),
                order.as_ref(),
                body,
                span,
            ),
            StmtKind::RepeatUntil { condition, body } => {
                self.enter_loop(span);
                let top = self.here();
                self.expr(condition);
                let exit = self.emit(Op::JumpIfTrue(0), condition.span);
                self.block(body);
                self.emit(Op::Jump(top), span);
                self.patch(exit);
                self.leave_loop(top);
            }
            StmtKind::RunConcurrently { within, body } => {
                // the first slice runs the tasks one after the other; the
                // bindings stay visible after `end`
                match within {
                    Some(limit) => {
                        self.expr(limit);
                        let slot = self.temp();
                        self.emit(Op::Deadline(slot), limit.span);
                        for statement in &body.statements {
                            self.statement(statement);
                            self.emit(Op::CheckDeadline(slot), statement.span);
                        }
                    }
                    None => self.statements_inline(body),
                }
            }
            StmtKind::Return(value) => match value {
                Some(value) => {
                    self.expr(value);
                    self.emit(Op::Return, span);
                }
                None => {
                    self.emit(Op::ReturnNothing, span);
                }
            },
            StmtKind::Fail(value) => match value {
                Some(error) => {
                    self.expr(error);
                    self.emit(Op::Fail, span);
                }
                None => self.bare_fail(span),
            },
            StmtKind::Crash(message) => {
                self.expr(message);
                self.emit(Op::Crash, span);
            }
            StmtKind::Break => self.emit_break(span),
            StmtKind::Continue => self.emit_continue(span),
            StmtKind::Ignore(value) | StmtKind::Expression(value) => {
                self.expr(value);
                self.emit(Op::Pop, span);
            }
            StmtKind::Check(condition) => {
                self.expr(condition);
                let text = self.source_text(condition.span);
                let index = self.code.constant(Value::text(text));
                self.emit(Op::Check(index), span);
            }
        }
    }

    /// `for each x in xs where ... sorted by ... do ... end`.
    fn for_each(
        &mut self,
        bindings: &[Name],
        source: &Expr,
        filter: Option<&Expr>,
        order: Option<&Ordering>,
        body: &Block,
        span: Span,
    ) {
        self.push_scope();
        match order {
            None => {
                self.expr(source);
                let iterator = self.temp();
                self.emit(Op::IterInit(iterator), span);
                self.enter_loop(span);
                let top = self.here();
                let next = self.emit(
                    Op::IterNext {
                        slot: iterator,
                        exit: 0,
                    },
                    span,
                );
                self.bind_item(bindings, span);
                if let Some(filter) = filter {
                    self.expr(filter);
                    let skip = self.emit(Op::JumpIfFalse(0), filter.span);
                    self.code.patch_to(skip, top);
                }
                self.block(body);
                self.emit(Op::Jump(top), span);
                self.patch(next);
                self.leave_loop(top);
            }
            Some(order) => {
                // pass 1: the items that pass the filter, each with its key
                let buffer = self.temp();
                self.emit(Op::MakeList(0), span);
                self.emit(Op::Store(buffer), span);
                self.expr(source);
                let iterator = self.temp();
                self.emit(Op::IterInit(iterator), span);
                let top = self.here();
                let next = self.emit(
                    Op::IterNext {
                        slot: iterator,
                        exit: 0,
                    },
                    span,
                );
                let item = self.temp();
                self.emit(Op::Dup, span);
                self.emit(Op::Store(item), span);
                self.bind_item(bindings, span);
                if let Some(filter) = filter {
                    self.expr(filter);
                    let skip = self.emit(Op::JumpIfFalse(0), filter.span);
                    self.code.patch_to(skip, top);
                }
                self.emit(Op::LoadMove(buffer), span);
                self.expr(&order.key);
                self.emit(Op::Load(item), span);
                self.emit(Op::MakePair, span);
                self.emit(Op::ListPush, span);
                self.emit(Op::Store(buffer), span);
                self.emit(Op::Jump(top), span);
                self.patch(next);
                // pass 2: the body over the sorted items
                self.emit(Op::Load(buffer), span);
                self.emit(
                    Op::SortByKey {
                        descending: order.descending,
                    },
                    span,
                );
                let sorted = self.temp();
                self.emit(Op::IterInit(sorted), span);
                self.enter_loop(span);
                let top = self.here();
                let next = self.emit(
                    Op::IterNext {
                        slot: sorted,
                        exit: 0,
                    },
                    span,
                );
                self.bind_item(bindings, span);
                self.block(body);
                self.emit(Op::Jump(top), span);
                self.patch(next);
                self.leave_loop(top);
            }
        }
        self.pop_scope();
    }

    /// Bind the item on top of the stack to the loop's names: one name takes
    /// the item, several take its parts.
    pub fn bind_item(&mut self, bindings: &[Name], span: Span) {
        match bindings {
            [name] => {
                let slot = self.declare(&name.text);
                self.emit(Op::Store(slot), span);
            }
            _ => {
                self.emit(Op::Unpack(bindings.len() as u16), span);
                for name in bindings.iter().rev() {
                    let slot = self.declare(&name.text);
                    self.emit(Op::Store(slot), span);
                }
            }
        }
    }
}

/// `change x to x.method(...)`: the span of the receiver's name token when the
/// variable being set is the receiver and the arguments do not read it, so
/// that the receiver can be moved out of its slot (decision O1).
fn move_candidate(name: &Name, value: &Expr) -> Option<Span> {
    let ExprKind::Call { callee, args } = &value.kind else {
        return None;
    };
    let ExprKind::Member { base, .. } = &callee.kind else {
        return None;
    };
    let ExprKind::Name(receiver) = &base.kind else {
        return None;
    };
    if receiver.text != name.text || args.iter().any(|arg| mentions(&arg.value, &name.text)) {
        return None;
    }
    Some(receiver.span)
}

/// Whether an expression reads a name anywhere inside it.
fn mentions(expr: &Expr, name: &str) -> bool {
    use renyi_syntax::ast::{Outcome, QueryTerminal, TextPiece};
    let any = |items: &[Expr]| items.iter().any(|e| mentions(e, name));
    let outcome = |outcome: &Outcome| match outcome {
        Outcome::Value(e) | Outcome::Crash(e, _) => mentions(e, name),
        Outcome::Fail(e, _) | Outcome::Return(e, _) => {
            e.as_ref().is_some_and(|e| mentions(e, name))
        }
        Outcome::Break(_) | Outcome::Continue(_) => false,
    };
    match &expr.kind {
        ExprKind::Name(n) => n.text == name,
        ExprKind::Integer(_)
        | ExprKind::Decimal(_)
        | ExprKind::RawText(_)
        | ExprKind::Boolean(_)
        | ExprKind::Nothing
        | ExprKind::SelfValue
        | ExprKind::TypeName(_) => false,
        ExprKind::Text { pieces, .. } => pieces.iter().any(|p| match p {
            TextPiece::Hole(e) => mentions(e, name),
            TextPiece::Text(_) => false,
        }),
        ExprKind::Member { base, .. } => mentions(base, name),
        ExprKind::Call { callee, args } => {
            mentions(callee, name) || args.iter().any(|a| mentions(&a.value, name))
        }
        ExprKind::Construct { args, .. } => args.iter().any(|a| mentions(&a.value, name)),
        ExprKind::List(items) => any(items),
        ExprKind::Map(entries) => entries
            .iter()
            .any(|(k, v)| mentions(k, name) || mentions(v, name)),
        ExprKind::Range { from, to, by } => {
            mentions(from, name)
                || mentions(to, name)
                || by.as_ref().is_some_and(|b| mentions(b, name))
        }
        ExprKind::Not(inner) | ExprKind::Paren(inner) => mentions(inner, name),
        ExprKind::Binary { left, right, .. } => mentions(left, name) || mentions(right, name),
        ExprKind::With { base, updates } => {
            mentions(base, name) || updates.iter().any(|a| mentions(&a.value, name))
        }
        ExprKind::Otherwise { value, fallback } => mentions(value, name) || outcome(fallback),
        ExprKind::If {
            branches,
            otherwise,
        } => {
            branches
                .iter()
                .any(|(c, o)| mentions(c, name) || outcome(o))
                || outcome(otherwise)
        }
        ExprKind::Match {
            subject,
            arms,
            otherwise,
        } => {
            mentions(subject, name)
                || arms.iter().any(|arm| {
                    arm.guard.as_ref().is_some_and(|g| mentions(g, name)) || outcome(&arm.body)
                })
                || otherwise.as_ref().is_some_and(|o| outcome(o))
        }
        ExprKind::Query(query) => {
            query.sources.iter().any(|s| mentions(&s.source, name))
                || query.within.as_ref().is_some_and(|e| mentions(e, name))
                || query.filter.as_ref().is_some_and(|e| mentions(e, name))
                || query.order.as_ref().is_some_and(|o| mentions(&o.key, name))
                || query.group_by.as_ref().is_some_and(|e| mentions(e, name))
                || match &query.terminal {
                    QueryTerminal::Collect(e)
                    | QueryTerminal::Sum(e)
                    | QueryTerminal::Any(e)
                    | QueryTerminal::All(e) => mentions(e, name),
                    QueryTerminal::Count | QueryTerminal::First | QueryTerminal::None => false,
                }
        }
    }
}
