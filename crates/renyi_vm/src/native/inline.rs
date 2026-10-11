//! Small callees expanded into the code object that calls them, before the
//! Cranelift tier compiles it (decision AU50). A direct call costs about a
//! hundred instructions of frame protocol on that tier, and the compiler
//! written in Renyi makes most of its calls to functions of a dozen ops.
//! The measure found the expanded code slower than the calls it replaces
//! (the entry has the numbers), so the expansion is off unless
//! `RENYI_NATIVE_INLINE=1` names it on.
//!
//! The expansion is a rewrite of the bytecode the tier sees, never of the
//! program: a `Call` to a small callee becomes a `Jump` into a region
//! appended after the caller's own ops, where the callee's ops run with
//! their slots moved up past the caller's locals (the region's slots),
//! their jump targets moved to the region, their constants appended to the
//! caller's pool, and every `Return` a jump to the region's exit, which
//! releases the region's slots and jumps back to the op after the call.
//! The caller's own ops keep their pcs, so the interpreter, the template
//! tier, the loop headers and the bytecode file see nothing of it; a pc at
//! or past the caller's length lies in a region, and the regions say which
//! callee's op it is, for a hand-back to the interpreter (`rt_deopt`
//! rebuilds the frames) and for the location of a crash.
//!
//! What is expanded: a call to a declared function with code, not the
//! caller itself nor one of the expansions it lies in, without
//! capabilities (its frame would narrow the grant), of at most
//! `CALLEE_LIMIT` ops, with no loop, no query, no mark, no deadline, no
//! `check` and no `fail` (a failure op leaves the frame, which the
//! caller's would be), whose analysis settles and returns with exactly its
//! result on the stack and no handled region of its own open. A callee's
//! own small calls are expanded once more (`NESTING_LIMIT`). A caller grows
//! by at most half its own size, the smallest callees first
//! (`RENYI_NATIVE_INLINE_BUDGET` sets the number of ops instead and
//! `RENYI_NATIVE_INLINE_LIMIT` the largest callee, development aids for
//! the measure of another rule, as `RENYI_NATIVE_INLINE_DUMP` prints
//! every expansion and why a call was not expanded).

use std::ops::Range;

use renyi_syntax::Span;

use super::infer::analyse;
use crate::bytecode::{Code, CodeKind, Op};
use crate::compile::{CodeId, Program};

/// The largest callee expanded, in ops.
pub const CALLEE_LIMIT: usize = 20;

/// How deep expansions nest: a callee's own small callee once more.
pub const NESTING_LIMIT: usize = 2;

/// A callee expanded into a caller: where its ops lie in the caller's
/// expanded ops and which slots are its.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Region {
    /// The code object expanded here.
    pub callee: u32,
    /// The pc of the `Call` the region replaces, now a `Jump` into it: a
    /// pc of the caller's own ops, or of the parent region's body.
    pub call_pc: u32,
    /// The region's first op: the stores of the arguments into its slots.
    pub start: u32,
    /// The callee's op 0; its op `q` lies at `body + q`.
    pub body: u32,
    /// One past the region's last op, the tails included.
    pub end: u32,
    /// The slot the callee's slot 0 maps to.
    pub slot_base: u16,
    /// The region whose body holds the call, when the call was inside an
    /// expanded callee.
    pub parent: Option<u32>,
}

impl Region {
    pub fn holds(&self, pc: usize) -> bool {
        (self.start as usize) <= pc && pc < self.end as usize
    }

    /// The callee's pc of an op of the body, `None` in the prologue and
    /// the tails.
    pub fn callee_pc(&self, pc: usize, callee: &Code) -> Option<usize> {
        let body = self.body as usize;
        (body <= pc && pc < body + callee.ops.len()).then(|| pc - body)
    }
}

/// A caller with its small callees expanded.
pub struct Expansion {
    pub code: Code,
    pub regions: Vec<Region>,
}

