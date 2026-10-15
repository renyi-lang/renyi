//! The last reads of locals (decision AU55). A read of a local that is the
//! only mention of it in its statement, with the local mentioned nowhere
//! after that statement in its block or in the blocks around it, and
//! bound anew by every loop around it, is the last read: the value is
//! moved out of its slot (`LoadMove`), or the field is taken out of the
//! record in the slot (`TakeField`), so that a record or a collection held
//! once stays held once into the call or the update that follows, where
//! `with`, `append` and the natives change it in place. A read inside a
//! query runs once per item and is never the last. The emitter written in
//! Renyi computes the same set (`compiler/emit.ry`), which the judge of
//! decision Z3 holds equal.
//!
//! The names are interned to small indexes as the walk meets them, one
//! hash per occurrence, and every set of names is a row of bits
//! (decision AW67): a name's membership is one bit test, the names after
//! a statement joined by a word-wise or, and the mentions and the bound
//! names of an expression are gathered in one pass over it.

use std::collections::{HashMap, HashSet};

use renyi_syntax::ast::{
    Block, Expr, ExprKind, Outcome, Pattern, Query, QueryTerminal, Stmt, StmtKind, TextPiece,
};

/// The starts of the name tokens whose read is the last of their local.
pub fn last_uses(body: &Block) -> HashSet<usize> {
    let mut walk = Walk {
        names: HashMap::new(),
        out: HashSet::new(),
    };
    walk.block(body, &mut Vec::new());
    walk.out
}

/// A set of names as bits, indexed by the interned name.
#[derive(Clone, Default)]
struct Bits(Vec<u64>);

impl Bits {
    fn insert(&mut self, name: u32) {
        let word = (name / 64) as usize;
        if word >= self.0.len() {
            self.0.resize(word + 1, 0);
        }
        self.0[word] |= 1 << (name % 64);
    }

    fn contains(&self, name: u32) -> bool {
        self.0
            .get((name / 64) as usize)
            .is_some_and(|word| word & (1 << (name % 64)) != 0)
    }

    fn extend(&mut self, other: &Bits) {
        if other.0.len() > self.0.len() {
            self.0.resize(other.0.len(), 0);
        }
        for (word, more) in self.0.iter_mut().zip(&other.0) {
            *word |= more;
        }
    }
}

/// A block around the one being walked: the names mentioned after the
/// statement that holds it, and, for a loop, the names its repeating part
/// binds anew on every turn.
struct Frame {
    later: Bits,
    bound_by_loop: Option<Bits>,
}

/// A read of a name at a statement's own level: the token's start and
/// whether it is a candidate (a read outside any query and any part that
/// runs more than once).
struct Mention {
    name: u32,
    start: usize,
    candidate: bool,
}

/// The walk over one body: the names interned so far and the last reads
/// found.
struct Walk<'a> {
    names: HashMap<&'a str, u32>,
    out: HashSet<usize>,
}

impl<'a> Walk<'a> {
    fn intern(&mut self, name: &'a str) -> u32 {
        let next = self.names.len() as u32;
        *self.names.entry(name).or_insert(next)
    }

    /// The last reads in a block, walked from its last statement so that
    /// the names mentioned after each statement are known; what comes
    /// back is every name the block mentions (read or bound) at any
    /// depth.
    fn block(&mut self, block: &'a Block, frames: &mut Vec<Frame>) -> Bits {
        let mut after = Bits::default();
        for statement in block.statements.iter().rev() {
            let mut own = Vec::new();
            let mut bound = Bits::default();
            self.own_level(statement, &mut own, &mut bound);
            let nested = self.nested(statement, &after, frames);
            for mention in &own {
                if !mention.candidate {
                    continue;
                }
                if own.iter().filter(|m| m.name == mention.name).count() != 1 {
                    continue;
                }
                if nested.contains(mention.name) || after.contains(mention.name) {
                    continue;
                }
                let allowed = frames.iter().all(|frame| {
                    !frame.later.contains(mention.name)
                        && frame
                            .bound_by_loop
                            .as_ref()
                            .is_none_or(|bound| bound.contains(mention.name))
                });
                if allowed {
                    self.out.insert(mention.start);
                }
            }
            for mention in &own {
                after.insert(mention.name);
            }
            after.extend(&bound);
            after.extend(&nested);
        }
        after
    }

    /// A nested block walked with the frames around it, or with no frame
    /// at all when it never falls through.
    fn body(&mut self, frames: &mut Vec<Frame>, body: &'a Block) -> Bits {
        if leaves(body) {
            self.block(body, &mut Vec::new())
        } else {
            self.block(body, frames)
        }
    }

