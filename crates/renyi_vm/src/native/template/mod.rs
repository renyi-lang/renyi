//! The template tier (decision AU18): machine code for a code object at
//! its first call, without Cranelift, from a fixed sequence per op. The
//! frame stays in the interpreter's layout at every op (the locals, then
//! the operands, every value boxed on the VM's stack), so the code needs
//! no registers filled at an entry and hands nothing back: the trivial
//! ops are inline, every other op calls the helper the Cranelift tier
//! calls for boxed operands (`runtime.rs`) and checks its status. The
//! function has the signature of a trampoline (`native::Entry`): the VM,
//! the frame's base and the pc to enter at, the status out. The code
//! holds no address (the helpers, the constants and the counts are
//! reached through `NativeState`, decision AS1) and follows the platform's
//! C convention (`x64::Abi`).
//!
//! The hotness: the code counts the ops it runs as the interpreter
//! counts them, a basic block's length added to the code object's count
//! as the block starts (a count of the whole code at every entry, as the
//! first build had it, made short functions that return early look hot,
//! and the JIT compiled half as much again); a back edge that finds the
//! count past the threshold asks `rt_osr` for the Cranelift code, which
//! takes the frame over at the loop header when it is ready, and a call
//! finds the Cranelift code through `Jit::entry` once it is.

pub mod x64;

use std::time::{Duration, Instant};

use crate::bytecode::{Code, Op};
use crate::compile::Program;
use crate::extension::TypedKind;
use crate::native::codegen::{
    borrowed_operands, helper_index, is_comparison, kind_agrees, native_state_offset, stack_offset,
    Skipped, SIGNATURES, STACK_SAFE, STATE_CONSTANTS, STATE_HELPERS, STATE_HOTNESS,
};
use crate::native::infer::{abs_of_constant, abs_of_result, analyse, Analysis};
use crate::native::runtime::{
    binary_code, fold_code, BOXED, CONTINUE, FAILURE, INTERRUPT, LEFT, RETURNED, STAY,
};
use crate::value::layout::{
    INT_PAYLOAD, INT_SMALL, INT_TAG, PAYLOAD, SIZE, TAG_BOOLEAN, TAG_FAILURE, TAG_FLOAT,
    TAG_INTEGER, TAG_NOTHING,
};
use crate::vm::CallKind;
use x64::{host_abi, Abi, Asm, Cond, Label, Reg};

/// The code of one code object: its bytes, which hold no address, the
/// loop headers it can be entered at, and the counts.
pub struct Template {
    pub body: Vec<u8>,
    pub headers: Vec<u32>,
    pub ops: usize,
    pub time: Duration,
}

// The registers the code keeps across the ops.
/// The VM.
const VM: Reg = Reg::Rbx;
/// The frame's base, an index into the stack.
const BASE: Reg = Reg::R12;
/// The address of the frame's first slot: the stack's pointer plus the
/// base in bytes, read again after every helper that may move the stack.
const FRAME: Reg = Reg::R13;
/// The helpers' table.
const HELPERS: Reg = Reg::R14;
/// The base in bytes.
const BASE24: Reg = Reg::R15;
/// The address of the code object's count in `Vm::hotness`, when the
/// code counts.
const COUNT: Reg = Reg::Rbp;
/// The callee-saved registers the code keeps, in the order they are
/// pushed.
const KEPT: [Reg; 6] = [VM, BASE, FRAME, HELPERS, BASE24, COUNT];
// The scratch registers, which no convention passes arguments in: the
// first for copies and answers, the second for the pc a landing needs
// and a constant's address, the third for the stack's length, written
// while the first may hold a condition.
const SCRATCH: Reg = Reg::Rax;
const SCRATCH2: Reg = Reg::R10;
const SCRATCH3: Reg = Reg::R11;

/// The function's own stack frame: its size, and where the answer slot
/// of the helpers, the entry pc and the stack-passed arguments lie.
struct Layout {
    frame: i32,
    out: i32,
    pc: i32,
    stack_args: i32,
}

fn layout(abi: &Abi) -> Layout {
    // the return address and six callee-saved registers leave the stack
    // at eight past a multiple of sixteen; the frame aligns it, with room
    // for the arguments past the registers (one at most under System V,
    // three under Windows x64, above its shadow space)
    if abi.shadow == 0 {
        Layout {
            frame: 40,
            out: 8,
            pc: 16,
            stack_args: 0,
        }
    } else {
        Layout {
            frame: 88,
            out: 56,
            pc: 64,
            stack_args: 32,
        }
    }
}

/// An argument of a helper call.
#[derive(Clone, Copy)]
enum Arg {
    Vm,
    Base,
    Imm(u64),
    /// The address `base + disp`.
    Addr(Reg, i32),
    /// The answer slot.
    Out,
    /// A scratch register holding the value already.
    Reg(Reg),
}

