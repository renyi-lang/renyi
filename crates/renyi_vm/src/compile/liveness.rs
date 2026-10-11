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

use std::collections::HashSet;

use renyi_syntax::ast::{
    Block, Expr, ExprKind, Outcome, Pattern, Query, QueryTerminal, Stmt, StmtKind, TextPiece,
};

/// The starts of the name tokens whose read is the last of their local.
pub fn last_uses(body: &Block) -> HashSet<usize> {
    let mut out = HashSet::new();
    walk_block(body, &mut Vec::new(), &mut out);
    out
}

/// A block around the one being walked: the names mentioned after the
/// statement that holds it, and, for a loop, the names its repeating part
/// binds anew on every turn.
struct Frame<'a> {
    later: HashSet<&'a str>,
    bound_by_loop: Option<HashSet<&'a str>>,
}

/// A read of a name at a statement's own level: the token's start and
/// whether it is a candidate (a read outside any query and any part that
/// runs more than once).
struct Mention<'a> {
    name: &'a str,
    start: usize,
    candidate: bool,
}

/// The last reads in a block, walked from its last statement so that the
/// names mentioned after each statement are known; what comes back is
/// every name the block mentions (read or bound) at any depth.
fn walk_block<'a>(
    block: &'a Block,
    frames: &mut Vec<Frame<'a>>,
    out: &mut HashSet<usize>,
) -> HashSet<&'a str> {
    let mut after: HashSet<&'a str> = HashSet::new();
    for statement in block.statements.iter().rev() {
        let mut own = Vec::new();
        let mut bound = HashSet::new();
        own_level(statement, &mut own, &mut bound);
        let nested = walk_nested(statement, &after, frames, out);
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
                out.insert(mention.start);
            }
        }
        after.extend(own.iter().map(|mention| mention.name));
        after.extend(bound);
        after.extend(nested);
    }
    after
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

/// The blocks nested in a statement, each walked with the statement's
/// frame around it (what is mentioned after the statement, and for a loop
/// what its repeating part binds), or with no frame at all when the block
/// never falls through; the names they mention come back.
fn walk_nested<'a>(
    statement: &'a Stmt,
    after: &HashSet<&'a str>,
    frames: &mut Vec<Frame<'a>>,
    out: &mut HashSet<usize>,
) -> HashSet<&'a str> {
    let mut names = HashSet::new();
    let mut walk = |frames: &mut Vec<Frame<'a>>, body: &'a Block| {
        if leaves(body) {
            names.extend(walk_block(body, &mut Vec::new(), out));
        } else {
            names.extend(walk_block(body, frames, out));
        }
    };
    let mut plain = |frames: &mut Vec<Frame<'a>>, bodies: Vec<&'a Block>| {
        frames.push(Frame {
            later: after.clone(),
            bound_by_loop: None,
        });
        for body in bodies {
            walk(frames, body);
        }
        frames.pop();
    };
    match &statement.kind {
        StmtKind::If {
            branches,
            otherwise,
        } => {
            let mut bodies: Vec<&Block> = branches.iter().map(|(_, body)| body).collect();
            bodies.extend(otherwise.iter());
            plain(frames, bodies);
        }
        StmtKind::Match {
            arms, otherwise, ..
        } => {
            let mut bodies: Vec<&Block> = arms.iter().map(|arm| &arm.body).collect();
            bodies.extend(otherwise.iter());
            plain(frames, bodies);
        }
        StmtKind::RunConcurrently { body, .. } => plain(frames, vec![body]),
        StmtKind::ForEach { bindings, body, .. } => {
            let mut bound: HashSet<&str> = bindings.iter().map(|b| b.text.as_str()).collect();
            block_bound(body, &mut bound);
            frames.push(Frame {
                later: after.clone(),
                bound_by_loop: Some(bound),
            });
            walk(frames, body);
            frames.pop();
        }
        StmtKind::RepeatUntil { body, .. } => {
            let mut bound = HashSet::new();
            block_bound(body, &mut bound);
            frames.push(Frame {
                later: after.clone(),
                bound_by_loop: Some(bound),
            });
            walk(frames, body);
            frames.pop();
        }
        _ => {}
    }
    names
}