/// The caller's code with every small callee it calls directly expanded
/// into it, or `None` when nothing was expanded.
pub fn expand(program: &Program, code_id: CodeId) -> Option<Expansion> {
    let original = &program.codes[code_id];
    if original.ops.len() >= u32::MAX as usize / 4 {
        return None;
    }
    let mut expander = Expander {
        program,
        ops: original.ops.clone(),
        spans: original.spans.clone(),
        types: original.types.clone(),
        constants: original.constants.clone(),
        regions: Vec::new(),
        next_slot: original.locals as usize,
        appended: 0,
        budget: budget_for(original.ops.len()),
        limit: callee_limit(),
    };
    expander.scan(0..original.ops.len(), None, &mut vec![code_id], 1);
    if expander.regions.is_empty() {
        return None;
    }
    if std::env::var_os("RENYI_NATIVE_INLINE_DUMP").is_some() {
        eprintln!(
            "inline: {} ({} ops, {} locals)",
            original.name,
            original.ops.len(),
            original.locals
        );
        for region in &expander.regions {
            eprintln!(
                "  {} at {} as ops {}..{} (body {}), slots from {}{}",
                program.codes[region.callee as usize].name,
                region.call_pc,
                region.start,
                region.end,
                region.body,
                region.slot_base,
                match region.parent {
                    Some(parent) => format!(", inside region {parent}"),
                    None => String::new(),
                }
            );
        }
        for (pc, op) in expander.ops.iter().enumerate() {
            eprintln!("  {pc:4} {op:?}");
        }
    }
    let mut code = original.clone();
    code.ops = expander.ops;
    code.spans = expander.spans;
    code.types = expander.types;
    code.constants = expander.constants;
    code.locals = u16::try_from(expander.next_slot).ok()?;
    Some(Expansion {
        code,
        regions: expander.regions,
    })
}

/// How many ops a caller may gain: half its own, unless
/// `RENYI_NATIVE_INLINE_BUDGET` names a number of ops (a development aid,
/// for a small program whose callers would otherwise expand nothing, and
/// for the measure of another rule).
fn budget_for(original_len: usize) -> usize {
    std::env::var("RENYI_NATIVE_INLINE_BUDGET")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(original_len / 2)
}

/// The largest callee expanded: `CALLEE_LIMIT`, unless
/// `RENYI_NATIVE_INLINE_LIMIT` names a number of ops (a development aid,
/// for the measure of another rule).
fn callee_limit() -> usize {
    std::env::var("RENYI_NATIVE_INLINE_LIMIT")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(CALLEE_LIMIT)
}

struct Expander<'p> {
    program: &'p Program,
    ops: Vec<Op>,
    spans: Vec<Span>,
    types: Vec<Option<u32>>,
    constants: Vec<crate::value::Value>,
    regions: Vec<Region>,
    next_slot: usize,
    appended: usize,
    budget: usize,
    limit: usize,
}

/// What an expanded callee looks like: its analysis settled, and whether
/// it returns nothing anywhere.
struct Shape {
    returns_nothing: bool,
}