    /// The blocks nested in a statement, each walked with the statement's
    /// frame around it (what is mentioned after the statement, and for a
    /// loop what its repeating part binds), or with no frame at all when
    /// the block never falls through; the names they mention come back.
    fn nested(&mut self, statement: &'a Stmt, after: &Bits, frames: &mut Vec<Frame>) -> Bits {
        let mut names = Bits::default();
        match &statement.kind {
            StmtKind::If {
                branches,
                otherwise,
            } => {
                let mut bodies: Vec<&Block> = branches.iter().map(|(_, body)| body).collect();
                bodies.extend(otherwise.iter());
                self.plain(frames, after, &bodies, &mut names);
            }
            StmtKind::Match {
                arms, otherwise, ..
            } => {
                let mut bodies: Vec<&Block> = arms.iter().map(|arm| &arm.body).collect();
                bodies.extend(otherwise.iter());
                self.plain(frames, after, &bodies, &mut names);
            }
            StmtKind::RunConcurrently { body, .. } => {
                self.plain(frames, after, &[body], &mut names)
            }
            StmtKind::ForEach { bindings, body, .. } => {
                let mut bound = Bits::default();
                for binding in bindings {
                    let name = self.intern(&binding.text);
                    bound.insert(name);
                }
                self.block_bound(body, &mut bound);
                frames.push(Frame {
                    later: after.clone(),
                    bound_by_loop: Some(bound),
                });
                names.extend(&self.body(frames, body));
                frames.pop();
            }
            StmtKind::RepeatUntil { body, .. } => {
                let mut bound = Bits::default();
                self.block_bound(body, &mut bound);
                frames.push(Frame {
                    later: after.clone(),
                    bound_by_loop: Some(bound),
                });
                names.extend(&self.body(frames, body));
                frames.pop();
            }
            _ => {}
        }
        names
    }