/// A statement's own level, its nested blocks left out: the reads
/// (candidates outside a query and outside a part that runs more than
/// once), and the names bound there (its own binding, a loop's, a
/// pattern's, a query's).
fn own_level<'a>(
    statement: &'a Stmt,
    mentions: &mut Vec<Mention<'a>>,
    bound: &mut HashSet<&'a str>,
) {
    let expr =
        |e: &'a Expr, own: bool, mentions: &mut Vec<Mention<'a>>, bound: &mut HashSet<&'a str>| {
            expr_mentions(e, own, mentions);
            expr_bound(e, bound);
        };
    match &statement.kind {
        StmtKind::Let { name, value, .. } => {
            bound.insert(&name.text);
            expr(value, true, mentions, bound);
        }
        // a write is not a mention: it reads nothing from the slot, and a
        // read after it sees what it stored
        StmtKind::Change { value, .. } => expr(value, true, mentions, bound),
        StmtKind::If { branches, .. } => {
            for (condition, _) in branches {
                expr(condition, true, mentions, bound);
            }
        }
        StmtKind::Match { subject, arms, .. } => {
            expr(subject, true, mentions, bound);
            for arm in arms {
                pattern_bound(&arm.pattern, bound);
                if let Some(guard) = &arm.guard {
                    expr(guard, true, mentions, bound);
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
            bound.extend(bindings.iter().map(|b| b.text.as_str()));
            expr(source, true, mentions, bound);
            if let Some(filter) = filter {
                expr(filter, false, mentions, bound);
            }
            if let Some(order) = order {
                expr(&order.key, false, mentions, bound);
            }
        }
        StmtKind::RepeatUntil { condition, .. } => expr(condition, false, mentions, bound),
        StmtKind::RunConcurrently { within, .. } => {
            if let Some(within) = within {
                expr(within, true, mentions, bound);
            }
        }
        StmtKind::Return(value) | StmtKind::Fail(value) => {
            if let Some(value) = value {
                expr(value, true, mentions, bound);
            }
        }
        StmtKind::Crash(value)
        | StmtKind::Ignore(value)
        | StmtKind::Check(value)
        | StmtKind::Expression(value) => expr(value, true, mentions, bound),
        StmtKind::Break | StmtKind::Continue => {}
    }
}

// ------------------------------------------------------------ mentions

fn expr_mentions<'a>(expr: &'a Expr, own: bool, out: &mut Vec<Mention<'a>>) {
    let outcome = |outcome: &'a Outcome, out: &mut Vec<Mention<'a>>| match outcome {
        Outcome::Value(e) | Outcome::Crash(e, _) => expr_mentions(e, own, out),
        Outcome::Fail(e, _) | Outcome::Return(e, _) => {
            if let Some(e) = e {
                expr_mentions(e, own, out);
            }
        }
        Outcome::Break(_) | Outcome::Continue(_) => {}
    };
    match &expr.kind {
        ExprKind::Name(name) => out.push(Mention {
            name: &name.text,
            start: name.span.start,
            candidate: own,
        }),
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
                    expr_mentions(e, own, out);
                }
            }
        }
        ExprKind::Member { base, .. } => expr_mentions(base, own, out),
        ExprKind::Call { callee, args } => {
            expr_mentions(callee, own, out);
            for arg in args {
                expr_mentions(&arg.value, own, out);
            }
        }
        ExprKind::Construct { args, .. } => {
            for arg in args {
                expr_mentions(&arg.value, own, out);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                expr_mentions(item, own, out);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                expr_mentions(key, own, out);
                expr_mentions(value, own, out);
            }
        }
        ExprKind::Range { from, to, by } => {
            expr_mentions(from, own, out);
            expr_mentions(to, own, out);
            if let Some(by) = by {
                expr_mentions(by, own, out);
            }
        }
        ExprKind::Not(inner) | ExprKind::Paren(inner) => expr_mentions(inner, own, out),
        ExprKind::Binary { left, right, .. } => {
            expr_mentions(left, own, out);
            expr_mentions(right, own, out);
        }
        ExprKind::With { base, updates } => {
            expr_mentions(base, own, out);
            for update in updates {
                expr_mentions(&update.value, own, out);
            }
        }
        ExprKind::Otherwise { value, fallback } => {
            expr_mentions(value, own, out);
            outcome(fallback, out);
        }
        ExprKind::If {
            branches,
            otherwise,
        } => {
            for (condition, result) in branches {
                expr_mentions(condition, own, out);
                outcome(result, out);
            }
            outcome(otherwise, out);
        }
        ExprKind::Match {
            subject,
            arms,
            otherwise,
        } => {
            expr_mentions(subject, own, out);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    expr_mentions(guard, own, out);
                }
                outcome(&arm.body, out);
            }
            if let Some(fallback) = otherwise {
                outcome(fallback, out);
            }
        }
        // every part of a query runs once per item, or before the loop
        // with a binding of the query's own in scope: never a last read
        ExprKind::Query(query) => query_mentions(query, out),
    }
}