/// The tier's code for a code object, or why it stays with the
/// interpreter. `hot_factor` sets the count a back edge promotes at;
/// without `promotion` the code counts nothing.
pub fn compile(
    program: &Program,
    code_id: usize,
    calls: &[CallKind],
    hot_factor: u32,
    promotion: bool,
) -> Result<Template, Skipped> {
    let started = Instant::now();
    let code = &program.codes[code_id];
    let analysis = analyse(program, code).map_err(Skipped::Analysis)?;
    let (borrowed, masks) = borrowed_operands(program, code, &analysis, calls);
    let abi = host_abi();
    let mut asm = Asm::new();
    let labels = (0..=code.ops.len()).map(|_| asm.label()).collect();
    let epilogue = asm.label();
    let leave = asm.label();
    let not_bool = asm.label();
    // the count a back edge promotes at, within the compare's immediate
    let threshold = promotion.then(|| {
        (code.ops.len() as u32)
            .saturating_mul(hot_factor)
            .min(i32::MAX as u32)
    });
    let counts = if threshold.is_some() {
        block_lengths(code)
    } else {
        vec![None; code.ops.len()]
    };
    let mut gen = Gen {
        program,
        code,
        code_id,
        analysis: &analysis,
        calls,
        borrowed,
        masks,
        abi,
        layout: layout(abi),
        asm,
        labels,
        depth: 0,
        terminated: false,
        epilogue,
        leave,
        not_bool,
        landings: Vec::new(),
        edges: Vec::new(),
        threshold,
        counts,
    };
    let headers = gen.headers();
    gen.prologue(&headers);
    gen.body();
    gen.tail();
    let body = gen.asm.finish().map_err(Skipped::Codegen)?;
    Ok(Template {
        body,
        headers,
        ops: code.ops.len(),
        time: started.elapsed(),
    })
}

struct Gen<'a> {
    program: &'a Program,
    code: &'a Code,
    code_id: usize,
    analysis: &'a Analysis,
    calls: &'a [CallKind],
    /// Per op, whether it pushes an operand a typed call or a comparison
    /// borrows (decisions AU2 and AU13), and per such call the mask.
    borrowed: Vec<bool>,
    masks: Vec<u32>,
    abi: &'static Abi,
    layout: Layout,
    asm: Asm,
    /// Per pc, and one for the end of the code.
    labels: Vec<Label>,
    /// The operand stack's depth at the op being translated.
    depth: usize,
    terminated: bool,
    epilogue: Label,
    /// Where a status other than `CONTINUE` and `FAILURE` leaves the
    /// function: `LEFT` as `RETURNED`, anything else as `INTERRUPT`.
    leave: Label,
    /// Where a branch on a value that is not a Boolean goes: the crash
    /// the interpreter raises, through `rt_take_bool`.
    not_bool: Label,
    /// The shared blocks a failed status goes to, keyed by the innermost
    /// handled region (its target and the depth below it), or none.
    landings: Vec<(Option<(u32, usize)>, Label)>,
    /// The back edges: the loop header and the block that checks the
    /// count and jumps.
    edges: Vec<(usize, Label)>,
    /// The count a back edge promotes at; `None` counts nothing.
    threshold: Option<u32>,
    /// Per op that starts a basic block, the block's length in ops, which
    /// the code adds to the count as the block starts.
    counts: Vec<Option<u32>>,
}

/// Per op, the length of the basic block it starts: a block starts at
/// the first op, at every target of a jump, an iteration's exit or a
/// handled region's landing, and after every op that jumps or leaves;
/// the ops of a block run together, but for a failure in its middle,
/// which counts the rest too.
fn block_lengths(code: &Code) -> Vec<Option<u32>> {
    let count = code.ops.len();
    let mut starts = vec![false; count + 1];
    starts[0] = true;
    for (pc, op) in code.ops.iter().enumerate() {
        let target = match op {
            Op::Jump(target)
            | Op::JumpIfFalse(target)
            | Op::JumpIfTrue(target)
            | Op::JumpIfAbsent(target)
            | Op::JumpIfFailure(target)
            | Op::PushHandler(target) => Some(*target as usize),
            Op::IterNext { exit, .. } => Some(*exit as usize),
            _ => None,
        };
        if let Some(target) = target {
            starts[target.min(count)] = true;
        }
        let ends = matches!(
            op,
            Op::Jump(_)
                | Op::JumpIfFalse(_)
                | Op::JumpIfTrue(_)
                | Op::JumpIfAbsent(_)
                | Op::JumpIfFailure(_)
                | Op::IterNext { .. }
                | Op::Return
                | Op::ReturnNothing
                | Op::Fail
                | Op::Crash
                | Op::Check(_)
        );
        if ends {
            starts[pc + 1] = true;
        }
    }
    let mut lengths = vec![None; count];
    let mut pc = 0;
    while pc < count {
        let mut end = pc + 1;
        while end < count && !starts[end] {
            end += 1;
        }
        lengths[pc] = Some((end - pc) as u32);
        pc = end;
    }
    lengths
}

