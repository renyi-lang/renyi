//! The depth and branch metrics of design document 05, section 2.
//!
//! `depth` is the deepest block nesting inside a body: the body itself is 0
//! and every `if`, `match`, loop and `run concurrently` body adds 1.
//! `branches` counts decision points: one per `if` and `otherwise if`
//! condition (statement and expression forms), per `match` arm including
//! its `otherwise`, per loop header and query, per `otherwise` fallback on a
//! `maybe` or fallible value, and per `and` or `or`.

use renyi_syntax::ast::*;

/// `(depth, branches)` of a function or test body.
pub fn block_metrics(block: &Block) -> (usize, usize) {
    let mut walk = Walk::default();
    walk.block(block, false);
    (walk.max_depth, walk.branches)
}

/// `(depth, branches)` of a constant's value: an expression nests no
/// blocks, so the depth is 0.
pub fn expr_metrics(expr: &Expr) -> (usize, usize) {
    let mut walk = Walk::default();
    walk.expr(expr);
    (walk.max_depth, walk.branches)
}

#[derive(Default)]
struct Walk {
    depth: usize,
    max_depth: usize,
    branches: usize,
}

impl Walk {
    fn block(&mut self, block: &Block, nested: bool) {
        if nested {
            self.depth += 1;
            self.max_depth = self.max_depth.max(self.depth);
        }
        for statement in &block.statements {
            self.statement(statement);
        }
        if nested {
            self.depth -= 1;
        }
    }

    fn statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Let { value, .. } | StmtKind::Set { value, .. } => self.expr(value),
            StmtKind::If {
                branches,
                otherwise,
            } => {
                for (condition, block) in branches {
                    self.branches += 1;
                    self.expr(condition);
                    self.block(block, true);
                }
                if let Some(block) = otherwise {
                    self.block(block, true);
                }
            }
            StmtKind::Match {
                subject,
                arms,
                otherwise,
            } => {
                self.expr(subject);
                for arm in arms {
                    self.branches += 1;
                    if let Some(guard) = &arm.guard {
                        self.expr(guard);
                    }
                    self.block(&arm.body, true);
                }
                if let Some(block) = otherwise {
                    self.branches += 1;
                    self.block(block, true);
                }
            }
            StmtKind::ForEach {
                source,
                filter,
                order,
                body,
                ..
            } => {
                self.branches += 1;
                self.expr(source);
                if let Some(filter) = filter {
                    self.expr(filter);
                }
                if let Some(order) = order {
                    self.expr(&order.key);
                }
                self.block(body, true);
            }
            StmtKind::RepeatUntil { condition, body } => {
                self.branches += 1;
                self.expr(condition);
                self.block(body, true);
            }
            StmtKind::RunConcurrently { within, body } => {
                if let Some(within) = within {
                    self.expr(within);
                }
                self.block(body, true);
            }
            StmtKind::Return(value) | StmtKind::Fail(value) => {
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            StmtKind::Crash(value)
            | StmtKind::Ignore(value)
            | StmtKind::Check(value)
            | StmtKind::Expression(value) => self.expr(value),
            StmtKind::Break | StmtKind::Continue => {}
        }
    }

    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Integer(_)
            | ExprKind::Decimal(_)
            | ExprKind::RawText(_)
            | ExprKind::Boolean(_)
            | ExprKind::Nothing
            | ExprKind::SelfValue
            | ExprKind::Name(_)
            | ExprKind::TypeName(_) => {}
            ExprKind::Text { pieces, .. } => {
                for piece in pieces {
                    if let TextPiece::Hole(hole) = piece {
                        self.expr(hole);
                    }
                }
            }
            ExprKind::Member { base, .. } => self.expr(base),
            ExprKind::Call { callee, args } => {
                self.expr(callee);
                self.args(args);
            }
            ExprKind::Construct { args, .. } => self.args(args),
            ExprKind::List(items) => {
                for item in items {
                    self.expr(item);
                }
            }
            ExprKind::Map(entries) => {
                for (key, value) in entries {
                    self.expr(key);
                    self.expr(value);
                }
            }
            ExprKind::Range { from, to, by } => {
                self.expr(from);
                self.expr(to);
                if let Some(by) = by {
                    self.expr(by);
                }
            }
            ExprKind::Not(inner) | ExprKind::Paren(inner) => self.expr(inner),
            ExprKind::Binary { op, left, right } => {
                if matches!(op, BinaryOp::And | BinaryOp::Or) {
                    self.branches += 1;
                }
                self.expr(left);
                self.expr(right);
            }
            ExprKind::With { base, updates } => {
                self.expr(base);
                self.args(updates);
            }
            ExprKind::Otherwise { value, fallback } => {
                self.branches += 1;
                self.expr(value);
                self.outcome(fallback);
            }
            ExprKind::If {
                branches,
                otherwise,
            } => {
                for (condition, outcome) in branches {
                    self.branches += 1;
                    self.expr(condition);
                    self.outcome(outcome);
                }
                self.outcome(otherwise);
            }
            ExprKind::Match {
                subject,
                arms,
                otherwise,
            } => {
                self.expr(subject);
                for arm in arms {
                    self.branches += 1;
                    if let Some(guard) = &arm.guard {
                        self.expr(guard);
                    }
                    self.outcome(&arm.body);
                }
                if let Some(outcome) = otherwise {
                    self.branches += 1;
                    self.outcome(outcome);
                }
            }
            ExprKind::Query(query) => self.query(query),
        }
    }

    fn args(&mut self, args: &[Arg]) {
        for arg in args {
            self.expr(&arg.value);
        }
    }

    fn outcome(&mut self, outcome: &Outcome) {
        match outcome {
            Outcome::Value(value) | Outcome::Crash(value, _) => self.expr(value),
            Outcome::Fail(Some(value), _) | Outcome::Return(Some(value), _) => self.expr(value),
            Outcome::Fail(None, _)
            | Outcome::Return(None, _)
            | Outcome::Break(_)
            | Outcome::Continue(_) => {}
        }
    }

    fn query(&mut self, query: &Query) {
        self.branches += 1;
        for source in &query.sources {
            self.expr(&source.source);
        }
        if let Some(within) = &query.within {
            self.expr(within);
        }
        if let Some(filter) = &query.filter {
            self.expr(filter);
        }
        if let Some(order) = &query.order {
            self.expr(&order.key);
        }
        if let Some(key) = &query.group_by {
            self.expr(key);
        }
        match &query.terminal {
            QueryTerminal::Collect(value)
            | QueryTerminal::Sum(value)
            | QueryTerminal::Any(value)
            | QueryTerminal::All(value) => self.expr(value),
            QueryTerminal::Count | QueryTerminal::First | QueryTerminal::None => {}
        }
    }
}
