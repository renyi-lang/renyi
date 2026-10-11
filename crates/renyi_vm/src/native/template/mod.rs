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

use renyi_syntax::ast::BinaryOp;

use crate::bytecode::{Code, Op};
use crate::compile::Program;
use crate::extension::TypedKind;
use crate::native::codegen::{
    borrowed_operands, field_cache_offset, frames_offset, handlers_offset, helper_index,
    is_comparison, kind_agrees, native_state_offset, stack_offset, unit_variants_offset, Skipped,
    FRAME_BASE, FRAME_CODE, FRAME_GRANT, FRAME_HANDLER_BASE, FRAME_PC, FRAME_SIZE, SIGNATURES,
    SITE_INDEX, SITE_SIZE, SITE_TAG, SITE_TY, STACK_SAFE, STATE_CONSTANTS, STATE_DEPTH,
    STATE_ENTRIES, STATE_HELPERS, STATE_HOTNESS, STATE_LIST_ITEMS, STATE_LIST_LEN,
};
use crate::native::infer::{
    abs_of_constant, abs_of_result, analyse, is_integer, is_text, Abs, Analysis,
};
use crate::native::runtime::{
    binary_code, fold_code, BOXED, CONTINUE, DECLINED, FAILURE, INTERRUPT, LEFT, RETURNED, STAY,
};
use crate::native::DEPTH_LIMIT;
use crate::value::layout::{
    COUNTED_OR_INTEGER, INT_PAYLOAD, INT_SMALL, INT_TAG, PAYLOAD, RECORD_FIELDS, RECORD_TAG,
    RECORD_TY, SIZE, TAG_BOOLEAN, TAG_FAILURE, TAG_FLOAT, TAG_INTEGER, TAG_LIST, TAG_NOTHING,
    TAG_RECORD, TAG_SMALL_TEXT,
};
use crate::vm::{unit_slot, CallKind, InPlace};
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
    /// Where a direct call keeps its callee's entry across the helpers
    /// that push the frame.
    spare: i32,
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
            spare: 24,
        }
    } else {
        Layout {
            frame: 88,
            out: 56,
            pc: 64,
            stack_args: 32,
            spare: 72,
        }
    }
}

/// A path the code rarely takes, emitted after the body so that the
/// paths it takes lie together (decision AU21).
enum Cold {
    /// A direct call without the callee's machine code, or at the depth
    /// limit: through `rt_call`.
    Call {
        label: Label,
        done: Label,
        function: usize,
        count: usize,
        pc: usize,
    },
    /// A direct call whose callee handed its frame to the interpreter or
    /// was interrupted, the status in `eax`.
    Handed {
        label: Label,
        done: Label,
        pc: usize,
    },
    /// A direct call whose callee left a failure.
    Failed { label: Label, pc: usize },
    /// A value to retain whose tag is an Integer's (decision AU29): a big
    /// one raises the count of its block, a small one holds none.
    Retain {
        label: Label,
        done: Label,
        at: (Reg, i32),
    },
    /// A value to release whose tag is an Integer's, through the helper
    /// unless it is small; or a counted block whose count reached zero,
    /// raised back to one for the helper's drop, which frees it (decision
    /// AU29).
    Release {
        integer: Label,
        last: Label,
        done: Label,
        at: (Reg, i32),
    },
    /// A variant without fields not built yet: through
    /// `rt_construct_variant`, which builds it and keeps it (decision
    /// AU24).
    Unit {
        label: Label,
        done: Label,
        ty: usize,
        tag: u16,
        pc: usize,
    },
    /// A field read the site's cache did not answer: through
    /// `rt_load_field`, which fills the cache.
    Field {
        label: Label,
        done: Label,
        slot: u16,
        name: u32,
        site: u32,
        pc: usize,
    },
}

/// How many fresh locals a direct call writes `Nothing` into in place;
/// past this many, a helper does it.
const CLEARS_INLINE: usize = 4;

