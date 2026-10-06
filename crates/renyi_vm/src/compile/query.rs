//! Queries: `for each ... [where] [sorted by] [group by] collect | sum |
//! count | first | any | all`. A query is a loop over its sources with an
//! accumulator slot; `sorted by` collects the items with their keys first
//! and runs the terminal over the sorted list.

use renyi_check::NumberKind;
use renyi_syntax::ast::{Query, QuerySource, QueryTerminal};
use renyi_syntax::Span;

use super::Compiler;
use crate::bytecode::{GroupFold, Op};
use crate::decimal::Decimal;
use crate::value::Value;

impl Compiler<'_, '_> {
    pub fn query(&mut self, query: &Query) {
        let span = query.span;
        self.push_scope();
        let accumulator = self.temp();
        let grouped = query.group_by.is_some();
        match (&query.terminal, grouped) {
            (QueryTerminal::Collect(_), false) => {
                self.emit(Op::MakeList(0), span);
            }
            // a terminal after `group by` applies per group (decision M1)
            (_, true) | (QueryTerminal::None, false) => {
                self.emit(Op::MakeMap(0), span);
            }
            (QueryTerminal::Sum(_), _) => {
                let zero = match self.number_at(span) {
                    Some(NumberKind::Decimal) => Value::Decimal(Decimal::zero()),
                    Some(NumberKind::Float) => Value::Float(0.0),
                    _ => Value::integer(0),
                };
                self.constant(zero, span);
            }
            (QueryTerminal::Count, _) => self.constant(Value::integer(0), span),
            (QueryTerminal::First, _) => {
                self.emit(Op::Nothing, span);
            }
            (QueryTerminal::Any(_), _) => self.constant(Value::Boolean(false), span),
            (QueryTerminal::All(_), _) => self.constant(Value::Boolean(true), span),
        }
        self.emit(Op::Store(accumulator), span);
        let deadline = query.within.as_ref().map(|limit| {
            self.expr(limit);
            let slot = self.temp();
            self.emit(Op::Deadline(slot), limit.span);
            slot
        });
        let buffer = query.order.as_ref().map(|_| {
            let slot = self.temp();
            self.emit(Op::MakeList(0), span);
            self.emit(Op::Store(slot), span);
            slot
        });
        let mut dones = Vec::new();
        // pass 1: the source loops
        let (tops, nexts, item) = self.source_loops(&query.sources, span);
        let innermost = *tops.last().expect("a source");
        if let Some(filter) = &query.filter {
            self.expr(filter);
            let skip = self.emit(Op::JumpIfFalse(0), filter.span);
            self.code.patch_to(skip, innermost);
        }
        if let Some(slot) = deadline {
            self.emit(Op::CheckDeadline(slot), span);
        }
        match (buffer, &query.order) {
            (Some(buffer), Some(order)) => {
                self.emit(Op::LoadMove(buffer), span);
                self.expr(&order.key);
                self.emit(Op::Load(item), span);
                self.emit(Op::MakePair, span);
                self.emit(Op::ListPush, span);
                self.emit(Op::Store(buffer), span);
            }
            _ => self.terminal_step(query, accumulator, item, innermost, &mut dones),
        }
        self.emit(Op::Jump(innermost), span);
        self.close_source_loops(&tops, &nexts, span);
        // pass 2: the terminal over the sorted items
        if let (Some(buffer), Some(order)) = (buffer, &query.order) {
            self.emit(Op::Load(buffer), span);
            self.emit(
                Op::SortByKey {
                    descending: order.descending,
                },
                span,
            );
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
            self.emit(Op::Dup, span);
            self.emit(Op::Store(item), span);
            self.rebind_sources(&query.sources, span);
            self.terminal_step(query, accumulator, item, top, &mut dones);
            self.emit(Op::Jump(top), span);
            self.patch(next);
        }
        for done in dones {
            self.patch(done);
        }
        self.emit(Op::Load(accumulator), span);
        self.pop_scope();
    }

    /// One loop per source, its names bound; returns the loop tops (outer
    /// first), the `IterNext` ops to patch with the exits, and the slot that
    /// holds the current item (the raw item of a single source, the list of
    /// every binding for several).
    fn source_loops(&mut self, sources: &[QuerySource], span: Span) -> (Vec<u32>, Vec<usize>, u16) {
        let mut tops = Vec::new();
        let mut nexts = Vec::new();
        let item = self.temp();
        let several = sources.len() > 1;
        for source in sources {
            self.expr(&source.source);
            let iterator = self.temp();
            self.emit(Op::IterInit(iterator), span);
            tops.push(self.here());
            nexts.push(self.emit(
                Op::IterNext {
                    slot: iterator,
                    exit: 0,
                },
                span,
            ));
            if !several {
                self.emit(Op::Dup, span);
                self.emit(Op::Store(item), span);
            }
            self.bind_item(&source.bindings, span);
        }
        if several {
            let mut count = 0;
            for source in sources {
                for name in &source.bindings {
                    if let Some(slot) = self.lookup(&name.text) {
                        self.emit(Op::Load(slot), span);
                        count += 1;
                    }
                }
            }
            self.emit(Op::MakeList(count), span);
            self.emit(Op::Store(item), span);
        }
        (tops, nexts, item)
    }