impl Gen<'_> {
    // ------------------------------------------------------------ places

    /// The address of a local slot.
    fn slot(&self, slot: usize) -> (Reg, i32) {
        (FRAME, (slot * SIZE) as i32)
    }

    /// The address of the operand at a depth.
    fn operand(&self, depth: usize) -> (Reg, i32) {
        (FRAME, ((self.code.locals as usize + depth) * SIZE) as i32)
    }

    /// The address of the next free position.
    fn top(&self) -> (Reg, i32) {
        self.operand(self.depth)
    }

    /// The stack's length written for the current depth: the helpers and
    /// the interpreter read it.
    fn store_len(&mut self) {
        let height = (self.code.locals as usize + self.depth) as i32;
        self.asm.lea(SCRATCH3, BASE, height);
        self.asm.mov_mr(VM, stack_offset() as i32 + 8, SCRATCH3);
    }

    /// The frame pointer read again from the VM, after a helper that may
    /// have grown the stack.
    fn reload_frame(&mut self) {
        self.asm.mov_rm(FRAME, VM, stack_offset() as i32);
        self.asm.add_rr(FRAME, BASE24);
    }

    /// The three words of a value copied, as a `Value` is moved.
    fn copy(&mut self, from: (Reg, i32), to: (Reg, i32)) {
        for offset in [0, 8, 16] {
            self.asm.mov_rm(SCRATCH, from.0, from.1 + offset);
            self.asm.mov_mr(to.0, to.1 + offset, SCRATCH);
        }
    }

    /// One more reference to the value at the address (decision AT6).
    fn retain(&mut self, at: (Reg, i32)) {
        self.call("rt_retain_at", &[Arg::Addr(at.0, at.1)]);
    }

    /// One reference fewer to the value at the address, which is dead
    /// after.
    fn release(&mut self, at: (Reg, i32)) {
        self.call("rt_drop_at", &[Arg::Addr(at.0, at.1)]);
    }

    // ------------------------------------------------------------ calls

    /// A helper called with its arguments in the convention's registers
    /// (and on the stack past them), the frame pointer read again after
    /// unless the helper cannot move the stack.
    fn call(&mut self, name: &'static str, args: &[Arg]) {
        let index = helper_index(name);
        let (_, params, _) = SIGNATURES[index];
        debug_assert_eq!(params.len(), args.len(), "the arguments of {name}");
        let regs = self.abi.args;
        for (position, arg) in args.iter().enumerate() {
            if position < regs.len() {
                self.materialize(regs[position], *arg);
            } else {
                self.materialize(SCRATCH, *arg);
                let at = self.layout.stack_args + 8 * (position - regs.len()) as i32;
                self.asm.mov_mr(Reg::Rsp, at, SCRATCH);
            }
        }
        self.asm.call_m(HELPERS, (index * 8) as i32);
        if !STACK_SAFE.contains(&name) {
            self.reload_frame();
        }
    }

    fn materialize(&mut self, dst: Reg, arg: Arg) {
        match arg {
            Arg::Vm => self.asm.mov_rr(dst, VM),
            Arg::Base => self.asm.mov_rr(dst, BASE),
            Arg::Imm(value) => self.asm.mov_ri(dst, value as i64),
            Arg::Addr(base, disp) => self.asm.lea(dst, base, disp),
            Arg::Out => self.asm.lea(dst, Reg::Rsp, self.layout.out),
            Arg::Reg(reg) => {
                if reg != dst {
                    self.asm.mov_rr(dst, reg);
                }
            }
        }
    }

    /// The status in `eax` checked: `CONTINUE` falls through, anything
    /// else goes to the landing of the op's innermost handled region.
    fn check(&mut self, pc: usize) {
        let handler = self.analysis.handlers[pc].last().copied();
        let landing = self.landing(handler);
        if handler.is_none() {
            // the pc for `rt_unhandled`, which locates the crash
            self.asm.mov_ri32(SCRATCH2, pc as u32);
        }
        self.asm.test_rr32(Reg::Rax, Reg::Rax);
        self.asm.jcc(Cond::Ne, landing);
    }

    fn landing(&mut self, handler: Option<(u32, usize)>) -> Label {
        if let Some((_, label)) = self.landings.iter().find(|(known, _)| *known == handler) {
            return *label;
        }
        let label = self.asm.label();
        self.landings.push((handler, label));
        label
    }

    /// A jump to a pc: through the block that checks the count when it
    /// is a back edge and the code counts.
    fn jump_to(&mut self, target: usize, pc: usize, cond: Option<Cond>) {
        let label = if target <= pc && self.threshold.is_some() {
            self.edge(target)
        } else {
            self.labels[target]
        };
        match cond {
            Some(cond) => self.asm.jcc(cond, label),
            None => self.asm.jmp(label),
        }
    }

    fn edge(&mut self, target: usize) -> Label {
        if let Some((_, label)) = self.edges.iter().find(|(known, _)| *known == target) {
            return *label;
        }
        let label = self.asm.label();
        self.edges.push((target, label));
        label
    }

    /// The Boolean on top popped into `eax` (0 or 1); a value of another
    /// type is the crash the interpreter raises.
    fn pop_condition(&mut self, pc: usize) {
        let at = self.operand(self.depth - 1);
        self.asm.mov_ri32(SCRATCH2, pc as u32);
        self.asm.cmp_m8i(at.0, at.1, TAG_BOOLEAN);
        self.asm.jcc(Cond::Ne, self.not_bool);
        self.asm.movzx_rm8(Reg::Rax, at.0, at.1 + PAYLOAD);
        self.depth -= 1;
        self.store_len();
    }

    /// A scalar answered into the out slot boxed at the address.
    fn box_out(&mut self, kind: TypedKind, at: (Reg, i32)) {
        match kind {
            TypedKind::Bool => {
                self.asm.movzx_rm8(SCRATCH, Reg::Rsp, self.layout.out);
                self.asm.mov_m8i(at.0, at.1, TAG_BOOLEAN);
                self.asm.mov_m8r(at.0, at.1 + PAYLOAD, SCRATCH);
            }
            TypedKind::Int => {
                self.asm.mov_rm(SCRATCH, Reg::Rsp, self.layout.out);
                self.asm.mov_m8i(at.0, at.1, TAG_INTEGER);
                self.asm.mov_m8i(at.0, at.1 + INT_TAG, INT_SMALL);
                self.asm.mov_mr(at.0, at.1 + INT_PAYLOAD, SCRATCH);
            }
            TypedKind::Float => {
                self.asm.mov_rm(SCRATCH, Reg::Rsp, self.layout.out);
                self.asm.mov_m8i(at.0, at.1, TAG_FLOAT);
                self.asm.mov_mr(at.0, at.1 + PAYLOAD, SCRATCH);
            }
            TypedKind::Value => unreachable!("a value is on the stack"),
        }
    }

    /// A helper that answers a scalar into the out slot with `CONTINUE`,
    /// or a boxed value on the stack with `BOXED`, or fails: the scalar
    /// boxed at the operand position `at` (the arguments consumed), the
    /// depth set to `after`; a failure lands on the handler when
    /// `settles` (a call), and stays on the stack otherwise (a
    /// comparison, whose general path the interpreter pushes as it is).
    fn scalar_or_boxed(
        &mut self,
        kind: TypedKind,
        at: usize,
        after: usize,
        pc: usize,
        settles: bool,
    ) {
        let done = self.asm.label();
        let other = self.asm.label();
        self.asm.cmp_r32i(Reg::Rax, CONTINUE);
        self.asm.jcc(Cond::Ne, other);
        if kind != TypedKind::Value {
            let place = self.operand(at);
            self.box_out(kind, place);
        }
        self.depth = after;
        self.store_len();
        self.asm.jmp(done);
        self.asm.bind(other);
        self.asm.cmp_r32i(Reg::Rax, BOXED);
        self.asm.jcc(Cond::E, done);
        if settles {
            self.check(pc);
        } else {
            self.leave_on_interrupt();
        }
        self.asm.bind(done);
        self.depth = after;
    }

    // ------------------------------------------------------------ the shape

    /// The loop headers: the targets of the backward jumps that a path
    /// reaches (a jump in dead code may name a target no path reaches,
    /// whose label is never bound).
    fn headers(&self) -> Vec<u32> {
        let reached = |pc: usize| matches!(self.analysis.entry.get(pc), Some(Some(_)));
        let mut headers: Vec<u32> = self
            .code
            .ops
            .iter()
            .enumerate()
            .filter_map(|(pc, op)| match op {
                Op::Jump(target)
                | Op::JumpIfFalse(target)
                | Op::JumpIfTrue(target)
                | Op::JumpIfAbsent(target)
                | Op::JumpIfFailure(target)
                    if *target as usize <= pc && reached(pc) && reached(*target as usize) =>
                {
                    Some(*target)
                }
                _ => None,
            })
            .collect();
        headers.sort_unstable();
        headers.dedup();
        headers
    }

    fn prologue(&mut self, headers: &[u32]) {
        let abi = self.abi;
        for reg in KEPT {
            self.asm.push(reg);
        }
        self.asm.sub_ri(Reg::Rsp, self.layout.frame);
        let args = abi.args;
        self.asm.mov_rr(VM, args[0]);
        self.asm.mov_rr(BASE, args[1]);
        self.asm.mov_mr32(Reg::Rsp, self.layout.pc, args[2]);
        self.asm
            .mov_rm(HELPERS, VM, native_state_offset() as i32 + STATE_HELPERS);
        self.asm.imul_rri(BASE24, BASE, SIZE as i32);
        self.reload_frame();
        // room on the stack for the locals and the deepest operand stack,
        // checked once (decision AR4)
        let deepest = self
            .analysis
            .entry
            .iter()
            .flatten()
            .map(|state| state.len())
            .max()
            .unwrap_or(0);
        let room = (self.code.locals as usize + deepest + 1) as i32;
        self.asm.mov_rm(SCRATCH, VM, stack_offset() as i32 + 16);
        self.asm.lea(SCRATCH2, BASE, room);
        self.asm.cmp_rr(SCRATCH, SCRATCH2);
        let fits = self.asm.label();
        self.asm.jcc(Cond::Ae, fits);
        self.call("rt_room", &[Arg::Vm, Arg::Addr(BASE, room)]);
        self.asm.bind(fits);
        // the address of the count of ops run, which every block adds to
        if self.threshold.is_some() {
            self.asm
                .mov_rm(COUNT, VM, native_state_offset() as i32 + STATE_HOTNESS);
            self.asm.add_ri(COUNT, (self.code_id * 4) as i32);
        }
        // the entry: the start, or a loop header
        self.asm.mov_rm32(SCRATCH2, Reg::Rsp, self.layout.pc);
        self.asm.test_rr32(SCRATCH2, SCRATCH2);
        self.asm.jcc(Cond::E, self.labels[0]);
        for header in headers {
            self.asm.cmp_r32i(SCRATCH2, *header as i32);
            self.asm.jcc(Cond::E, self.labels[*header as usize]);
        }
        // an entry the code does not have: the interpreter carries on
        self.asm.mov_ri32(Reg::Rax, STAY as u32);
        self.asm.jmp(self.epilogue);
    }

    fn body(&mut self) {
        let count = self.code.ops.len();
        for pc in 0..count {
            let Some(state) = &self.analysis.entry[pc] else {
                // no path reaches the op
                self.terminated = true;
                continue;
            };
            if self.analysis.block_starts[pc] || self.terminated {
                self.depth = state.len();
                self.terminated = false;
            }
            debug_assert_eq!(
                self.depth,
                state.len(),
                "the depth at {pc} of {}",
                self.code.name
            );
            self.asm.bind(self.labels[pc]);
            if let Some(length) = self.counts[pc] {
                self.asm.add_m32i(COUNT, 0, length as i32);
            }
            self.op(pc);
        }
        // the end of the code: an implicit `return nothing`
        self.asm.bind(self.labels[count]);
        self.return_nothing();
    }

    /// The shared blocks after the body: the landings, the counting back
    /// edges, the leave, the Boolean crash and the epilogue.
    fn tail(&mut self) {
        let landings = std::mem::take(&mut self.landings);
        for (handler, label) in landings {
            self.asm.bind(label);
            self.asm.cmp_r32i(Reg::Rax, FAILURE);
            self.asm.jcc(Cond::Ne, self.leave);
            match handler {
                Some((target, floor)) => {
                    self.call(
                        "rt_settle_handler",
                        &[
                            Arg::Vm,
                            Arg::Base,
                            Arg::Imm(self.code.locals as u64),
                            Arg::Imm(floor as u64),
                        ],
                    );
                    self.asm.jmp(self.labels[target as usize]);
                }
                None => {
                    self.call("rt_unhandled", &[Arg::Vm, Arg::Reg(SCRATCH2)]);
                    self.asm.jmp(self.leave);
                }
            }
        }
        let edges = std::mem::take(&mut self.edges);
        for (target, label) in edges {
            self.asm.bind(label);
            let threshold = self.threshold.expect("an edge counts");
            self.asm.cmp_m32i(COUNT, 0, threshold as i32);
            self.asm.jcc(Cond::B, self.labels[target]);
            // hot: the Cranelift code takes the loop over when it can
            self.call(
                "rt_osr",
                &[
                    Arg::Vm,
                    Arg::Base,
                    Arg::Imm(self.code_id as u64),
                    Arg::Imm(target as u64),
                ],
            );
            self.asm.cmp_r32i(Reg::Rax, STAY);
            self.asm.jcc(Cond::E, self.labels[target]);
            self.asm.jmp(self.epilogue);
        }
        self.asm.bind(self.not_bool);
        self.call("rt_take_bool", &[Arg::Vm, Arg::Reg(SCRATCH2), Arg::Out]);
        self.asm.bind(self.leave);
        let interrupted = self.asm.label();
        self.asm.cmp_r32i(Reg::Rax, LEFT);
        self.asm.jcc(Cond::Ne, interrupted);
        self.asm.mov_ri32(Reg::Rax, RETURNED as u32);
        self.asm.jmp(self.epilogue);
        self.asm.bind(interrupted);
        self.asm.mov_ri32(Reg::Rax, INTERRUPT as u32);
        self.asm.bind(self.epilogue);
        self.asm.add_ri(Reg::Rsp, self.layout.frame);
        for reg in KEPT.iter().rev() {
            self.asm.pop(*reg);
        }
        self.asm.ret();
    }

    /// The frame returned with the value on top of the stack.
    fn return_top(&mut self) {
        self.call("rt_return", &[Arg::Vm]);
        self.asm.mov_ri32(Reg::Rax, RETURNED as u32);
        self.asm.jmp(self.epilogue);
        self.terminated = true;
    }

    fn return_nothing(&mut self) {
        let top = self.top();
        self.asm.mov_m8i(top.0, top.1, TAG_NOTHING);
        self.depth += 1;
        self.store_len();
        self.return_top();
    }

    /// A helper that pops `pops` operands and pushes `pushes`, whose
    /// failure the interpreter settles (a call, a construction, an
    /// update): `FAILURE` lands on the handler.
    fn helper(&mut self, name: &'static str, args: &[Arg], pops: usize, pushes: usize, pc: usize) {
        self.call(name, args);
        self.depth = self.depth - pops + pushes;
        self.check(pc);
    }

    /// A helper whose result the interpreter pushes as it is, a failure
    /// among the values (a field, an operator, a text, a global): only an
    /// interrupt leaves.
    fn pushes(&mut self, name: &'static str, args: &[Arg], pops: usize, pushes: usize) {
        self.call(name, args);
        self.depth = self.depth - pops + pushes;
        self.leave_on_interrupt();
    }

    /// The status in `eax` past `FAILURE` (an interrupt, or a frame left)
    /// leaves the function.
    fn leave_on_interrupt(&mut self) {
        self.asm.cmp_r32i(Reg::Rax, FAILURE);
        self.asm.jcc(Cond::A, self.leave);
    }

    /// A helper without a status.
    fn plain(&mut self, name: &'static str, args: &[Arg], pops: usize, pushes: usize) {
        self.call(name, args);
        self.depth = self.depth - pops + pushes;
    }

    // ------------------------------------------------------------ the ops

    fn op(&mut self, pc: usize) {
        let code_id = self.code_id as u64;
        let pc_arg = Arg::Imm(pc as u64);
        match &self.code.ops[pc] {
            Op::Const(index) => {
                let constant = &self.code.constants[*index as usize];
                let plain = !abs_of_constant(constant).is_boxed();
                let top = self.top();
                self.asm
                    .mov_rm(SCRATCH2, VM, native_state_offset() as i32 + STATE_CONSTANTS);
                self.asm
                    .mov_rm(SCRATCH2, SCRATCH2, (self.code_id * 8) as i32);
                self.copy((SCRATCH2, (*index as usize * SIZE) as i32), top);
                if !plain && !self.borrowed[pc] {
                    self.retain(top);
                }
                self.depth += 1;
                self.store_len();
            }
            Op::Nothing => {
                let top = self.top();
                self.asm.mov_m8i(top.0, top.1, TAG_NOTHING);
                self.depth += 1;
                self.store_len();
            }
            Op::Global(index) => {
                self.pushes(
                    "rt_global",
                    &[Arg::Vm, Arg::Imm(*index as u64), pc_arg],
                    0,
                    1,
                );
            }
            Op::Load(slot) | Op::LoadMove(slot) => {
                let from = self.slot(*slot as usize);
                let top = self.top();
                self.copy(from, top);
                let moving = matches!(self.code.ops[pc], Op::LoadMove(_)) && !self.borrowed[pc];
                if moving {
                    self.asm.mov_m8i(from.0, from.1, TAG_NOTHING);
                } else if !self.borrowed[pc] {
                    self.retain(top);
                }
                self.depth += 1;
                self.store_len();
            }
            Op::Store(slot) => {
                let to = self.slot(*slot as usize);
                let from = self.operand(self.depth - 1);
                self.release(to);
                self.copy(from, to);
                self.depth -= 1;
                self.store_len();
            }
            Op::Pop => {
                let at = self.operand(self.depth - 1);
                self.release(at);
                self.depth -= 1;
                self.store_len();
            }
            Op::Dup => {
                let from = self.operand(self.depth - 1);
                let top = self.top();
                self.copy(from, top);
                self.retain(top);
                self.depth += 1;
                self.store_len();
            }
            Op::MakeList(count) => {
                let n = *count as usize;
                self.plain("rt_make_list", &[Arg::Vm, Arg::Imm(n as u64)], n, 1);
            }
            Op::MakeMap(count) => {
                let n = *count as usize;
                self.plain("rt_make_map", &[Arg::Vm, Arg::Imm(n as u64)], 2 * n, 1);
            }
            Op::MakePair => self.plain("rt_make_pair", &[Arg::Vm], 2, 1),
            Op::MakeRange { stepped } => {
                let operands = if *stepped { 3 } else { 2 };
                self.pushes(
                    "rt_make_range",
                    &[Arg::Vm, Arg::Imm(*stepped as u64), pc_arg],
                    operands,
                    1,
                );
            }
            Op::Construct { ty, fields } => {
                self.helper(
                    "rt_construct",
                    &[
                        Arg::Vm,
                        Arg::Imm(*ty as u64),
                        Arg::Imm(*fields as u64),
                        pc_arg,
                    ],
                    *fields as usize,
                    1,
                    pc,
                );
            }
            Op::ConstructVariant { ty, tag, fields } => {
                self.helper(
                    "rt_construct_variant",
                    &[
                        Arg::Vm,
                        Arg::Imm(*ty as u64),
                        Arg::Imm(*tag as u64),
                        Arg::Imm(*fields as u64),
                        pc_arg,
                    ],
                    *fields as usize,
                    1,
                    pc,
                );
            }
            Op::Field { name, site } => {
                self.pushes(
                    "rt_field",
                    &[
                        Arg::Vm,
                        Arg::Imm(code_id),
                        Arg::Imm(*name as u64),
                        Arg::Imm(*site as u64),
                        pc_arg,
                    ],
                    1,
                    1,
                );
            }
            Op::LoadField { slot, name, site } => {
                self.pushes(
                    "rt_load_field",
                    &[
                        Arg::Vm,
                        Arg::Base,
                        Arg::Imm(*slot as u64),
                        Arg::Imm(code_id),
                        Arg::Imm(*name as u64),
                        Arg::Imm(*site as u64),
                        pc_arg,
                    ],
                    0,
                    1,
                );
            }
            Op::With(count) => {
                let n = *count as usize;
                self.helper(
                    "rt_with",
                    &[Arg::Vm, Arg::Imm(n as u64), pc_arg],
                    2 * n + 1,
                    1,
                    pc,
                );
            }
            Op::WithSlot { slot, fields } => {
                let n = *fields as usize;
                self.helper(
                    "rt_with_slot",
                    &[
                        Arg::Vm,
                        Arg::Base,
                        Arg::Imm(*slot as u64),
                        Arg::Imm(n as u64),
                        pc_arg,
                    ],
                    2 * n,
                    1,
                    pc,
                );
            }
            Op::TakeField { slot, name, site } => {
                self.pushes(
                    "rt_take_field",
                    &[
                        Arg::Vm,
                        Arg::Base,
                        Arg::Imm(*slot as u64),
                        Arg::Imm(code_id),
                        Arg::Imm(*name as u64),
                        Arg::Imm(*site as u64),
                        pc_arg,
                    ],
                    0,
                    1,
                );
            }
            Op::Call { function, args } => {
                let count = *args as usize;
                let result = abs_of_result(self.program, *function);
                match self.calls.get(*function).copied() {
                    Some(CallKind::Typed(kind)) if kind_agrees(kind, result) => {
                        let mask = self.masks[pc];
                        let at = self.depth - count;
                        self.call(
                            "rt_call_typed",
                            &[
                                Arg::Vm,
                                Arg::Imm(*function as u64),
                                Arg::Imm(count as u64),
                                Arg::Imm(mask as u64),
                                pc_arg,
                                Arg::Out,
                            ],
                        );
                        self.scalar_or_boxed(kind, at, at + 1, pc, true);
                    }
                    Some(CallKind::Pure) => self.helper(
                        "rt_call_pure",
                        &[
                            Arg::Vm,
                            Arg::Imm(*function as u64),
                            Arg::Imm(count as u64),
                            pc_arg,
                        ],
                        count,
                        1,
                        pc,
                    ),
                    _ => self.helper(
                        "rt_call",
                        &[
                            Arg::Vm,
                            Arg::Imm(*function as u64),
                            Arg::Imm(count as u64),
                            pc_arg,
                        ],
                        count,
                        1,
                        pc,
                    ),
                }
            }
            Op::CallAbility {
                ability,
                method,
                args,
            } => {
                self.helper(
                    "rt_call_ability",
                    &[
                        Arg::Vm,
                        Arg::Imm(*ability as u64),
                        Arg::Imm(*method as u64),
                        Arg::Imm(*args as u64),
                        pc_arg,
                    ],
                    *args as usize,
                    1,
                    pc,
                );
            }
            Op::CallValue(args) => {
                self.helper(
                    "rt_call_value",
                    &[Arg::Vm, Arg::Imm(*args as u64), pc_arg],
                    *args as usize + 1,
                    1,
                    pc,
                );
            }
            Op::ResultType(index) => {
                self.plain("rt_result_type", &[Arg::Vm, Arg::Imm(*index as u64)], 0, 0);
            }
            Op::Not => self.pushes("rt_not", &[Arg::Vm, pc_arg], 1, 1),
            Op::Binary(op) => {
                if is_comparison(*op) {
                    let mask = self.masks[pc];
                    let at = self.depth - 2;
                    self.call(
                        "rt_compare",
                        &[
                            Arg::Vm,
                            Arg::Imm(binary_code(*op) as u64),
                            Arg::Imm(mask as u64),
                            pc_arg,
                            Arg::Out,
                        ],
                    );
                    self.scalar_or_boxed(TypedKind::Bool, at, at + 1, pc, false);
                } else {
                    self.pushes(
                        "rt_binary",
                        &[Arg::Vm, Arg::Imm(binary_code(*op) as u64), pc_arg],
                        2,
                        1,
                    );
                }
            }
            Op::ToText => self.pushes("rt_to_text", &[Arg::Vm, pc_arg], 1, 1),
            Op::Concat(count) => {
                let n = *count as usize;
                self.pushes("rt_concat", &[Arg::Vm, Arg::Imm(n as u64), pc_arg], n, 1);
            }
            Op::Jump(target) => {
                self.jump_to(*target as usize, pc, None);
                self.terminated = true;
            }
            Op::JumpIfFalse(target) => {
                self.pop_condition(pc);
                self.asm.test_rr32(Reg::Rax, Reg::Rax);
                self.jump_to(*target as usize, pc, Some(Cond::E));
            }
            Op::JumpIfTrue(target) => {
                self.pop_condition(pc);
                self.asm.test_rr32(Reg::Rax, Reg::Rax);
                self.jump_to(*target as usize, pc, Some(Cond::Ne));
            }
            Op::JumpIfAbsent(target) | Op::JumpIfFailure(target) => {
                let at = self.operand(self.depth - 1);
                self.asm.movzx_rm8(SCRATCH, at.0, at.1);
                self.asm.cmp_r32i(SCRATCH, TAG_FAILURE as i32);
                self.jump_to(*target as usize, pc, Some(Cond::E));
                if matches!(self.code.ops[pc], Op::JumpIfAbsent(_)) {
                    self.asm.cmp_r32i(SCRATCH, TAG_NOTHING as i32);
                    self.jump_to(*target as usize, pc, Some(Cond::E));
                }
            }
            Op::PushHandler(_) | Op::PopHandler | Op::MarkStack(_) => {}
            Op::Return => {
                self.depth -= 1;
                self.return_top();
            }
            Op::ReturnNothing => self.return_nothing(),
            Op::Fail => {
                self.call("rt_fail", &[Arg::Vm, pc_arg]);
                self.asm.jmp(self.leave);
                self.terminated = true;
            }
            Op::Crash => {
                self.call("rt_crash", &[Arg::Vm, pc_arg]);
                self.asm.jmp(self.leave);
                self.terminated = true;
            }
            Op::IsVariant(tag) => {
                self.call("rt_is_variant", &[Arg::Vm, Arg::Imm(*tag as u64), Arg::Out]);
                let at = self.operand(self.depth - 1);
                self.box_out(TypedKind::Bool, at);
                self.store_len();
            }
            Op::IsNothing | Op::IsFailure => {
                let name = if matches!(self.code.ops[pc], Op::IsNothing) {
                    "rt_is_nothing"
                } else {
                    "rt_is_failure"
                };
                self.call(name, &[Arg::Vm, Arg::Out]);
                let at = self.operand(self.depth - 1);
                self.box_out(TypedKind::Bool, at);
                self.store_len();
            }
            Op::IsType(ty) => {
                self.call("rt_is_type", &[Arg::Vm, Arg::Imm(*ty as u64), Arg::Out]);
                let at = self.operand(self.depth - 1);
                self.box_out(TypedKind::Bool, at);
                self.store_len();
            }
            Op::Unpack(count) => {
                let n = *count as usize;
                self.pushes("rt_unpack", &[Arg::Vm, Arg::Imm(n as u64), pc_arg], 1, n);
            }
            Op::UnwrapFailure => self.plain("rt_unwrap_failure", &[Arg::Vm], 1, 1),
            Op::IterInit(slot) => {
                self.pushes(
                    "rt_iter_init",
                    &[Arg::Vm, Arg::Base, Arg::Imm(*slot as u64), pc_arg],
                    1,
                    0,
                );
            }
            Op::IterNext { slot, exit } => {
                self.call(
                    "rt_iter_next",
                    &[Arg::Vm, Arg::Base, Arg::Imm(*slot as u64)],
                );
                self.asm.movzx_rr8(Reg::Rax, Reg::Rax);
                self.asm.test_rr32(Reg::Rax, Reg::Rax);
                self.jump_to(*exit as usize, pc, Some(Cond::E));
                self.depth += 1;
            }
            Op::ListPush => self.pushes("rt_list_push", &[Arg::Vm, pc_arg], 2, 1),
            Op::GroupInsert => self.pushes("rt_group_insert", &[Arg::Vm, pc_arg], 3, 1),
            Op::GroupFold(fold) => {
                self.pushes(
                    "rt_group_fold",
                    &[Arg::Vm, Arg::Imm(fold_code(*fold) as u64), pc_arg],
                    3,
                    1,
                );
            }
            Op::SortByKey { descending } => {
                self.pushes(
                    "rt_sort_by_key",
                    &[Arg::Vm, Arg::Imm(*descending as u64), pc_arg],
                    1,
                    1,
                );
            }
            Op::Deadline(slot) => {
                self.pushes(
                    "rt_deadline",
                    &[Arg::Vm, Arg::Base, Arg::Imm(*slot as u64), pc_arg],
                    1,
                    0,
                );
            }
            Op::CheckDeadline(slot) => {
                // the failure, when there is one, is on top for the landing
                self.helper(
                    "rt_check_deadline",
                    &[Arg::Vm, Arg::Base, Arg::Imm(*slot as u64)],
                    0,
                    0,
                    pc,
                );
            }
            Op::UnwindStack(slot) => {
                let keep = self.analysis.marks[*slot as usize].unwrap_or(0);
                if self.depth > keep {
                    self.call(
                        "rt_truncate",
                        &[
                            Arg::Vm,
                            Arg::Base,
                            Arg::Imm(self.code.locals as u64),
                            Arg::Imm(keep as u64),
                        ],
                    );
                }
                self.depth = keep;
            }
            Op::Check(message) => {
                self.pop_condition(pc);
                let holds = self.asm.label();
                self.asm.test_rr32(Reg::Rax, Reg::Rax);
                self.asm.jcc(Cond::Ne, holds);
                self.call(
                    "rt_check_failed",
                    &[Arg::Vm, Arg::Imm(code_id), Arg::Imm(*message as u64)],
                );
                self.asm.jmp(self.leave);
                self.asm.bind(holds);
            }
        }
    }
}