impl Expander<'_> {
    fn push(&mut self, op: Op, span: Span, ty: Option<u32>) {
        self.ops.push(op);
        self.spans.push(span);
        self.types.push(ty);
    }

    /// Every call in the range whose callee fits expanded while the
    /// budget lasts, the smallest callee first (the call's protocol is the
    /// largest share of what a small callee does, so the budget buys most
    /// there), the regions appended after everything emitted so far; then
    /// the calls inside the new regions, one level deeper.
    fn scan(
        &mut self,
        range: Range<usize>,
        parent: Option<u32>,
        chain: &mut Vec<CodeId>,
        depth: usize,
    ) {
        let dump = std::env::var_os("RENYI_NATIVE_INLINE_DUMP").is_some();
        // the calls that fit: the ops the region takes, the pc, the callee
        // and its shape
        let mut candidates: Vec<(usize, usize, CodeId, Shape)> = Vec::new();
        for pc in range {
            let Op::Call { function, args } = self.ops[pc] else {
                continue;
            };
            let fit = self
                .callee_of(function, args as usize, chain)
                .and_then(|callee| self.shape_of(callee).map(|shape| (callee, shape)));
            match fit {
                Ok((callee, shape)) => {
                    let callee_code = &self.program.codes[callee];
                    let size = args as usize
                        + callee_code.ops.len()
                        + 2 * callee_code.locals as usize
                        + 1
                        + if shape.returns_nothing { 2 } else { 0 };
                    candidates.push((size, pc, callee, shape));
                }
                Err(why) => {
                    if dump {
                        let name = self.program.function_metas[function].name.as_str();
                        eprintln!("  {name} at {pc} not expanded: {why}");
                    }
                }
            }
        }
        candidates.sort_by_key(|(size, pc, ..)| (*size, *pc));
        let mut emitted: Vec<(usize, CodeId)> = Vec::new();
        for (size, pc, callee, shape) in candidates {
            let callee_code = &self.program.codes[callee];
            let locals = callee_code.locals as usize;
            if self.appended + size > self.budget || self.next_slot + locals > u16::MAX as usize {
                if dump {
                    let name = callee_code.name.as_str();
                    eprintln!("  {name} at {pc} not expanded: the budget ({size} ops)");
                }
                continue;
            }
            let Op::Call { args, .. } = self.ops[pc] else {
                unreachable!("a candidate is a call");
            };
            let region = self.emit(pc, callee, args as usize, &shape, parent);
            self.appended += size;
            emitted.push((region, callee));
        }
        if depth < NESTING_LIMIT {
            for (region, callee) in emitted {
                let body = self.regions[region].body as usize;
                let len = self.program.codes[callee].ops.len();
                chain.push(callee);
                self.scan(body..body + len, Some(region as u32), chain, depth + 1);
                chain.pop();
            }
        }
    }

    /// The code object of a function the rules admit, or why not (what
    /// the dump prints).
    fn callee_of(
        &self,
        function: usize,
        args: usize,
        chain: &[CodeId],
    ) -> Result<CodeId, &'static str> {
        let program = self.program;
        let callee = program
            .function_codes
            .get(function)
            .copied()
            .flatten()
            .ok_or("not a declared function")?;
        if chain.contains(&callee) {
            return Err("recursive");
        }
        let code = &program.codes[callee];
        if code.kind != CodeKind::Function {
            return Err("not a function's code");
        }
        if code.params as usize != args {
            return Err("the arguments do not match the parameters");
        }
        if code.ops.len() > self.limit {
            return Err("too big");
        }
        if !program.function_metas[function].needs.is_empty() {
            return Err("it needs capabilities");
        }
        for (pc, op) in code.ops.iter().enumerate() {
            match op {
                Op::Jump(target)
                | Op::JumpIfFalse(target)
                | Op::JumpIfTrue(target)
                | Op::JumpIfAbsent(target)
                | Op::JumpIfFailure(target)
                    if *target as usize <= pc =>
                {
                    return Err("a loop");
                }
                Op::IterInit(_)
                | Op::IterNext { .. }
                | Op::ListPush
                | Op::GroupInsert
                | Op::GroupFold(_)
                | Op::SortByKey { .. }
                | Op::Deadline(_)
                | Op::CheckDeadline(_)
                | Op::MarkStack(_)
                | Op::UnwindStack(_)
                | Op::Check(_) => return Err("a loop, a query, a deadline or a check"),
                Op::Fail => return Err("a `fail`"),
                _ => {}
            }
        }
        Ok(callee)
    }

    /// The callee's shape by its analysis, or why it does not fit: a
    /// `Return` must leave exactly the result, with no handled region of
    /// the callee's own open, since the region's exit is one jump.
    fn shape_of(&self, callee: CodeId) -> Result<Shape, &'static str> {
        let code = &self.program.codes[callee];
        let analysis = analyse(self.program, code).map_err(|_| "its analysis does not settle")?;
        let mut returns_nothing = false;
        for (pc, op) in code.ops.iter().enumerate() {
            // an op the analysis never reached (the `ReturnNothing` every
            // function ends with, after a `Return`) is left out, as the
            // generated code leaves it out
            let Some(state) = analysis.entry.get(pc).and_then(Option::as_ref) else {
                continue;
            };
            let depth = state.len();
            let open = analysis
                .handlers
                .get(pc)
                .is_some_and(|open| !open.is_empty());
            match op {
                Op::Return | Op::ReturnNothing if open => {
                    return Err("a return under a handler of its own")
                }
                Op::Return if depth != 1 => return Err("a return with operands below"),
                Op::ReturnNothing if depth != 0 => return Err("a return with operands below"),
                Op::ReturnNothing => returns_nothing = true,
                _ => {}
            }
        }
        Ok(Shape { returns_nothing })
    }

    /// The region of one call appended, the call replaced by the jump into
    /// it; the index of the region.
    fn emit(
        &mut self,
        pc: usize,
        callee: CodeId,
        args: usize,
        shape: &Shape,
        parent: Option<u32>,
    ) -> usize {
        let program = self.program;
        let code = &program.codes[callee];
        let locals = code.locals as usize;
        let slot_base = self.next_slot;
        let call_span = self.spans[pc];
        let start = self.ops.len();
        // the arguments into the region's slots, the last first
        for slot in (0..args).rev() {
            self.push(Op::Store((slot_base + slot) as u16), call_span, None);
        }
        let body = self.ops.len();
        let const_offset = self.constants.len() as u32;
        self.constants.extend(code.constants.iter().cloned());
        // where the tails will lie
        let exit = body + code.ops.len();
        let nothing = shape.returns_nothing.then_some(exit + 2 * locals + 1);
        let shift = |target: &u32| (*target as usize + body) as u32;
        let slot = |slot: &u16| (*slot as usize + slot_base) as u16;
        for (q, op) in code.ops.iter().enumerate() {
            let mapped = match op {
                Op::Const(index) => Op::Const(index + const_offset),
                Op::Load(s) => Op::Load(slot(s)),
                Op::LoadMove(s) => Op::LoadMove(slot(s)),
                Op::Store(s) => Op::Store(slot(s)),
                Op::Field { name, site } => Op::Field {
                    name: name + const_offset,
                    site: *site,
                },
                Op::LoadField {
                    slot: s,
                    name,
                    site,
                } => Op::LoadField {
                    slot: slot(s),
                    name: name + const_offset,
                    site: *site,
                },
                Op::WithSlot { slot: s, fields } => Op::WithSlot {
                    slot: slot(s),
                    fields: *fields,
                },
                Op::TakeField {
                    slot: s,
                    name,
                    site,
                } => Op::TakeField {
                    slot: slot(s),
                    name: name + const_offset,
                    site: *site,
                },
                Op::Jump(target) => Op::Jump(shift(target)),
                Op::JumpIfFalse(target) => Op::JumpIfFalse(shift(target)),
                Op::JumpIfTrue(target) => Op::JumpIfTrue(shift(target)),
                Op::JumpIfAbsent(target) => Op::JumpIfAbsent(shift(target)),
                Op::JumpIfFailure(target) => Op::JumpIfFailure(shift(target)),
                Op::PushHandler(target) => Op::PushHandler(shift(target)),
                Op::Return => Op::Jump(exit as u32),
                // a `ReturnNothing` the analysis never reached has no tail
                // and is never generated: a jump to the exit stands in
                Op::ReturnNothing => Op::Jump(nothing.unwrap_or(exit) as u32),
                other => other.clone(),
            };
            let ty = code.types.get(q).copied().flatten();
            self.push(mapped, code.spans[q], ty);
        }
        // the exit: the region's slots released as the callee's frame
        // would be, then back to the op after the call, the result on top
        debug_assert_eq!(self.ops.len(), exit);
        self.release(slot_base, locals, call_span);
        self.push(Op::Jump((pc + 1) as u32), call_span, None);
        if let Some(at) = nothing {
            debug_assert_eq!(self.ops.len(), at);
            self.push(Op::Nothing, call_span, None);
            self.push(Op::Jump(exit as u32), call_span, None);
        }
        let end = self.ops.len();
        self.ops[pc] = Op::Jump(start as u32);
        self.next_slot += locals;
        self.regions.push(Region {
            callee: callee as u32,
            call_pc: pc as u32,
            start: start as u32,
            body: body as u32,
            end: end as u32,
            slot_base: slot_base as u16,
            parent,
        });
        self.regions.len() - 1
    }

    /// Every slot of a region moved out and dropped: what leaving the
    /// callee's frame would release, released where the callee ends.
    fn release(&mut self, slot_base: usize, locals: usize, span: Span) {
        for slot in 0..locals {
            self.push(Op::LoadMove((slot_base + slot) as u16), span, None);
            self.push(Op::Pop, span, None);
        }
    }
}