    /// After the innermost body: each loop's exit continues the loop around
    /// it.
    fn close_source_loops(&mut self, tops: &[u32], nexts: &[usize], span: Span) {
        for level in (0..tops.len()).rev() {
            self.patch(nexts[level]);
            if level > 0 {
                self.emit(Op::Jump(tops[level - 1]), span);
            }
        }
    }

    /// Bind the sources' names again from the item on top of the stack (the
    /// second pass of a sorted query).
    fn rebind_sources(&mut self, sources: &[QuerySource], span: Span) {
        if let [source] = sources {
            self.bind_item(&source.bindings, span);
            return;
        }
        let names: Vec<_> = sources
            .iter()
            .flat_map(|s| s.bindings.iter().cloned())
            .collect();
        self.bind_item(&names, span);
    }

    /// What one item contributes to the accumulator; `continue_target` is
    /// where a rejected item goes, `dones` collects the jumps of an early
    /// answer (`first`, `any`, `all`).
    fn terminal_step(
        &mut self,
        query: &Query,
        accumulator: u16,
        item: u16,
        continue_target: u32,
        dones: &mut Vec<usize>,
    ) {
        let span = query.span;
        if let Some(key) = &query.group_by {
            // every terminal applies per group (decision M1): the map's entry
            // for the key takes the item, or folds with it
            self.emit(Op::LoadMove(accumulator), span);
            self.expr(key);
            let op = match &query.terminal {
                QueryTerminal::Collect(value) => {
                    self.expr(value);
                    Op::GroupInsert
                }
                QueryTerminal::None => {
                    self.emit(Op::Load(item), span);
                    Op::GroupInsert
                }
                QueryTerminal::Sum(value) => {
                    self.expr(value);
                    Op::GroupFold(GroupFold::Sum)
                }
                QueryTerminal::Count => {
                    self.constant(Value::integer(1), span);
                    Op::GroupFold(GroupFold::Sum)
                }
                QueryTerminal::First => {
                    self.emit(Op::Load(item), span);
                    Op::GroupFold(GroupFold::First)
                }
                QueryTerminal::Any(condition) => {
                    self.expr(condition);
                    Op::GroupFold(GroupFold::Any)
                }
                QueryTerminal::All(condition) => {
                    self.expr(condition);
                    Op::GroupFold(GroupFold::All)
                }
            };
            self.emit(op, span);
            self.emit(Op::Store(accumulator), span);
            return;
        }
        match &query.terminal {
            QueryTerminal::Collect(value) => {
                self.emit(Op::LoadMove(accumulator), span);
                self.expr(value);
                self.emit(Op::ListPush, span);
                self.emit(Op::Store(accumulator), span);
            }
            QueryTerminal::None => {}
            QueryTerminal::Sum(value) => {
                self.emit(Op::Load(accumulator), span);
                self.expr(value);
                self.emit(Op::Binary(renyi_syntax::ast::BinaryOp::Add), span);
                self.emit(Op::Store(accumulator), span);
            }
            QueryTerminal::Count => {
                self.emit(Op::Load(accumulator), span);
                self.constant(Value::integer(1), span);
                self.emit(Op::Binary(renyi_syntax::ast::BinaryOp::Add), span);
                self.emit(Op::Store(accumulator), span);
            }
            QueryTerminal::First => {
                self.emit(Op::Load(item), span);
                self.emit(Op::Store(accumulator), span);
                dones.push(self.emit(Op::Jump(0), span));
            }
            QueryTerminal::Any(condition) => {
                self.expr(condition);
                let skip = self.emit(Op::JumpIfFalse(0), condition.span);
                self.code.patch_to(skip, continue_target);
                self.constant(Value::Boolean(true), span);
                self.emit(Op::Store(accumulator), span);
                dones.push(self.emit(Op::Jump(0), span));
            }
            QueryTerminal::All(condition) => {
                self.expr(condition);
                let skip = self.emit(Op::JumpIfTrue(0), condition.span);
                self.code.patch_to(skip, continue_target);
                self.constant(Value::Boolean(false), span);
                self.emit(Op::Store(accumulator), span);
                dones.push(self.emit(Op::Jump(0), span));
            }
        }
    }
}