    /// The bodies of a statement that is not a loop, each walked under a
    /// frame of what is mentioned after the statement.
    fn plain(
        &mut self,
        frames: &mut Vec<Frame>,
        after: &Bits,
        bodies: &[&'a Block],
        names: &mut Bits,
    ) {
        frames.push(Frame {
            later: after.clone(),
            bound_by_loop: None,
        });
        for body in bodies {
            names.extend(&self.body(frames, body));
        }
        frames.pop();
    }

    /// A statement's own level, its nested blocks left out: the reads
    /// (candidates outside a query and outside a part that runs more than
    /// once), and the names bound there (its own binding, a loop's, a
    /// pattern's, a query's), in one pass over each expression.
    fn own_level(&mut self, statement: &'a Stmt, mentions: &mut Vec<Mention>, bound: &mut Bits) {
        match &statement.kind {
            StmtKind::Let { name, value, .. } => {
                let name = self.intern(&name.text);
                bound.insert(name);
                self.expr(value, true, Some(mentions), bound);
            }
            // a write is not a mention: it reads nothing from the slot,
            // and a read after it sees what it stored
            StmtKind::Change { value, .. } => self.expr(value, true, Some(mentions), bound),
            StmtKind::If { branches, .. } => {
                for (condition, _) in branches {
                    self.expr(condition, true, Some(&mut *mentions), bound);
                }
            }
            StmtKind::Match { subject, arms, .. } => {
                self.expr(subject, true, Some(&mut *mentions), bound);
                for arm in arms {
                    self.pattern_bound(&arm.pattern, bound);
                    if let Some(guard) = &arm.guard {
                        self.expr(guard, true, Some(&mut *mentions), bound);
                    }
                }
            }
            StmtKind::ForEach {
                bindings,
                source,
                filter,
                order,
                ..
            } => {
                for binding in bindings {
                    let name = self.intern(&binding.text);
                    bound.insert(name);
                }
                self.expr(source, true, Some(&mut *mentions), bound);
                if let Some(filter) = filter {
                    self.expr(filter, false, Some(&mut *mentions), bound);
                }
                if let Some(order) = order {
                    self.expr(&order.key, false, Some(&mut *mentions), bound);
                }
            }
            StmtKind::RepeatUntil { condition, .. } => {
                self.expr(condition, false, Some(mentions), bound)
            }
            StmtKind::RunConcurrently { within, .. } => {
                if let Some(within) = within {
                    self.expr(within, true, Some(mentions), bound);
                }
            }
            StmtKind::Return(value) | StmtKind::Fail(value) => {
                if let Some(value) = value {
                    self.expr(value, true, Some(mentions), bound);
                }
            }
            StmtKind::Crash(value)
            | StmtKind::Ignore(value)
            | StmtKind::Check(value)
            | StmtKind::Expression(value) => self.expr(value, true, Some(mentions), bound),
            StmtKind::Break | StmtKind::Continue => {}
        }
    }

    /// The reads in an expression (into `mentions` when they are wanted,
    /// candidates when `own`) and the names it binds (a match arm's
    /// pattern, a query's bindings, at any depth), in one pass.
    fn expr(
        &mut self,
        expr: &'a Expr,
        own: bool,
        mut mentions: Option<&mut Vec<Mention>>,
        bound: &mut Bits,
    ) {
        match &expr.kind {
            ExprKind::Name(name) => {
                if let Some(mentions) = mentions {
                    let name_index = self.intern(&name.text);
                    mentions.push(Mention {
                        name: name_index,
                        start: name.span.start,
                        candidate: own,
                    });
                }
            }
            ExprKind::Integer(_)
            | ExprKind::Decimal(_)
            | ExprKind::RawText(_)
            | ExprKind::Boolean(_)
            | ExprKind::Nothing
            | ExprKind::SelfValue
            | ExprKind::TypeName(_) => {}
            ExprKind::Text { pieces, .. } => {
                for piece in pieces {
                    if let TextPiece::Hole(e) = piece {
                        self.expr(e, own, mentions.as_deref_mut(), bound);
                    }
                }
            }
            ExprKind::Member { base, .. } => self.expr(base, own, mentions, bound),
            ExprKind::Call { callee, args } => {
                self.expr(callee, own, mentions.as_deref_mut(), bound);
                for arg in args {
                    self.expr(&arg.value, own, mentions.as_deref_mut(), bound);
                }
            }
            ExprKind::Construct { args, .. } => {
                for arg in args {
                    self.expr(&arg.value, own, mentions.as_deref_mut(), bound);
                }
            }
            ExprKind::List(items) => {
                for item in items {
                    self.expr(item, own, mentions.as_deref_mut(), bound);
                }
            }
            ExprKind::Map(entries) => {
                for (key, value) in entries {
                    self.expr(key, own, mentions.as_deref_mut(), bound);
                    self.expr(value, own, mentions.as_deref_mut(), bound);
                }
            }
            ExprKind::Range { from, to, by } => {
                self.expr(from, own, mentions.as_deref_mut(), bound);
                self.expr(to, own, mentions.as_deref_mut(), bound);
                if let Some(by) = by {
                    self.expr(by, own, mentions, bound);
                }
            }
            ExprKind::Not(inner) | ExprKind::Paren(inner) => self.expr(inner, own, mentions, bound),
            ExprKind::Binary { left, right, .. } => {
                self.expr(left, own, mentions.as_deref_mut(), bound);
                self.expr(right, own, mentions, bound);
            }
            ExprKind::With { base, updates } => {
                self.expr(base, own, mentions.as_deref_mut(), bound);
                for update in updates {
                    self.expr(&update.value, own, mentions.as_deref_mut(), bound);
                }
            }
            ExprKind::Otherwise { value, fallback } => {
                self.expr(value, own, mentions.as_deref_mut(), bound);
                self.outcome(fallback, own, mentions, bound);
            }
            ExprKind::If {
                branches,
                otherwise,
            } => {
                for (condition, result) in branches {
                    self.expr(condition, own, mentions.as_deref_mut(), bound);
                    self.outcome(result, own, mentions.as_deref_mut(), bound);
                }
                self.outcome(otherwise, own, mentions, bound);
            }
            ExprKind::Match {
                subject,
                arms,
                otherwise,
            } => {
                self.expr(subject, own, mentions.as_deref_mut(), bound);
                for arm in arms {
                    self.pattern_bound(&arm.pattern, bound);
                    if let Some(guard) = &arm.guard {
                        self.expr(guard, own, mentions.as_deref_mut(), bound);
                    }
                    self.outcome(&arm.body, own, mentions.as_deref_mut(), bound);
                }
                if let Some(fallback) = otherwise {
                    self.outcome(fallback, own, mentions, bound);
                }
            }
            // every part of a query runs once per item, or before the
            // loop with a binding of the query's own in scope: never a
            // last read
            ExprKind::Query(query) => self.query(query, mentions, bound),
        }
    }

    fn outcome(
        &mut self,
        outcome: &'a Outcome,
        own: bool,
        mentions: Option<&mut Vec<Mention>>,
        bound: &mut Bits,
    ) {
        match outcome {
            Outcome::Value(e) | Outcome::Crash(e, _) => self.expr(e, own, mentions, bound),
            Outcome::Fail(e, _) | Outcome::Return(e, _) => {
                if let Some(e) = e {
                    self.expr(e, own, mentions, bound);
                }
            }
            Outcome::Break(_) | Outcome::Continue(_) => {}
        }
    }

    fn query(
        &mut self,
        query: &'a Query,
        mut mentions: Option<&mut Vec<Mention>>,
        bound: &mut Bits,
    ) {
        for source in &query.sources {
            for binding in &source.bindings {
                let name = self.intern(&binding.text);
                bound.insert(name);
            }
            self.expr(&source.source, false, mentions.as_deref_mut(), bound);
        }
        if let Some(within) = &query.within {
            self.expr(within, false, mentions.as_deref_mut(), bound);
        }
        if let Some(filter) = &query.filter {
            self.expr(filter, false, mentions.as_deref_mut(), bound);
        }
        if let Some(order) = &query.order {
            self.expr(&order.key, false, mentions.as_deref_mut(), bound);
        }
        if let Some(key) = &query.group_by {
            self.expr(key, false, mentions.as_deref_mut(), bound);
        }
        match &query.terminal {
            QueryTerminal::Collect(e)
            | QueryTerminal::Sum(e)
            | QueryTerminal::Any(e)
            | QueryTerminal::All(e) => self.expr(e, false, mentions, bound),
            QueryTerminal::Count | QueryTerminal::First | QueryTerminal::None => {}
        }
    }

    // -------------------------------------------------------------- names

    /// Every name a block binds, at any depth.
    fn block_bound(&mut self, block: &'a Block, out: &mut Bits) {
        for statement in &block.statements {
            self.statement_bound(statement, out);
        }
    }

    fn statement_bound(&mut self, statement: &'a Stmt, out: &mut Bits) {
        match &statement.kind {
            StmtKind::Let { name, value, .. } => {
                let name = self.intern(&name.text);
                out.insert(name);
                self.expr(value, false, None, out);
            }
            StmtKind::Change { value, .. } => self.expr(value, false, None, out),
            StmtKind::If {
                branches,
                otherwise,
            } => {
                for (condition, body) in branches {
                    self.expr(condition, false, None, out);
                    self.block_bound(body, out);
                }
                if let Some(body) = otherwise {
                    self.block_bound(body, out);
                }
            }
            StmtKind::Match {
                subject,
                arms,
                otherwise,
            } => {
                self.expr(subject, false, None, out);
                for arm in arms {
                    self.pattern_bound(&arm.pattern, out);
                    if let Some(guard) = &arm.guard {
                        self.expr(guard, false, None, out);
                    }
                    self.block_bound(&arm.body, out);
                }
                if let Some(body) = otherwise {
                    self.block_bound(body, out);
                }
            }
            StmtKind::ForEach {
                bindings,
                source,
                filter,
                order,
                body,
            } => {
                for binding in bindings {
                    let name = self.intern(&binding.text);
                    out.insert(name);
                }
                self.expr(source, false, None, out);
                if let Some(filter) = filter {
                    self.expr(filter, false, None, out);
                }
                if let Some(order) = order {
                    self.expr(&order.key, false, None, out);
                }
                self.block_bound(body, out);
            }
            StmtKind::RepeatUntil { condition, body } => {
                self.expr(condition, false, None, out);
                self.block_bound(body, out);
            }
            StmtKind::RunConcurrently { within, body } => {
                if let Some(within) = within {
                    self.expr(within, false, None, out);
                }
                self.block_bound(body, out);
            }
            StmtKind::Return(value) | StmtKind::Fail(value) => {
                if let Some(value) = value {
                    self.expr(value, false, None, out);
                }
            }
            StmtKind::Crash(value)
            | StmtKind::Ignore(value)
            | StmtKind::Check(value)
            | StmtKind::Expression(value) => self.expr(value, false, None, out),
            StmtKind::Break | StmtKind::Continue => {}
        }
    }

    /// Every name a pattern binds.
    fn pattern_bound(&mut self, pattern: &'a Pattern, out: &mut Bits) {
        match pattern {
            Pattern::Binding(name) | Pattern::Typed { name, .. } => {
                let name = self.intern(&name.text);
                out.insert(name);
            }
            Pattern::Nothing(_) | Pattern::Literal(_) => {}
            Pattern::Some(inner, _) | Pattern::Success(inner, _) | Pattern::Failure(inner, _) => {
                self.pattern_bound(inner, out)
            }
            Pattern::Variant { fields, .. } => {
                for field in fields {
                    match &field.pattern {
                        Some(inner) => self.pattern_bound(inner, out),
                        None => {
                            let name = self.intern(&field.field.text);
                            out.insert(name);
                        }
                    }
                }
            }
        }
    }
}

/// Whether a block never falls through: its last statement leaves the
/// function (`return`, `fail`) or the program (`crash`), so that nothing
/// after the statement that holds the block, and no next turn of a loop
/// around it, can read what the block moved out.
fn leaves(block: &Block) -> bool {
    matches!(
        block.statements.last().map(|s| &s.kind),
        Some(StmtKind::Return(_) | StmtKind::Fail(_) | StmtKind::Crash(_))
    )
}