fn query_mentions<'a>(query: &'a Query, out: &mut Vec<Mention<'a>>) {
    for source in &query.sources {
        expr_mentions(&source.source, false, out);
    }
    if let Some(within) = &query.within {
        expr_mentions(within, false, out);
    }
    if let Some(filter) = &query.filter {
        expr_mentions(filter, false, out);
    }
    if let Some(order) = &query.order {
        expr_mentions(&order.key, false, out);
    }
    if let Some(key) = &query.group_by {
        expr_mentions(key, false, out);
    }
    match &query.terminal {
        QueryTerminal::Collect(e)
        | QueryTerminal::Sum(e)
        | QueryTerminal::Any(e)
        | QueryTerminal::All(e) => expr_mentions(e, false, out),
        QueryTerminal::Count | QueryTerminal::First | QueryTerminal::None => {}
    }
}

// ------------------------------------------------------------ names

/// Every name a block binds, at any depth.
fn block_bound<'a>(block: &'a Block, out: &mut HashSet<&'a str>) {
    for statement in &block.statements {
        statement_bound(statement, out);
    }
}

fn statement_bound<'a>(statement: &'a Stmt, out: &mut HashSet<&'a str>) {
    match &statement.kind {
        StmtKind::Let { name, value, .. } => {
            out.insert(&name.text);
            expr_bound(value, out);
        }
        StmtKind::Change { value, .. } => expr_bound(value, out),
        StmtKind::If {
            branches,
            otherwise,
        } => {
            for (condition, body) in branches {
                expr_bound(condition, out);
                block_bound(body, out);
            }
            if let Some(body) = otherwise {
                block_bound(body, out);
            }
        }
        StmtKind::Match {
            subject,
            arms,
            otherwise,
        } => {
            expr_bound(subject, out);
            for arm in arms {
                pattern_bound(&arm.pattern, out);
                if let Some(guard) = &arm.guard {
                    expr_bound(guard, out);
                }
                block_bound(&arm.body, out);
            }
            if let Some(body) = otherwise {
                block_bound(body, out);
            }
        }
        StmtKind::ForEach {
            bindings,
            source,
            filter,
            order,
            body,
        } => {
            out.extend(bindings.iter().map(|b| b.text.as_str()));
            expr_bound(source, out);
            if let Some(filter) = filter {
                expr_bound(filter, out);
            }
            if let Some(order) = order {
                expr_bound(&order.key, out);
            }
            block_bound(body, out);
        }
        StmtKind::RepeatUntil { condition, body } => {
            expr_bound(condition, out);
            block_bound(body, out);
        }
        StmtKind::RunConcurrently { within, body } => {
            if let Some(within) = within {
                expr_bound(within, out);
            }
            block_bound(body, out);
        }
        StmtKind::Return(value) | StmtKind::Fail(value) => {
            if let Some(value) = value {
                expr_bound(value, out);
            }
        }
        StmtKind::Crash(value)
        | StmtKind::Ignore(value)
        | StmtKind::Check(value)
        | StmtKind::Expression(value) => expr_bound(value, out),
        StmtKind::Break | StmtKind::Continue => {}
    }
}