/// The innermost region holding a pc of an expanded code object.
pub fn region_at(regions: &[Region], pc: usize) -> Option<usize> {
    // a nested region is appended after its parent and never overlaps it
    regions.iter().rposition(|region| region.holds(pc))
}

/// The chain of regions around a pc, outermost first.
pub fn chain_at(regions: &[Region], pc: usize) -> Vec<usize> {
    let mut chain = Vec::new();
    let mut next = region_at(regions, pc);
    while let Some(index) = next {
        chain.push(index);
        next = regions[index].parent.map(|parent| parent as usize);
    }
    chain.reverse();
    chain
}

/// The module and the span of an op at a pc of an expanded code object
/// (one at or past the original's length): the callee's op, or the call's
/// own site for the prologue and the tails of a region.
pub fn location(
    program: &Program,
    code: CodeId,
    regions: &[Region],
    pc: usize,
) -> Option<(usize, Span)> {
    let index = region_at(regions, pc)?;
    let region = &regions[index];
    let callee = &program.codes[region.callee as usize];
    if let Some(q) = region.callee_pc(pc, callee) {
        return Some((callee.module, callee.spans[q]));
    }
    // the call's site: in the caller's own ops, or in the parent's body
    match region.parent {
        None => {
            let caller = &program.codes[code];
            Some((caller.module, caller.spans[region.call_pc as usize]))
        }
        Some(parent) => {
            let parent = &regions[parent as usize];
            let outer = &program.codes[parent.callee as usize];
            let q = parent.callee_pc(region.call_pc as usize, outer)?;
            Some((outer.module, outer.spans[q]))
        }
    }
}