/// The bits a retain or a release in place tests (decision AU29), as the
/// 32-bit immediate the test takes them in.
const COUNTED_OR_INTEGER_32: u32 = COUNTED_OR_INTEGER as u32;
const _: () = assert!(COUNTED_OR_INTEGER >> 32 == 0);

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
        cold: Vec::new(),
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
    /// The paths emitted after the body.
    cold: Vec<Cold>,
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

    /// One more reference to the value at the address, in place (decision
    /// AU29): a value whose tag says its payload points at a counted block
    /// raises the block's first word, the count; an Integer goes out of
    /// line, where a big one raises its own; any other value holds no
    /// count.
    fn retain(&mut self, at: (Reg, i32)) {
        debug_assert!(
            at.0 != SCRATCH && at.0 != SCRATCH2,
            "a value off the scratch registers"
        );
        let done = self.asm.label();
        let integer = self.asm.label();
        self.asm.movzx_rm8(SCRATCH, at.0, at.1);
        self.asm.mov_ri32(SCRATCH2, COUNTED_OR_INTEGER_32);
        self.asm.bt_rr32(SCRATCH2, SCRATCH);
        self.asm.jcc_short(Cond::Ae, done);
        self.asm.cmp_r32i(SCRATCH, TAG_INTEGER as i32);
        self.asm.jcc(Cond::E, integer);
        self.asm.mov_rm(SCRATCH2, at.0, at.1 + PAYLOAD);
        self.asm.add_m64i(SCRATCH2, 0, 1);
        self.asm.bind(done);
        self.cold.push(Cold::Retain {
            label: integer,
            done,
            at,
        });
    }

    /// One reference fewer to the value at the address, which is dead
    /// after, in place (decision AU29): the count of a counted block
    /// lowered, and only the last reference, raised back, goes through
    /// `rt_drop_at`, which frees what the value holds; an Integer goes out
    /// of line, where a big one is dropped by the helper.
    fn release(&mut self, at: (Reg, i32)) {
        debug_assert!(
            at.0 != SCRATCH && at.0 != SCRATCH2,
            "a value off the scratch registers"
        );
        let done = self.asm.label();
        let integer = self.asm.label();
        let last = self.asm.label();
        self.asm.movzx_rm8(SCRATCH, at.0, at.1);
        self.asm.mov_ri32(SCRATCH2, COUNTED_OR_INTEGER_32);
        self.asm.bt_rr32(SCRATCH2, SCRATCH);
        self.asm.jcc_short(Cond::Ae, done);
        self.asm.cmp_r32i(SCRATCH, TAG_INTEGER as i32);
        self.asm.jcc(Cond::E, integer);
        self.asm.mov_rm(SCRATCH2, at.0, at.1 + PAYLOAD);
        self.asm.sub_m64i(SCRATCH2, 0, 1);
        self.asm.jcc(Cond::E, last);
        self.asm.bind(done);
        self.cold.push(Cold::Release {
            integer,
            last,
            done,
            at,
        });
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

    /// A call through `rt_call_typed` (decision AU1): the arguments the
    /// mask names pushed borrowed, the answer boxed at the first one's
    /// place.
    fn call_typed(&mut self, function: usize, count: usize, kind: TypedKind, mask: u32, pc: usize) {
        let at = self.depth - count;
        self.call(
            "rt_call_typed",
            &[
                Arg::Vm,
                Arg::Imm(function as u64),
                Arg::Imm(count as u64),
                Arg::Imm(mask as u64),
                Arg::Imm(pc as u64),
                Arg::Out,
            ],
        );
        self.scalar_or_boxed(kind, at, at + 1, pc, true);
    }

    /// `List.at` in place (decision AU34), as the Cranelift tier makes it:
    /// on a list and a small Integer, the item read through the list's
    /// block where the native state says its items' address and their
    /// count lie, copied with one more reference, or `Nothing` past either
    /// end; the list released unless the call borrows it, after the item
    /// is copied (over the index's operand, a small Integer that needs no
    /// release), since the release may free the list. Anything else takes
    /// `rt_call_typed`, the operands untouched.
    fn list_at_in_place(&mut self, function: usize, mask: u32, pc: usize) {
        let at = self.depth - 2;
        let list = self.operand(at);
        let index = self.operand(at + 1);
        let slow = self.asm.label();
        let past = self.asm.label();
        let done = self.asm.label();
        self.asm.cmp_m8i(list.0, list.1, TAG_LIST);
        self.asm.jcc(Cond::Ne, slow);
        self.asm.cmp_m8i(index.0, index.1, TAG_INTEGER);
        self.asm.jcc(Cond::Ne, slow);
        self.asm.cmp_m8i(index.0, index.1 + INT_TAG, INT_SMALL);
        self.asm.jcc(Cond::Ne, slow);
        let state = native_state_offset() as i32;
        // the count of the items against the index, unsigned
        self.asm.mov_rm(SCRATCH2, list.0, list.1 + PAYLOAD);
        self.asm.mov_rm(SCRATCH3, VM, state + STATE_LIST_LEN);
        self.asm.add_rr(SCRATCH3, SCRATCH2);
        self.asm.mov_rm(SCRATCH3, SCRATCH3, 0);
        self.asm.mov_rm(SCRATCH, index.0, index.1 + INT_PAYLOAD);
        self.asm.cmp_rr(SCRATCH, SCRATCH3);
        self.asm.jcc(Cond::Ae, past);
        // the item's address
        self.asm.mov_rm(SCRATCH3, VM, state + STATE_LIST_ITEMS);
        self.asm.add_rr(SCRATCH3, SCRATCH2);
        self.asm.mov_rm(SCRATCH3, SCRATCH3, 0);
        self.asm.imul_rri(SCRATCH, SCRATCH, SIZE as i32);
        self.asm.add_rr(SCRATCH3, SCRATCH);
        self.asm.cmp_m8i(SCRATCH3, 0, TAG_FAILURE);
        self.asm.jcc(Cond::E, slow);
        self.retain((SCRATCH3, 0));
        if mask & 1 != 0 {
            self.copy((SCRATCH3, 0), list);
        } else {
            self.copy((SCRATCH3, 0), index);
            self.release(list);
            self.copy(index, list);
        }
        self.asm.jmp(done);
        self.asm.bind(past);
        if mask & 1 == 0 {
            self.release(list);
        }
        self.asm.mov_m8i(list.0, list.1, TAG_NOTHING);
        self.asm.jmp(done);
        self.asm.bind(slow);
        self.call_typed(function, 2, TypedKind::Value, mask, pc);
        self.asm.bind(done);
        self.depth = at + 1;
        self.store_len();
    }

    /// A call through a lean helper (decision AU34): the operands' place
    /// and the mask of those the call borrows, the Boolean answer boxed
    /// over the first; when the helper declines, `rt_call_typed` on the
    /// operands it left as they were.
    fn call_lean(
        &mut self,
        helper: &'static str,
        function: usize,
        kind: TypedKind,
        mask: u32,
        pc: usize,
    ) {
        let at = self.depth - 2;
        let place = self.operand(at);
        let general = self.asm.label();
        let done = self.asm.label();
        self.call(
            helper,
            &[Arg::Addr(place.0, place.1), Arg::Imm(mask as u64)],
        );
        self.asm.cmp_r32i(Reg::Rax, DECLINED);
        self.asm.jcc(Cond::E, general);
        self.asm.mov_m8i(place.0, place.1, TAG_BOOLEAN);
        self.asm.mov_m8r(place.0, place.1 + PAYLOAD, Reg::Rax);
        self.asm.jmp(done);
        self.asm.bind(general);
        self.call_typed(function, 2, kind, mask, pc);
        self.asm.bind(done);
        self.depth = at + 1;
        self.store_len();
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
        // the entry: the start, where a code object found hot asks for its
        // Cranelift code (a direct call does not go through `Jit::entry`,
        // decision AU21), or a loop header
        self.asm.mov_rm32(SCRATCH2, Reg::Rsp, self.layout.pc);
        self.asm.test_rr32(SCRATCH2, SCRATCH2);
        match self.threshold {
            Some(threshold) => {
                let elsewhere = self.asm.label();
                self.asm.jcc(Cond::Ne, elsewhere);
                self.asm.cmp_m32i(COUNT, 0, threshold as i32);
                self.asm.jcc(Cond::B, self.labels[0]);
                self.call("rt_promote", &[Arg::Vm, Arg::Imm(self.code_id as u64)]);
                self.asm.jmp(self.labels[0]);
                self.asm.bind(elsewhere);
                self.asm.mov_rm32(SCRATCH2, Reg::Rsp, self.layout.pc);
            }
            None => self.asm.jcc(Cond::E, self.labels[0]),
        }
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
        for cold in std::mem::take(&mut self.cold) {
            self.emit_cold(cold);
        }
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

    /// Whether the checker noted one of a comparison's operands a Text,
    /// which the code then compares in place when both are held in the
    /// value itself (decision AU26).
    fn texts_compared(&self, pc: usize) -> bool {
        let Some(state) = &self.analysis.entry[pc] else {
            return false;
        };
        let n = state.len();
        n >= 2 && (is_text(self.program, state[n - 2]) || is_text(self.program, state[n - 1]))
    }

    /// `is` or `is not` on the two operands at `at` (decision AU26): when
    /// both are texts held in the value itself, their sixteen bytes past
    /// the tag are compared as two words, the Boolean written over the
    /// left operand and the code goes on at `done` with nothing to
    /// release; anything else falls through to the general comparison.
    fn compare_small_texts(&mut self, op: BinaryOp, at: usize, done: Label) {
        let general = self.asm.label();
        let answer = self.asm.label();
        let left = self.operand(at);
        let right = self.operand(at + 1);
        self.asm.cmp_m8i(left.0, left.1, TAG_SMALL_TEXT);
        self.asm.jcc(Cond::Ne, general);
        self.asm.cmp_m8i(right.0, right.1, TAG_SMALL_TEXT);
        self.asm.jcc(Cond::Ne, general);
        self.asm.mov_rm(SCRATCH, left.0, left.1 + PAYLOAD);
        self.asm.mov_rm(SCRATCH2, right.0, right.1 + PAYLOAD);
        self.asm.cmp_rr(SCRATCH, SCRATCH2);
        self.asm.jcc(Cond::Ne, answer);
        self.asm.mov_rm(SCRATCH, left.0, left.1 + PAYLOAD + 8);
        self.asm.mov_rm(SCRATCH2, right.0, right.1 + PAYLOAD + 8);
        self.asm.cmp_rr(SCRATCH, SCRATCH2);
        self.asm.bind(answer);
        let same = if op == BinaryOp::Is {
            Cond::E
        } else {
            Cond::Ne
        };
        self.asm.setcc(same, SCRATCH);
        self.asm.mov_m8i(left.0, left.1, TAG_BOOLEAN);
        self.asm.mov_m8r(left.0, left.1 + PAYLOAD, SCRATCH);
        self.depth = at + 1;
        self.store_len();
        self.asm.jmp(done);
        self.asm.bind(general);
        self.depth = at + 2;
    }

    /// Whether both operands of the comparison at `pc` are Integers by
    /// the checker's types, or small ones the analysis knows as such
    /// (decision AU45); on this tier every value lies boxed in its slot,
    /// and the sequence tests the tags before it compares.
    fn integers_compared(&self, pc: usize) -> bool {
        let Some(state) = &self.analysis.entry[pc] else {
            return false;
        };
        let n = state.len();
        let typed = |abs: Abs| matches!(abs, Abs::Int) || is_integer(self.program, abs);
        n >= 2 && typed(state[n - 2]) && typed(state[n - 1])
    }

    /// A comparison of the two operands at `at`, both typed Integer
    /// (decision AU45): when both are small, their payloads are compared
    /// with the op's condition, the Boolean written over the left operand
    /// and the code goes on at `done` with nothing to release, since a
    /// small Integer holds no count; a big Integer on either side falls
    /// through to the general comparison, which has the big Integers.
    fn compare_small_integers(&mut self, op: BinaryOp, at: usize, done: Label) {
        let general = self.asm.label();
        let left = self.operand(at);
        let right = self.operand(at + 1);
        for (base, disp) in [left, right] {
            self.asm.cmp_m8i(base, disp, TAG_INTEGER);
            self.asm.jcc(Cond::Ne, general);
            self.asm.cmp_m8i(base, disp + INT_TAG, INT_SMALL);
            self.asm.jcc(Cond::Ne, general);
        }
        self.asm.mov_rm(SCRATCH, left.0, left.1 + INT_PAYLOAD);
        self.asm.mov_rm(SCRATCH2, right.0, right.1 + INT_PAYLOAD);
        self.asm.cmp_rr(SCRATCH, SCRATCH2);
        let holds = match op {
            BinaryOp::Is => Cond::E,
            BinaryOp::IsNot => Cond::Ne,
            BinaryOp::IsLessThan => Cond::L,
            BinaryOp::IsAtMost => Cond::Le,
            BinaryOp::IsGreaterThan => Cond::G,
            _ => Cond::Ge,
        };
        self.asm.setcc(holds, SCRATCH);
        self.asm.mov_m8i(left.0, left.1, TAG_BOOLEAN);
        self.asm.mov_m8r(left.0, left.1 + PAYLOAD, SCRATCH);
        self.depth = at + 1;
        self.store_len();
        self.asm.jmp(done);
        self.asm.bind(general);
        self.depth = at + 2;
    }

    /// A call of a declared function (decision AU21): when the callee has
    /// machine code (the entry table names it) and the machine stack has
    /// room for one more native frame, the callee's frame is pushed in
    /// place, as `Vm::push_frame_in_place` pushes it (the arguments where
    /// they lie as its first slots, every other local `Nothing`, the
    /// record with the grant of decision Q1), and its code is entered; it
    /// leaves with a trampoline's status: the result where the arguments
    /// were (a failure lands on the handler, as the interpreter settles
    /// it), the frame handed to the interpreter (which runs it to its
    /// end), or an interrupt (the frame abandoned). Anything else goes
    /// through `rt_call`, which makes the callee's template on its way.
    fn call_direct(&mut self, function: usize, callee: usize, count: usize, pc: usize) {
        let program = self.program;
        let meta = &program.codes[callee];
        let callee_locals = meta.locals as usize;
        let narrows = meta
            .function
            .is_some_and(|function| !program.function_metas[function].needs.is_empty());
        let state = native_state_offset() as i32;
        let stack = stack_offset() as i32;
        let frames = frames_offset() as i32;
        let handlers = handlers_offset() as i32;
        let at = self.depth - count;
        // the callee's base and the height its locals reach, as offsets
        // from this frame's base
        let base_disp = (self.code.locals as usize + at) as i32;
        let needed_disp = base_disp + callee_locals as i32;
        let slow = self.asm.label();
        let done = self.asm.label();
        // the callee's entry, and room for one more native frame
        self.asm.mov_rm(SCRATCH2, VM, state + STATE_ENTRIES);
        self.asm.mov_rm(SCRATCH2, SCRATCH2, (callee * 8) as i32);
        self.asm.test_rr(SCRATCH2, SCRATCH2);
        self.asm.jcc(Cond::E, slow);
        self.asm
            .cmp_m64i(VM, state + STATE_DEPTH, DEPTH_LIMIT as i32);
        self.asm.jcc(Cond::Ae, slow);
        self.asm.mov_mr(Reg::Rsp, self.layout.spare, SCRATCH2);
        // room on the stack for the callee's locals
        self.asm.mov_rm(SCRATCH, VM, stack + 16);
        self.asm.lea(SCRATCH2, BASE, needed_disp);
        self.asm.cmp_rr(SCRATCH, SCRATCH2);
        let roomy = self.asm.label();
        self.asm.jcc(Cond::Ae, roomy);
        self.call("rt_room", &[Arg::Vm, Arg::Addr(BASE, needed_disp)]);
        self.asm.bind(roomy);
        // room for one more frame record
        self.asm.mov_rm(SCRATCH, VM, frames + 8);
        self.asm.mov_rm(SCRATCH2, VM, frames + 16);
        self.asm.cmp_rr(SCRATCH, SCRATCH2);
        let spacious = self.asm.label();
        self.asm.jcc(Cond::B, spacious);
        self.call("rt_grow_frames", &[Arg::Vm]);
        self.asm.bind(spacious);
        // the stack's length takes in the callee's locals, every one past
        // the arguments `Nothing` (a callee handed to the interpreter at
        // its start drops them as values)
        self.asm.lea(SCRATCH, BASE, needed_disp);
        self.asm.mov_mr(VM, stack + 8, SCRATCH);
        let fresh = callee_locals.saturating_sub(count);
        if fresh <= CLEARS_INLINE {
            for slot in count..callee_locals {
                let disp = ((base_disp as usize + slot) * SIZE) as i32;
                self.asm.mov_m8i(FRAME, disp, TAG_NOTHING);
            }
        } else {
            let first = ((base_disp as usize + count) * SIZE) as i32;
            self.call(
                "rt_clear_slots",
                &[Arg::Addr(FRAME, first), Arg::Imm(fresh as u64)],
            );
        }
        // the grant: the caller's, on top, unless the callee narrows it
        if narrows {
            self.call("rt_frame_grant", &[Arg::Vm, Arg::Imm(callee as u64)]);
            self.asm.mov_rr(SCRATCH3, Reg::Rax);
        } else {
            self.asm.mov_rm(SCRATCH, VM, frames + 8);
            self.asm.imul_rri(SCRATCH, SCRATCH, FRAME_SIZE as i32);
            self.asm.mov_rm(SCRATCH2, VM, frames);
            self.asm.add_rr(SCRATCH2, SCRATCH);
            self.asm
                .mov_rm32(SCRATCH3, SCRATCH2, FRAME_GRANT - FRAME_SIZE as i32);
        }
        // the record, in the registers no convention keeps
        self.asm.mov_rm(SCRATCH, VM, frames + 8);
        self.asm.imul_rri(Reg::R8, SCRATCH, FRAME_SIZE as i32);
        self.asm.mov_rm(Reg::R9, VM, frames);
        self.asm.add_rr(Reg::R8, Reg::R9);
        self.asm.mov_m64i(Reg::R8, FRAME_CODE, callee as i32);
        self.asm.mov_m64i(Reg::R8, FRAME_PC, 0);
        self.asm.lea(Reg::R9, BASE, base_disp);
        self.asm.mov_mr(Reg::R8, FRAME_BASE, Reg::R9);
        self.asm.mov_rm(Reg::R9, VM, handlers + 8);
        self.asm.mov_mr(Reg::R8, FRAME_HANDLER_BASE, Reg::R9);
        self.asm.mov_mr32(Reg::R8, FRAME_GRANT, SCRATCH3);
        self.asm.add_ri(SCRATCH, 1);
        self.asm.mov_mr(VM, frames + 8, SCRATCH);
        // the call, one native frame deeper, as `Vm::run_generated` makes it
        self.asm.add_m64i(VM, state + STATE_DEPTH, 1);
        let args = self.abi.args;
        self.asm.mov_rr(args[0], VM);
        self.asm.lea(args[1], BASE, base_disp);
        self.asm.zero(args[2]);
        self.asm.call_m(Reg::Rsp, self.layout.spare);
        self.asm.sub_m64i(VM, state + STATE_DEPTH, 1);
        self.reload_frame();
        self.depth = at + 1;
        // the frame handed to the interpreter or interrupted, out of line
        let handed = self.asm.label();
        self.asm.test_rr32(Reg::Rax, Reg::Rax);
        self.asm.jcc(Cond::Ne, handed);
        self.cold.push(Cold::Handed {
            label: handed,
            done,
            pc,
        });
        // the result where the arguments were; a failure lands
        let place = self.operand(at);
        let failed = self.asm.label();
        self.asm.cmp_m8i(place.0, place.1, TAG_FAILURE);
        self.asm.jcc(Cond::E, failed);
        self.cold.push(Cold::Failed { label: failed, pc });
        self.asm.bind(done);
        // without machine code, or at the depth limit, out of line
        self.cold.push(Cold::Call {
            label: slow,
            done,
            function,
            count,
            pc,
        });
    }

    /// A path the body jumps to rarely, after the body.
    fn emit_cold(&mut self, cold: Cold) {
        match cold {
            Cold::Call {
                label,
                done,
                function,
                count,
                pc,
            } => {
                self.asm.bind(label);
                self.call(
                    "rt_call",
                    &[
                        Arg::Vm,
                        Arg::Imm(function as u64),
                        Arg::Imm(count as u64),
                        Arg::Imm(pc as u64),
                    ],
                );
                self.check(pc);
                self.asm.jmp(done);
            }
            Cold::Handed { label, done, pc } => {
                self.asm.bind(label);
                let interrupted = self.asm.label();
                self.asm.cmp_r32i(Reg::Rax, INTERRUPT);
                self.asm.jcc(Cond::E, interrupted);
                // the interpreter runs the frame to its end
                self.call("rt_finish_frame", &[Arg::Vm]);
                self.check(pc);
                self.asm.jmp(done);
                self.asm.bind(interrupted);
                self.call("rt_abandon_frame", &[Arg::Vm]);
                self.asm.jmp(self.leave);
            }
            Cold::Failed { label, pc } => {
                self.asm.bind(label);
                self.asm.mov_ri32(Reg::Rax, FAILURE as u32);
                self.check(pc);
            }
            Cold::Retain { label, done, at } => {
                self.asm.bind(label);
                self.asm.cmp_m8i(at.0, at.1 + INT_TAG, INT_SMALL);
                self.asm.jcc(Cond::E, done);
                self.asm.mov_rm(SCRATCH2, at.0, at.1 + INT_PAYLOAD);
                self.asm.add_m64i(SCRATCH2, 0, 1);
                self.asm.jmp(done);
            }
            Cold::Release {
                integer,
                last,
                done,
                at,
            } => {
                let helper = self.asm.label();
                self.asm.bind(integer);
                self.asm.cmp_m8i(at.0, at.1 + INT_TAG, INT_SMALL);
                self.asm.jcc(Cond::E, done);
                self.asm.jmp(helper);
                self.asm.bind(last);
                self.asm.add_m64i(SCRATCH2, 0, 1);
                self.asm.bind(helper);
                self.call("rt_drop_at", &[Arg::Addr(at.0, at.1)]);
                self.asm.jmp(done);
            }
            Cold::Unit {
                label,
                done,
                ty,
                tag,
                pc,
            } => {
                self.asm.bind(label);
                self.call(
                    "rt_construct_variant",
                    &[
                        Arg::Vm,
                        Arg::Imm(ty as u64),
                        Arg::Imm(tag as u64),
                        Arg::Imm(0),
                        Arg::Imm(pc as u64),
                    ],
                );
                self.leave_on_interrupt();
                self.asm.jmp(done);
            }
            Cold::Field {
                label,
                done,
                slot,
                name,
                site,
                pc,
            } => {
                self.asm.bind(label);
                self.call(
                    "rt_load_field",
                    &[
                        Arg::Vm,
                        Arg::Base,
                        Arg::Imm(slot as u64),
                        Arg::Imm(self.code_id as u64),
                        Arg::Imm(name as u64),
                        Arg::Imm(site as u64),
                        Arg::Imm(pc as u64),
                    ],
                );
                self.leave_on_interrupt();
                self.asm.jmp(done);
            }
        }
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
            Op::ConstructVariant { ty, tag, fields: 0 }
                if unit_slot(self.program, *ty, *tag as usize).is_some() =>
            {
                // a variant without fields is the same value every time
                // (decision AG6): once built it is copied from its slot in
                // place, with one more reference counted on its block, the
                // first word of which is the count (decision AU24); the
                // helper builds it the first time, and cannot fail
                let slot = unit_slot(self.program, *ty, *tag as usize).expect("a slot");
                let unit = (slot * SIZE) as i32;
                let slow = self.asm.label();
                let done = self.asm.label();
                self.asm.mov_rm(SCRATCH2, VM, unit_variants_offset() as i32);
                self.asm.cmp_m8i(SCRATCH2, unit, TAG_NOTHING);
                self.asm.jcc(Cond::E, slow);
                let top = self.top();
                self.copy((SCRATCH2, unit), top);
                self.asm.mov_rm(SCRATCH2, top.0, top.1 + PAYLOAD);
                self.asm.add_m64i(SCRATCH2, 0, 1);
                self.asm.bind(done);
                self.cold.push(Cold::Unit {
                    label: slow,
                    done,
                    ty: *ty,
                    tag: *tag,
                    pc,
                });
                self.depth += 1;
                self.store_len();
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
                // the site's cache entry and the holder read in place
                // (decision AU21, as the Cranelift tier reads them, AR4): a
                // record or a variant of the cached type with the cached tag
                // gives its field with one more reference; anything else
                // goes through the helper, which fills the cache. A record
                // and a variant hold their type, their tag (a record's is
                // `usize::MAX`, as the cache has it) and their fields at the
                // same offsets (AT2)
                let holder = self.slot(*slot as usize);
                let entry = (*site as usize * SITE_SIZE) as i32;
                let slow = self.asm.label();
                let done = self.asm.label();
                self.asm.movzx_rm8(SCRATCH, holder.0, holder.1);
                self.asm.sub_ri(SCRATCH, TAG_RECORD as i32);
                self.asm.cmp_r32i(SCRATCH, 1);
                self.asm.jcc(Cond::A, slow);
                self.asm.mov_rm(SCRATCH2, holder.0, holder.1 + PAYLOAD);
                self.asm.mov_rm(SCRATCH3, VM, field_cache_offset() as i32);
                self.asm.mov_rm(SCRATCH, SCRATCH2, RECORD_TY);
                self.asm.mov_rm(Reg::Rcx, SCRATCH3, entry + SITE_TY);
                self.asm.cmp_rr(SCRATCH, Reg::Rcx);
                self.asm.jcc(Cond::Ne, slow);
                self.asm.mov_rm(SCRATCH, SCRATCH2, RECORD_TAG);
                self.asm.mov_rm(Reg::Rcx, SCRATCH3, entry + SITE_TAG);
                self.asm.cmp_rr(SCRATCH, Reg::Rcx);
                self.asm.jcc(Cond::Ne, slow);
                // a hit needs no range check: the index was cached from a
                // holder of the same type and tag, which has as many fields
                self.asm.mov_rm(Reg::Rcx, SCRATCH3, entry + SITE_INDEX);
                self.asm.imul_rri(Reg::Rcx, Reg::Rcx, SIZE as i32);
                self.asm.lea(SCRATCH, SCRATCH2, RECORD_FIELDS);
                self.asm.add_rr(Reg::Rcx, SCRATCH);
                let top = self.top();
                self.copy((Reg::Rcx, 0), top);
                self.retain(top);
                self.asm.bind(done);
                self.cold.push(Cold::Field {
                    label: slow,
                    done,
                    slot: *slot,
                    name: *name,
                    site: *site,
                    pc,
                });
                self.depth += 1;
                self.store_len();
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
                    Some(CallKind::Typed { kind, in_place }) if kind_agrees(kind, result) => {
                        let mask = self.masks[pc];
                        match in_place {
                            Some(InPlace::ListAt) if count == 2 => {
                                self.list_at_in_place(*function, mask, pc);
                            }
                            Some(InPlace::TextContains) if count == 2 => {
                                self.call_lean("rt_text_contains", *function, kind, mask, pc);
                            }
                            Some(InPlace::ListContains) if count == 2 => {
                                self.call_lean("rt_list_contains", *function, kind, mask, pc);
                            }
                            _ => self.call_typed(*function, count, kind, mask, pc),
                        }
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
                    _ if self
                        .program
                        .function_codes
                        .get(*function)
                        .copied()
                        .flatten()
                        .is_some() =>
                    {
                        let callee = self.program.function_codes[*function].expect("a code object");
                        self.call_direct(*function, callee, count, pc);
                    }
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
                    let done = self.asm.label();
                    if matches!(op, BinaryOp::Is | BinaryOp::IsNot) && self.texts_compared(pc) {
                        self.compare_small_texts(*op, at, done);
                    }
                    if self.integers_compared(pc) {
                        self.compare_small_integers(*op, at, done);
                    }
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
                    self.asm.bind(done);
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
