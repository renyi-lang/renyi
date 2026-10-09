//! Statements and loops.

use renyi_syntax::ast::{Arg, Block, Expr, ExprKind, Name, Ordering, Stmt, StmtKind};
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
                // the slot is stored into right after: an update of the
                // record in it is done in place (AU11), a call takes the
                // value out of it (O1)
                if !self.with_in_slot(name, value) {
                    self.move_receiver = move_candidate(name, value);
                    self.expr(value);
                    self.move_receiver = None;
                }
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
                    // the frame's slots die with the return: `return x with
                    // ...` updates the record in its slot (AU11)
                    let holder = match &value.kind {
                        ExprKind::With { base, .. } => match &base.kind {
                            ExprKind::Name(holder) => Some(holder),
                            _ => None,
                        },
                        _ => None,
                    };
                    if !holder.is_some_and(|holder| self.with_in_slot(holder, value)) {
                        self.expr(value);
                    }
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

    /// `x with f: e, ...` where the slot of `x` is dead or stored into right
    /// after the statement (decision AU11): the update done in the slot, so
    /// that a record held once is changed in place, with each field whose
    /// update is a call on it taken out of the record first, so that the
    /// call finds it held once too; `false` when the value is not such an
    /// update of `holder`, or when its updates may leave the loop with the
    /// record moved out (`update_plan`), and nothing was emitted.
    fn with_in_slot(&mut self, holder: &Name, value: &Expr) -> bool {
        let ExprKind::With { base, updates } = &value.kind else {
            return false;
        };
        let ExprKind::Name(receiver) = &base.kind else {
            return false;
        };
        if receiver.text != holder.text {
            return false;
        }
        let Some(slot) = self.lookup(&holder.text) else {
            return false;
        };
        let plan = update_plan(holder, updates);
        if plan == UpdatePlan::Copy {
            return false;
        }
        for update in updates {
            let name = update
                .name
                .as_ref()
                .map(|n| n.text.clone())
                .unwrap_or_default();
            let index = self.name_constant(&name);
            self.emit(Op::Const(index), update.span);
            if plan == UpdatePlan::Takes {
                self.take_receiver = taken_receiver(holder, update);
            }
            self.expr(&update.value);
            self.take_receiver = None;
        }
        self.emit(
            Op::WithSlot {
                slot,
                fields: updates.len() as u16,
            },
            value.span,
        );
        true
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

/// `change x to x.method(...)` and `change x to f(..., x, ...)`: the span of
/// the name token of `x` that is moved out of its slot into the call
/// (decision O1; the argument form since AU11): the receiver, or the one
/// argument that is `x` itself, when nothing else in the call pins `x`.
fn move_candidate(name: &Name, value: &Expr) -> Option<Span> {
    let ExprKind::Call { callee, args } = &value.kind else {
        return None;
    };
    let text = Some(name.text.as_str());
    let receiver = match &callee.kind {
        ExprKind::Member { base, .. } => match &base.kind {
            ExprKind::Name(receiver) if receiver.text == name.text => Some(receiver.span),
            _ => None,
        },
        _ => None,
    };
    if let Some(span) = receiver {
        return (!args.iter().any(|arg| pins(&arg.value, text))).then_some(span);
    }
    if pins(callee, text) {
        return None;
    }
    let mut moved = None;
    for arg in args {
        match &arg.value.kind {
            ExprKind::Name(found) if found.text == name.text && moved.is_none() => {
                moved = Some(found.span);
            }
            _ => {
                if pins(&arg.value, text) {
                    return None;
                }
            }
        }
    }
    moved
}

/// How `x with f: e, ...` is emitted when the slot of `x` is dead or stored
/// into right after (decision AU11).
#[derive(Clone, Copy, PartialEq, Eq)]
enum UpdatePlan {
    /// Every update is `f: x.f.method(args)` with the names distinct and no
    /// argument pinning `x`: each field is taken out of the record
    /// (`TakeField`) and the record updated in the slot (`WithSlot`).
    Takes,
    /// No update may leave the loop: the record updated in the slot, the
    /// fields read as they are.
    Move,
    /// The updated copy, as `with` is elsewhere.
    Copy,
}

fn update_plan(holder: &Name, updates: &[Arg]) -> UpdatePlan {
    let names: Vec<&str> = updates
        .iter()
        .filter_map(|update| update.name.as_ref().map(|n| n.text.as_str()))
        .collect();
    let distinct = names.len() == updates.len()
        && names
            .iter()
            .enumerate()
            .all(|(index, name)| !names[..index].contains(name));
    if distinct
        && updates
            .iter()
            .all(|update| taken_receiver(holder, update).is_some())
    {
        return UpdatePlan::Takes;
    }
    if updates.iter().all(|update| !pins(&update.value, None)) {
        return UpdatePlan::Move;
    }
    UpdatePlan::Copy
}

/// The name token of `x` in the update `f: x.f.method(args)`, when the update
/// has that form and no argument pins `x`: the field is taken out of the
/// record there.
fn taken_receiver(holder: &Name, update: &Arg) -> Option<Span> {
    let field = update.name.as_ref()?;
    let ExprKind::Call { callee, args } = &update.value.kind else {
        return None;
    };
    let ExprKind::Member { base: receiver, .. } = &callee.kind else {
        return None;
    };
    let ExprKind::Member { base, name: read } = &receiver.kind else {
        return None;
    };
    let ExprKind::Name(found) = &base.kind else {
        return None;
    };
    if found.text != holder.text || read.text != field.text {
        return None;
    }
    if args
        .iter()
        .any(|arg| pins(&arg.value, Some(holder.text.as_str())))
    {
        return None;
    }
    Some(found.span)
}

/// Whether an expression pins the slot of a name while it runs: it reads the
/// name anywhere inside it, or it may leave the loop it is in (`break`,
/// `continue`) with the slot's value moved out and the statement unfinished;
/// with no name, the second alone. What keeps a value from being moved out of
/// its slot into a call or an update (decisions O1 and AU11).
fn pins(expr: &Expr, name: Option<&str>) -> bool {
    use renyi_syntax::ast::{Outcome, QueryTerminal, TextPiece};
    let any = |items: &[Expr]| items.iter().any(|e| pins(e, name));
    let outcome = |outcome: &Outcome| match outcome {
        Outcome::Value(e) | Outcome::Crash(e, _) => pins(e, name),
        Outcome::Fail(e, _) | Outcome::Return(e, _) => e.as_ref().is_some_and(|e| pins(e, name)),
        Outcome::Break(_) | Outcome::Continue(_) => true,
    };
    match &expr.kind {
        ExprKind::Name(n) => name == Some(n.text.as_str()),
        ExprKind::Integer(_)
        | ExprKind::Decimal(_)
        | ExprKind::RawText(_)
        | ExprKind::Boolean(_)
        | ExprKind::Nothing
        | ExprKind::SelfValue
        | ExprKind::TypeName(_) => false,
        ExprKind::Text { pieces, .. } => pieces.iter().any(|p| match p {
            TextPiece::Hole(e) => pins(e, name),
            TextPiece::Text(_) => false,
        }),
        ExprKind::Member { base, .. } => pins(base, name),
        ExprKind::Call { callee, args } => {
            pins(callee, name) || args.iter().any(|a| pins(&a.value, name))
        }
        ExprKind::Construct { args, .. } => args.iter().any(|a| pins(&a.value, name)),
        ExprKind::List(items) => any(items),
        ExprKind::Map(entries) => entries.iter().any(|(k, v)| pins(k, name) || pins(v, name)),
        ExprKind::Range { from, to, by } => {
            pins(from, name) || pins(to, name) || by.as_ref().is_some_and(|b| pins(b, name))
        }
        ExprKind::Not(inner) | ExprKind::Paren(inner) => pins(inner, name),
        ExprKind::Binary { left, right, .. } => pins(left, name) || pins(right, name),
        ExprKind::With { base, updates } => {
            pins(base, name) || updates.iter().any(|a| pins(&a.value, name))
        }
        ExprKind::Otherwise { value, fallback } => pins(value, name) || outcome(fallback),
        ExprKind::If {
            branches,
            otherwise,
        } => branches.iter().any(|(c, o)| pins(c, name) || outcome(o)) || outcome(otherwise),
        ExprKind::Match {
            subject,
            arms,
            otherwise,
        } => {
            pins(subject, name)
                || arms.iter().any(|arm| {
                    arm.guard.as_ref().is_some_and(|g| pins(g, name)) || outcome(&arm.body)
                })
                || otherwise.as_ref().is_some_and(|o| outcome(o))
        }
        ExprKind::Query(query) => {
            query.sources.iter().any(|s| pins(&s.source, name))
                || query.within.as_ref().is_some_and(|e| pins(e, name))
                || query.filter.as_ref().is_some_and(|e| pins(e, name))
                || query.order.as_ref().is_some_and(|o| pins(&o.key, name))
                || query.group_by.as_ref().is_some_and(|e| pins(e, name))
                || match &query.terminal {
                    QueryTerminal::Collect(e)
                    | QueryTerminal::Sum(e)
                    | QueryTerminal::Any(e)
                    | QueryTerminal::All(e) => pins(e, name),
                    QueryTerminal::Count | QueryTerminal::First | QueryTerminal::None => false,
                }
        }
    }
}