/// Every name an expression binds: a match arm's pattern, a query's.
fn expr_bound<'a>(expr: &'a Expr, out: &mut HashSet<&'a str>) {
    let outcome = |outcome: &'a Outcome, out: &mut HashSet<&'a str>| match outcome {
        Outcome::Value(e) | Outcome::Crash(e, _) => expr_bound(e, out),
        Outcome::Fail(e, _) | Outcome::Return(e, _) => {
            if let Some(e) = e {
                expr_bound(e, out);
            }
        }
        Outcome::Break(_) | Outcome::Continue(_) => {}
    };
    match &expr.kind {
        ExprKind::Name(_)
        | ExprKind::Integer(_)
        | ExprKind::Decimal(_)
        | ExprKind::RawText(_)
        | ExprKind::Boolean(_)
        | ExprKind::Nothing
        | ExprKind::SelfValue
        | ExprKind::TypeName(_) => {}
        ExprKind::Text { pieces, .. } => {
            for piece in pieces {
                if let TextPiece::Hole(e) = piece {
                    expr_bound(e, out);
                }
            }
        }
        ExprKind::Member { base, .. } => expr_bound(base, out),
        ExprKind::Call { callee, args } => {
            expr_bound(callee, out);
            for arg in args {
                expr_bound(&arg.value, out);
            }
        }
        ExprKind::Construct { args, .. } => {
            for arg in args {
                expr_bound(&arg.value, out);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                expr_bound(item, out);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                expr_bound(key, out);
                expr_bound(value, out);
            }
        }
        ExprKind::Range { from, to, by } => {
            expr_bound(from, out);
            expr_bound(to, out);
            if let Some(by) = by {
                expr_bound(by, out);
            }
        }
        ExprKind::Not(inner) | ExprKind::Paren(inner) => expr_bound(inner, out),
        ExprKind::Binary { left, right, .. } => {
            expr_bound(left, out);
            expr_bound(right, out);
        }
        ExprKind::With { base, updates } => {
            expr_bound(base, out);
            for update in updates {
                expr_bound(&update.value, out);
            }
        }
        ExprKind::Otherwise { value, fallback } => {
            expr_bound(value, out);
            outcome(fallback, out);
        }
        ExprKind::If {
            branches,
            otherwise,
        } => {
            for (condition, result) in branches {
                expr_bound(condition, out);
                outcome(result, out);
            }
            outcome(otherwise, out);
        }
        ExprKind::Match {
            subject,
            arms,
            otherwise,
        } => {
            expr_bound(subject, out);
            for arm in arms {
                pattern_bound(&arm.pattern, out);
                if let Some(guard) = &arm.guard {
                    expr_bound(guard, out);
                }
                outcome(&arm.body, out);
            }
            if let Some(fallback) = otherwise {
                outcome(fallback, out);
            }
        }
        ExprKind::Query(query) => {
            for source in &query.sources {
                out.extend(source.bindings.iter().map(|b| b.text.as_str()));
                expr_bound(&source.source, out);
            }
            if let Some(within) = &query.within {
                expr_bound(within, out);
            }
            if let Some(filter) = &query.filter {
                expr_bound(filter, out);
            }
            if let Some(order) = &query.order {
                expr_bound(&order.key, out);
            }
            if let Some(key) = &query.group_by {
                expr_bound(key, out);
            }
            match &query.terminal {
                QueryTerminal::Collect(e)
                | QueryTerminal::Sum(e)
                | QueryTerminal::Any(e)
                | QueryTerminal::All(e) => expr_bound(e, out),
                QueryTerminal::Count | QueryTerminal::First | QueryTerminal::None => {}
            }
        }
    }
}

/// Every name a pattern binds.
fn pattern_bound<'a>(pattern: &'a Pattern, out: &mut HashSet<&'a str>) {
    match pattern {
        Pattern::Binding(name) | Pattern::Typed { name, .. } => {
            out.insert(&name.text);
        }
        Pattern::Nothing(_) | Pattern::Literal(_) => {}
        Pattern::Some(inner, _) | Pattern::Success(inner, _) | Pattern::Failure(inner, _) => {
            pattern_bound(inner, out)
        }
        Pattern::Variant { fields, .. } => {
            for field in fields {
                match &field.pattern {
                    Some(inner) => pattern_bound(inner, out),
                    None => {
                        out.insert(&field.field.text);
                    }
                }
            }
        }
    }
}
