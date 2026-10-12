//! The translation of one code object to Cranelift IR (decision AG1).
//! Every op becomes either a few machine instructions, when its operands
//! are unboxed (small Integers, Booleans and Floats in registers, and the
//! bounds of a range), or a call to the runtime helper that does what the
//! interpreter's arm does on the VM's stack. The operand stack is static
//! (`infer.rs`): an unboxed operand at depth `i` is a Cranelift variable,
//! so that the frontend's SSA construction joins the paths; a boxed one is
//! a `Value` on the VM's stack, where the helpers find it. Where a typed
//! assumption may not hold at run time (a parameter that is a big or a
//! guarded Integer, a call or a field whose declared Integer is not small,
//! an addition that overflows), the generated code hands the frame to the
//! interpreter at that very op (`rt_deopt`), with every value boxed where
//! the interpreter expects it.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    types, AbiParam, Block, InstBuilder, MemFlagsData, SigRef, Signature, StackSlot, StackSlotData,
    StackSlotKind, Type, UserFuncName, Value as IrValue,
};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use renyi_syntax::ast::BinaryOp;

use crate::bytecode::{Code, CodeKind, Op};
use crate::compile::Program;
use crate::extension::TypedKind;
use crate::integer::Int;
use crate::native::infer::{
    abs_of_binary, abs_of_constant, abs_of_field_at, abs_of_params, abs_of_result, abs_of_slot,
    analyse, boxed_depth, handler_only, is_integer, is_text, noted_kind, otherwise_end,
    owned_slots, pair_fused, pair_result, static_field, sum_compared, update_by_index, Abs,
    Analysis, SlotKind,
};
use crate::native::runtime::{
    binary_code, fold_code, CONTINUE, DEOPT, D_BOXED, D_DEOPT, D_FAILURE, D_INTERRUPT, D_RETURNED,
    D_STAY, FAILURE, LEFT, RETURNED,
};
use crate::native::{DeoptPoint, DEPTH_LIMIT};
use crate::value::layout::{
    COUNTED_OR_INTEGER, INT_PAYLOAD, INT_SMALL, INT_TAG, ITER_ITEMS, ITER_LEN, ITER_POSITION,
    NATIVE_ITERATOR, NATIVE_PAYLOAD, NATIVE_TAG, PAYLOAD, RC_VALUE, RECORD_FIELDS, RECORD_TAG,
    RECORD_TY, SIZE, TAG_BOOLEAN, TAG_FAILURE, TAG_FLOAT, TAG_GUARDED, TAG_INTEGER, TAG_LIST,
    TAG_NATIVE, TAG_NOTHING, TAG_RECORD, TAG_SMALL_TEXT, TAG_VARIANT,
};
use crate::value::Value;
use crate::vm::{
    plain_record_type, plain_variant_type, unit_slot, CallKind, Entry, FieldSite, Frame,
    GlobalSlot, NativeState, ReadInPlace, Vm,
};

/// Where the VM keeps its stack (decision AR4): the generated code reads
/// the pinned vector's pointer, length and capacity there.
pub(crate) fn stack_offset() -> i64 {
    std::mem::offset_of!(Vm<'static>, stack) as i64
}

/// Where the VM keeps whether provenance guards are on (decision P3): a
/// pair goes back as its record under them (decision AW9).
pub(crate) fn guarding_offset() -> i64 {
    std::mem::offset_of!(Vm<'static>, guarding) as i64
}

/// Where the VM keeps its frames and its handlers, and the layout of a
/// frame record (decision AR4): a direct call pushes the callee's frame
/// in place and a return pops it.
pub(crate) fn frames_offset() -> i64 {
    std::mem::offset_of!(Vm<'static>, frames) as i64
}

pub(crate) fn handlers_offset() -> i64 {
    std::mem::offset_of!(Vm<'static>, handlers) as i64
}

pub(crate) const FRAME_SIZE: usize = std::mem::size_of::<Frame>();
pub(crate) const FRAME_CODE: i32 = std::mem::offset_of!(Frame, code) as i32;
pub(crate) const FRAME_PC: i32 = std::mem::offset_of!(Frame, pc) as i32;
pub(crate) const FRAME_BASE: i32 = std::mem::offset_of!(Frame, base) as i32;
pub(crate) const FRAME_HANDLER_BASE: i32 = std::mem::offset_of!(Frame, handler_base) as i32;
pub(crate) const FRAME_GRANT: i32 = std::mem::offset_of!(Frame, grant) as i32;

/// Where the VM keeps the cache of the field sites, and an entry's layout
/// (decision AR4): a field load reads the entry and the holder in place.
pub(crate) fn field_cache_offset() -> i64 {
    std::mem::offset_of!(Vm<'static>, field_cache) as i64
}

/// Where the VM keeps the variants without fields once built: a vector
/// with a fixed layout, whose pointer the generated code reads to copy
/// one in place (decision AU24).
pub(crate) fn unit_variants_offset() -> i64 {
    std::mem::offset_of!(Vm<'static>, unit_variants) as i64
}

pub(crate) const SITE_SIZE: usize = std::mem::size_of::<FieldSite>();
pub(crate) const SITE_TY: i32 = std::mem::offset_of!(FieldSite, ty) as i32;
pub(crate) const SITE_TAG: i32 = std::mem::offset_of!(FieldSite, tag) as i32;
pub(crate) const SITE_INDEX: i32 = std::mem::offset_of!(FieldSite, index) as i32;

/// How many values a return releases one by one before it calls the
/// helper that cuts the stack instead: the slots that may hold a counted
/// block there (decision AW4) and the boxed operands above the locals.
const RELEASES_INLINE: usize = 8;

/// The helpers that cannot move the stack's vector (they grow nothing
/// and run no program code), after which the frame pointer needs no
/// reload (decision AT6; a helper not listed is reloaded after, which is
/// always sound). The list is kept short on purpose: a helper that runs
/// program code, as most do, may grow the stack through the frames it
/// pushes.
pub(crate) const STACK_SAFE: &[&str] = &[
    "rt_construct_plain",
    "rt_construct_variant_plain",
    "rt_clear_slots",
    "rt_record_copy",
    "rt_pair_fits",
    "rt_pair_resume",
    "rt_drop_at",
    "rt_frame_grant",
    "rt_grow_frames",
    "rt_take_int",
    "rt_take_float",
    "rt_take_bool",
    "rt_resume_int",
    "rt_resume_bool",
    "rt_resume_float",
    "rt_resume_range",
    "rt_is_nothing",
    "rt_is_failure",
    "rt_top_is_absent",
    "rt_top_is_failure",
];

/// Where the VM keeps what the generated code reaches by pointer
/// (decision AS1): the count of generated frames, the count of calls,
/// the table of compiled bodies, the helpers and the constants. The code
/// holds no address of its own, so an image of it loads anywhere.
pub(crate) fn native_state_offset() -> i64 {
    std::mem::offset_of!(Vm<'static>, native_state) as i64
}

pub(crate) const STATE_DEPTH: i32 = std::mem::offset_of!(NativeState, depth) as i32;
const STATE_DIRECT: i32 = std::mem::offset_of!(NativeState, direct_table) as i32;
pub(crate) const STATE_HELPERS: i32 = std::mem::offset_of!(NativeState, helpers) as i32;
pub(crate) const STATE_CONSTANTS: i32 = std::mem::offset_of!(NativeState, constants) as i32;
pub(crate) const STATE_HOTNESS: i32 = std::mem::offset_of!(NativeState, hotness) as i32;
pub(crate) const STATE_ENTRIES: i32 = std::mem::offset_of!(NativeState, entries) as i32;
pub(crate) const STATE_LIST_ITEMS: i32 = std::mem::offset_of!(NativeState, list_items) as i32;
pub(crate) const STATE_LIST_LEN: i32 = std::mem::offset_of!(NativeState, list_len) as i32;
pub(crate) const STATE_GLOBALS: i32 = std::mem::offset_of!(NativeState, globals) as i32;
pub(crate) const STATE_IN_PLACE: i32 = std::mem::offset_of!(NativeState, in_place) as i32;
/// A module constant's slot in the JIT's table (decision AU53): its size,
/// the flag that says the VM has built it and the value.
pub(crate) const GLOBAL_SIZE: usize = std::mem::size_of::<GlobalSlot>();
pub(crate) const GLOBAL_READY: i32 = std::mem::offset_of!(GlobalSlot, ready) as i32;
pub(crate) const GLOBAL_VALUE: i32 = std::mem::offset_of!(GlobalSlot, value) as i32;

/// The position of a helper in `SIGNATURES`, which is its index in the
/// VM's table of helpers.
pub(crate) fn helper_index(name: &str) -> usize {
    SIGNATURES
        .iter()
        .position(|(known, _, _)| *known == name)
        .unwrap_or_else(|| panic!("no helper {name}"))
}

/// The signature of every helper: its parameters and its result, one
/// letter each (`p` a pointer, `z` a `usize`, `w` a `u32`, `q` an `i64`,
/// `b` an `i8`, `u` a `u8`, `f` an `f64`; `v` no result, `i` an `i32`).
pub const SIGNATURES: &[(&str, &str, char)] = &[
    ("rt_push_const", "pzw", 'v'),
    ("rt_push_nothing", "p", 'v'),
    ("rt_insert_int", "pzq", 'v'),
    ("rt_insert_bool", "pzb", 'v'),
    ("rt_insert_float", "pzf", 'v'),
    ("rt_insert_range", "pzqqq", 'v'),
    ("rt_global", "pww", 'i'),
    ("rt_load", "pzw", 'v'),
    ("rt_load_move", "pzw", 'v'),
    ("rt_store", "pzw", 'v'),
    ("rt_store_int", "pzwq", 'v'),
    ("rt_store_bool", "pzwb", 'v'),
    ("rt_store_float", "pzwf", 'v'),
    ("rt_pop", "p", 'v'),
    ("rt_drop_at", "p", 'v'),
    ("rt_room", "pz", 'v'),
    ("rt_grow_frames", "p", 'v'),
    ("rt_frame_grant", "pz", 'i'),
    ("rt_dup", "p", 'v'),
    ("rt_truncate", "pzww", 'v'),
    ("rt_retain_params", "pzq", 'v'),
    ("rt_take_int", "pp", 'b'),
    ("rt_take_float", "pp", 'b'),
    ("rt_take_bool", "pwp", 'i'),
    ("rt_param_int", "pzwp", 'b'),
    ("rt_param_bool", "pzwp", 'b'),
    ("rt_param_float", "pzwp", 'b'),
    ("rt_resume_int", "pzwp", 'b'),
    ("rt_resume_bool", "pzwp", 'b'),
    ("rt_resume_float", "pzwp", 'b'),
    ("rt_resume_range", "pzwp", 'b'),
    ("rt_make_list", "pw", 'v'),
    ("rt_make_map", "pw", 'v'),
    ("rt_make_pair", "p", 'v'),
    ("rt_make_range", "pbw", 'i'),
    ("rt_construct", "pzww", 'i'),
    ("rt_construct_variant", "pzwww", 'i'),
    ("rt_construct_plain", "pzw", 'v'),
    ("rt_construct_variant_plain", "pzww", 'v'),
    ("rt_field", "pzwww", 'i'),
    ("rt_load_field", "pzwzwww", 'i'),
    ("rt_with", "pww", 'i'),
    ("rt_with_slot", "pzwww", 'i'),
    ("rt_with_by_index", "pzqw", 'i'),
    ("rt_with_slot_by_index", "pzwzqw", 'i'),
    ("rt_record_copy", "p", 'p'),
    ("rt_pair_record", "pzz", 'v'),
    ("rt_split_pair", "pzw", 'i'),
    ("rt_pair_fits", "pzwz", 'b'),
    ("rt_pair_resume", "pzwwzb", 'v'),
    ("rt_take_field", "pzwzwww", 'i'),
    ("rt_not", "pw", 'i'),
    ("rt_binary", "puw", 'i'),
    ("rt_compare", "puwwp", 'i'),
    ("rt_float_binary", "puffwp", 'i'),
    ("rt_int_power", "pqqwp", 'i'),
    ("rt_to_text", "pw", 'i'),
    ("rt_concat", "pww", 'i'),
    ("rt_is_variant", "pwp", 'v'),
    ("rt_is_nothing", "pp", 'v'),
    ("rt_is_failure", "pp", 'v'),
    ("rt_is_type", "pzp", 'v'),
    ("rt_unpack", "pww", 'i'),
    ("rt_unwrap_failure", "p", 'v'),
    ("rt_call", "pzww", 'i'),
    ("rt_call_pure", "pzww", 'i'),
    ("rt_call_typed", "pzwwwp", 'i'),
    ("rt_call_ability", "pzwww", 'i'),
    ("rt_call_value", "pww", 'i'),
    ("rt_result_type", "pw", 'v'),
    ("rt_return", "p", 'v'),
    ("rt_direct_after", "pw", 'i'),
    ("rt_left_status", "p", 'i'),
    ("rt_fail", "pw", 'i'),
    ("rt_crash", "pw", 'i'),
    ("rt_crash_text", "pww", 'i'),
    ("rt_check_failed", "pzw", 'i'),
    ("rt_unhandled", "pw", 'i'),
    ("rt_settle_handler", "pzww", 'v'),
    ("rt_iter_init", "pzww", 'i'),
    ("rt_iter_next", "pzw", 'b'),
    ("rt_list_push", "pw", 'i'),
    ("rt_group_insert", "pw", 'i'),
    ("rt_group_fold", "puw", 'i'),
    ("rt_sort_by_key", "pbw", 'i'),
    ("rt_deadline", "pzww", 'i'),
    ("rt_check_deadline", "pzw", 'i'),
    ("rt_deopt", "pzzwp", 'v'),
    ("rt_top_is_absent", "p", 'b'),
    ("rt_top_is_failure", "p", 'b'),
    ("rt_osr", "pzzw", 'i'),
    ("rt_finish_frame", "p", 'i'),
    ("rt_abandon_frame", "p", 'i'),
    ("rt_promote", "pz", 'v'),
    ("rt_clear_slots", "pz", 'v'),
];

/// The Cranelift signature of a helper.
pub fn signature(isa: &dyn TargetIsa, params: &str, result: char) -> Signature {
    let pointer = isa.pointer_type();
    let mut sig = Signature::new(isa.default_call_conv());
    for letter in params.chars() {
        let ty = match letter {
            'p' | 'z' | 'q' => pointer,
            'w' => types::I32,
            'b' | 'u' => types::I8,
            'f' => types::F64,
            other => panic!("no parameter letter {other}"),
        };
        sig.params.push(AbiParam::new(ty));
    }
    match result {
        'v' => {}
        'i' => sig.returns.push(AbiParam::new(types::I32)),
        'b' => sig.returns.push(AbiParam::new(types::I8)),
        'p' | 'z' => sig.returns.push(AbiParam::new(pointer)),
        other => panic!("no result letter {other}"),
    }
    sig
}

/// The signature of a code object's body (`renyi_direct_*`, decision AR3):
/// the VM, the frame's base, the pc to enter at, whether a direct call
/// entered (a typed result then goes back in a register; a trampoline's
/// caller wants it on the stack), the bits of the parameters the caller
/// passed borrowed (decision AW5; none from a trampoline), the frame's
/// grant by its index in `Vm::grants` (decision AW6: a direct caller
/// passes its own, or the helper's answer when the callee narrows it; a
/// trampoline reads the record's), then the typed parameters in slot
/// order; the status (`runtime::D_*`) and the bits of a typed result
/// come back.
pub fn direct_signature(isa: &dyn TargetIsa, kinds: &[Abs]) -> Signature {
    let pointer = isa.pointer_type();
    let mut sig = Signature::new(isa.default_call_conv());
    sig.params.push(AbiParam::new(pointer));
    sig.params.push(AbiParam::new(pointer));
    sig.params.push(AbiParam::new(types::I32));
    sig.params.push(AbiParam::new(types::I8));
    sig.params.push(AbiParam::new(types::I64));
    sig.params.push(AbiParam::new(types::I32));
    for kind in kinds {
        match kind {
            Abs::Int => sig.params.push(AbiParam::new(types::I64)),
            Abs::Bool => sig.params.push(AbiParam::new(types::I8)),
            Abs::Float => sig.params.push(AbiParam::new(types::F64)),
            _ => {}
        }
    }
    sig.returns.push(AbiParam::new(types::I32));
    sig.returns.push(AbiParam::new(types::I64));
    sig
}

/// The signature of a generated function: the VM, the frame's base and
/// the pc to enter at (`0`, or a loop header), the status it leaves with.
pub fn entry_signature(isa: &dyn TargetIsa) -> Signature {
    let pointer = isa.pointer_type();
    let mut sig = Signature::new(isa.default_call_conv());
    sig.params.push(AbiParam::new(pointer));
    sig.params.push(AbiParam::new(pointer));
    sig.params.push(AbiParam::new(types::I32));
    sig.returns.push(AbiParam::new(types::I32));
    sig
}

/// What compiling cost (for `RENYI_NATIVE_REPORT`, a development aid):
/// the time in the analysis, in building the IR and in Cranelift, and
/// the size of the IR.
#[derive(Default, Clone, Copy, Debug)]
pub struct Stats {
    pub analysis: Duration,
    pub ir: Duration,
    pub cranelift: Duration,
    pub instructions: usize,
    pub blocks: usize,
}

impl Stats {
    pub fn add(&mut self, other: &Stats) {
        self.analysis += other.analysis;
        self.ir += other.ir;
        self.cranelift += other.cranelift;
        self.instructions += other.instructions;
        self.blocks += other.blocks;
    }
}

/// What compiling a code object gives: the function, its deopt points,
/// the loop headers it can be entered at, the statistics.
/// The trampoline, the body, the deopt points, the loop headers that have
/// an entry, and the counts.
/// A compiled code object (decision AS1): the machine code of its body
/// and of its trampoline, which hold no address, with the deopt points
/// and the loop headers the VM keeps beside them.
pub struct Compiled {
    pub body: Vec<u8>,
    pub trampoline: Vec<u8>,
    pub deopts: Vec<DeoptPoint>,
    pub headers: Vec<u32>,
    /// The callees expanded into the body (decision AU50), filled in by
    /// `compile_with_inlining`.
    pub regions: Vec<crate::native::inline::Region>,
    pub stats: Stats,
}

/// A shared failure block: the innermost handler, the operand stack
/// below its depth, the block.
type Landing = (Option<(u32, usize)>, Vec<Abs>, Block);

/// Why a code object was not compiled.
#[derive(Debug)]
pub enum Skipped {
    /// The analysis could not settle its stack.
    Analysis(crate::native::infer::Rejection),
    /// Cranelift refused the function.
    Codegen(String),
}

/// The variables holding an operand at one depth, by representation.
#[derive(Default, Clone, Copy)]
struct PositionVars {
    int: Option<Variable>,
    bool: Option<Variable>,
    float: Option<Variable>,
    range: Option<[Variable; 3]>,
}

/// The variables of a slot the generated code keeps in registers.
#[derive(Clone, Copy)]
enum SlotVars {
    None,
    One(Variable),
    /// A range iterator: the position, the end and the step.
    Three([Variable; 3]),
}

struct Gen<'a, 'b> {
    program: &'a Program,
    code_id: usize,
    /// The module constants evaluated to a small Integer when the
    /// compilation was asked for (decision AU53): a read of one is the
    /// literal, since a constant never changes once built.
    globals: &'a [Option<i64>],
    /// The code object's own locals: the slots past them belong to the
    /// callees expanded into it (decision AU50), which a frame the
    /// interpreter hands over does not have.
    original_locals: usize,
    code: &'a Code,
    analysis: &'a Analysis,
    /// Per function, how a call to it is made (decision AU1).
    calls: &'a [CallKind],
    /// Per op, whether a `Load` or a `Const` pushes an operand a typed
    /// call borrows: pushed without a reference of its own, consumed by
    /// the call without a release (decision AU1).
    borrowed: Vec<bool>,
    /// Per `Call` op, the bits of the arguments it borrows.
    masks: Vec<u32>,
    /// Per op, the slots that may hold a counted block on entry to it
    /// (decision AW4): what a leave at the op releases.
    owned: Vec<Vec<bool>>,
    b: FunctionBuilder<'b>,
    pointer: Type,
    isa: &'a dyn TargetIsa,
    /// The VM's table of helpers, read once at the entry: every helper is
    /// called through it (decision AS1).
    helper_table: IrValue,
    /// The VM's table of in-place entries (decision AW11), read once as
    /// the helpers are, and the shims' signature once imported.
    in_place_table: IrValue,
    in_place_sig: Option<SigRef>,
    /// The signatures imported so far, by helper name.
    helper_sigs: HashMap<&'static str, SigRef>,
    vm: IrValue,
    base: IrValue,
    /// The pc to enter at: `0`, or one of `headers`.
    pc_param: IrValue,
    /// Whether a direct call entered the frame (an `i8`): a typed result
    /// then leaves in a register, else on the stack.
    direct: IrValue,
    /// The bits of the parameters the caller passed borrowed (decision
    /// AW5): read-only parameters whose operands a direct call pushed
    /// without a reference of their own; `0` from a trampoline. The leave
    /// skips a slot so marked, a `LoadMove` of it copies with a reference
    /// instead of moving, and a path that treats the frame as owned (a
    /// hand-back, a leave through a helper, an interrupt, the leave
    /// through `rt_truncate`) gives the slots so marked a reference first
    /// (`rt_retain_params`).
    borrowed_params: IrValue,
    /// The bits of the parameters a direct call may pass borrowed
    /// (`accepted_params`), the bits every caller reads.
    flagged: u64,
    /// The frame's grant (decision Q1), by its index in `Vm::grants`, as
    /// the signature passed it (decision AW6): what a direct call writes
    /// into the record of a callee that does not narrow it.
    grant: IrValue,
    /// Whether some `Call` of the body is made directly (`direct_call`):
    /// the prologue then makes room in the frames for the record such a
    /// call pushes (decision AW6).
    calls_directly: bool,
    /// The typed parameters, each with its slot: in the registers the
    /// signature put them in.
    typed_params: Vec<(usize, IrValue)>,
    /// The frame's base times the width of a value: the offset of its
    /// first slot in the stack's items (decision AR4).
    base24: IrValue,
    /// The frame pointer: the stack's pointer plus `base24`, kept in a
    /// register between helper calls (`reload_frame`), which alone can
    /// move the stack (decision AT4).
    frame_var: Variable,
    /// The deepest operand stack of the body, by the analysis.
    max_depth: usize,
    /// The kind the declared result is passed back as.
    result_kind: Abs,
    /// The type of the record of two fields the body returns in two cells
    /// to a direct caller (decision AW9), when its declared result is one.
    pair_result: Option<u32>,
    /// Whether a record of two fields is held as a pair in this body
    /// (decision AW9): not in an expanded one, whose regions' slots lie
    /// where the pairs' cells go.
    pairs: bool,
    /// Per slot, the cell of the frame past the code object's locals that
    /// holds the second field of a pair slot (decision AW9).
    pair_cells: Vec<Option<usize>>,
    /// The frame's locals: the code object's and a cell per pair slot.
    frame_locals: usize,
    /// The loop headers the interpreter may hand a frame over at: the
    /// targets of jumps backwards whose operand stack holds nothing in
    /// registers.
    headers: Vec<usize>,
    positions: Vec<PositionVars>,
    slots: Vec<SlotVars>,
    blocks: HashMap<usize, Block>,
    exit: Block,
    /// The blocks a failed status goes to (`bad_block`), one per innermost
    /// handler and operand stack below it, filled at the end.
    landings: Vec<Landing>,
    out_slot: StackSlot,
    deopt_slot: StackSlot,
    deopts: Vec<DeoptPoint>,
    /// The blocks that hand the frame to the interpreter (`deopt_block`),
    /// one per pc and operand state, filled at the end.
    deopt_blocks: Vec<((usize, Vec<Abs>), Block)>,
    state: Vec<Abs>,
    /// Whether the current block has been ended by a terminator.
    terminated: bool,
    /// The op being translated.
    current_pc: usize,
}

/// Whether a typed entry's kind is the representation the declared result
/// type gives a call's answer (decision AU1): the generated code trusts
/// the entry only where the two agree.
pub(crate) fn kind_agrees(kind: TypedKind, result: Abs) -> bool {
    matches!(
        (kind, result),
        (TypedKind::Bool, Abs::Bool)
            | (TypedKind::Int, Abs::Int)
            | (TypedKind::Float, Abs::Float)
            | (TypedKind::Value, Abs::Boxed(_))
    )
}

/// Whether the operator is a comparison: its answer is a Boolean whatever
/// the operands.
pub(crate) fn is_comparison(op: BinaryOp) -> bool {
    use BinaryOp::*;
    matches!(
        op,
        Is | IsNot | IsLessThan | IsAtMost | IsGreaterThan | IsAtLeast
    )
}

/// The condition of a comparison on two signed integers.
fn comparison_cc(op: BinaryOp) -> IntCC {
    use BinaryOp::*;
    match op {
        Is => IntCC::Equal,
        IsNot => IntCC::NotEqual,
        IsLessThan => IntCC::SignedLessThan,
        IsAtMost => IntCC::SignedLessThanOrEqual,
        IsGreaterThan => IntCC::SignedGreaterThan,
        _ => IntCC::SignedGreaterThanOrEqual,
    }
}

/// The direct call the `Call` of `function` with `count` arguments makes
/// at a state (decisions AR3 and AW3): the callee's code object and its
/// parameters' kinds, when the callee has a code object, the count
/// agrees, and every typed parameter has a typed operand there, or a
/// boxed one the checker noted as the parameter's type, which the site
/// takes from the stack; `None` for the call through the VM's path.
pub(crate) fn direct_call(
    program: &Program,
    state: &[Abs],
    function: usize,
    count: usize,
) -> Option<(usize, Vec<Abs>)> {
    let callee = program.function_codes.get(function).copied().flatten()?;
    let params = program.codes[callee].params as usize;
    let kinds = abs_of_params(program, function, params);
    if kinds.len() != count || count > 64 || state.len() < count {
        return None;
    }
    let first = state.len() - count;
    let fits = kinds.iter().enumerate().all(|(index, kind)| {
        let operand = state[first + index];
        kind.is_boxed() || operand == *kind || noted_kind(program, operand) == *kind
    });
    fits.then_some((callee, kinds))
}

/// Whether the generated code calls `function` through a typed entry
/// (decision AU1), a primitive carrying one whose kind agrees with the
/// function's result, or through an in-place entry (decision AW11),
/// which reads or takes its arguments where they lie as a typed entry
/// does: the operands of either call are lent, not retained.
pub(crate) fn typed_entry(program: &Program, calls: &[CallKind], function: usize) -> bool {
    match calls.get(function) {
        Some(CallKind::Typed { kind, .. }) => kind_agrees(*kind, abs_of_result(program, function)),
        Some(CallKind::Pure { entry }) => entry.is_some(),
        _ => false,
    }
}

/// Whether the generated code calls `function` through an in-place entry
/// that takes its arguments (decision AW11): a moved operand of such a
/// call is the call's to consume, not lent (`Entry::Takes`).
pub(crate) fn taking_entry(calls: &[CallKind], function: usize) -> bool {
    matches!(
        calls.get(function),
        Some(CallKind::Typed {
            entry: Some(Entry::Takes),
            ..
        }) | Some(CallKind::Pure {
            entry: Some(Entry::Takes)
        })
    )
}

/// The kind the generated code takes an in-place entry's answer in
/// (decision AW11): the function's result, a scalar read from the boxed
/// answer or the boxed answer itself.
pub(crate) fn entry_kind(program: &Program, function: usize) -> TypedKind {
    match abs_of_result(program, function) {
        Abs::Int => TypedKind::Int,
        Abs::Bool => TypedKind::Bool,
        Abs::Float => TypedKind::Float,
        _ => TypedKind::Value,
    }
}

/// The pushes a call or a comparison at `pc` takes from the ops just
/// before it, walking back over the `Load`s, `LoadMove`s and `Const`s
/// and, before a `Binary`, `Nothing`, which push one operand each, up to
/// `count`: each with the index of the argument it is (the first 0). The
/// walk stops at any other op: what was pushed before it is whatever the
/// stack holds.
fn simple_pushes(code: &Code, pc: usize, count: usize) -> Vec<(usize, usize)> {
    let binary = matches!(code.ops[pc], Op::Binary(_));
    let mut pushes = Vec::with_capacity(count.min(pc));
    for back in 1..=count.min(pc) {
        let at = pc - back;
        let simple = match &code.ops[at] {
            Op::Load(_) | Op::LoadMove(_) | Op::Const(_) => true,
            Op::Nothing => binary,
            _ => false,
        };
        if !simple {
            break;
        }
        pushes.push((at, count - back));
    }
    pushes
}

/// The parameters of a code object a direct call may pass borrowed
/// (decision AW5), as bits (the first parameter bit 0): among the first
/// thirty-two, boxed, read-only (`Code::read_only`), and never moved into
/// an owned sink, that is, every `LoadMove` of the slot is an argument of
/// a call through a typed entry or an operand of a comparison, which read
/// it where it lies (decision AU17), so that the callee's code never
/// needs a reference of its own to the value. The bits come from the
/// code object's ops and the program's tables alone, so that a caller
/// compiled on another thread, or for an image, reads the same bits the
/// callee's compilation does, which the protocol requires: the callee's
/// leave releases every parameter not among them and, for one among
/// them, asks the call. A moved load this rule counts as read where it
/// lies but the analysis finds owned (a comparison with a typed operand)
/// is moved conditionally, the fallback of the callee's `LoadMove`.
pub(crate) fn accepted_params(program: &Program, calls: &[CallKind], code_id: usize) -> u32 {
    let code = &program.codes[code_id];
    let (Some(function), CodeKind::Function) = (code.function, code.kind) else {
        return 0;
    };
    let params = (code.params as usize).min(32);
    // per op, whether a push there is taken by a call or a comparison
    // that reads it where it lies
    let mut lent = vec![false; code.ops.len()];
    for (pc, op) in code.ops.iter().enumerate() {
        let (count, takes) = match op {
            Op::Call { function, args } => {
                let count = *args as usize;
                if count == 0 || count > 32 || !typed_entry(program, calls, *function) {
                    continue;
                }
                (count, taking_entry(calls, *function))
            }
            Op::Binary(op) if is_comparison(*op) => (2, false),
            _ => continue,
        };
        for (at, _) in simple_pushes(code, pc, count) {
            // a moved operand of an entry that takes is the call's to
            // consume, not lent (decision AW11)
            if takes && matches!(code.ops[at], Op::LoadMove(_)) {
                continue;
            }
            lent[at] = true;
        }
    }
    // per parameter, whether a move of it goes anywhere else
    let mut moved = vec![false; params];
    for (pc, op) in code.ops.iter().enumerate() {
        if let Op::LoadMove(slot) = op {
            if let Some(flag) = moved.get_mut(*slot as usize) {
                *flag |= !lent[pc];
            }
        }
    }
    let kinds = abs_of_params(program, function, code.params as usize);
    (0..params)
        .filter(|slot| {
            kinds[*slot].is_boxed()
                && code.read_only.get(*slot).copied().unwrap_or(false)
                && !moved[*slot]
        })
        .fold(0, |mask, slot| mask | 1 << slot)
}

/// Whether some `Call` of the code object is made directly
/// (`direct_call`) at the state the analysis found there.
fn has_direct_call(program: &Program, code: &Code, analysis: &Analysis) -> bool {
    code.ops.iter().enumerate().any(|(pc, op)| match op {
        Op::Call { function, args } => matches!(
            analysis.entry.get(pc).and_then(|state| state.as_deref()),
            Some(state) if direct_call(program, state, *function, *args as usize).is_some()
        ),
        _ => false,
    })
}

/// The operands a typed call, a comparison or a direct call borrows
/// (decisions AU1, AU13 and AW5): for a `Call` of a primitive with a
/// typed entry, the arguments pushed by the `Load`s and `LoadMove`s of
/// boxed slots and the boxed `Const`s immediately before it, which are
/// its last arguments in order, each pushed without a reference of its
/// own; for a `Binary` comparison on boxed operands, the same kinds of
/// pushes and `Nothing` just before it; for a `Call` made directly
/// (`direct_call`), the plain `Load`s and the boxed `Const`s among its
/// last arguments whose parameter the callee accepts borrowed
/// (`accepted_params`), which the callee's frame then holds without a
/// reference of its own and its leave skips, when `direct_calls` says
/// the tier makes such calls (the template tier passes every parameter
/// owned and says no); a moved load, the caller's last use of the slot,
/// is not borrowed by a direct call but moved into the callee as before,
/// which then holds the only reference and may update the value in place
/// (borrowed, the caller's slot kept the value, the callee's copy had two
/// references, and every append or update copied the list or the record
/// and the caller's leave freed the old one: the first build of decision
/// AW5 measured 4.5% more instructions on the check row for it). A push
/// of a typed slot or a typed constant among them is stepped over (it is
/// one operand, in a register). Per op, whether it pushes a borrowed
/// operand; per call or comparison, the bits of the operands so pushed
/// (the first argument, or the left operand, bit 0). For a typed call or
/// a comparison, a moved load is borrowed like a plain one (decision
/// AU17): the entries read their arguments where they lie and update
/// nothing in place, so the slot keeps its value, which nothing reads
/// again, and the helper counts the copy out; moving it would cost a
/// release after the call where the plain load cost nothing.
pub(crate) fn borrowed_operands(
    program: &Program,
    code: &Code,
    analysis: &Analysis,
    calls: &[CallKind],
    direct_calls: bool,
) -> (Vec<bool>, Vec<u32>) {
    let mut borrowed = vec![false; code.ops.len()];
    let mut masks = vec![0u32; code.ops.len()];
    for (pc, op) in code.ops.iter().enumerate() {
        // per argument, whether a boxed push of it is borrowed; and whether
        // a moved load among them is (not for a direct call)
        let mut moves_borrowed = true;
        let takes: Vec<bool> = match op {
            Op::Call { function, args } => {
                let count = *args as usize;
                if count == 0 || count > 32 {
                    continue;
                }
                let state = analysis.entry.get(pc).and_then(|state| state.as_deref());
                let direct = match state {
                    Some(state) if direct_calls => direct_call(program, state, *function, count),
                    _ => None,
                };
                match direct {
                    Some((callee, _)) => {
                        let accepted = accepted_params(program, calls, callee);
                        moves_borrowed = false;
                        (0..count)
                            .map(|index| accepted & (1 << index) != 0)
                            .collect()
                    }
                    None => {
                        if !typed_entry(program, calls, *function) {
                            continue;
                        }
                        // an entry that takes consumes a moved operand
                        // (decision AW11)
                        if taking_entry(calls, *function) {
                            moves_borrowed = false;
                        }
                        vec![true; count]
                    }
                }
            }
            Op::Binary(op) if is_comparison(*op) => {
                // both operands boxed by the analysis: a register operand
                // takes the typed paths
                let boxed = match analysis.entry.get(pc) {
                    Some(Some(state)) => {
                        state.len() >= 2
                            && state[state.len() - 2..].iter().all(|abs| abs.is_boxed())
                    }
                    _ => false,
                };
                if !boxed {
                    continue;
                }
                vec![true; 2]
            }
            _ => continue,
        };
        let count = takes.len();
        let mut mask = 0u32;
        for (at, index) in simple_pushes(code, pc, count) {
            // whether the operand pushed is boxed: a typed one is in a
            // register
            let boxed = match &code.ops[at] {
                Op::Load(slot) => matches!(analysis.slots[*slot as usize], SlotKind::Boxed(_)),
                Op::LoadMove(slot) => {
                    moves_borrowed && matches!(analysis.slots[*slot as usize], SlotKind::Boxed(_))
                }
                Op::Const(constant) => {
                    abs_of_constant(&code.constants[*constant as usize]).is_boxed()
                }
                _ => true,
            };
            if boxed && takes[index] {
                borrowed[at] = true;
                mask |= 1 << index;
            }
        }
        masks[pc] = mask;
    }
    (borrowed, masks)
}

/// Compile one code object (decision AS1): its body and its trampoline
/// as machine code that holds no address, with the deopt points and the
/// loop headers the VM keeps beside them.
/// The machine code of a code object: `body` is its ops as the tier sees
/// them, the code object itself or its expansion with small callees
/// inlined (decision AU50), whose first ops are the code object's own, as
/// many as the program's code object has, where the loop headers are
/// looked for.
#[allow(clippy::too_many_arguments)]
pub fn compile(
    program: &Program,
    code_id: usize,
    body: &Code,
    calls: &[CallKind],
    globals: &[Option<i64>],
    isa: &dyn TargetIsa,
    ctx: &mut Context,
    fctx: &mut FunctionBuilderContext,
) -> Result<Compiled, Skipped> {
    let code = body;
    let original_len = program.codes[code_id].ops.len();
    let mut stats = Stats::default();
    let started = Instant::now();
    // the pairs of decision AW9 in a code object's own ops: the regions of
    // an expansion (decision AU50) lay their slots where the pairs' cells
    // would go
    let pairs = code.ops.len() == original_len;
    let analysis = analyse(program, code, globals, pairs).map_err(Skipped::Analysis)?;
    let (borrowed, masks) = borrowed_operands(program, code, &analysis, calls, true);
    let owned = owned_slots(code, &analysis, &borrowed);
    let calls_directly = has_direct_call(program, code, &analysis);
    stats.analysis = started.elapsed();
    let mut headers: Vec<usize> = Vec::new();
    for (pc, op) in code.ops.iter().enumerate().take(original_len) {
        if let Op::Jump(target) = op {
            let target = *target as usize;
            let fits = matches!(
                analysis.entry.get(target),
                Some(Some(state)) if state.iter().all(|abs| abs.is_boxed())
            );
            if target <= pc && fits && !headers.contains(&target) {
                headers.push(target);
            }
        }
    }
    let kinds: Vec<Abs> = (0..code.params as usize)
        .map(|slot| abs_of_slot(analysis.slots[slot]))
        .collect();
    // the parameters a direct call may pass borrowed (decision AW5), the
    // bits every caller's compilation computes the same way
    let flagged = u64::from(accepted_params(program, calls, code_id));
    let result_kind = match (code.kind, code.function) {
        (CodeKind::Function, Some(function)) => abs_of_result(program, function),
        _ => Abs::Boxed(None),
    };
    let returns_pair = match (code.kind, code.function) {
        (CodeKind::Function, Some(function)) => pair_result(program, function),
        _ => None,
    };
    // a pair slot's second field in a cell past the locals (decision AW9)
    let mut pair_cells = vec![None; code.locals as usize];
    let mut frame_locals = code.locals as usize;
    for (slot, kind) in analysis.slots.iter().enumerate() {
        if let SlotKind::Pair(_) = kind {
            pair_cells[slot] = Some(frame_locals);
            frame_locals += 1;
        }
    }
    ctx.clear();
    ctx.func.signature = direct_signature(isa, &kinds);
    ctx.func.name = UserFuncName::user(0, code_id as u32);
    let pointer = isa.pointer_type();
    let started = Instant::now();
    let deopts = {
        let mut builder = FunctionBuilder::new(&mut ctx.func, fctx);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        let vm = builder.block_params(entry)[0];
        let base = builder.block_params(entry)[1];
        let pc_param = builder.block_params(entry)[2];
        let direct = builder.block_params(entry)[3];
        let borrowed_params = builder.block_params(entry)[4];
        let grant = builder.block_params(entry)[5];
        let typed_params: Vec<(usize, IrValue)> = kinds
            .iter()
            .enumerate()
            .filter(|(_, kind)| !kind.is_boxed())
            .zip(builder.block_params(entry)[6..].iter())
            .map(|((slot, _), value)| (slot, *value))
            .collect();
        let exit = builder.create_block();
        builder.append_block_param(exit, types::I32);
        let base24 = builder.ins().imul_imm_s(base, SIZE as i64);
        let frame_var = builder.declare_var(pointer);
        // the VM's table of helpers, read once (decision AS1), and its
        // table of in-place entries (decision AW11)
        let helper_table = builder.ins().load(
            pointer,
            MemFlagsData::trusted(),
            vm,
            (native_state_offset() + STATE_HELPERS as i64) as i32,
        );
        let in_place_table = builder.ins().load(
            pointer,
            MemFlagsData::trusted(),
            vm,
            (native_state_offset() + STATE_IN_PLACE as i64) as i32,
        );
        // one word for a helper's answer, three for a range iterator's
        let out_slot =
            builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 24, 3));
        let max_depth = analysis
            .entry
            .iter()
            .flatten()
            .map(Vec::len)
            .max()
            .unwrap_or(0);
        let words = (frame_locals + max_depth) * 3 + 1;
        let deopt_slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            (words * 8) as u32,
            3,
        ));
        let mut gen = Gen {
            program,
            code_id,
            original_locals: program.codes[code_id].locals as usize,
            globals,
            code,
            analysis: &analysis,
            calls,
            borrowed,
            masks,
            owned,
            b: builder,
            pointer,
            isa,
            helper_table,
            in_place_table,
            in_place_sig: None,
            helper_sigs: HashMap::new(),
            vm,
            base,
            pc_param,
            direct,
            borrowed_params,
            flagged,
            grant,
            calls_directly,
            typed_params,
            base24,
            frame_var,
            max_depth,
            result_kind,
            pair_result: returns_pair,
            pairs,
            pair_cells,
            frame_locals,
            headers: headers.clone(),
            positions: Vec::new(),
            slots: vec![SlotVars::None; code.locals as usize],
            blocks: HashMap::new(),
            exit,
            landings: Vec::new(),
            out_slot,
            deopt_slot,
            deopts: Vec::new(),
            deopt_blocks: Vec::new(),
            state: Vec::new(),
            terminated: false,
            current_pc: 0,
        };
        gen.declare_slots();
        gen.prologue();
        gen.body();
        gen.epilogue();
        gen.b.seal_all_blocks();
        let deopts = std::mem::take(&mut gen.deopts);
        gen.b.finalize(isa.frontend_config());
        deopts
    };
    stats.ir = started.elapsed();
    stats.instructions = ctx.func.dfg.num_insts();
    stats.blocks = ctx.func.layout.blocks().count();
    let started = Instant::now();
    // the IR of a code object named by `RENYI_NATIVE_IR`, a development aid
    if std::env::var("RENYI_NATIVE_IR").is_ok_and(|name| name == code.name) {
        eprintln!("{}", ctx.func.display());
    }
    let body = machine_code(ctx, isa)?;
    let trampoline = trampoline(isa, ctx, fctx, code_id, &kinds)?;
    stats.cranelift = started.elapsed();
    let headers = headers.into_iter().map(|header| header as u32).collect();
    Ok(Compiled {
        body,
        trampoline,
        deopts,
        headers,
        regions: Vec::new(),
        stats,
    })
}

/// The machine code of the function in the context. It must hold no
/// relocation: a reference to an address outside it would not survive
/// the copy into an image (decision AS1), and the code reaches the VM's
/// helpers and tables through the VM pointer instead.
fn machine_code(ctx: &mut Context, isa: &dyn TargetIsa) -> Result<Vec<u8>, Skipped> {
    let compiled = ctx
        .compile(isa, &mut ControlPlane::default())
        .map_err(|error| Skipped::Codegen(format!("{error:?}")))?;
    if !compiled.buffer.relocs().is_empty() {
        return Err(Skipped::Codegen(
            "the code refers to an address outside it".to_string(),
        ));
    }
    Ok(compiled.code_buffer().to_vec())
}

/// The entry of a code object (`renyi_code_*`, the `Entry` signature): a
/// trampoline into the body (decision AR3), which it reaches through the
/// VM's table of bodies (decision AS1). At the start it takes the typed
/// parameters from the frame's slots under a check, and hands the frame
/// back (`DEOPT`) when one does not fit; at a loop header the body takes
/// the slots itself and the registers carry zeros. The body leaves its
/// result on the stack for a trampoline's caller (the flag it passes
/// says so), and the body's status becomes the entry's.
fn trampoline(
    isa: &dyn TargetIsa,
    ctx: &mut Context,
    fctx: &mut FunctionBuilderContext,
    code_id: usize,
    kinds: &[Abs],
) -> Result<Vec<u8>, Skipped> {
    ctx.clear();
    ctx.func.signature = entry_signature(isa);
    ctx.func.name = UserFuncName::user(1, code_id as u32);
    let pointer = isa.pointer_type();
    let flags = MemFlagsData::trusted();
    {
        let mut b = FunctionBuilder::new(&mut ctx.func, fctx);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let vm = b.block_params(entry)[0];
        let base = b.block_params(entry)[1];
        let pc = b.block_params(entry)[2];
        let state = b.ins().iadd_imm_s(vm, native_state_offset());
        let helpers = b.ins().load(pointer, flags, state, STATE_HELPERS);
        let direct_table = b.ins().load(pointer, flags, state, STATE_DIRECT);
        let body = b
            .ins()
            .load(pointer, flags, direct_table, (code_id * 8) as i32);
        let body_sig = b.import_signature(direct_signature(isa, kinds));
        let out = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
        let zero8 = b.ins().iconst(types::I8, 0);
        // no parameter borrowed: the frame owns every slot (decision AW5)
        let zero64 = b.ins().iconst(types::I64, 0);
        // the frame's grant from its record, the frame on top (decision AW6)
        let frames = b.ins().iadd_imm_s(vm, frames_offset());
        let frames_len = b.ins().load(pointer, flags, frames, 8);
        let frames_ptr = b.ins().load(pointer, flags, frames, 0);
        let top = b.ins().iadd_imm_s(frames_len, -1);
        let offset = b.ins().imul_imm_s(top, FRAME_SIZE as i64);
        let record = b.ins().iadd(frames_ptr, offset);
        let grant = b.ins().load(types::I32, flags, record, FRAME_GRANT);
        let start = b.create_block();
        let resume = b.create_block();
        let finish = b.create_block();
        b.append_block_param(finish, types::I32);
        b.append_block_param(finish, types::I64);
        let at_start = b.ins().icmp_imm_s(IntCC::Equal, pc, 0);
        b.ins().brif(at_start, start, &[], resume, &[]);
        // the start: the typed parameters from the frame's slots
        b.switch_to_block(start);
        let mut args = vec![vm, base, pc, zero8, zero64, grant];
        for (slot, kind) in kinds.iter().enumerate() {
            let (name, ty) = match kind {
                Abs::Int => ("rt_param_int", types::I64),
                Abs::Bool => ("rt_param_bool", types::I8),
                Abs::Float => ("rt_param_float", types::F64),
                _ => continue,
            };
            let index = helper_index(name);
            let (_, params, result) = SIGNATURES[index];
            let sig_ref = b.import_signature(signature(isa, params, result));
            let address = b.ins().load(pointer, flags, helpers, (index * 8) as i32);
            let slot_value = b.ins().iconst(types::I32, slot as i64);
            let out_address = b.ins().stack_addr(pointer, out, 0);
            let call =
                b.ins()
                    .call_indirect(sig_ref, address, &[vm, base, slot_value, out_address]);
            let ok = b.inst_results(call)[0];
            let next = b.create_block();
            let unfit = b.create_block();
            b.ins().brif(ok, next, &[], unfit, &[]);
            b.switch_to_block(unfit);
            let deopt = b.ins().iconst(types::I32, DEOPT as i64);
            b.ins().return_(&[deopt]);
            b.switch_to_block(next);
            args.push(b.ins().stack_load(pointer, ty, out, 0));
        }
        let call = b.ins().call_indirect(body_sig, body, &args);
        let results = b.inst_results(call).to_vec();
        b.ins()
            .jump(finish, &[results[0].into(), results[1].into()]);
        // a loop header: the body takes the slots itself
        b.switch_to_block(resume);
        let mut args = vec![vm, base, pc, zero8, zero64, grant];
        for kind in kinds {
            match kind {
                Abs::Int => args.push(b.ins().iconst(types::I64, 0)),
                Abs::Bool => args.push(b.ins().iconst(types::I8, 0)),
                Abs::Float => args.push(b.ins().f64const(0.0)),
                _ => {}
            }
        }
        let call = b.ins().call_indirect(body_sig, body, &args);
        let results = b.inst_results(call).to_vec();
        b.ins()
            .jump(finish, &[results[0].into(), results[1].into()]);
        // the status: a typed result is pushed; a boxed result or a failure
        // on the stack is `RETURNED`; the frame handed back is `DEOPT`; an
        // interrupt and a frame left to the interpreter keep their numbers
        b.switch_to_block(finish);
        let status = b.block_params(finish)[0];
        let boxed = b.ins().icmp_imm_s(IntCC::Equal, status, D_BOXED as i64);
        let failed = b.ins().icmp_imm_s(IntCC::Equal, status, D_FAILURE as i64);
        let left = b.ins().bor(boxed, failed);
        let deopted = b.ins().icmp_imm_s(IntCC::Equal, status, D_DEOPT as i64);
        let returned = b.ins().iconst(types::I32, RETURNED as i64);
        let deopt = b.ins().iconst(types::I32, DEOPT as i64);
        let mapped = b.ins().select(deopted, deopt, status);
        let mapped = b.ins().select(left, returned, mapped);
        b.ins().return_(&[mapped]);
        b.seal_all_blocks();
        b.finalize(isa.frontend_config());
    }
    machine_code(ctx, isa)
}

impl Gen<'_, '_> {
    // ------------------------------------------------------------ variables

    fn declare_slots(&mut self) {
        for (slot, kind) in self.analysis.slots.iter().enumerate() {
            self.slots[slot] = match kind {
                SlotKind::Int => SlotVars::One(self.b.declare_var(types::I64)),
                SlotKind::Bool => SlotVars::One(self.b.declare_var(types::I8)),
                SlotKind::Float => SlotVars::One(self.b.declare_var(types::F64)),
                SlotKind::RangeIter => SlotVars::Three([
                    self.b.declare_var(types::I64),
                    self.b.declare_var(types::I64),
                    self.b.declare_var(types::I64),
                ]),
                _ => SlotVars::None,
            };
        }
    }

    fn slot_var(&self, slot: usize) -> Variable {
        match self.slots[slot] {
            SlotVars::One(var) => var,
            _ => panic!("slot {slot} has no register"),
        }
    }

    fn slot_vars3(&self, slot: usize) -> [Variable; 3] {
        match self.slots[slot] {
            SlotVars::Three(vars) => vars,
            _ => panic!("slot {slot} is not a range iterator"),
        }
    }

    fn position(&mut self, depth: usize) -> &mut PositionVars {
        while self.positions.len() <= depth {
            self.positions.push(PositionVars::default());
        }
        &mut self.positions[depth]
    }

    fn int_var(&mut self, depth: usize) -> Variable {
        if let Some(var) = self.position(depth).int {
            return var;
        }
        let var = self.b.declare_var(types::I64);
        self.position(depth).int = Some(var);
        var
    }

    fn bool_var(&mut self, depth: usize) -> Variable {
        if let Some(var) = self.position(depth).bool {
            return var;
        }
        let var = self.b.declare_var(types::I8);
        self.position(depth).bool = Some(var);
        var
    }

    fn float_var(&mut self, depth: usize) -> Variable {
        if let Some(var) = self.position(depth).float {
            return var;
        }
        let var = self.b.declare_var(types::F64);
        self.position(depth).float = Some(var);
        var
    }

    fn range_vars(&mut self, depth: usize) -> [Variable; 3] {
        if let Some(vars) = self.position(depth).range {
            return vars;
        }
        let vars = [
            self.b.declare_var(types::I64),
            self.b.declare_var(types::I64),
            self.b.declare_var(types::I64),
        ];
        self.position(depth).range = Some(vars);
        vars
    }

    // ------------------------------------------------------------ helpers

    fn call(&mut self, name: &'static str, args: &[IrValue]) -> Option<IrValue> {
        let index = helper_index(name);
        let sig_ref = match self.helper_sigs.get(name) {
            Some(sig_ref) => *sig_ref,
            None => {
                let (_, params, result) = SIGNATURES[index];
                let sig_ref = self.b.import_signature(signature(self.isa, params, result));
                self.helper_sigs.insert(name, sig_ref);
                sig_ref
            }
        };
        let address = self.b.ins().load(
            self.pointer,
            MemFlagsData::trusted(),
            self.helper_table,
            (index * 8) as i32,
        );
        let inst = self.b.ins().call_indirect(sig_ref, address, args);
        let result = self.b.inst_results(inst).first().copied();
        // a helper may have moved the stack, unless it is one that cannot
        if !STACK_SAFE.contains(&name) {
            self.reload_frame();
        }
        result
    }

    /// The VM's native state (decision AS1): the counters and the tables.
    fn native_state(&mut self) -> IrValue {
        self.b.ins().iadd_imm_s(self.vm, native_state_offset())
    }

    fn iconst(&mut self, ty: Type, value: i64) -> IrValue {
        self.b.ins().iconst(ty, value)
    }

    fn u32(&mut self, value: u32) -> IrValue {
        self.iconst(types::I32, value as i64)
    }

    fn usize(&mut self, value: usize) -> IrValue {
        self.iconst(self.pointer, value as i64)
    }

    fn u8(&mut self, value: u8) -> IrValue {
        self.iconst(types::I8, value as i64)
    }

    fn out_address(&mut self) -> IrValue {
        self.b.ins().stack_addr(self.pointer, self.out_slot, 0)
    }

    fn out_read(&mut self, ty: Type) -> IrValue {
        self.b.ins().stack_load(self.pointer, ty, self.out_slot, 0)
    }

    fn block_for(&mut self, pc: usize) -> Block {
        if let Some(block) = self.blocks.get(&pc) {
            return *block;
        }
        let block = self.b.create_block();
        self.blocks.insert(pc, block);
        block
    }

    fn switch_to(&mut self, block: Block) {
        self.b.switch_to_block(block);
        self.terminated = false;
    }

    /// Leave the function with a status: `LEFT` means the frame returned.
    fn exit_with(&mut self, status: IrValue) {
        self.b.ins().jump(self.exit, &[status.into()]);
        self.terminated = true;
    }

    /// Leave the body with a status of `runtime::D_*` and no payload.
    fn return_direct(&mut self, status: i32) {
        let value = self.iconst(types::I32, status as i64);
        let payload = self.iconst(types::I64, 0);
        self.b.ins().return_(&[value, payload]);
        self.terminated = true;
    }

    /// Leave the body with a status and the bits of a typed result.
    fn return_with(&mut self, status: IrValue, payload: IrValue) {
        self.b.ins().return_(&[status, payload]);
        self.terminated = true;
    }

    /// A typed result in a register as the `i64` bits the signature
    /// carries it as.
    fn payload_bits(&mut self, kind: Abs, value: IrValue) -> IrValue {
        match kind {
            Abs::Bool => self.b.ins().uextend(types::I64, value),
            Abs::Float => self.b.ins().bitcast(types::I64, MemFlagsData::new(), value),
            _ => value,
        }
    }

    /// The bits of a typed result into the register of the next operand.
    fn push_payload(&mut self, kind: Abs, payload: IrValue) {
        let depth = self.state.len();
        match kind {
            Abs::Int => {
                let var = self.int_var(depth);
                self.b.def_var(var, payload);
            }
            Abs::Bool => {
                let flag = self.b.ins().ireduce(types::I8, payload);
                let var = self.bool_var(depth);
                self.b.def_var(var, flag);
            }
            _ => {
                let float = self
                    .b
                    .ins()
                    .bitcast(types::F64, MemFlagsData::new(), payload);
                let var = self.float_var(depth);
                self.b.def_var(var, float);
            }
        }
        self.state.push(kind);
    }

    /// After a helper that returns a status: continue on `CONTINUE`; else
    /// a `FAILURE` goes to the op's handler (when the op may fail) and
    /// anything else leaves the function.
    fn check_status(&mut self, status: IrValue, pc: usize, may_fail: bool) {
        let ok = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, status, CONTINUE as i64);
        let cont = self.b.create_block();
        if may_fail {
            let bad = self.bad_block(pc);
            let pc_value = self.u32(pc as u32);
            self.b
                .ins()
                .brif(ok, cont, &[], bad, &[status.into(), pc_value.into()]);
        } else {
            let exit = self.exit;
            self.b.ins().brif(ok, cont, &[], exit, &[status.into()]);
        }
        self.b.seal_block(cont);
        self.switch_to(cont);
    }

    /// The block a failed status of the op at `pc` goes to, with the
    /// status and the pc as its parameters: one block serves every op
    /// with the same innermost handler and the same operand stack below
    /// the handler's depth, since the landing boxes that stack the same
    /// way. It is filled by `emit_landings`.
    fn bad_block(&mut self, pc: usize) -> Block {
        let handler = self.analysis.handlers[pc].last().copied();
        // keyed by the kinds alone, as the hand-backs are
        let below: Vec<Abs> = match handler {
            Some((_, depth)) => self.state[..depth]
                .iter()
                .map(|abs| abs.untyped())
                .collect(),
            None => Vec::new(),
        };
        let known = self
            .landings
            .iter()
            .find(|(h, b, _)| *h == handler && *b == below);
        if let Some((_, _, block)) = known {
            return *block;
        }
        let block = self.b.create_block();
        self.b.append_block_param(block, types::I32);
        self.b.append_block_param(block, types::I32);
        self.landings.push((handler, below, block));
        block
    }

    /// Fill the blocks of `bad_block`: a `FAILURE` (on top of the stack)
    /// lands on the floor of the innermost handled region, or, with no
    /// handler, leaves the frame with it or crashes (`settle`); any other
    /// status leaves the function.
    fn emit_landings(&mut self) {
        let landings = std::mem::take(&mut self.landings);
        for (handler, below, block) in landings {
            self.switch_to(block);
            let status = self.b.block_params(block)[0];
            let pc_value = self.b.block_params(block)[1];
            let failed = self
                .b
                .ins()
                .icmp_imm_s(IntCC::Equal, status, FAILURE as i64);
            let landing = self.b.create_block();
            let exit = self.exit;
            self.b
                .ins()
                .brif(failed, landing, &[], exit, &[status.into()]);
            self.b.seal_block(landing);
            self.switch_to(landing);
            match handler {
                Some((target, depth)) => {
                    debug_assert_eq!(below.len(), depth);
                    self.state = below;
                    let floor = boxed_depth(&self.state) as u32;
                    let locals = self.u32(self.frame_locals as u32);
                    let floor = self.u32(floor);
                    self.call("rt_settle_handler", &[self.vm, self.base, locals, floor]);
                    self.state.push(Abs::Boxed(None));
                    self.jump_to(target as usize);
                }
                None => {
                    self.retain_params();
                    let status = self
                        .call("rt_unhandled", &[self.vm, pc_value])
                        .expect("a status");
                    self.exit_with(status);
                }
            }
        }
    }

    // ------------------------------------------------------------ boxing

    /// How many boxed operands lie above depth `i`.
    fn boxed_above(&self, depth: usize) -> usize {
        boxed_depth(&self.state[depth + 1..])
    }

    /// Box the operand at `depth` onto the VM's stack, among the boxed
    /// operands in order.
    fn box_position(&mut self, depth: usize) {
        let above = self.boxed_above(depth);
        let kind = self.state[depth];
        if above == 0 && matches!(kind, Abs::Int | Abs::Bool | Abs::Float) {
            // the register pushed as a value in place (decision AR4)
            let var = match kind {
                Abs::Int => self.int_var(depth),
                Abs::Bool => self.bool_var(depth),
                _ => self.float_var(depth),
            };
            let value = self.b.use_var(var);
            let height = self.height();
            self.push_typed_value(height, kind, value);
            self.state[depth] = Abs::Boxed(None);
            return;
        }
        let above_value = self.usize(above);
        match kind {
            Abs::Int => {
                let var = self.int_var(depth);
                let value = self.b.use_var(var);
                self.call("rt_insert_int", &[self.vm, above_value, value]);
            }
            Abs::Bool => {
                let var = self.bool_var(depth);
                let value = self.b.use_var(var);
                self.call("rt_insert_bool", &[self.vm, above_value, value]);
            }
            Abs::Float => {
                let var = self.float_var(depth);
                let value = self.b.use_var(var);
                self.call("rt_insert_float", &[self.vm, above_value, value]);
            }
            Abs::Range => {
                let vars = self.range_vars(depth);
                let from = self.b.use_var(vars[0]);
                let to = self.b.use_var(vars[1]);
                let by = self.b.use_var(vars[2]);
                self.call("rt_insert_range", &[self.vm, above_value, from, to, by]);
            }
            Abs::Pair(ty) => {
                // the pair made into its record where it lies (decision AW9)
                let ty_value = self.usize(ty as usize);
                self.call("rt_pair_record", &[self.vm, above_value, ty_value]);
            }
            Abs::Boxed(_) | Abs::Unset => return,
        }
        self.state[depth] = Abs::Boxed(None);
    }

    /// Box the top `count` operands.
    fn box_top(&mut self, count: usize) {
        let from = self.state.len().saturating_sub(count);
        for depth in from..self.state.len() {
            self.box_position(depth);
        }
    }

    /// Make the state fit a target state: every operand unboxed here and
    /// boxed there is boxed.
    fn coerce_to(&mut self, target: &[Abs]) {
        debug_assert_eq!(self.state.len(), target.len());
        for (depth, there) in target.iter().enumerate() {
            if there.is_boxed() && !self.state[depth].is_boxed() {
                self.box_position(depth);
            }
        }
    }

    /// Jump to the op at `pc`, coercing the state to what it expects.
    fn jump_to(&mut self, pc: usize) {
        if let Some(Some(target)) = self.analysis.entry.get(pc) {
            let target = target.clone();
            self.coerce_to(&target);
        }
        let block = self.block_for(pc);
        self.b.ins().jump(block, &[]);
        self.terminated = true;
    }

    /// The block an edge to `pc` goes through: the target itself when the
    /// state already fits, else a block that coerces first.
    fn edge_to(&mut self, pc: usize) -> (Block, bool) {
        let fits = match self.analysis.entry.get(pc) {
            Some(Some(target)) => self
                .state
                .iter()
                .zip(target)
                .all(|(here, there)| !there.is_boxed() || here.is_boxed()),
            _ => true,
        };
        if fits {
            (self.block_for(pc), false)
        } else {
            (self.b.create_block(), true)
        }
    }

    /// Fill an edge block made by `edge_to`.
    fn fill_edge(&mut self, block: Block, pc: usize) {
        let saved = self.state.clone();
        self.switch_to(block);
        self.jump_to(pc);
        self.state = saved;
    }

    /// Branch on an `i8` condition to two ops.
    fn branch(&mut self, condition: IrValue, when_true: usize, when_false: usize) {
        let (true_block, fill_true) = self.edge_to(when_true);
        let (false_block, fill_false) = self.edge_to(when_false);
        self.b
            .ins()
            .brif(condition, true_block, &[], false_block, &[]);
        self.terminated = true;
        if fill_true {
            self.fill_edge(true_block, when_true);
        }
        if fill_false {
            self.fill_edge(false_block, when_false);
        }
    }

    // ------------------------------------------------------------ deopt

    /// Hand the frame to the interpreter at `pc` with the current state:
    /// the registers are boxed into the slots and the operand stack.
    fn deopt_here(&mut self, pc: usize) {
        let point = DeoptPoint {
            pc: pc as u32,
            locals: self.frame_locals as u16,
            stack: self.state.clone(),
            slots: self.analysis.slots.clone(),
            marks: self.analysis.marks.clone(),
            handlers: self.analysis.handlers.get(pc).cloned().unwrap_or_default(),
        };
        let index = self.deopts.len() as u32;
        self.deopts.push(point);
        let mut offset = 0i32;
        let mut store = |gen: &mut Self, value: IrValue| {
            gen.b
                .ins()
                .stack_store(gen.pointer, value, gen.deopt_slot, offset);
            offset += 8;
        };
        let analysis = self.analysis;
        for (slot, &kind) in analysis.slots.iter().enumerate() {
            match kind {
                SlotKind::Int => {
                    let var = self.slot_var(slot);
                    let value = self.b.use_var(var);
                    store(self, value);
                }
                SlotKind::Bool => {
                    let var = self.slot_var(slot);
                    let value = self.b.use_var(var);
                    let wide = self.b.ins().uextend(types::I64, value);
                    store(self, wide);
                }
                SlotKind::Float => {
                    let var = self.slot_var(slot);
                    let value = self.b.use_var(var);
                    store(self, value);
                }
                SlotKind::RangeIter => {
                    let vars = self.slot_vars3(slot);
                    for var in vars {
                        let value = self.b.use_var(var);
                        store(self, value);
                    }
                }
                SlotKind::Pair(_) => {
                    // whether the slot holds its pair at the point (decision
                    // AW9): the hand-back makes the record of its two cells
                    let whole = self.owned.get(pc).is_none_or(|owned| owned[slot]);
                    let bit = self.iconst(types::I64, i64::from(whole));
                    store(self, bit);
                }
                _ => {}
            }
        }
        for depth in 0..self.state.len() {
            match self.state[depth] {
                Abs::Int => {
                    let var = self.int_var(depth);
                    let value = self.b.use_var(var);
                    store(self, value);
                }
                Abs::Bool => {
                    let var = self.bool_var(depth);
                    let value = self.b.use_var(var);
                    let wide = self.b.ins().uextend(types::I64, value);
                    store(self, wide);
                }
                Abs::Float => {
                    let var = self.float_var(depth);
                    let value = self.b.use_var(var);
                    store(self, value);
                }
                Abs::Range => {
                    let vars = self.range_vars(depth);
                    for var in vars {
                        let value = self.b.use_var(var);
                        store(self, value);
                    }
                }
                Abs::Boxed(_) | Abs::Pair(_) | Abs::Unset => {}
            }
        }
        let code = self.usize(self.code_id);
        let point = self.u32(index);
        let buffer = self.b.ins().stack_addr(self.pointer, self.deopt_slot, 0);
        // the interpreter owns every slot of the frame it takes over
        self.retain_params();
        self.call("rt_deopt", &[self.vm, self.base, code, point, buffer]);
        self.return_direct(D_DEOPT);
    }

    /// Continue when `ok` (an `i8`), else hand the frame to the
    /// interpreter at `pc` with the current state.
    fn deopt_unless(&mut self, ok: IrValue, pc: usize) {
        let cont = self.b.create_block();
        let deopt = self.deopt_block(pc);
        self.b.ins().brif(ok, cont, &[], deopt, &[]);
        self.b.seal_block(cont);
        self.switch_to(cont);
    }

    /// The block that hands the frame to the interpreter at `pc` with the
    /// current operand state: one per pc and state, shared by every guard
    /// that reaches the op so (the checks of a call's result on its paths,
    /// for one; decision AT2), filled by `emit_deopts`.
    fn deopt_block(&mut self, pc: usize) -> Block {
        // keyed by the kinds alone: a type changes nothing in a hand-back
        let state: Vec<Abs> = self.state.iter().map(|abs| abs.untyped()).collect();
        let known = self
            .deopt_blocks
            .iter()
            .find(|((at, known), _)| *at == pc && *known == state);
        if let Some((_, block)) = known {
            return *block;
        }
        let block = self.b.create_block();
        self.b.set_cold_block(block);
        self.deopt_blocks.push(((pc, state), block));
        block
    }

    /// Fill the blocks of `deopt_block`.
    fn emit_deopts(&mut self) {
        let blocks = std::mem::take(&mut self.deopt_blocks);
        for ((pc, state), block) in blocks {
            self.state = state;
            self.switch_to(block);
            self.deopt_here(pc);
        }
    }

    /// Take the boxed value on top of the stack into a register of the
    /// kind, or hand the frame to the interpreter at `pc` (the value stays
    /// boxed on the stack, where the interpreter expects it).
    fn unbox_top(&mut self, kind: Abs, pc: usize) {
        let depth = self.state.len() - 1;
        let flags = MemFlagsData::trusted();
        match kind {
            Abs::Int => {
                // the top's tags read in place (decision AR4): an Integer
                // that is small is taken, anything else stays for the
                // interpreter
                let at = self.last_address();
                let tag = self.tag_at(at, 0);
                let is_int = self
                    .b
                    .ins()
                    .icmp_imm_s(IntCC::Equal, tag, TAG_INTEGER as i64);
                let int_tag = self.tag_at(at, INT_TAG);
                let is_small = self
                    .b
                    .ins()
                    .icmp_imm_s(IntCC::Equal, int_tag, INT_SMALL as i64);
                let ok = self.b.ins().band(is_int, is_small);
                self.deopt_unless(ok, pc);
                let value = self.b.ins().load(types::I64, flags, at, INT_PAYLOAD);
                let height = self.height() - 1;
                self.store_height(height);
                let var = self.int_var(depth);
                self.b.def_var(var, value);
            }
            Abs::Float => {
                let at = self.last_address();
                let tag = self.tag_at(at, 0);
                let ok = self.b.ins().icmp_imm_s(IntCC::Equal, tag, TAG_FLOAT as i64);
                self.deopt_unless(ok, pc);
                let value = self.b.ins().load(types::F64, flags, at, PAYLOAD);
                let height = self.height() - 1;
                self.store_height(height);
                let var = self.float_var(depth);
                self.b.def_var(var, value);
            }
            Abs::Bool => {
                let value = self.take_bool_top(pc);
                let var = self.bool_var(depth);
                self.b.def_var(var, value);
            }
            _ => return,
        }
        self.state[depth] = kind;
    }

    /// The Boolean on top of the stack taken into an `i8`, in place; a
    /// value of another type is the crash the interpreter raises, through
    /// the helper (decision AR4).
    fn take_bool_top(&mut self, pc: usize) -> IrValue {
        let flags = MemFlagsData::trusted();
        let at = self.last_address();
        let tag = self.tag_at(at, 0);
        let ok = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, tag, TAG_BOOLEAN as i64);
        let cont = self.b.create_block();
        let bad = self.b.create_block();
        self.b.ins().brif(ok, cont, &[], bad, &[]);
        self.b.set_cold_block(bad);
        self.switch_to(bad);
        let out = self.out_address();
        let pc_value = self.u32(pc as u32);
        let status = self
            .call("rt_take_bool", &[self.vm, pc_value, out])
            .expect("a status");
        self.exit_with(status);
        self.switch_to(cont);
        let value = self.b.ins().load(types::I8, flags, at, PAYLOAD);
        let height = self.height() - 1;
        self.store_height(height);
        value
    }

    /// Pop the top operand as an `i8` condition: from its register, or
    /// from the stack (a crash when it is not a Boolean).
    fn pop_condition(&mut self, pc: usize) -> IrValue {
        let depth = self.state.len() - 1;
        let value = match self.state[depth] {
            Abs::Bool => {
                let var = self.bool_var(depth);
                self.b.use_var(var)
            }
            _ => {
                self.box_position(depth);
                self.take_bool_top(pc)
            }
        };
        self.state.pop();
        value
    }

    // ------------------------------------------------------------ pushes

    fn push_int(&mut self, value: IrValue) {
        let depth = self.state.len();
        let var = self.int_var(depth);
        self.b.def_var(var, value);
        self.state.push(Abs::Int);
    }

    fn push_bool(&mut self, value: IrValue) {
        let depth = self.state.len();
        let var = self.bool_var(depth);
        self.b.def_var(var, value);
        self.state.push(Abs::Bool);
    }

    fn push_float(&mut self, value: IrValue) {
        let depth = self.state.len();
        let var = self.float_var(depth);
        self.b.def_var(var, value);
        self.state.push(Abs::Float);
    }

    /// A boxed operand pushed by the op being translated: with the type
    /// the checker noted for its expression when the op pushes that
    /// expression's value (decision AU1, stage iv), as the analysis has it.
    fn push_boxed(&mut self) {
        let noted = if self.code.ops[self.current_pc].pushes_its_expression() {
            self.code.types.get(self.current_pc).copied().flatten()
        } else {
            None
        };
        self.state.push(Abs::Boxed(noted));
    }

    /// The top operand as a register value of its kind, popped.
    fn pop_int(&mut self) -> IrValue {
        let depth = self.state.len() - 1;
        let var = self.int_var(depth);
        self.state.pop();
        self.b.use_var(var)
    }

    fn pop_bool(&mut self) -> IrValue {
        let depth = self.state.len() - 1;
        let var = self.bool_var(depth);
        self.state.pop();
        self.b.use_var(var)
    }

    fn pop_float(&mut self) -> IrValue {
        let depth = self.state.len() - 1;
        let var = self.float_var(depth);
        self.state.pop();
        self.b.use_var(var)
    }

    /// Box the top `count` operands and call a helper whose arguments are
    /// the VM and the given extra values, then account for `pops` taken
    /// and the pushed result.
    fn helper_on_stack(
        &mut self,
        name: &'static str,
        count: usize,
        extra: &[IrValue],
    ) -> Option<IrValue> {
        self.box_top(count);
        let mut args = vec![self.vm];
        args.extend_from_slice(extra);
        let result = self.call(name, &args);
        for _ in 0..count {
            self.state.pop();
        }
        result
    }

    // ------------------------------------------------------ the stack in place

    /// The address of the stack's pinned vector in the VM.
    fn stack_vector(&mut self) -> IrValue {
        self.b.ins().iadd_imm_s(self.vm, stack_offset())
    }

    /// How many values of the frame lie on the stack by the current
    /// state: the locals (a cell per pair slot among them, decision AW9),
    /// then the boxed operands (a typed operand is in a register, a pair
    /// in two cells). The stack's length is `base` plus this at every op,
    /// so the generated code computes it and never loads it (decision
    /// AT4).
    fn height(&self) -> usize {
        self.frame_locals + boxed_depth(&self.state)
    }

    /// The stack's length for a height: `base` plus it.
    fn length_at(&mut self, height: usize) -> IrValue {
        self.b.ins().iadd_imm_s(self.base, height as i64)
    }

    /// The VM's stack length written for a height: a helper and the
    /// interpreter read it, the generated code never does.
    fn store_height(&mut self, height: usize) {
        let len = self.length_at(height);
        let vector = self.stack_vector();
        self.b.ins().store(MemFlagsData::trusted(), len, vector, 8);
    }

    /// The frame pointer, the address of the frame's first slot.
    fn frame_ptr(&mut self) -> IrValue {
        self.b.use_var(self.frame_var)
    }

    /// The frame pointer read again from the VM: once at the entry, and
    /// after every call out of the generated code, which may have grown
    /// the stack.
    fn reload_frame(&mut self) {
        let vector = self.stack_vector();
        let ptr = self
            .b
            .ins()
            .load(self.pointer, MemFlagsData::trusted(), vector, 0);
        let frame = self.b.ins().iadd(ptr, self.base24);
        self.b.def_var(self.frame_var, frame);
    }

    /// The address of the frame's value at a height: a slot by its index,
    /// a boxed operand by `locals` plus its depth among the boxed ones.
    fn address_at(&mut self, height: usize) -> IrValue {
        let frame = self.frame_ptr();
        self.b.ins().iadd_imm_s(frame, (height * SIZE) as i64)
    }

    /// The address just above the stack, where a push goes.
    fn top_address(&mut self) -> IrValue {
        let height = self.height();
        self.address_at(height)
    }

    /// The address of the boxed operand on top of the stack.
    fn last_address(&mut self) -> IrValue {
        let height = self.height() - 1;
        self.address_at(height)
    }

    /// The address of the boxed operand at `depth` of the state.
    fn operand_address(&mut self, depth: usize) -> IrValue {
        let height = self.height() - 1 - self.boxed_above(depth);
        self.address_at(height)
    }

    /// The address of the item at `index` of a pinned vector of values,
    /// from its pointer.
    fn item_address(&mut self, ptr: IrValue, index: IrValue) -> IrValue {
        let offset = self.b.ins().imul_imm_s(index, SIZE as i64);
        self.b.ins().iadd(ptr, offset)
    }

    /// The three words of a value copied, as a `Value` is moved.
    fn copy_value(&mut self, from: IrValue, to: IrValue) {
        let flags = MemFlagsData::trusted();
        for offset in [0, 8, 16] {
            let word = self.b.ins().load(types::I64, flags, from, offset);
            self.b.ins().store(flags, word, to, offset);
        }
    }

    /// The tag of the value at the address, as an `i32`.
    fn tag_at(&mut self, at: IrValue, offset: i32) -> IrValue {
        self.b
            .ins()
            .uload8(types::I32, MemFlagsData::trusted(), at, offset)
    }

    /// The branch a retain and a release in place start with (decision
    /// AU32, AU29's sequence on this tier): the tag's bit in
    /// `COUNTED_OR_INTEGER`. A value with neither goes to `done`, an
    /// Integer to `integer`, a cold block, and a counted block to
    /// `counted`; the three are returned, `done` unsealed for the
    /// caller's jumps.
    fn branch_on_count(&mut self, at: IrValue) -> (Block, Block, Block) {
        let tag = self.tag_at(at, 0);
        let tag_wide = self.b.ins().uextend(types::I64, tag);
        let mask = self.iconst(types::I64, COUNTED_OR_INTEGER as i64);
        let shifted = self.b.ins().ushr(mask, tag_wide);
        let bit = self.b.ins().band_imm_s(shifted, 1);
        let check = self.b.create_block();
        let counted = self.b.create_block();
        let integer = self.b.create_block();
        let done = self.b.create_block();
        self.b.set_cold_block(integer);
        self.b.ins().brif(bit, check, &[], done, &[]);
        self.b.seal_block(check);
        self.switch_to(check);
        let is_integer = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, tag, TAG_INTEGER as i64);
        self.b.ins().brif(is_integer, integer, &[], counted, &[]);
        self.b.seal_block(counted);
        self.b.seal_block(integer);
        (counted, integer, done)
    }

    /// The count in the first word of the block raised by one.
    fn raise_count(&mut self, block: IrValue) {
        let flags = MemFlagsData::trusted();
        let count = self.b.ins().load(types::I64, flags, block, 0);
        let more = self.b.ins().iadd_imm_s(count, 1);
        self.b.ins().store(flags, more, block, 0);
    }

    /// From the cold block of an Integer: a small one holds no count and
    /// goes to `done`; a big one goes on in the block returned, cold too.
    fn big_integer(&mut self, at: IrValue, done: Block) -> Block {
        let int_tag = self.tag_at(at, INT_TAG);
        let is_small = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, int_tag, INT_SMALL as i64);
        let big = self.b.create_block();
        self.b.set_cold_block(big);
        self.b.ins().brif(is_small, done, &[], big, &[]);
        self.b.seal_block(big);
        big
    }

    /// One more reference to the value at the address, in place (decision
    /// AU32): a counted block has its count raised; an Integer goes out of
    /// line, where a big one raises its block's count and a small one
    /// holds none; any other value holds no count. AT6 had called a helper
    /// here against AR4's sequence, which tested the big Integer and read
    /// both payload words at every site; this one tests one bit and skips,
    /// and calls nothing (a call in a cold block costs the register
    /// allocation of the whole body: with one, the self-check's code was
    /// 3% larger and ran more instructions).
    fn retain(&mut self, at: IrValue) {
        let flags = MemFlagsData::trusted();
        let (counted, integer, done) = self.branch_on_count(at);
        self.switch_to(counted);
        let block = self.b.ins().load(self.pointer, flags, at, PAYLOAD);
        self.raise_count(block);
        self.b.ins().jump(done, &[]);
        self.switch_to(integer);
        let big = self.big_integer(at, done);
        self.switch_to(big);
        let block = self.b.ins().load(self.pointer, flags, at, INT_PAYLOAD);
        self.raise_count(block);
        self.b.ins().jump(done, &[]);
        self.b.seal_block(done);
        self.switch_to(done);
    }

    /// One reference fewer to the value at the address, which is dead
    /// after, in place (decision AU32): a counted block has its count
    /// lowered, and only the last reference goes out of line, the count
    /// raised back to one for `rt_drop_at`, which drops the value as Rust
    /// does and frees the block; a big Integer goes to the helper, a small
    /// one nowhere. The two share one call: a call in a cold block costs
    /// the register allocation of the whole body, and with one call per
    /// release the self-check ran 0.2% fewer instructions than with two.
    fn release(&mut self, at: IrValue) {
        let flags = MemFlagsData::trusted();
        let (counted, integer, done) = self.branch_on_count(at);
        self.switch_to(counted);
        let block = self.b.ins().load(self.pointer, flags, at, PAYLOAD);
        let count = self.b.ins().load(types::I64, flags, block, 0);
        let fewer = self.b.ins().iadd_imm_s(count, -1);
        self.b.ins().store(flags, fewer, block, 0);
        let last = self.b.create_block();
        let drop = self.b.create_block();
        self.b.set_cold_block(last);
        self.b.set_cold_block(drop);
        self.b.ins().brif(fewer, done, &[], last, &[]);
        self.b.seal_block(last);
        self.switch_to(last);
        let one = self.iconst(types::I64, 1);
        self.b.ins().store(flags, one, block, 0);
        self.b.ins().jump(drop, &[]);
        self.switch_to(integer);
        let int_tag = self.tag_at(at, INT_TAG);
        let is_small = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, int_tag, INT_SMALL as i64);
        self.b.ins().brif(is_small, done, &[], drop, &[]);
        self.b.seal_block(drop);
        self.switch_to(drop);
        self.call("rt_drop_at", &[at]);
        self.b.ins().jump(done, &[]);
        self.b.seal_block(done);
        self.switch_to(done);
    }

    /// A value pushed on the stack from the address, one more reference to
    /// it when `retain` says so (a move needs none).
    fn push_copy(&mut self, from: IrValue, retain: bool) {
        let to = self.top_address();
        self.copy_value(from, to);
        if retain {
            self.retain(to);
        }
        let height = self.height() + 1;
        self.store_height(height);
    }

    /// A typed value written into the three words at the address.
    fn write_typed(&mut self, at: IrValue, kind: Abs, value: IrValue) {
        let flags = MemFlagsData::trusted();
        match kind {
            Abs::Int => {
                let tag = self.iconst(types::I32, TAG_INTEGER as i64);
                self.b.ins().istore8(flags, tag, at, 0);
                let small = self.iconst(types::I32, INT_SMALL as i64);
                self.b.ins().istore8(flags, small, at, INT_TAG);
                self.b.ins().store(flags, value, at, INT_PAYLOAD);
            }
            Abs::Bool => {
                let tag = self.iconst(types::I32, TAG_BOOLEAN as i64);
                self.b.ins().istore8(flags, tag, at, 0);
                self.b.ins().store(flags, value, at, PAYLOAD);
            }
            Abs::Float => {
                let tag = self.iconst(types::I32, TAG_FLOAT as i64);
                self.b.ins().istore8(flags, tag, at, 0);
                self.b.ins().store(flags, value, at, PAYLOAD);
            }
            _ => unreachable!("a typed kind"),
        }
    }

    /// A typed value pushed on the stack as a `Value`, at a height: the
    /// state's for a push, `0` for a result left where the frame was.
    fn push_typed_value(&mut self, height: usize, kind: Abs, value: IrValue) {
        let to = self.address_at(height);
        self.write_typed(to, kind, value);
        self.store_height(height + 1);
    }

    // ------------------------------------------------------ the frames in place

    fn frames_vector(&mut self) -> IrValue {
        self.b.ins().iadd_imm_s(self.vm, frames_offset())
    }

    fn handlers_vector(&mut self) -> IrValue {
        self.b.ins().iadd_imm_s(self.vm, handlers_offset())
    }

    /// The address of the frame record at `index` of the frames.
    fn frame_address(&mut self, ptr: IrValue, index: IrValue) -> IrValue {
        let offset = self.b.ins().imul_imm_s(index, FRAME_SIZE as i64);
        self.b.ins().iadd(ptr, offset)
    }

    /// `Nothing` pushed on the stack at a height (as `push_typed_value`).
    fn push_nothing_at(&mut self, height: usize) {
        let to = self.address_at(height);
        let nothing = self.iconst(types::I32, TAG_NOTHING as i64);
        self.b
            .ins()
            .istore8(MemFlagsData::trusted(), nothing, to, 0);
        self.store_height(height + 1);
    }

    /// The callee's frame pushed in place (decision AR4), as
    /// `Vm::push_frame_in_place` pushes one: the boxed arguments on top
    /// of the stack move to their parameters' slots (every other local is
    /// written `Nothing` by the callee's prologue, decision AT2, which
    /// writes the stack's length too, decision AW6), and the frame record
    /// carries the grant of decision Q1: this frame's, from the register
    /// the signature passed it in, unless the callee narrows it, when the
    /// helper computes it. The frames have room for the record by this
    /// body's prologue (decision AW6), and the stack has room for the
    /// callee's locals by the callee's own prologue, except where an
    /// argument moves to a slot past the room this frame's prologue made,
    /// which is checked here first. The new frame's base and its grant
    /// are returned.
    fn push_frame_inline(&mut self, callee: usize, mask: u64) -> (IrValue, IrValue) {
        let flags = MemFlagsData::trusted();
        let pointer = self.pointer;
        let program = self.program;
        let meta = &program.codes[callee];
        let locals = meta.locals as usize;
        let params = meta.params as usize;
        let boxed = mask.count_ones() as usize;
        let narrows = meta
            .function
            .is_some_and(|function| !program.function_metas[function].needs.is_empty());
        let grant = if narrows {
            let callee_value = self.usize(callee);
            self.call("rt_frame_grant", &[self.vm, callee_value])
                .expect("a grant")
        } else {
            self.grant
        };
        // the callee's base is this frame's height less the boxed
        // arguments (decision AT4)
        let callee_height = self.height() - boxed;
        let base = self.length_at(callee_height);
        // a slot an argument moves to past this frame's room (its locals
        // and its deepest operand stack, one more; the prologue) needs the
        // stack's room checked for the callee's locals first
        let farthest = (0..params)
            .rev()
            .find(|slot| mask & (1 << slot) != 0)
            .map_or(0, |slot| callee_height + slot + 1);
        if farthest > self.frame_locals + self.max_depth + 1 {
            let stack = self.stack_vector();
            let needed = self.b.ins().iadd_imm_s(base, locals as i64);
            let cap = self.b.ins().load(pointer, flags, stack, 16);
            let short = self.b.ins().icmp(IntCC::UnsignedLessThan, cap, needed);
            let grow = self.b.create_block();
            let cont = self.b.create_block();
            self.b.ins().brif(short, grow, &[], cont, &[]);
            self.b.set_cold_block(grow);
            self.switch_to(grow);
            self.call("rt_room", &[self.vm, needed]);
            self.b.ins().jump(cont, &[]);
            self.switch_to(cont);
        }
        let first = self.address_at(callee_height);
        // the boxed arguments to their slots, from the last parameter
        // down: the k-th lies at slot k and goes to the k-th set bit,
        // never past one that is still to move
        let mut next = boxed;
        for slot in (0..params).rev() {
            if mask & (1 << slot) != 0 {
                next -= 1;
                if next != slot {
                    let from = self.b.ins().iadd_imm_s(first, (next * SIZE) as i64);
                    let to = self.b.ins().iadd_imm_s(first, (slot * SIZE) as i64);
                    self.copy_value(from, to);
                }
            }
        }
        // the frame record
        let frames = self.frames_vector();
        let frames_len = self.b.ins().load(pointer, flags, frames, 8);
        let frames_ptr = self.b.ins().load(pointer, flags, frames, 0);
        let record = self.frame_address(frames_ptr, frames_len);
        let code_value = self.usize(callee);
        self.b.ins().store(flags, code_value, record, FRAME_CODE);
        let zero = self.usize(0);
        self.b.ins().store(flags, zero, record, FRAME_PC);
        self.b.ins().store(flags, base, record, FRAME_BASE);
        let handlers = self.handlers_vector();
        let handlers_len = self.b.ins().load(pointer, flags, handlers, 8);
        self.b
            .ins()
            .store(flags, handlers_len, record, FRAME_HANDLER_BASE);
        self.b.ins().store(flags, grant, record, FRAME_GRANT);
        let more = self.b.ins().iadd_imm_s(frames_len, 1);
        self.b.ins().store(flags, more, frames, 8);
        (base, grant)
    }

    /// The frame left in place (decision AR4): every local that may hold
    /// a reference at the op (`owned_slots`, decision AW4: a boxed slot
    /// moved out or updated in place holds `Nothing` and is skipped) and
    /// the `operands` boxed operands above the locals are released, the
    /// stack is cut to the base, the handlers to the frame's height, and
    /// the frame is popped. A typed local holds `Nothing` or a plain
    /// value and needs no release. A read-only parameter is released only
    /// when the frame owns it (decision AW5: the caller's bit in
    /// `borrowed_params` says it was passed borrowed, and the caller's
    /// slot keeps the reference); the leave through `rt_truncate` gives
    /// the borrowed parameters a reference first.
    fn leave_frame_inline(&mut self, operands: usize, pc: usize) {
        let flags = MemFlagsData::trusted();
        let pointer = self.pointer;
        let locals = self.frame_locals;
        // the slots of the expanded callees hold `Nothing` at a return of
        // the code object's own (every region leaves by its exit, which
        // moves them out; decision AU50): only its own locals may hold a
        // reference, a pair slot in its two cells (decision AW9)
        let mut holders: Vec<usize> = Vec::new();
        for slot in 0..self.original_locals {
            match self.analysis.slots[slot] {
                SlotKind::Int | SlotKind::Bool | SlotKind::Float => {}
                SlotKind::Boxed(_) => {
                    if self.owned[pc][slot] {
                        holders.push(slot);
                    }
                }
                SlotKind::Pair(_) => {
                    if self.owned[pc][slot] {
                        holders.push(slot);
                        holders.push(self.pair_cells[slot].expect("a pair slot"));
                    }
                }
                _ => holders.push(slot),
            }
        }
        if holders.len() + operands <= RELEASES_INLINE {
            for slot in holders {
                let at = self.address_at(slot);
                if self.may_be_borrowed(slot) {
                    let owned = self.b.create_block();
                    let next = self.b.create_block();
                    let bit = self.borrowed_bit(slot);
                    self.b.ins().brif(bit, next, &[], owned, &[]);
                    self.b.seal_block(owned);
                    self.switch_to(owned);
                    self.release(at);
                    self.b.ins().jump(next, &[]);
                    self.b.seal_block(next);
                    self.switch_to(next);
                } else {
                    self.release(at);
                }
            }
            for operand in 0..operands {
                let at = self.address_at(locals + operand);
                self.release(at);
            }
            self.store_height(0);
        } else {
            self.retain_params();
            let zero = self.u32(0);
            self.call("rt_truncate", &[self.vm, self.base, zero, zero]);
        }
        let frames = self.frames_vector();
        let frames_len = self.b.ins().load(pointer, flags, frames, 8);
        let top = self.b.ins().iadd_imm_s(frames_len, -1);
        let frames_ptr = self.b.ins().load(pointer, flags, frames, 0);
        let record = self.frame_address(frames_ptr, top);
        let handler_base = self
            .b
            .ins()
            .load(pointer, flags, record, FRAME_HANDLER_BASE);
        let handlers = self.handlers_vector();
        self.b.ins().store(flags, handler_base, handlers, 8);
        self.b.ins().store(flags, top, frames, 8);
    }

    /// Whether the parameter in `slot` may have been passed borrowed
    /// (decision AW5): among those `accepted_params` names.
    fn may_be_borrowed(&self, slot: usize) -> bool {
        slot < 32 && self.flagged & (1 << slot) != 0
    }

    /// The bit of `borrowed_params` for the parameter in `slot`: non-zero
    /// when the caller passed it borrowed.
    fn borrowed_bit(&mut self, slot: usize) -> IrValue {
        self.b.ins().band_imm_s(self.borrowed_params, 1 << slot)
    }

    /// The parameters the caller passed borrowed given a reference each
    /// (decision AW5), before a path that treats every slot of the frame
    /// as owned: a hand-back, a leave through a helper, an interrupt, the
    /// leave through `rt_truncate`. Nothing when no parameter may be
    /// borrowed.
    fn retain_params(&mut self) {
        if self.flagged != 0 {
            self.call(
                "rt_retain_params",
                &[self.vm, self.base, self.borrowed_params],
            );
        }
    }

    /// The arguments of the call at `pc` that were pushed borrowed
    /// (decision AW5) given a reference each, before the call goes through
    /// the VM's path, which takes its arguments owned.
    fn retain_borrowed_arguments(&mut self, pc: usize, count: usize) {
        let mask = self.masks[pc];
        if mask == 0 {
            return;
        }
        let first = self.state.len() - count;
        for index in 0..count {
            if mask & (1 << index) != 0 {
                let at = self.operand_address(first + index);
                self.retain(at);
            }
        }
    }

    /// The boxed result on top of the stack returned (decision AR4): it
    /// moves out, the frame is left, and it goes where the frame's first
    /// local was; the status is `D_FAILURE` for a failure, else
    /// `D_BOXED`.
    fn leave_boxed_inline(&mut self, operands: usize, pc: usize) -> IrValue {
        let flags = MemFlagsData::trusted();
        // the result lies above the state's height: the caller popped it
        // from the state, not from the stack
        let height = self.height();
        let from = self.address_at(height);
        let words = [
            self.b.ins().load(types::I64, flags, from, 0),
            self.b.ins().load(types::I64, flags, from, 8),
            self.b.ins().load(types::I64, flags, from, 16),
        ];
        self.store_height(height);
        self.leave_frame_inline(operands, pc);
        // the result's slot was the frame's: the stack has the room, and
        // nothing moved it
        let to = self.address_at(0);
        for (word, offset) in words.into_iter().zip([0, 8, 16]) {
            self.b.ins().store(flags, word, to, offset);
        }
        self.store_height(1);
        let tag = self.b.ins().band_imm_s(words[0], 0xff);
        let failed = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, tag, TAG_FAILURE as i64);
        let failure = self.iconst(types::I32, D_FAILURE as i64);
        let boxed = self.iconst(types::I32, D_BOXED as i64);
        self.b.ins().select(failed, failure, boxed)
    }

    // ------------------------------------------------------------ the body

    fn prologue(&mut self) {
        // room on the stack for the frame's locals and its deepest operand
        // stack, checked once (decision AR4): every push of the body then
        // has it, and a frame handed over at a loop header has at most
        // that much on the stack
        let flags = MemFlagsData::trusted();
        let stack = self.stack_vector();
        let cap = self.b.ins().load(self.pointer, flags, stack, 16);
        let room = (self.frame_locals + self.max_depth + 1) as i64;
        let needed = self.b.ins().iadd_imm_s(self.base, room);
        let short = self.b.ins().icmp(IntCC::UnsignedLessThan, cap, needed);
        let grow = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(short, grow, &[], cont, &[]);
        self.b.set_cold_block(grow);
        self.switch_to(grow);
        self.call("rt_room", &[self.vm, needed]);
        self.b.ins().jump(cont, &[]);
        self.switch_to(cont);
        self.reload_frame();
        // room in the frames for the record a direct call of this body
        // pushes (decision AW6), checked once: the frames' length is the
        // same at every such call, this frame on top, and the capacity
        // never falls
        if self.calls_directly {
            let frames = self.frames_vector();
            let frames_len = self.b.ins().load(self.pointer, flags, frames, 8);
            let frames_cap = self.b.ins().load(self.pointer, flags, frames, 16);
            let full = self.b.ins().icmp(IntCC::Equal, frames_len, frames_cap);
            let grow = self.b.create_block();
            let cont = self.b.create_block();
            self.b.ins().brif(full, grow, &[], cont, &[]);
            self.b.set_cold_block(grow);
            self.switch_to(grow);
            self.call("rt_grow_frames", &[self.vm]);
            self.b.ins().jump(cont, &[]);
            self.switch_to(cont);
        }
        // the stack's length is `base` plus the code object's own locals
        // as the caller left it; the expanded code's locals go on past
        // them (decision AU50), as the pairs' second cells do (decision
        // AW9), and the length follows before the first helper pushes at
        // it: in the entries at the loop headers through
        // `make_room_for_expansion`, once their checks have passed (an
        // entry refused leaves the frame as the interpreter had it), and
        // on the start path below through the height alone
        // the entries at the loop headers are tried first: `pc` names one
        // when the interpreter hands a frame over in a loop
        let headers = self.headers.clone();
        let mut resumes = Vec::new();
        for header in headers {
            let here = self
                .b
                .ins()
                .icmp_imm_s(IntCC::Equal, self.pc_param, header as i64);
            let resume = self.b.create_block();
            let next = self.b.create_block();
            self.b.ins().brif(here, resume, &[], next, &[]);
            self.b.seal_block(resume);
            self.b.seal_block(next);
            resumes.push((header, resume));
            self.switch_to(next);
        }
        // the stack's length takes in the locals (decision AW6): a direct
        // caller left it at the arguments, having checked nothing for the
        // locals, which the room above covers; the interpreter's frame and
        // the template tier's have it so already
        self.store_height(self.frame_locals);
        // the typed parameters, in the registers the signature put them in
        // (the trampoline took them from the frame's slots under a check,
        // a direct caller had them in registers)
        let typed = self.typed_params.clone();
        for (slot, value) in &typed {
            let var = self.slot_var(*slot);
            self.b.def_var(var, *value);
        }
        // every local that is not a boxed parameter starts as `Nothing`
        // (decision AT2): a direct caller leaves those slots as they were,
        // written here once per callee rather than at every call site; the
        // interpreter's frame has them so already, and a typed parameter's
        // slot is written too, its value being in a register now (a plain
        // value, with nothing to release); the pairs' second cells too
        // (decision AW9)
        let params = self.code.params as usize;
        let locals = self.frame_locals;
        let fresh: Vec<usize> = (0..locals)
            .filter(|slot| *slot >= params || typed.iter().any(|(typed, _)| typed == slot))
            .collect();
        if !fresh.is_empty() {
            let nothing = self.iconst(types::I32, TAG_NOTHING as i64);
            for slot in fresh {
                let at = self.address_at(slot);
                self.b.ins().istore8(flags, nothing, at, 0);
            }
        }
        self.jump_to(0);
        for (header, block) in resumes {
            self.switch_to(block);
            self.resume_at(header);
        }
    }

    /// A frame the interpreter hands over at a loop header has the code
    /// object's own locals and its operands right above them; the
    /// expanded code keeps the slots of its expanded callees between the
    /// two (decision AU50), and a body with pair slots the cells of their
    /// second fields (decision AW9), so the operands move up past them,
    /// which hold `Nothing` outside the regions, and the stack's length
    /// follows.
    fn make_room_for_expansion(&mut self, header: usize) {
        let original = self.original_locals;
        let expanded = self.frame_locals;
        if expanded == original {
            return;
        }
        let depth = self.analysis.entry[header].as_ref().map_or(0, Vec::len);
        let flags = MemFlagsData::trusted();
        for index in (0..depth).rev() {
            let from = self.address_at(original + index);
            let to = self.address_at(expanded + index);
            self.copy_value(from, to);
        }
        let nothing = self.iconst(types::I32, TAG_NOTHING as i64);
        for slot in original..expanded {
            let at = self.address_at(slot);
            self.b.ins().istore8(flags, nothing, at, 0);
        }
        self.store_height(expanded + depth);
    }

    /// Continue when `ok` (an `i8`), else leave the body with the status
    /// and the frame untouched.
    fn leave_unless(&mut self, ok: IrValue, status: i32) {
        let cont = self.b.create_block();
        let leave = self.b.create_block();
        self.b.ins().brif(ok, cont, &[], leave, &[]);
        self.b.set_cold_block(leave);
        self.b.seal_block(cont);
        self.b.seal_block(leave);
        self.switch_to(leave);
        self.return_direct(status);
        self.switch_to(cont);
    }

    /// The entry at a loop header (decision AG3): every slot the code
    /// keeps in registers is taken from the frame as the interpreter left
    /// it, under a check; one that fails returns `STAY`, and the
    /// interpreter carries on with the frame as it is. A pair slot that
    /// holds its pair at the header (decision AW9) has the interpreter's
    /// record in the slot, checked to be a plain record of the type or
    /// `Nothing` before anything moves; then the room past the locals is
    /// made, and every pair slot is taken apart into its cells, or
    /// cleared where it holds no pair.
    fn resume_at(&mut self, header: usize) {
        let out = self.out_address();
        let analysis = self.analysis;
        // the slots of expanded callees hold `Nothing` at a loop header of
        // the caller (decision AU50): nothing to take from them
        for (slot, &kind) in analysis.slots.iter().enumerate().take(self.original_locals) {
            let slot_value = self.u32(slot as u32);
            match kind {
                SlotKind::Int | SlotKind::Bool | SlotKind::Float => {
                    let (helper, ty) = match kind {
                        SlotKind::Int => ("rt_resume_int", types::I64),
                        SlotKind::Bool => ("rt_resume_bool", types::I8),
                        _ => ("rt_resume_float", types::F64),
                    };
                    let ok = self
                        .call(helper, &[self.vm, self.base, slot_value, out])
                        .expect("an answer");
                    self.leave_unless(ok, D_STAY);
                    let value = self.out_read(ty);
                    let var = self.slot_var(slot);
                    self.b.def_var(var, value);
                }
                SlotKind::RangeIter => {
                    let ok = self
                        .call("rt_resume_range", &[self.vm, self.base, slot_value, out])
                        .expect("an answer");
                    self.leave_unless(ok, D_STAY);
                    let vars = self.slot_vars3(slot);
                    for (index, var) in vars.into_iter().enumerate() {
                        let value = self.b.ins().stack_load(
                            self.pointer,
                            types::I64,
                            self.out_slot,
                            (index * 8) as i32,
                        );
                        self.b.def_var(var, value);
                    }
                }
                _ => {}
            }
        }
        let pairs: Vec<(usize, u32, bool)> = analysis
            .slots
            .iter()
            .enumerate()
            .take(self.original_locals)
            .filter_map(|(slot, kind)| match kind {
                SlotKind::Pair(ty) => Some((slot, *ty, self.owned[header][slot])),
                _ => None,
            })
            .collect();
        for &(slot, ty, owned) in &pairs {
            if !owned {
                continue;
            }
            let slot_value = self.u32(slot as u32);
            let ty_value = self.usize(ty as usize);
            let ok = self
                .call("rt_pair_fits", &[self.vm, self.base, slot_value, ty_value])
                .expect("an answer");
            self.leave_unless(ok, D_STAY);
        }
        self.make_room_for_expansion(header);
        for &(slot, ty, owned) in &pairs {
            let extra = self.pair_cells[slot].expect("a pair slot");
            let slot_value = self.u32(slot as u32);
            let extra_value = self.u32(extra as u32);
            let ty_value = self.usize(ty as usize);
            let owned_value = self.u8(u8::from(owned));
            self.call(
                "rt_pair_resume",
                &[
                    self.vm,
                    self.base,
                    slot_value,
                    extra_value,
                    ty_value,
                    owned_value,
                ],
            );
        }
        self.state = self.analysis.entry[header]
            .clone()
            .expect("a loop header is reachable");
        self.jump_to(header);
    }

    fn epilogue(&mut self) {
        self.emit_deopts();
        self.emit_landings();
        // the end of the code: an implicit `return nothing`
        let count = self.code.ops.len();
        if let Some(block) = self.blocks.get(&count).copied() {
            self.switch_to(block);
            self.call("rt_return_nothing", &[self.vm]);
            self.return_direct(D_BOXED);
        }
        // the exit: `LEFT` is a return with the value on the caller's
        // stack (a failure or not), anything else an interrupt
        let exit = self.exit;
        self.switch_to(exit);
        let status = self.b.block_params(exit)[0];
        let left = self.b.ins().icmp_imm_s(IntCC::Equal, status, LEFT as i64);
        let left_block = self.b.create_block();
        let other = self.b.create_block();
        self.b.ins().brif(left, left_block, &[], other, &[]);
        self.switch_to(left_block);
        let status = self.call("rt_left_status", &[self.vm]).expect("a status");
        let zero = self.iconst(types::I64, 0);
        self.return_with(status, zero);
        self.switch_to(other);
        self.retain_params();
        self.return_direct(D_INTERRUPT);
    }

    fn body(&mut self) {
        let count = self.code.ops.len();
        let mut unreachable = false;
        for pc in 0..count {
            // a jump target, or the op a branch falls through to, which got
            // its block when the branch was made
            let starts = self.analysis.block_starts[pc] || self.blocks.contains_key(&pc);
            if starts {
                if !self.terminated {
                    self.jump_to(pc);
                }
                match &self.analysis.entry[pc] {
                    Some(state) => {
                        self.state = state.clone();
                        let block = self.block_for(pc);
                        self.switch_to(block);
                        unreachable = false;
                    }
                    None => {
                        unreachable = true;
                    }
                }
            } else if self.terminated {
                // an op after a terminator that nothing jumps to
                unreachable = true;
            }
            if unreachable {
                continue;
            }
            // the kinds must agree with the analysis; the types may not: the
            // analysis joins the states of every round of its fixed point
            // and loses a type where a kind changed, while the generator
            // keeps the type of what the op before pushed
            debug_assert_eq!(
                self.state
                    .iter()
                    .map(|abs| abs.untyped())
                    .collect::<Vec<_>>(),
                self.analysis.entry[pc]
                    .as_ref()
                    .map(|state| state.iter().map(|abs| abs.untyped()).collect::<Vec<_>>())
                    .unwrap_or_default(),
                "the state at {pc} of {}",
                self.code.name
            );
            self.op(pc);
        }
        if !self.terminated {
            self.jump_to(count);
        }
    }

    fn op(&mut self, pc: usize) {
        self.current_pc = pc;
        let pc_value = self.u32(pc as u32);
        let code_value = self.usize(self.code_id);
        match &self.code.ops[pc] {
            Op::Const(index) => {
                let constant = &self.code.constants[*index as usize];
                match abs_of_constant(constant) {
                    Abs::Int => {
                        let Value::Integer(Int::Small(value)) = constant else {
                            unreachable!()
                        };
                        let value = self.iconst(types::I64, *value);
                        self.push_int(value);
                    }
                    Abs::Bool => {
                        let Value::Boolean(value) = constant else {
                            unreachable!()
                        };
                        let value = self.u8(*value as u8);
                        self.push_bool(value);
                    }
                    Abs::Float => {
                        let Value::Float(value) = constant else {
                            unreachable!()
                        };
                        let value = self.b.ins().f64const(*value);
                        self.push_float(value);
                    }
                    _ => {
                        // the constant copied from the code's table, which
                        // lives as long as the program, with one more
                        // reference
                        let state = self.native_state();
                        let table = self.b.ins().load(
                            self.pointer,
                            MemFlagsData::trusted(),
                            state,
                            STATE_CONSTANTS,
                        );
                        let base = self.b.ins().load(
                            self.pointer,
                            MemFlagsData::trusted(),
                            table,
                            (self.code_id * 8) as i32,
                        );
                        let from = self
                            .b
                            .ins()
                            .iadd_imm_s(base, (*index as usize * SIZE) as i64);
                        let borrowed = self.borrowed[pc];
                        self.push_copy(from, !borrowed);
                        self.push_boxed();
                    }
                }
            }
            Op::Nothing => {
                let height = self.height();
                self.push_nothing_at(height);
                self.push_boxed();
            }
            Op::Global(index) => {
                if let Some(Some(value)) = self.globals.get(*index as usize) {
                    // evaluated to a small Integer already: the literal
                    // (decision AU53)
                    let value = self.iconst(types::I64, *value);
                    self.push_int(value);
                } else {
                    // the constant copied from the JIT's table once the
                    // VM has built it, with one more reference (decision
                    // AU53); until then built through the helper
                    let flags = MemFlagsData::trusted();
                    let state = self.native_state();
                    let table = self.b.ins().load(self.pointer, flags, state, STATE_GLOBALS);
                    let slot = self
                        .b
                        .ins()
                        .iadd_imm_s(table, (*index as usize * GLOBAL_SIZE) as i64);
                    let ready = self.b.ins().load(types::I64, flags, slot, GLOBAL_READY);
                    let built = self.b.create_block();
                    let unbuilt = self.b.create_block();
                    let done = self.b.create_block();
                    self.b.set_cold_block(unbuilt);
                    self.b.ins().brif(ready, built, &[], unbuilt, &[]);
                    self.b.seal_block(built);
                    self.b.seal_block(unbuilt);
                    self.switch_to(built);
                    let from = self.b.ins().iadd_imm_s(slot, GLOBAL_VALUE as i64);
                    self.push_copy(from, true);
                    self.b.ins().jump(done, &[]);
                    self.switch_to(unbuilt);
                    let index = self.u32(*index);
                    let status = self
                        .call("rt_global", &[self.vm, index, pc_value])
                        .expect("a status");
                    self.check_status(status, pc, false);
                    self.b.ins().jump(done, &[]);
                    self.b.seal_block(done);
                    self.switch_to(done);
                    self.push_boxed();
                }
            }
            Op::Load(slot) | Op::LoadMove(slot) => {
                let index = *slot as usize;
                match self.analysis.slots[index] {
                    SlotKind::Int => {
                        let var = self.slot_var(index);
                        let value = self.b.use_var(var);
                        self.push_int(value);
                    }
                    SlotKind::Bool => {
                        let var = self.slot_var(index);
                        let value = self.b.use_var(var);
                        self.push_bool(value);
                    }
                    SlotKind::Float => {
                        let var = self.slot_var(index);
                        let value = self.b.use_var(var);
                        self.push_float(value);
                    }
                    SlotKind::Pair(ty) => {
                        // the pair's two cells pushed (decision AW9), copied
                        // with a reference each or moved at the last read (a
                        // pair slot is never a parameter, so never borrowed),
                        // as they lie for the op after, else as the record
                        let moving = matches!(self.code.ops[pc], Op::LoadMove(_));
                        self.push_pair_from_slot(index, moving);
                        if self.pairs && pair_fused(self.code, pc) {
                            self.state.push(Abs::Pair(ty));
                        } else {
                            self.pair_record_on_top(ty);
                            self.push_boxed();
                        }
                    }
                    _ => {
                        // the slot's value pushed in place: a copy with one
                        // more reference, the move that leaves `Nothing`, or,
                        // for an operand a typed call or a comparison borrows
                        // (a move or not), a copy without one, the slot
                        // keeping its value
                        let moving =
                            matches!(self.code.ops[pc], Op::LoadMove(_)) && !self.borrowed[pc];
                        let from = self.address_at(index);
                        let to = self.top_address();
                        self.copy_value(from, to);
                        if moving && self.may_be_borrowed(index) {
                            // a read-only parameter (decision AW5): moved
                            // when the frame owns it, copied with a
                            // reference of its own when the caller passed
                            // it borrowed
                            let lent = self.b.create_block();
                            let owned = self.b.create_block();
                            let done = self.b.create_block();
                            let bit = self.borrowed_bit(index);
                            self.b.ins().brif(bit, lent, &[], owned, &[]);
                            self.b.seal_block(lent);
                            self.b.seal_block(owned);
                            self.switch_to(owned);
                            let nothing = self.iconst(types::I32, TAG_NOTHING as i64);
                            self.b
                                .ins()
                                .istore8(MemFlagsData::trusted(), nothing, from, 0);
                            self.b.ins().jump(done, &[]);
                            self.switch_to(lent);
                            self.retain(to);
                            self.b.ins().jump(done, &[]);
                            self.b.seal_block(done);
                            self.switch_to(done);
                        } else if moving {
                            let nothing = self.iconst(types::I32, TAG_NOTHING as i64);
                            self.b
                                .ins()
                                .istore8(MemFlagsData::trusted(), nothing, from, 0);
                        } else if !self.borrowed[pc] {
                            self.retain(to);
                        }
                        let height = self.height() + 1;
                        self.store_height(height);
                        self.push_boxed();
                    }
                }
            }
            Op::Store(slot) => {
                let index = *slot as usize;
                let mut top = *self.state.last().expect("an operand");
                if let (SlotKind::Boxed(_), Abs::Pair(ty)) = (self.analysis.slots[index], top) {
                    // a slot that holds other things too takes the record
                    // (decision AW9)
                    self.pair_record_on_top(ty);
                    top = Abs::Boxed(None);
                    *self.state.last_mut().expect("an operand") = top;
                }
                match (self.analysis.slots[index], top) {
                    (SlotKind::Pair(_), Abs::Pair(_)) => {
                        // the pair's cells moved into the slot and its second
                        // cell (decision AW9), whose old values go
                        let extra = self.pair_cells[index].expect("a pair slot");
                        let height = self.height();
                        for (part, to_height) in [index, extra].into_iter().enumerate() {
                            let from = self.address_at(height - 2 + part);
                            let to = self.address_at(to_height);
                            self.release(to);
                            self.copy_value(from, to);
                        }
                        self.store_height(height - 2);
                        self.state.pop();
                    }
                    (SlotKind::Pair(_), _) => unreachable!("a pair slot takes pairs alone"),
                    (SlotKind::Int, Abs::Int) => {
                        let value = self.pop_int();
                        let var = self.slot_var(index);
                        self.b.def_var(var, value);
                    }
                    (SlotKind::Bool, Abs::Bool) => {
                        let value = self.pop_bool();
                        let var = self.slot_var(index);
                        self.b.def_var(var, value);
                    }
                    (SlotKind::Float, Abs::Float) => {
                        let value = self.pop_float();
                        let var = self.slot_var(index);
                        self.b.def_var(var, value);
                    }
                    (SlotKind::Boxed(_), Abs::Int | Abs::Bool | Abs::Float) => {
                        // the register boxed into the slot, whose old value
                        // goes
                        let value = match top {
                            Abs::Int => self.pop_int(),
                            Abs::Bool => self.pop_bool(),
                            _ => self.pop_float(),
                        };
                        let to = self.address_at(index);
                        self.release(to);
                        self.write_typed(to, top, value);
                    }
                    _ => {
                        // the top moved into the slot, whose old value goes
                        self.box_top(1);
                        let from = self.last_address();
                        let to = self.address_at(index);
                        self.release(to);
                        self.copy_value(from, to);
                        let height = self.height() - 1;
                        self.store_height(height);
                        self.state.pop();
                    }
                }
            }
            Op::Pop => {
                if self.state.last().is_some_and(|abs| abs.is_boxed()) {
                    let at = self.last_address();
                    self.release(at);
                    let height = self.height() - 1;
                    self.store_height(height);
                }
                self.state.pop();
            }
            Op::Dup => {
                let depth = self.state.len() - 1;
                match self.state[depth] {
                    Abs::Int => {
                        let var = self.int_var(depth);
                        let value = self.b.use_var(var);
                        self.push_int(value);
                    }
                    Abs::Bool => {
                        let var = self.bool_var(depth);
                        let value = self.b.use_var(var);
                        self.push_bool(value);
                    }
                    Abs::Float => {
                        let var = self.float_var(depth);
                        let value = self.b.use_var(var);
                        self.push_float(value);
                    }
                    Abs::Range => {
                        let vars = self.range_vars(depth);
                        let values = [
                            self.b.use_var(vars[0]),
                            self.b.use_var(vars[1]),
                            self.b.use_var(vars[2]),
                        ];
                        let copy = self.range_vars(depth + 1);
                        for (var, value) in copy.iter().zip(values) {
                            self.b.def_var(*var, value);
                        }
                        self.state.push(Abs::Range);
                    }
                    _ => {
                        // the top copied with one more reference, its type
                        // with it
                        let from = self.last_address();
                        let to = self.top_address();
                        self.copy_value(from, to);
                        self.retain(to);
                        let height = self.height() + 1;
                        self.store_height(height);
                        let top = self.state[depth];
                        self.state.push(top);
                    }
                }
            }
            Op::MakeList(count) => {
                let count_value = self.u32(*count as u32);
                self.helper_on_stack("rt_make_list", *count as usize, &[count_value]);
                self.push_boxed();
            }
            Op::MakeMap(count) => {
                let count_value = self.u32(*count as u32);
                self.helper_on_stack("rt_make_map", 2 * *count as usize, &[count_value]);
                self.push_boxed();
            }
            Op::MakePair => {
                self.helper_on_stack("rt_make_pair", 2, &[]);
                self.push_boxed();
            }
            Op::MakeRange { stepped } => {
                let n = self.state.len();
                let operands = if *stepped { 3 } else { 2 };
                let all_int = self.state[n - operands..]
                    .iter()
                    .all(|abs| *abs == Abs::Int);
                if all_int {
                    let by = if *stepped {
                        self.pop_int()
                    } else {
                        self.iconst(types::I64, 1)
                    };
                    let to = self.pop_int();
                    let from = self.pop_int();
                    let depth = self.state.len();
                    let vars = self.range_vars(depth);
                    self.b.def_var(vars[0], from);
                    self.b.def_var(vars[1], to);
                    self.b.def_var(vars[2], by);
                    self.state.push(Abs::Range);
                } else {
                    let stepped_value = self.u8(*stepped as u8);
                    let status = self
                        .helper_on_stack("rt_make_range", operands, &[stepped_value, pc_value])
                        .expect("a status");
                    self.push_boxed();
                    self.check_status(status, pc, false);
                }
            }
            Op::Construct { ty, fields } => {
                if *fields == 2
                    && self.pair_result == Some(*ty as u32)
                    && matches!(self.code.ops.get(pc + 1), Some(Op::Return))
                    && !self.analysis.block_starts[pc + 1]
                {
                    // the record returned at once (decision AW9): its two
                    // fields go back as they lie, the `Return` after this
                    // op being the record's only reader
                    self.pair_return(*ty as u32, 2, pc);
                    return;
                }
                let ty_value = self.usize(*ty);
                let fields_value = self.u32(*fields as u32);
                if plain_record_type(self.program, *ty) {
                    // a plain record made in place cannot fail: no status
                    // to test and no pc (decision AW12)
                    self.helper_on_stack(
                        "rt_construct_plain",
                        *fields as usize,
                        &[ty_value, fields_value],
                    );
                    self.push_boxed();
                    return;
                }
                let status = self
                    .helper_on_stack(
                        "rt_construct",
                        *fields as usize,
                        &[ty_value, fields_value, pc_value],
                    )
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, true);
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
                let flags = MemFlagsData::trusted();
                let pointer = self.pointer;
                let units = self.b.ins().iadd_imm_s(self.vm, unit_variants_offset());
                let units_ptr = self.b.ins().load(pointer, flags, units, 0);
                let unit = self.b.ins().iadd_imm_s(units_ptr, (slot * SIZE) as i64);
                let tag_value = self.tag_at(unit, 0);
                let built = self.b.create_block();
                let slow = self.b.create_block();
                let join = self.b.create_block();
                let is_built =
                    self.b
                        .ins()
                        .icmp_imm_s(IntCC::NotEqual, tag_value, TAG_NOTHING as i64);
                self.b.ins().brif(is_built, built, &[], slow, &[]);
                self.b.seal_block(built);
                self.b.seal_block(slow);
                self.switch_to(built);
                let to = self.top_address();
                self.copy_value(unit, to);
                let rc = self.b.ins().load(pointer, flags, to, PAYLOAD);
                let count = self.b.ins().load(types::I64, flags, rc, 0);
                let count = self.b.ins().iadd_imm_s(count, 1);
                self.b.ins().store(flags, count, rc, 0);
                let height = self.height() + 1;
                self.store_height(height);
                self.b.ins().jump(join, &[]);
                self.switch_to(slow);
                let ty_value = self.usize(*ty);
                let tag_value = self.u32(*tag as u32);
                let fields_value = self.u32(0);
                let status = self
                    .helper_on_stack(
                        "rt_construct_variant",
                        0,
                        &[ty_value, tag_value, fields_value, pc_value],
                    )
                    .expect("a status");
                self.check_status(status, pc, false);
                self.b.ins().jump(join, &[]);
                self.b.seal_block(join);
                self.switch_to(join);
                self.push_boxed();
            }
            Op::ConstructVariant { ty, tag, fields } => {
                let ty_value = self.usize(*ty);
                let tag_value = self.u32(*tag as u32);
                let fields_value = self.u32(*fields as u32);
                if *fields > 0 && plain_variant_type(self.program, *ty) {
                    // a variant with fields of a type without refinements,
                    // made in place, cannot fail: no status to test and no
                    // pc (decision AW12)
                    self.helper_on_stack(
                        "rt_construct_variant_plain",
                        *fields as usize,
                        &[ty_value, tag_value, fields_value],
                    );
                    self.push_boxed();
                    return;
                }
                let status = self
                    .helper_on_stack(
                        "rt_construct_variant",
                        *fields as usize,
                        &[ty_value, tag_value, fields_value, pc_value],
                    )
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, true);
            }
            Op::Field { name, site } => {
                let field_name = self.code.constants[*name as usize]
                    .as_text()
                    .unwrap_or("")
                    .to_string();
                if let Some(Abs::Pair(ty)) = self.state.last().copied() {
                    match self.program.types.field_index(ty as usize, &field_name) {
                        Some(index) => {
                            // the field of a pair as it lies (decision AW9):
                            // the other cell released, the field left in the
                            // lower cell
                            self.pair_field_on_top(index);
                            self.state.pop();
                            self.push_boxed();
                            let kind = abs_of_field_at(self.program, self.code, pc, &field_name);
                            if !kind.is_boxed() {
                                self.unbox_top(kind, pc + 1);
                            }
                            return;
                        }
                        None => {
                            self.pair_record_on_top(ty);
                            *self.state.last_mut().expect("an operand") = Abs::Boxed(None);
                        }
                    }
                }
                let name_value = self.u32(*name);
                let site_value = self.u32(*site);
                let fixed = self
                    .state
                    .last()
                    .and_then(|abs| abs.type_index())
                    .and_then(|ty| static_field(self.program, ty, &field_name));
                if let Some((ty_id, index)) = fixed {
                    // the holder's type is known (decision AU8): a record of
                    // it holds the field at this index, so the tag and the
                    // type are checked and the field takes the holder's
                    // place on the stack, its words read and one more
                    // reference to it before the holder goes (which may free
                    // the record); anything else goes through the helper
                    let flags = MemFlagsData::trusted();
                    let pointer = self.pointer;
                    let at = self.last_address();
                    let tag = self.tag_at(at, 0);
                    let holder = self.b.create_block();
                    let hit = self.b.create_block();
                    let slow = self.b.create_block();
                    let join = self.b.create_block();
                    let kind = self.b.ins().iadd_imm_s(tag, -(TAG_RECORD as i64));
                    let is_holder = self.b.ins().icmp_imm_u(IntCC::UnsignedLessThan, kind, 2);
                    self.b.ins().brif(is_holder, holder, &[], slow, &[]);
                    self.switch_to(holder);
                    let rc = self.b.ins().load(pointer, flags, at, PAYLOAD);
                    let ty = self.b.ins().load(types::I64, flags, rc, RECORD_TY);
                    let same_ty = self.b.ins().icmp_imm_s(IntCC::Equal, ty, ty_id as i64);
                    self.b.ins().brif(same_ty, hit, &[], slow, &[]);
                    self.switch_to(hit);
                    let fields_ptr = self.b.ins().iadd_imm_s(rc, RECORD_FIELDS as i64);
                    let field = self.b.ins().iadd_imm_s(fields_ptr, (index * SIZE) as i64);
                    let words: Vec<IrValue> = [0, 8, 16]
                        .into_iter()
                        .map(|offset| self.b.ins().load(types::I64, flags, field, offset))
                        .collect();
                    self.retain(field);
                    self.release(at);
                    for (offset, word) in [0, 8, 16].into_iter().zip(words) {
                        self.b.ins().store(flags, word, at, offset);
                    }
                    self.b.ins().jump(join, &[]);
                    self.switch_to(slow);
                    let status = self
                        .helper_on_stack(
                            "rt_field",
                            1,
                            &[code_value, name_value, site_value, pc_value],
                        )
                        .expect("a status");
                    self.check_status(status, pc, false);
                    self.b.ins().jump(join, &[]);
                    self.switch_to(join);
                    self.push_boxed();
                } else {
                    let status = self
                        .helper_on_stack(
                            "rt_field",
                            1,
                            &[code_value, name_value, site_value, pc_value],
                        )
                        .expect("a status");
                    self.push_boxed();
                    self.check_status(status, pc, false);
                }
                let kind = abs_of_field_at(self.program, self.code, pc, &field_name);
                if !kind.is_boxed() {
                    self.unbox_top(kind, pc + 1);
                }
            }
            Op::LoadField { slot, name, site } => {
                let field_name = self.code.constants[*name as usize]
                    .as_text()
                    .unwrap_or("")
                    .to_string();
                if let SlotKind::Pair(ty) = self.analysis.slots[*slot as usize] {
                    // a field of a pair slot copied from its cell with one
                    // more reference (decision AW9)
                    match self.program.types.field_index(ty as usize, &field_name) {
                        Some(index) => {
                            let from_height = self.pair_cell(*slot as usize, index);
                            let from = self.address_at(from_height);
                            self.push_copy(from, true);
                        }
                        None => {
                            self.pair_field_through_helper(*slot as usize, ty, *name, *site, pc)
                        }
                    }
                    self.push_boxed();
                    let kind = abs_of_field_at(self.program, self.code, pc, &field_name);
                    if !kind.is_boxed() {
                        self.unbox_top(kind, pc + 1);
                    }
                    return;
                }
                let fixed = self.analysis.slot_types[*slot as usize]
                    .and_then(|ty| static_field(self.program, ty, &field_name));
                if let Some((ty_id, index)) = fixed {
                    // the holder's type is known (decision AU8): a record of
                    // it holds the field at this index, so the tag and the
                    // type are checked and the field read in place with one
                    // more reference, with no cache; anything else goes
                    // through the helper
                    let flags = MemFlagsData::trusted();
                    let pointer = self.pointer;
                    let at = self.address_at(*slot as usize);
                    let tag = self.tag_at(at, 0);
                    let holder = self.b.create_block();
                    let hit = self.b.create_block();
                    let slow = self.b.create_block();
                    let join = self.b.create_block();
                    let kind = self.b.ins().iadd_imm_s(tag, -(TAG_RECORD as i64));
                    let is_holder = self.b.ins().icmp_imm_u(IntCC::UnsignedLessThan, kind, 2);
                    self.b.ins().brif(is_holder, holder, &[], slow, &[]);
                    self.switch_to(holder);
                    let rc = self.b.ins().load(pointer, flags, at, PAYLOAD);
                    let ty = self.b.ins().load(types::I64, flags, rc, RECORD_TY);
                    let same_ty = self.b.ins().icmp_imm_s(IntCC::Equal, ty, ty_id as i64);
                    self.b.ins().brif(same_ty, hit, &[], slow, &[]);
                    self.switch_to(hit);
                    let fields_ptr = self.b.ins().iadd_imm_s(rc, RECORD_FIELDS as i64);
                    let field = self.b.ins().iadd_imm_s(fields_ptr, (index * SIZE) as i64);
                    self.push_copy(field, true);
                    self.b.ins().jump(join, &[]);
                    self.switch_to(slow);
                    let slot_value = self.u32(*slot as u32);
                    let name_value = self.u32(*name);
                    let site_value = self.u32(*site);
                    let status = self
                        .call(
                            "rt_load_field",
                            &[
                                self.vm, self.base, slot_value, code_value, name_value, site_value,
                                pc_value,
                            ],
                        )
                        .expect("a status");
                    self.check_status(status, pc, false);
                    self.b.ins().jump(join, &[]);
                    self.switch_to(join);
                } else {
                    // the site's cache entry and the holder read in place
                    // (decision AR4): a record or a variant of the cached type
                    // with the cached tag gives its field with one more
                    // reference; anything else goes through the helper, which
                    // fills the cache. A record and a variant hold the type
                    // first; the tag and the fields lie further in a variant,
                    // and a record's tag is `usize::MAX` as the cache has it
                    let flags = MemFlagsData::trusted();
                    let pointer = self.pointer;
                    let at = self.address_at(*slot as usize);
                    let tag = self.tag_at(at, 0);
                    let cache = self.b.ins().iadd_imm_s(self.vm, field_cache_offset());
                    let cache_ptr = self.b.ins().load(pointer, flags, cache, 0);
                    let entry = self
                        .b
                        .ins()
                        .iadd_imm_s(cache_ptr, (*site as usize * SITE_SIZE) as i64);
                    let cached_ty = self.b.ins().load(types::I64, flags, entry, SITE_TY);
                    let cached_tag = self.b.ins().load(types::I64, flags, entry, SITE_TAG);
                    let cached_index = self.b.ins().load(types::I64, flags, entry, SITE_INDEX);
                    let holder = self.b.create_block();
                    let hit = self.b.create_block();
                    self.b.append_block_param(hit, pointer);
                    let slow = self.b.create_block();
                    let join = self.b.create_block();
                    // a record or a variant: the two tags are adjacent, and
                    // both hold their type, their tag (a record's is
                    // `usize::MAX`, as the cache has it) and their fields at
                    // the same offsets (decision AT2), so one path reads either
                    let kind = self.b.ins().iadd_imm_s(tag, -(TAG_RECORD as i64));
                    let is_holder = self.b.ins().icmp_imm_u(IntCC::UnsignedLessThan, kind, 2);
                    self.b.ins().brif(is_holder, holder, &[], slow, &[]);
                    self.switch_to(holder);
                    let rc = self.b.ins().load(pointer, flags, at, PAYLOAD);
                    let ty = self.b.ins().load(types::I64, flags, rc, RECORD_TY);
                    let holder_tag = self.b.ins().load(types::I64, flags, rc, RECORD_TAG);
                    let fields_ptr = self.b.ins().iadd_imm_s(rc, RECORD_FIELDS as i64);
                    let same_ty = self.b.ins().icmp(IntCC::Equal, ty, cached_ty);
                    let same_tag = self.b.ins().icmp(IntCC::Equal, holder_tag, cached_tag);
                    // a hit needs no range check: the index was cached from a
                    // holder of the same type and tag, which has as many fields
                    let ok = self.b.ins().band(same_ty, same_tag);
                    let field = self.item_address(fields_ptr, cached_index);
                    self.b.ins().brif(ok, hit, &[field.into()], slow, &[]);
                    // the field copied, with one more reference
                    self.switch_to(hit);
                    let from = self.b.block_params(hit)[0];
                    self.push_copy(from, true);
                    self.b.ins().jump(join, &[]);
                    // the helper: by name, filling the cache
                    self.switch_to(slow);
                    let slot_value = self.u32(*slot as u32);
                    let name_value = self.u32(*name);
                    let site_value = self.u32(*site);
                    let status = self
                        .call(
                            "rt_load_field",
                            &[
                                self.vm, self.base, slot_value, code_value, name_value, site_value,
                                pc_value,
                            ],
                        )
                        .expect("a status");
                    self.check_status(status, pc, false);
                    self.b.ins().jump(join, &[]);
                    self.switch_to(join);
                }
                self.push_boxed();
                let kind = abs_of_field_at(self.program, self.code, pc, &field_name);
                if !kind.is_boxed() {
                    self.unbox_top(kind, pc + 1);
                }
            }
            Op::With(count) => {
                let n = *count as usize;
                match update_by_index(self.program, self.code, self.analysis, pc, n) {
                    Some((ty, packed)) => self.update_in_place(None, n, ty, packed, pc),
                    None => {
                        let count_value = self.u32(*count as u32);
                        let status = self
                            .helper_on_stack("rt_with", 2 * n + 1, &[count_value, pc_value])
                            .expect("a status");
                        self.push_boxed();
                        self.check_status(status, pc, true);
                    }
                }
            }
            Op::WithSlot { slot, fields } => {
                // the record moved out of the slot and updated in place
                // when it is held once (decision AU11): in the generated
                // code when the checker typed the record (decision AW8),
                // else through the helper
                let n = *fields as usize;
                match update_by_index(self.program, self.code, self.analysis, pc, n) {
                    Some((ty, packed)) => {
                        self.update_in_place(Some(*slot as usize), n, ty, packed, pc)
                    }
                    None => {
                        let slot_value = self.u32(*slot as u32);
                        let count_value = self.u32(*fields as u32);
                        let status = self
                            .helper_on_stack(
                                "rt_with_slot",
                                2 * n,
                                &[self.base, slot_value, count_value, pc_value],
                            )
                            .expect("a status");
                        self.push_boxed();
                        self.check_status(status, pc, true);
                    }
                }
            }
            Op::TakeField { slot, name, site } => {
                // as `LoadField` through the site's cache in place (decision
                // AU55): a record of the cached type held once gives up its
                // field, `Nothing` left in its place, as the interpreter's
                // `op_take_field` does; a shared one gives a copy with one
                // more reference; anything else, a variant among them, goes
                // through the helper, which fills the cache; what is pushed
                // is typed as a field read
                let field_name = self.code.constants[*name as usize]
                    .as_text()
                    .unwrap_or("")
                    .to_string();
                if let SlotKind::Pair(ty) = self.analysis.slots[*slot as usize] {
                    // the field of a pair slot moved out of its cell, the
                    // other field released, both cells left `Nothing`
                    // (decision AW9): the slot is dead after its last read,
                    // as the interpreter's record with a hole in it is
                    match self.program.types.field_index(ty as usize, &field_name) {
                        Some(index) => {
                            let flags = MemFlagsData::trusted();
                            let taken = self.pair_cell(*slot as usize, index);
                            let other = self.pair_cell(*slot as usize, 1 - index);
                            let from = self.address_at(taken);
                            self.push_copy(from, false);
                            let nothing = self.iconst(types::I32, TAG_NOTHING as i64);
                            self.b.ins().istore8(flags, nothing, from, 0);
                            let other_at = self.address_at(other);
                            self.release(other_at);
                            self.b.ins().istore8(flags, nothing, other_at, 0);
                        }
                        None => {
                            self.pair_field_through_helper(*slot as usize, ty, *name, *site, pc)
                        }
                    }
                    self.push_boxed();
                    let kind = abs_of_field_at(self.program, self.code, pc, &field_name);
                    if !kind.is_boxed() {
                        self.unbox_top(kind, pc + 1);
                    }
                    return;
                }
                let flags = MemFlagsData::trusted();
                let pointer = self.pointer;
                let at = self.address_at(*slot as usize);
                let tag = self.tag_at(at, 0);
                let cache = self.b.ins().iadd_imm_s(self.vm, field_cache_offset());
                let cache_ptr = self.b.ins().load(pointer, flags, cache, 0);
                let entry = self
                    .b
                    .ins()
                    .iadd_imm_s(cache_ptr, (*site as usize * SITE_SIZE) as i64);
                let cached_ty = self.b.ins().load(types::I64, flags, entry, SITE_TY);
                let cached_tag = self.b.ins().load(types::I64, flags, entry, SITE_TAG);
                let cached_index = self.b.ins().load(types::I64, flags, entry, SITE_INDEX);
                let holder = self.b.create_block();
                let hit = self.b.create_block();
                self.b.append_block_param(hit, pointer);
                self.b.append_block_param(hit, pointer);
                let moved = self.b.create_block();
                let shared = self.b.create_block();
                let slow = self.b.create_block();
                let join = self.b.create_block();
                let is_record = self
                    .b
                    .ins()
                    .icmp_imm_s(IntCC::Equal, tag, TAG_RECORD as i64);
                self.b.ins().brif(is_record, holder, &[], slow, &[]);
                self.switch_to(holder);
                let rc = self.b.ins().load(pointer, flags, at, PAYLOAD);
                let ty = self.b.ins().load(types::I64, flags, rc, RECORD_TY);
                let holder_tag = self.b.ins().load(types::I64, flags, rc, RECORD_TAG);
                let fields_ptr = self.b.ins().iadd_imm_s(rc, RECORD_FIELDS as i64);
                let same_ty = self.b.ins().icmp(IntCC::Equal, ty, cached_ty);
                let same_tag = self.b.ins().icmp(IntCC::Equal, holder_tag, cached_tag);
                let ok = self.b.ins().band(same_ty, same_tag);
                let field = self.item_address(fields_ptr, cached_index);
                self.b
                    .ins()
                    .brif(ok, hit, &[field.into(), rc.into()], slow, &[]);
                self.switch_to(hit);
                let from = self.b.block_params(hit)[0];
                let block = self.b.block_params(hit)[1];
                let count = self.b.ins().load(types::I64, flags, block, 0);
                let once = self.b.ins().icmp_imm_s(IntCC::Equal, count, 1);
                self.b.ins().brif(once, moved, &[], shared, &[]);
                // held once: the field moved out, `Nothing` left in the record
                self.switch_to(moved);
                self.push_copy(from, false);
                let nothing = self.iconst(types::I32, TAG_NOTHING as i64);
                self.b.ins().istore8(flags, nothing, from, 0);
                self.b.ins().jump(join, &[]);
                // shared: the field copied, with one more reference
                self.switch_to(shared);
                self.push_copy(from, true);
                self.b.ins().jump(join, &[]);
                // the helper: by name, filling the cache
                self.switch_to(slow);
                let slot_value = self.u32(*slot as u32);
                let name_value = self.u32(*name);
                let site_value = self.u32(*site);
                let status = self
                    .call(
                        "rt_take_field",
                        &[
                            self.vm, self.base, slot_value, code_value, name_value, site_value,
                            pc_value,
                        ],
                    )
                    .expect("a status");
                self.check_status(status, pc, false);
                self.b.ins().jump(join, &[]);
                self.switch_to(join);
                self.push_boxed();
                let kind = abs_of_field_at(self.program, self.code, pc, &field_name);
                if !kind.is_boxed() {
                    self.unbox_top(kind, pc + 1);
                }
            }
            Op::Call { function, args } => {
                let count = *args as usize;
                let result_kind = abs_of_result(self.program, *function);
                // a direct call (decision AR3) when the callee has a code
                // object and every typed parameter has a typed operand here,
                // or a boxed one the checker noted as the parameter's type,
                // which the site takes from the stack (decision AW3)
                match direct_call(self.program, &self.state, *function, count) {
                    Some((callee, kinds)) => {
                        self.call_direct(callee, *function, &kinds, result_kind, pc);
                    }
                    None => match self.calls.get(*function).copied() {
                        Some(CallKind::Typed { kind, read, entry })
                            if kind_agrees(kind, result_kind) =>
                        {
                            let mask = self.masks[pc];
                            match (read, entry) {
                                (Some(ReadInPlace::ListAt), _) if count == 2 => {
                                    self.list_at_in_place(*function, mask, pc);
                                }
                                (_, Some(entry)) if (1..=32).contains(&count) => {
                                    let takes = matches!(entry, Entry::Takes);
                                    self.call_in_place(*function, count, kind, takes, mask, pc);
                                }
                                _ => self.call_typed(*function, count, kind, mask, pc),
                            }
                        }
                        Some(CallKind::Pure { entry: Some(entry) })
                            if (1..=32).contains(&count) =>
                        {
                            let kind = entry_kind(self.program, *function);
                            let takes = matches!(entry, Entry::Takes);
                            let mask = self.masks[pc];
                            self.call_in_place(*function, count, kind, takes, mask, pc);
                        }
                        Some(CallKind::Pure { .. }) => self.call_through_helper(
                            "rt_call_pure",
                            *function,
                            count,
                            result_kind,
                            pc,
                        ),
                        _ => {
                            // the operands a direct call would have borrowed
                            // (decision AW5) go owned through the VM's path
                            self.retain_borrowed_arguments(pc, count);
                            self.call_through_helper("rt_call", *function, count, result_kind, pc);
                            // a pair callee's record taken apart when the op
                            // after takes the pair as it lies (decision AW9)
                            if let Some(ty) = pair_result(self.program, *function) {
                                if self.pairs && pair_fused(self.code, pc) {
                                    self.state.pop();
                                    self.split_pair_top(ty, pc);
                                }
                            }
                        }
                    },
                }
                // the pair past the test of the `otherwise` whose region the
                // call ends (decision AW9): straight to the end, as the
                // analysis flows it; the test is left to the failures
                if let Some(end) = self.pair_bypass(*function, pc) {
                    self.jump_to(end);
                }
            }
            Op::CallAbility {
                ability,
                method,
                args,
            } => {
                let ability_value = self.usize(*ability);
                let method_value = self.u32(*method as u32);
                let args_value = self.u32(*args as u32);
                let status = self
                    .helper_on_stack(
                        "rt_call_ability",
                        *args as usize,
                        &[ability_value, method_value, args_value, pc_value],
                    )
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, true);
            }
            Op::CallValue(args) => {
                let args_value = self.u32(*args as u32);
                let status = self
                    .helper_on_stack("rt_call_value", *args as usize + 1, &[args_value, pc_value])
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, true);
            }
            Op::ResultType(index) => {
                let index = self.u32(*index);
                self.call("rt_result_type", &[self.vm, index]);
            }
            Op::Not => {
                let depth = self.state.len() - 1;
                if self.state[depth] == Abs::Bool {
                    let value = self.pop_bool();
                    let flipped = self.b.ins().bxor_imm_s(value, 1);
                    self.push_bool(flipped);
                } else {
                    let status = self
                        .helper_on_stack("rt_not", 1, &[pc_value])
                        .expect("a status");
                    self.push_boxed();
                    self.check_status(status, pc, false);
                    self.unbox_top(Abs::Bool, pc + 1);
                }
            }
            Op::Binary(op) => self.binary(*op, pc),
            Op::ToText => {
                let status = self
                    .helper_on_stack("rt_to_text", 1, &[pc_value])
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, false);
            }
            Op::Concat(count) => {
                let count_value = self.u32(*count as u32);
                let status = self
                    .helper_on_stack("rt_concat", *count as usize, &[count_value, pc_value])
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, false);
            }
            Op::Jump(target) => self.jump_to(*target as usize),
            Op::JumpIfFalse(target) => {
                let condition = self.pop_condition(pc);
                self.branch(condition, pc + 1, *target as usize);
            }
            Op::JumpIfTrue(target) => {
                let condition = self.pop_condition(pc);
                self.branch(condition, *target as usize, pc + 1);
            }
            Op::JumpIfAbsent(target) | Op::JumpIfFailure(target) => {
                let depth = self.state.len() - 1;
                if handler_only(self.code, &self.analysis.entry, pc) {
                    // reached through its handler alone (decision AW9): a
                    // failure on top, the jump taken
                    self.jump_to(*target as usize);
                } else if self.state[depth].is_boxed() {
                    // the top's tag: absent is `Nothing` or a failure
                    let at = self.last_address();
                    let tag = self.tag_at(at, 0);
                    let failed = self
                        .b
                        .ins()
                        .icmp_imm_s(IntCC::Equal, tag, TAG_FAILURE as i64);
                    let taken = if matches!(self.code.ops[pc], Op::JumpIfAbsent(_)) {
                        let nothing =
                            self.b
                                .ins()
                                .icmp_imm_s(IntCC::Equal, tag, TAG_NOTHING as i64);
                        self.b.ins().bor(failed, nothing)
                    } else {
                        failed
                    };
                    self.branch(taken, *target as usize, pc + 1);
                } else {
                    // an unboxed value is never absent: the jump is not
                    // taken, but the value is boxed, as both paths expect
                    self.box_position(depth);
                    self.jump_to(pc + 1);
                }
            }
            Op::PushHandler(_) | Op::PopHandler => {}
            Op::Return => {
                if let Some(Abs::Pair(found)) = self.state.last().copied() {
                    if self.pair_result == Some(found) {
                        // the pair returned as it lies (decision AW9)
                        self.pair_return(found, 1, pc);
                        return;
                    }
                    // a pair of another type than the declared result's
                    // (which the checker rules out): its record
                    self.pair_record_on_top(found);
                    *self.state.last_mut().expect("an operand") = Abs::Boxed(None);
                }
                let depth = self.state.len() - 1;
                let top = self.state[depth];
                let result = self.result_kind;
                let direct = self.direct;
                let flags = MemFlagsData::trusted();
                match (top, result) {
                    // a typed result of the declared kind goes back in a
                    // register to a direct caller, the frame left without
                    // a push; a trampoline's caller gets it on the stack
                    (Abs::Int, Abs::Int) | (Abs::Bool, Abs::Bool) | (Abs::Float, Abs::Float) => {
                        let value = match top {
                            Abs::Int => self.pop_int(),
                            Abs::Bool => self.pop_bool(),
                            _ => self.pop_float(),
                        };
                        let operands = boxed_depth(&self.state);
                        let in_register = self.b.create_block();
                        let on_stack = self.b.create_block();
                        self.b.ins().brif(direct, in_register, &[], on_stack, &[]);
                        self.switch_to(in_register);
                        self.leave_frame_inline(operands, pc);
                        let bits = self.payload_bits(top, value);
                        let status = self.iconst(types::I32, D_RETURNED as i64);
                        self.return_with(status, bits);
                        self.switch_to(on_stack);
                        self.leave_frame_inline(operands, pc);
                        self.push_typed_value(0, top, value);
                        self.return_direct(D_BOXED);
                    }
                    // a typed value where the declared result is boxed
                    // (`maybe Integer`): boxed onto the caller's stack
                    (Abs::Int | Abs::Bool | Abs::Float, _) => {
                        let value = match top {
                            Abs::Int => self.pop_int(),
                            Abs::Bool => self.pop_bool(),
                            _ => self.pop_float(),
                        };
                        let operands = boxed_depth(&self.state);
                        self.leave_frame_inline(operands, pc);
                        self.push_typed_value(0, top, value);
                        self.return_direct(D_BOXED);
                    }
                    // a boxed value where the declared result is typed:
                    // unboxed on the way out when it fits, for a direct
                    // caller; left on the stack for a trampoline's
                    (_, Abs::Int | Abs::Bool | Abs::Float) => {
                        self.box_top(1);
                        self.state.pop();
                        let operands = boxed_depth(&self.state);
                        let in_register = self.b.create_block();
                        let on_stack = self.b.create_block();
                        self.b.ins().brif(direct, in_register, &[], on_stack, &[]);
                        self.switch_to(in_register);
                        // the result lies above the state's height (popped
                        // from the state, on the stack still)
                        let height = self.height();
                        let at = self.address_at(height);
                        let tag = self.tag_at(at, 0);
                        let (fits, bits) = match result {
                            Abs::Int => {
                                let is_int =
                                    self.b
                                        .ins()
                                        .icmp_imm_s(IntCC::Equal, tag, TAG_INTEGER as i64);
                                let int_tag = self.tag_at(at, INT_TAG);
                                let is_small = self.b.ins().icmp_imm_s(
                                    IntCC::Equal,
                                    int_tag,
                                    INT_SMALL as i64,
                                );
                                let fits = self.b.ins().band(is_int, is_small);
                                let bits = self.b.ins().load(types::I64, flags, at, INT_PAYLOAD);
                                (fits, bits)
                            }
                            Abs::Bool => {
                                let fits =
                                    self.b
                                        .ins()
                                        .icmp_imm_s(IntCC::Equal, tag, TAG_BOOLEAN as i64);
                                let byte = self.b.ins().load(types::I8, flags, at, PAYLOAD);
                                let bits = self.b.ins().uextend(types::I64, byte);
                                (fits, bits)
                            }
                            _ => {
                                let fits =
                                    self.b.ins().icmp_imm_s(IntCC::Equal, tag, TAG_FLOAT as i64);
                                let bits = self.b.ins().load(types::I64, flags, at, PAYLOAD);
                                (fits, bits)
                            }
                        };
                        let unboxed = self.b.create_block();
                        let stays = self.b.create_block();
                        self.b.ins().brif(fits, unboxed, &[], stays, &[]);
                        self.switch_to(unboxed);
                        // a plain value: off the stack without a release
                        self.store_height(height);
                        self.leave_frame_inline(operands, pc);
                        let status = self.iconst(types::I32, D_RETURNED as i64);
                        self.return_with(status, bits);
                        self.switch_to(stays);
                        let status = self.leave_boxed_inline(operands, pc);
                        let zero = self.iconst(types::I64, 0);
                        self.return_with(status, zero);
                        self.switch_to(on_stack);
                        let status = self.leave_boxed_inline(operands, pc);
                        let zero = self.iconst(types::I64, 0);
                        self.return_with(status, zero);
                    }
                    _ => {
                        self.box_top(1);
                        self.state.pop();
                        let operands = boxed_depth(&self.state);
                        let status = self.leave_boxed_inline(operands, pc);
                        let zero = self.iconst(types::I64, 0);
                        self.return_with(status, zero);
                    }
                }
            }
            Op::ReturnNothing => {
                let operands = boxed_depth(&self.state);
                self.leave_frame_inline(operands, pc);
                self.push_nothing_at(0);
                self.return_direct(D_BOXED);
            }
            Op::Fail => {
                self.retain_params();
                let status = self
                    .helper_on_stack("rt_fail", 1, &[pc_value])
                    .expect("a status");
                self.exit_with(status);
            }
            Op::Crash => {
                let status = self
                    .helper_on_stack("rt_crash", 1, &[pc_value])
                    .expect("a status");
                self.exit_with(status);
            }
            Op::IsVariant(tag) => {
                // the top's tags read in place (decision AU49): a typed top
                // is no variant; a variant's block holds its tag at
                // `RECORD_TAG`; a guarded value, which the helper unwraps,
                // takes the helper as the cold path; the top is released
                if matches!(self.state.last(), Some(Abs::Int | Abs::Bool | Abs::Float)) {
                    self.state.pop();
                    let no = self.u8(0);
                    self.push_bool(no);
                    return;
                }
                self.box_top(1);
                self.state.pop();
                // the value lies above the state's height
                let height = self.height();
                let at = self.address_at(height);
                let flags = MemFlagsData::trusted();
                let kind = self.tag_at(at, 0);
                let variant = self.b.create_block();
                let other = self.b.create_block();
                let guarded = self.b.create_block();
                let answer = self.b.create_block();
                self.b.append_block_param(answer, types::I8);
                let join = self.b.create_block();
                self.b.append_block_param(join, types::I8);
                self.b.set_cold_block(guarded);
                let is_variant = self
                    .b
                    .ins()
                    .icmp_imm_s(IntCC::Equal, kind, TAG_VARIANT as i64);
                self.b.ins().brif(is_variant, variant, &[], other, &[]);
                self.b.seal_block(variant);
                self.b.seal_block(other);
                self.switch_to(variant);
                let rc = self.b.ins().load(self.pointer, flags, at, PAYLOAD);
                let held = self.b.ins().load(types::I64, flags, rc, RECORD_TAG);
                let fits = self.b.ins().icmp_imm_s(IntCC::Equal, held, *tag as i64);
                self.b.ins().jump(answer, &[fits.into()]);
                self.switch_to(other);
                let is_guarded = self
                    .b
                    .ins()
                    .icmp_imm_s(IntCC::Equal, kind, TAG_GUARDED as i64);
                let no = self.u8(0);
                self.b
                    .ins()
                    .brif(is_guarded, guarded, &[], answer, &[no.into()]);
                self.b.seal_block(guarded);
                self.b.seal_block(answer);
                self.switch_to(answer);
                let fits = self.b.block_params(answer)[0];
                self.release(at);
                self.b.ins().jump(join, &[fits.into()]);
                // the helper pops the value: the stack's length includes it
                self.switch_to(guarded);
                self.store_height(height + 1);
                let out = self.out_address();
                let tag_value = self.u32(*tag as u32);
                let vm = self.vm;
                self.call("rt_is_variant", &[vm, tag_value, out]);
                let unwrapped = self.out_read(types::I8);
                self.b.ins().jump(join, &[unwrapped.into()]);
                self.b.seal_block(join);
                self.switch_to(join);
                let fits = self.b.block_params(join)[0];
                self.store_height(height);
                self.push_bool(fits);
            }
            Op::IsNothing | Op::IsFailure => {
                // the top's tag read, the top dropped; a typed top is
                // neither
                let wanted = if matches!(self.code.ops[pc], Op::IsNothing) {
                    TAG_NOTHING
                } else {
                    TAG_FAILURE
                };
                if matches!(self.state.last(), Some(Abs::Int | Abs::Bool | Abs::Float)) {
                    self.state.pop();
                    let no = self.u8(0);
                    self.push_bool(no);
                    return;
                }
                self.box_top(1);
                self.state.pop();
                // the value lies above the state's height
                let height = self.height();
                let at = self.address_at(height);
                let tag = self.tag_at(at, 0);
                let is = self.b.ins().icmp_imm_s(IntCC::Equal, tag, wanted as i64);
                self.release(at);
                self.store_height(height);
                self.push_bool(is);
            }
            Op::IsType(ty) => {
                let out = self.out_address();
                let ty_value = self.usize(*ty);
                self.helper_on_stack("rt_is_type", 1, &[ty_value, out]);
                let value = self.out_read(types::I8);
                self.push_bool(value);
            }
            Op::Unpack(count) => {
                let count_value = self.u32(*count as u32);
                let status = self
                    .helper_on_stack("rt_unpack", 1, &[count_value, pc_value])
                    .expect("a status");
                for _ in 0..*count {
                    self.push_boxed();
                }
                self.check_status(status, pc, false);
            }
            Op::UnwrapFailure => {
                self.helper_on_stack("rt_unwrap_failure", 1, &[]);
                self.push_boxed();
            }
            Op::IterInit(slot) => {
                let index = *slot as usize;
                if self.analysis.slots[index] == SlotKind::RangeIter {
                    let depth = self.state.len() - 1;
                    let vars = self.range_vars(depth);
                    let from = self.b.use_var(vars[0]);
                    let to = self.b.use_var(vars[1]);
                    let by = self.b.use_var(vars[2]);
                    // a step of 0 is a crash; a range whose walk would leave
                    // the machine word goes to the interpreter
                    let zero = self.b.ins().icmp_imm_s(IntCC::Equal, by, 0);
                    let crash = self.b.create_block();
                    let cont = self.b.create_block();
                    self.b.ins().brif(zero, crash, &[], cont, &[]);
                    self.b.set_cold_block(crash);
                    self.switch_to(crash);
                    let which = self.u32(1);
                    let status = self
                        .call("rt_crash_text", &[self.vm, which, pc_value])
                        .expect("a status");
                    self.exit_with(status);
                    self.switch_to(cont);
                    let (_, overflow) = self.b.ins().sadd_overflow(to, by);
                    let fits = self.b.ins().icmp_imm_s(IntCC::Equal, overflow, 0);
                    self.deopt_unless(fits, pc);
                    self.state.pop();
                    let slot_vars = self.slot_vars3(index);
                    self.b.def_var(slot_vars[0], from);
                    self.b.def_var(slot_vars[1], to);
                    self.b.def_var(slot_vars[2], by);
                } else {
                    let slot_value = self.u32(*slot as u32);
                    let status = self
                        .helper_on_stack("rt_iter_init", 1, &[self.base, slot_value, pc_value])
                        .expect("a status");
                    self.check_status(status, pc, false);
                }
            }
            Op::IterNext { slot, exit } => {
                let index = *slot as usize;
                if self.analysis.slots[index] == SlotKind::RangeIter {
                    let vars = self.slot_vars3(index);
                    let current = self.b.use_var(vars[0]);
                    let end = self.b.use_var(vars[1]);
                    let step = self.b.use_var(vars[2]);
                    let ascending =
                        self.b
                            .ins()
                            .icmp_imm_s(IntCC::SignedGreaterThanOrEqual, step, 0);
                    let past_up = self.b.ins().icmp(IntCC::SignedGreaterThan, current, end);
                    let past_down = self.b.ins().icmp(IntCC::SignedLessThan, current, end);
                    let done = self.b.ins().select(ascending, past_up, past_down);
                    let (exit_block, fill_exit) = self.edge_to(*exit as usize);
                    let body = self.b.create_block();
                    self.b.ins().brif(done, exit_block, &[], body, &[]);
                    self.terminated = true;
                    if fill_exit {
                        self.fill_edge(exit_block, *exit as usize);
                    }
                    self.switch_to(body);
                    self.push_int(current);
                    let next = self.b.ins().iadd(current, step);
                    self.b.def_var(vars[0], next);
                } else {
                    // a list iterator read and advanced in place (decision
                    // AU1, stage ii): the items' address, their count and
                    // the position from the native's fixed layout, the item
                    // copied with one more reference; anything else in the
                    // slot (a range iterator, a value the analysis did not
                    // type) goes through the helper
                    let flags = MemFlagsData::trusted();
                    let slot_address = self.address_at(index);
                    let tag = self.tag_at(slot_address, 0);
                    let is_native = self
                        .b
                        .ins()
                        .icmp_imm_s(IntCC::Equal, tag, TAG_NATIVE as i64);
                    let kind_block = self.b.create_block();
                    let inline_block = self.b.create_block();
                    let next_block = self.b.create_block();
                    let slow = self.b.create_block();
                    let found = self.b.create_block();
                    let (exit_block, fill_exit) = self.edge_to(*exit as usize);
                    self.b.ins().brif(is_native, kind_block, &[], slow, &[]);
                    self.b.seal_block(kind_block);
                    self.switch_to(kind_block);
                    let rc = self
                        .b
                        .ins()
                        .load(self.pointer, flags, slot_address, PAYLOAD);
                    let kind = self
                        .b
                        .ins()
                        .load(types::I8, flags, rc, RC_VALUE + NATIVE_TAG);
                    let is_iterator =
                        self.b
                            .ins()
                            .icmp_imm_s(IntCC::Equal, kind, NATIVE_ITERATOR as i64);
                    self.b.ins().brif(is_iterator, inline_block, &[], slow, &[]);
                    self.b.seal_block(inline_block);
                    self.b.seal_block(slow);
                    self.switch_to(inline_block);
                    let iterator = RC_VALUE + NATIVE_PAYLOAD;
                    let items = self
                        .b
                        .ins()
                        .load(self.pointer, flags, rc, iterator + ITER_ITEMS);
                    let len = self
                        .b
                        .ins()
                        .load(types::I64, flags, rc, iterator + ITER_LEN);
                    let position =
                        self.b
                            .ins()
                            .load(types::I64, flags, rc, iterator + ITER_POSITION);
                    let done = self
                        .b
                        .ins()
                        .icmp(IntCC::UnsignedGreaterThanOrEqual, position, len);
                    self.b.ins().brif(done, exit_block, &[], next_block, &[]);
                    self.b.seal_block(next_block);
                    self.switch_to(next_block);
                    let offset = self.b.ins().imul_imm_s(position, SIZE as i64);
                    let item = self.b.ins().iadd(items, offset);
                    self.push_copy(item, true);
                    let advanced = self.b.ins().iadd_imm_s(position, 1);
                    self.b
                        .ins()
                        .store(flags, advanced, rc, iterator + ITER_POSITION);
                    self.b.ins().jump(found, &[]);
                    // the helper for anything else
                    self.switch_to(slow);
                    let slot_value = self.u32(*slot as u32);
                    let has = self
                        .call("rt_iter_next", &[self.vm, self.base, slot_value])
                        .expect("an answer");
                    self.b.ins().brif(has, found, &[], exit_block, &[]);
                    self.b.seal_block(found);
                    self.terminated = true;
                    if fill_exit {
                        self.fill_edge(exit_block, *exit as usize);
                    }
                    self.switch_to(found);
                    self.push_boxed();
                }
            }
            Op::ListPush => {
                let status = self
                    .helper_on_stack("rt_list_push", 2, &[pc_value])
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, false);
            }
            Op::GroupInsert => {
                let status = self
                    .helper_on_stack("rt_group_insert", 3, &[pc_value])
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, false);
            }
            Op::GroupFold(fold) => {
                let fold_value = self.u8(fold_code(*fold));
                let status = self
                    .helper_on_stack("rt_group_fold", 3, &[fold_value, pc_value])
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, false);
            }
            Op::SortByKey { descending } => {
                let descending_value = self.u8(*descending as u8);
                let status = self
                    .helper_on_stack("rt_sort_by_key", 1, &[descending_value, pc_value])
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, false);
            }
            Op::Deadline(slot) => {
                let slot_value = self.u32(*slot as u32);
                let status = self
                    .helper_on_stack("rt_deadline", 1, &[self.base, slot_value, pc_value])
                    .expect("a status");
                self.check_status(status, pc, false);
            }
            Op::CheckDeadline(slot) => {
                let slot_value = self.u32(*slot as u32);
                let status = self
                    .call("rt_check_deadline", &[self.vm, self.base, slot_value])
                    .expect("a status");
                // the failure, when there is one, is on top of the stack
                self.push_boxed();
                self.check_status(status, pc, true);
                self.state.pop();
            }
            Op::MarkStack(_) => {}
            Op::UnwindStack(slot) => {
                let depth = self.analysis.marks[*slot as usize].unwrap_or(0);
                let keep = boxed_depth(&self.state[..depth]);
                if boxed_depth(&self.state) > keep {
                    let locals = self.u32(self.frame_locals as u32);
                    let keep_value = self.u32(keep as u32);
                    self.call("rt_truncate", &[self.vm, self.base, locals, keep_value]);
                }
                self.state.truncate(depth);
            }
            Op::Check(message) => {
                let condition = self.pop_condition(pc);
                let cont = self.b.create_block();
                let failed = self.b.create_block();
                self.b.ins().brif(condition, cont, &[], failed, &[]);
                self.b.set_cold_block(failed);
                self.switch_to(failed);
                self.retain_params();
                let message_value = self.u32(*message);
                let status = self
                    .call("rt_check_failed", &[self.vm, code_value, message_value])
                    .expect("a status");
                self.exit_with(status);
                self.switch_to(cont);
            }
        }
    }

    // ------------------------------------------------------------ operators

    // ------------------------------------------------------------ pairs

    /// The cell of the pair slot that holds the field at `index` (decision
    /// AW9): the slot itself for the first, its second cell for the other.
    fn pair_cell(&self, slot: usize, index: usize) -> usize {
        if index == 0 {
            slot
        } else {
            self.pair_cells[slot].expect("a pair slot")
        }
    }

    /// The two fields of the pair slot pushed as two cells (decision AW9):
    /// moved out, `Nothing` left in both cells, or copied with a reference
    /// each. The state is the caller's to push.
    fn push_pair_from_slot(&mut self, slot: usize, moving: bool) {
        let flags = MemFlagsData::trusted();
        let extra = self.pair_cells[slot].expect("a pair slot");
        let height = self.height();
        for (part, from_height) in [slot, extra].into_iter().enumerate() {
            let from = self.address_at(from_height);
            let to = self.address_at(height + part);
            self.copy_value(from, to);
            if moving {
                let nothing = self.iconst(types::I32, TAG_NOTHING as i64);
                self.b.ins().istore8(flags, nothing, from, 0);
            } else {
                self.retain(to);
            }
        }
        self.store_height(height + 2);
    }

    /// The two cells on top of the stack made into their record in their
    /// place (decision AW9), through the helper. The state is the caller's.
    fn pair_record_on_top(&mut self, ty: u32) {
        let zero = self.usize(0);
        let ty_value = self.usize(ty as usize);
        self.call("rt_pair_record", &[self.vm, zero, ty_value]);
    }

    /// The field at `index` of the pair on top of the stack kept and the
    /// other released (decision AW9): the field in the lower of the two
    /// cells, the height one less. The state is the caller's.
    fn pair_field_on_top(&mut self, index: usize) {
        let height = self.height();
        let first = self.address_at(height - 2);
        let second = self.address_at(height - 1);
        if index == 0 {
            self.release(second);
        } else {
            self.release(first);
            self.copy_value(second, first);
        }
        self.store_height(height - 1);
    }

    /// The record on top of the stack taken apart into the pair's two
    /// cells through the helper (decision AW9), which crashes for anything
    /// else; the pair pushed on the state.
    fn split_pair_top(&mut self, ty: u32, pc: usize) {
        let ty_value = self.usize(ty as usize);
        let pc_value = self.u32(pc as u32);
        let status = self
            .call("rt_split_pair", &[self.vm, ty_value, pc_value])
            .expect("a status");
        self.check_status(status, pc, false);
        self.state.push(Abs::Pair(ty));
    }

    /// A field by name of a pair slot through the helper on its record
    /// (decision AW9): for a name that is no field of the type, which the
    /// checker rules out and the helper crashes on.
    fn pair_field_through_helper(&mut self, slot: usize, ty: u32, name: u32, site: u32, pc: usize) {
        self.push_pair_from_slot(slot, false);
        self.pair_record_on_top(ty);
        self.state.push(Abs::Boxed(None));
        let code_value = self.usize(self.code_id);
        let name_value = self.u32(name);
        let site_value = self.u32(site);
        let pc_value = self.u32(pc as u32);
        let status = self
            .helper_on_stack(
                "rt_field",
                1,
                &[code_value, name_value, site_value, pc_value],
            )
            .expect("a status");
        self.check_status(status, pc, false);
    }

    /// The three words of a value at the address.
    fn words_at(&mut self, at: IrValue) -> [IrValue; 3] {
        let flags = MemFlagsData::trusted();
        [0, 8, 16].map(|offset| self.b.ins().load(types::I64, flags, at, offset))
    }

    /// The three words of the operand at `depth` as a `Value`: a boxed
    /// one's from its cell, a typed one's made up as `write_typed` writes
    /// them.
    fn operand_words(&mut self, depth: usize) -> [IrValue; 3] {
        match self.state[depth] {
            Abs::Int => {
                let var = self.int_var(depth);
                let value = self.b.use_var(var);
                let tag = self.iconst(types::I64, TAG_INTEGER as i64);
                let small = self.iconst(types::I64, INT_SMALL as i64);
                [tag, small, value]
            }
            Abs::Bool => {
                let var = self.bool_var(depth);
                let value = self.b.use_var(var);
                let wide = self.b.ins().uextend(types::I64, value);
                let tag = self.iconst(types::I64, TAG_BOOLEAN as i64);
                let zero = self.iconst(types::I64, 0);
                [tag, wide, zero]
            }
            Abs::Float => {
                let var = self.float_var(depth);
                let value = self.b.use_var(var);
                let bits = self.b.ins().bitcast(types::I64, MemFlagsData::new(), value);
                let tag = self.iconst(types::I64, TAG_FLOAT as i64);
                let zero = self.iconst(types::I64, 0);
                [tag, bits, zero]
            }
            Abs::Boxed(_) | Abs::Unset => {
                let at = self.operand_address(depth);
                self.words_at(at)
            }
            Abs::Range | Abs::Pair(_) => unreachable!("a field is one value"),
        }
    }

    /// The two fields on top of the state returned as a pair (decision
    /// AW9; `entries` operands: the two fields as `Construct` found them,
    /// or one pair): to a direct caller, in the frame's first two cells
    /// with `D_RETURNED`; to a trampoline's caller, and under provenance
    /// guards (the record carries the union of the fields' origins), as
    /// the record, with the status `leave_boxed_inline` gives.
    fn pair_return(&mut self, ty: u32, entries: usize, pc: usize) {
        let flags = MemFlagsData::trusted();
        let saved = self.state.clone();
        let guarding = self
            .b
            .ins()
            .load(types::I8, flags, self.vm, guarding_offset() as i32);
        let in_cells = self.b.create_block();
        let as_record = self.b.create_block();
        let check = self.b.create_block();
        self.b.ins().brif(guarding, as_record, &[], check, &[]);
        self.switch_to(check);
        self.b
            .ins()
            .brif(self.direct, in_cells, &[], as_record, &[]);
        // the fields' words read before the leave, written where the
        // frame's first two slots were, the stack cut below the fields
        // first so that no leave releases them
        self.switch_to(in_cells);
        let len = self.state.len();
        let words: Vec<[IrValue; 3]> = if entries == 2 {
            vec![self.operand_words(len - 2), self.operand_words(len - 1)]
        } else {
            let height = self.height();
            let first = self.address_at(height - 2);
            let second = self.address_at(height - 1);
            vec![self.words_at(first), self.words_at(second)]
        };
        self.state.truncate(len - entries);
        let operands = boxed_depth(&self.state);
        let height = self.height();
        self.store_height(height);
        self.leave_frame_inline(operands, pc);
        for (cell, chunk) in words.iter().enumerate() {
            let to = self.address_at(cell);
            for (word, offset) in chunk.iter().zip([0, 8, 16]) {
                self.b.ins().store(flags, *word, to, offset);
            }
        }
        self.store_height(2);
        let status = self.iconst(types::I32, D_RETURNED as i64);
        let zero = self.iconst(types::I64, 0);
        self.return_with(status, zero);
        // the record
        self.switch_to(as_record);
        self.state = saved;
        if entries == 2 {
            self.box_top(2);
        }
        self.pair_record_on_top(ty);
        let len = self.state.len();
        self.state.truncate(len - entries);
        let operands = boxed_depth(&self.state);
        let status = self.leave_boxed_inline(operands, pc);
        let zero = self.iconst(types::I64, 0);
        self.return_with(status, zero);
    }

    /// A `with` whose base the checker typed, in place (decision AW8):
    /// the `n` pairs boxed on the stack, the holder (the frame's slot,
    /// left `Nothing`, or the operand under the pairs) checked to be a
    /// record of the type `ty` and every new value to be plain, the block
    /// updated in place when held once and a copy of it otherwise
    /// (`rt_record_copy`, the holder's reference given up), each field at
    /// its index in `packed` released and the new value moved in, each
    /// name released, and the record left where the first name lay (where
    /// the holder lay). Anything else goes through the helper of the
    /// same indices, which leaves the same stack.
    fn update_in_place(
        &mut self,
        slot: Option<usize>,
        n: usize,
        ty: usize,
        packed: u64,
        pc: usize,
    ) {
        let flags = MemFlagsData::trusted();
        let pointer = self.pointer;
        self.box_top(2 * n);
        let height = self.height();
        let first = height - 2 * n;
        let (holder_height, result_height) = match slot {
            Some(_) => (first, first + 1),
            None => (first - 1, first),
        };
        let holder_at = match slot {
            Some(slot) => self.address_at(slot),
            None => self.address_at(holder_height),
        };
        // the state after the op, for the landing of the helper's status
        for _ in 0..(2 * n + usize::from(slot.is_none())) {
            self.state.pop();
        }
        self.push_boxed();
        let typed = self.b.create_block();
        let shared = self.b.create_block();
        let update = self.b.create_block();
        self.b.append_block_param(update, pointer);
        let slow = self.b.create_block();
        let join = self.b.create_block();
        self.b.set_cold_block(slow);
        let tag = self.tag_at(holder_at, 0);
        let is_record = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, tag, TAG_RECORD as i64);
        self.b.ins().brif(is_record, typed, &[], slow, &[]);
        self.switch_to(typed);
        let rc = self.b.ins().load(pointer, flags, holder_at, PAYLOAD);
        let holder_ty = self.b.ins().load(types::I64, flags, rc, RECORD_TY);
        let mut ok = self.b.ins().icmp_imm_s(IntCC::Equal, holder_ty, ty as i64);
        for index in 0..n {
            let at = self.address_at(first + 2 * index + 1);
            let tag = self.tag_at(at, 0);
            let plain = self
                .b
                .ins()
                .icmp_imm_s(IntCC::NotEqual, tag, TAG_GUARDED as i64);
            ok = self.b.ins().band(ok, plain);
        }
        let counted = self.b.create_block();
        self.b.ins().brif(ok, counted, &[], slow, &[]);
        self.switch_to(counted);
        let count = self.b.ins().load(types::I64, flags, rc, 0);
        let held_once = self.b.ins().icmp_imm_s(IntCC::Equal, count, 1);
        self.b
            .ins()
            .brif(held_once, update, &[rc.into()], shared, &[]);
        // shared: the holder's reference moves to a copy
        self.switch_to(shared);
        let copy = self.call("rt_record_copy", &[rc]).expect("a block");
        let fewer = self.b.ins().iadd_imm_s(count, -1);
        self.b.ins().store(flags, fewer, rc, 0);
        self.b.ins().jump(update, &[copy.into()]);
        self.switch_to(update);
        let block = self.b.block_params(update)[0];
        let fields_ptr = self.b.ins().iadd_imm_s(block, RECORD_FIELDS as i64);
        // the last update first, as the helper pops them: of two updates
        // of one field the first in the source wins
        for index in (0..n).rev() {
            let field_index = (packed >> (8 * index)) as usize & 0xff;
            let field = self
                .b
                .ins()
                .iadd_imm_s(fields_ptr, (field_index * SIZE) as i64);
            self.release(field);
            let from = self.address_at(first + 2 * index + 1);
            self.copy_value(from, field);
            let name = self.address_at(first + 2 * index);
            self.release(name);
        }
        if slot.is_some() {
            let nothing = self.iconst(types::I32, TAG_NOTHING as i64);
            self.b.ins().istore8(flags, nothing, holder_at, 0);
        }
        let to = self.address_at(result_height - 1);
        let record_tag = self.iconst(types::I32, TAG_RECORD as i64);
        self.b.ins().istore8(flags, record_tag, to, 0);
        self.b.ins().store(flags, block, to, PAYLOAD);
        self.store_height(result_height);
        self.b.ins().jump(join, &[]);
        // the helper: the same indices, the same stack after
        self.switch_to(slow);
        let ty_value = self.usize(ty);
        let packed_value = self.iconst(types::I64, packed as i64);
        let pc_value = self.u32(pc as u32);
        let status = match slot {
            Some(slot) => {
                let slot_value = self.u32(slot as u32);
                self.call(
                    "rt_with_slot_by_index",
                    &[
                        self.vm,
                        self.base,
                        slot_value,
                        ty_value,
                        packed_value,
                        pc_value,
                    ],
                )
            }
            None => self.call(
                "rt_with_by_index",
                &[self.vm, ty_value, packed_value, pc_value],
            ),
        }
        .expect("a status");
        self.check_status(status, pc, true);
        self.b.ins().jump(join, &[]);
        self.switch_to(join);
    }

    /// A call through `rt_call`: the arguments boxed on the stack, the
    /// result boxed, and unboxed when the declared result is typed.
    fn call_through_helper(
        &mut self,
        helper: &'static str,
        function: usize,
        count: usize,
        result_kind: Abs,
        pc: usize,
    ) {
        let pc_value = self.u32(pc as u32);
        let function_value = self.usize(function);
        let args_value = self.u32(count as u32);
        let status = self
            .helper_on_stack(helper, count, &[function_value, args_value, pc_value])
            .expect("a status");
        self.push_boxed();
        self.check_status(status, pc, true);
        if !result_kind.is_boxed() {
            self.unbox_top(result_kind, pc + 1);
        }
    }

    /// `List.at` in place (decision AU34): on a list and a small Integer,
    /// the item read through the list's block (its items' address and
    /// their count where `layout::list_layout` found them, loaded from the
    /// native state), copied with one more reference, or `Nothing` past
    /// either end; the list released unless the call borrows it, after the
    /// item is copied, since the release may free the list. Anything else
    /// (a guarded or a big operand, a failure among the items) takes the
    /// typed call's general path, the operands untouched.
    fn list_at_in_place(&mut self, function: usize, mask: u32, pc: usize) {
        let flags = MemFlagsData::trusted();
        let pointer = self.pointer;
        let depth = self.state.len();
        let index_reg = (self.state[depth - 1] == Abs::Int).then(|| {
            let var = self.int_var(depth - 1);
            self.b.use_var(var)
        });
        // both operands on the stack, so that the general path finds them
        // as it always does
        self.box_top(2);
        let height = self.height();
        let list_at = self.address_at(height - 2);
        let index_at = self.address_at(height - 1);
        let slow = self.b.create_block();
        self.b.set_cold_block(slow);
        let join = self.b.create_block();
        let tag = self.tag_at(list_at, 0);
        let is_list = self.b.ins().icmp_imm_s(IntCC::Equal, tag, TAG_LIST as i64);
        let listed = self.b.create_block();
        self.b.ins().brif(is_list, listed, &[], slow, &[]);
        self.b.seal_block(listed);
        self.switch_to(listed);
        let index = match index_reg {
            Some(index) => index,
            None => {
                let index_tag = self.tag_at(index_at, 0);
                let is_integer =
                    self.b
                        .ins()
                        .icmp_imm_s(IntCC::Equal, index_tag, TAG_INTEGER as i64);
                let integer = self.b.create_block();
                self.b.ins().brif(is_integer, integer, &[], slow, &[]);
                self.b.seal_block(integer);
                self.switch_to(integer);
                let int_tag = self.tag_at(index_at, INT_TAG);
                let is_small = self
                    .b
                    .ins()
                    .icmp_imm_s(IntCC::Equal, int_tag, INT_SMALL as i64);
                let small = self.b.create_block();
                self.b.ins().brif(is_small, small, &[], slow, &[]);
                self.b.seal_block(small);
                self.switch_to(small);
                self.b.ins().load(types::I64, flags, index_at, INT_PAYLOAD)
            }
        };
        let block = self.b.ins().load(pointer, flags, list_at, PAYLOAD);
        let state = self.native_state();
        let len_offset = self.b.ins().load(types::I64, flags, state, STATE_LIST_LEN);
        let len_at = self.b.ins().iadd(block, len_offset);
        let len = self.b.ins().load(types::I64, flags, len_at, 0);
        let inside = self.b.ins().icmp(IntCC::UnsignedLessThan, index, len);
        let found = self.b.create_block();
        let past = self.b.create_block();
        self.b.ins().brif(inside, found, &[], past, &[]);
        self.b.seal_block(found);
        self.b.seal_block(past);
        // the item: copied over the list's operand, with a reference of
        // its own, the list released after
        self.switch_to(found);
        let items_offset = self
            .b
            .ins()
            .load(types::I64, flags, state, STATE_LIST_ITEMS);
        let items_at = self.b.ins().iadd(block, items_offset);
        let items = self.b.ins().load(pointer, flags, items_at, 0);
        let offset = self.b.ins().imul_imm_s(index, SIZE as i64);
        let item = self.b.ins().iadd(items, offset);
        let item_tag = self.tag_at(item, 0);
        let is_failure = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, item_tag, TAG_FAILURE as i64);
        let copy = self.b.create_block();
        self.b.ins().brif(is_failure, slow, &[], copy, &[]);
        self.b.seal_block(copy);
        self.switch_to(copy);
        self.retain(item);
        let words: Vec<IrValue> = [0, 8, 16]
            .iter()
            .map(|offset| self.b.ins().load(types::I64, flags, item, *offset))
            .collect();
        if mask & 1 == 0 {
            self.release(list_at);
        }
        for (word, offset) in words.into_iter().zip([0, 8, 16]) {
            self.b.ins().store(flags, word, list_at, offset);
        }
        self.store_height(height - 1);
        self.b.ins().jump(join, &[]);
        // past either end: `Nothing`
        self.switch_to(past);
        if mask & 1 == 0 {
            self.release(list_at);
        }
        let nothing = self.iconst(types::I32, TAG_NOTHING as i64);
        self.b.ins().istore8(flags, nothing, list_at, 0);
        self.store_height(height - 1);
        self.b.ins().jump(join, &[]);
        // the general path: it pops the operands and pushes the answer
        self.b.seal_block(slow);
        self.switch_to(slow);
        self.call_typed(function, 2, TypedKind::Value, mask, pc);
        self.b.ins().jump(join, &[]);
        self.b.seal_block(join);
        self.switch_to(join);
    }

    /// A call through the native's in-place entry (decision AW11): the
    /// entry's shim called directly through the VM's table, on the
    /// operands where they lie, with the mask of those the call borrows
    /// and the pc; it answers in the first operand's place with the
    /// others consumed, and the answer is the call's result, a scalar
    /// read from the boxed value the entry wrote or that value as it
    /// lies; when the entry declines, the typed call runs on the operands
    /// it left as they were. A scalar entry that reads its operands
    /// answers or declines and nothing else (`in_place_typed`): one test
    /// after the call, the length stored for the answer alone, as AU34's
    /// lean helpers had. An entry that answers a value or takes its
    /// operands may fail or find the budget spent: a failure or an
    /// interrupt lands where a typed call's does, the length stored for
    /// the landing before the status is tested.
    fn call_in_place(
        &mut self,
        function: usize,
        count: usize,
        kind: TypedKind,
        takes: bool,
        mask: u32,
        pc: usize,
    ) {
        self.box_top(count);
        let depth = self.state.len() - count;
        let height = self.height();
        let at = self.address_at(height - count);
        let flags = MemFlagsData::trusted();
        let entry = self.b.ins().load(
            self.pointer,
            flags,
            self.in_place_table,
            (function * 8) as i32,
        );
        let sig_ref = match self.in_place_sig {
            Some(sig_ref) => sig_ref,
            None => {
                let sig_ref = self.b.import_signature(signature(self.isa, "ppwww", 'i'));
                self.in_place_sig = Some(sig_ref);
                sig_ref
            }
        };
        let count_value = self.u32(count as u32);
        let mask_value = self.u32(mask);
        let pc_value = self.u32(pc as u32);
        let call = self.b.ins().call_indirect(
            sig_ref,
            entry,
            &[self.vm, at, count_value, mask_value, pc_value],
        );
        let status = self.b.inst_results(call)[0];
        // the entry grows nothing: the frame pointer holds
        let yes = self.b.create_block();
        let general = self.b.create_block();
        self.b.set_cold_block(general);
        let join = self.b.create_block();
        let declined = self.b.ins().icmp_imm_s(
            IntCC::Equal,
            status,
            crate::native::runtime::DECLINED as i64,
        );
        if takes || kind == TypedKind::Value {
            // the answer is in the first operand's place and the operands
            // above it are consumed, which the length says for the answer
            // and for a failure's landing
            self.store_height(height - count + 1);
            let answered = self
                .b
                .ins()
                .icmp_imm_s(IntCC::Equal, status, CONTINUE as i64);
            let other = self.b.create_block();
            self.b.ins().brif(answered, yes, &[], other, &[]);
            self.b.seal_block(yes);
            self.b.seal_block(other);
            // declined: the typed call on the operands, which lie as they
            // were; else the failure or the interrupt, the answer in place
            self.switch_to(other);
            let bad = self.bad_block(pc);
            self.b.ins().brif(
                declined,
                general,
                &[],
                bad,
                &[status.into(), pc_value.into()],
            );
            self.b.seal_block(general);
            self.switch_to(general);
            self.store_height(height);
        } else {
            self.b.ins().brif(declined, general, &[], yes, &[]);
            self.b.seal_block(yes);
            self.b.seal_block(general);
            self.switch_to(general);
        }
        self.call_typed(function, count, kind, mask, pc);
        self.b.ins().jump(join, &[]);
        // the answer: a scalar read from the boxed value the entry wrote
        // (small, a Boolean or a Float by its contract), or the boxed
        // value where it lies
        self.switch_to(yes);
        match kind {
            TypedKind::Value => {}
            TypedKind::Int => {
                let value = self.b.ins().load(types::I64, flags, at, INT_PAYLOAD);
                let var = self.int_var(depth);
                self.b.def_var(var, value);
                self.store_height(height - count);
            }
            TypedKind::Bool => {
                let value = self.b.ins().load(types::I8, flags, at, PAYLOAD);
                let var = self.bool_var(depth);
                self.b.def_var(var, value);
                self.store_height(height - count);
            }
            TypedKind::Float => {
                let value = self.b.ins().load(types::F64, flags, at, PAYLOAD);
                let var = self.float_var(depth);
                self.b.def_var(var, value);
                self.store_height(height - count);
            }
        }
        self.b.ins().jump(join, &[]);
        self.b.seal_block(join);
        self.switch_to(join);
    }

    /// A call to a primitive with a typed entry (decision AU1): the
    /// arguments the mask names were pushed borrowed, the entry answers a
    /// scalar into the out slot and the generated code takes it from
    /// there into its register, or a value onto the stack; when the entry
    /// declined and the general path answered, the answer is boxed on the
    /// stack (`BOXED`) and is unboxed as a call through `rt_call` is,
    /// handing the frame to the interpreter where it does not fit.
    fn call_typed(&mut self, function: usize, count: usize, kind: TypedKind, mask: u32, pc: usize) {
        let pc_value = self.u32(pc as u32);
        let function_value = self.usize(function);
        let count_value = self.u32(count as u32);
        let mask_value = self.u32(mask);
        let out = self.out_address();
        let status = self
            .helper_on_stack(
                "rt_call_typed",
                count,
                &[function_value, count_value, mask_value, pc_value, out],
            )
            .expect("a status");
        let abs = match kind {
            TypedKind::Bool => Abs::Bool,
            TypedKind::Int => Abs::Int,
            TypedKind::Float => Abs::Float,
            TypedKind::Value => {
                self.push_boxed();
                self.check_status(status, pc, true);
                return;
            }
        };
        let depth = self.state.len();
        let fast = self.b.create_block();
        let other = self.b.create_block();
        let join = self.b.create_block();
        let answered = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, status, CONTINUE as i64);
        self.b.ins().brif(answered, fast, &[], other, &[]);
        self.b.seal_block(fast);
        self.b.seal_block(other);
        // the general path answered on the stack, or failed: the state has
        // its boxed answer on top either way
        self.switch_to(other);
        self.push_boxed();
        let slow = self.b.create_block();
        let bad = self.bad_block(pc);
        let boxed =
            self.b
                .ins()
                .icmp_imm_s(IntCC::Equal, status, crate::native::runtime::BOXED as i64);
        self.b
            .ins()
            .brif(boxed, slow, &[], bad, &[status.into(), pc_value.into()]);
        self.b.seal_block(slow);
        self.switch_to(slow);
        // a boxed answer that does not fit the register goes to the
        // interpreter with the call done: at the op after it
        self.unbox_top(abs, pc + 1);
        self.b.ins().jump(join, &[]);
        // the entry answered into the out slot
        self.switch_to(fast);
        let flags = MemFlagsData::trusted();
        match abs {
            Abs::Int => {
                let value = self.b.ins().load(types::I64, flags, out, 0);
                let var = self.int_var(depth);
                self.b.def_var(var, value);
            }
            Abs::Bool => {
                let value = self.b.ins().load(types::I8, flags, out, 0);
                let var = self.bool_var(depth);
                self.b.def_var(var, value);
            }
            _ => {
                let value = self.b.ins().load(types::F64, flags, out, 0);
                let var = self.float_var(depth);
                self.b.def_var(var, value);
            }
        }
        self.b.ins().jump(join, &[]);
        self.b.seal_block(join);
        self.switch_to(join);
        self.state.truncate(depth);
        self.state.push(abs);
    }

    /// A direct call (decision AR3): the callee's body called with its
    /// typed arguments in registers and the boxed ones on the stack. A
    /// typed parameter whose operand is boxed here, noted by the checker as
    /// the parameter's type, is taken from the stack at the site (decision
    /// AW3): its tags tested in place, its payload passed in the register,
    /// and the operand left for the frame to move into the parameter's
    /// slot, where the trampoline's entry would read it; a big or a guarded
    /// Integer takes the helper's path with everything else. The
    /// body's address comes from the JIT's table; without one (the callee
    /// is cold, or not placed yet), or with no room on the machine stack,
    /// the call goes through `rt_call`, and the interpreter compiles a hot
    /// callee on the way and enters it through its trampoline (decision
    /// AT2). A typed result comes back in a register and a boxed one on
    /// the stack; anything else goes through `rt_direct_after`.
    fn call_direct(
        &mut self,
        callee: usize,
        function: usize,
        kinds: &[Abs],
        result_kind: Abs,
        pc: usize,
    ) {
        let count = kinds.len();
        let first = self.state.len() - count;
        let saved = self.state.clone();
        let pointer = self.pointer;
        let flags = MemFlagsData::trusted();
        let have = self.b.create_block();
        self.b.append_block_param(have, pointer);
        let slow = self.b.create_block();
        let join = self.b.create_block();
        // the typed parameters whose operands are boxed here (decision
        // AW3): the tags tested in place, a small Integer, a Boolean or a
        // Float of the parameter's kind going on, anything else through
        // the helper's path
        let unboxed: Vec<usize> = (0..count)
            .filter(|index| !kinds[*index].is_boxed() && self.state[first + index].is_boxed())
            .collect();
        let mut fits: Option<IrValue> = None;
        for &index in &unboxed {
            let at = self.operand_address(first + index);
            let tags: &[(i32, u8)] = match kinds[index] {
                Abs::Int => &[(0, TAG_INTEGER), (INT_TAG, INT_SMALL)],
                Abs::Bool => &[(0, TAG_BOOLEAN)],
                Abs::Float => &[(0, TAG_FLOAT)],
                _ => unreachable!("a parameter is typed or boxed"),
            };
            for &(offset, tag) in tags {
                let found = self.tag_at(at, offset);
                let is = self.b.ins().icmp_imm_s(IntCC::Equal, found, tag as i64);
                fits = Some(match fits {
                    Some(before) => self.b.ins().band(before, is),
                    None => is,
                });
            }
        }
        if let Some(fits) = fits {
            let fit = self.b.create_block();
            self.b.ins().brif(fits, fit, &[], slow, &[]);
            self.switch_to(fit);
        }
        // the body's address from the table
        let state = self.native_state();
        let table = self.b.ins().load(pointer, flags, state, STATE_DIRECT);
        let known = self
            .b
            .ins()
            .load(pointer, flags, table, (callee * 8) as i32);
        let found = self.b.ins().icmp_imm_s(IntCC::NotEqual, known, 0);
        self.b.ins().brif(found, have, &[known.into()], slow, &[]);
        // the machine stack must have room; the frame counts on it
        self.switch_to(have);
        let entry = self.b.block_params(have)[0];
        let depth_address = self.b.ins().iadd_imm_s(state, STATE_DEPTH as i64);
        let depth = self.b.ins().load(pointer, flags, depth_address, 0);
        let room = self
            .b
            .ins()
            .icmp_imm_u(IntCC::UnsignedLessThan, depth, DEPTH_LIMIT as i64);
        let go = self.b.create_block();
        self.b.ins().brif(room, go, &[], slow, &[]);
        self.switch_to(go);
        let deeper = self.b.ins().iadd_imm_s(depth, 1);
        self.b.ins().store(flags, deeper, depth_address, 0);
        // the boxed parameters' operands onto the stack, the frame pushed
        let mut mask: u64 = 0;
        for (index, kind) in kinds.iter().enumerate() {
            if kind.is_boxed() {
                mask |= 1 << index;
                self.box_position(first + index);
            }
        }
        // the payloads of the operands taken from the stack, read before
        // the frame moves them to their parameters' slots
        let mut payloads: Vec<Option<IrValue>> = vec![None; count];
        for &index in &unboxed {
            mask |= 1 << index;
            let at = self.operand_address(first + index);
            payloads[index] = Some(match kinds[index] {
                Abs::Int => self.b.ins().load(types::I64, flags, at, INT_PAYLOAD),
                Abs::Bool => self.b.ins().load(types::I8, flags, at, PAYLOAD),
                _ => self.b.ins().load(types::F64, flags, at, PAYLOAD),
            });
        }
        let (base, grant) = self.push_frame_inline(callee, mask);
        let pc0 = self.u32(0);
        let one = self.u8(1);
        // the parameters passed borrowed (decision AW5): the callee's leave
        // skips them and its other ways out give them a reference first;
        // the callee's grant as its record has it (decision AW6)
        let borrowed_mask = self.iconst(types::I64, self.masks[pc] as i64);
        let mut arguments = vec![self.vm, base, pc0, one, borrowed_mask, grant];
        for (index, kind) in kinds.iter().enumerate() {
            let depth = first + index;
            if let Some(payload) = payloads[index] {
                arguments.push(payload);
                continue;
            }
            match kind {
                Abs::Int => {
                    let var = self.int_var(depth);
                    arguments.push(self.b.use_var(var));
                }
                Abs::Bool => {
                    let var = self.bool_var(depth);
                    arguments.push(self.b.use_var(var));
                }
                Abs::Float => {
                    let var = self.float_var(depth);
                    arguments.push(self.b.use_var(var));
                }
                _ => {}
            }
        }
        let sig = direct_signature(self.isa, kinds);
        let sig_ref = self.b.import_signature(sig);
        let call = self.b.ins().call_indirect(sig_ref, entry, &arguments);
        let results = self.b.inst_results(call).to_vec();
        // the callee may have grown the stack
        self.reload_frame();
        let (status, payload) = (results[0], results[1]);
        self.state.truncate(first);
        // the frame is off the machine stack
        let depth = self.b.ins().load(pointer, flags, depth_address, 0);
        let shallower = self.b.ins().iadd_imm_s(depth, -1);
        self.b.ins().store(flags, shallower, depth_address, 0);
        // a pair callee (decision AW9) leaves its two cells with
        // `D_RETURNED` and its record on every other way out: the pair
        // stays as it lies when the op after takes it so (`fused`), the
        // record then taken apart on the other paths; else the pair
        // becomes its record here
        let pair = pair_result(self.program, function);
        let fused = pair.is_some() && self.pairs && pair_fused(self.code, pc);
        let split = fused.then(|| self.b.create_block());
        let rest = self.b.create_block();
        if let Some(ty) = pair {
            let returned = self
                .b
                .ins()
                .icmp_imm_s(IntCC::Equal, status, D_RETURNED as i64);
            let fast = self.b.create_block();
            self.b.ins().brif(returned, fast, &[], rest, &[]);
            self.switch_to(fast);
            if fused {
                self.state.push(Abs::Pair(ty));
            } else {
                self.pair_record_on_top(ty);
                self.push_boxed();
            }
            self.b.ins().jump(join, &[]);
            self.terminated = true;
        } else if result_kind.is_boxed() {
            self.b.ins().jump(rest, &[]);
            self.terminated = true;
        } else {
            let returned = self
                .b
                .ins()
                .icmp_imm_s(IntCC::Equal, status, D_RETURNED as i64);
            let fast = self.b.create_block();
            self.b.ins().brif(returned, fast, &[], rest, &[]);
            self.switch_to(fast);
            self.push_payload(result_kind, payload);
            self.b.ins().jump(join, &[]);
            self.terminated = true;
        }
        // a boxed result is on the stack; anything else is the helper's
        self.switch_to(rest);
        self.state.truncate(first);
        let boxed = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, status, D_BOXED as i64);
        let ok = self.b.create_block();
        let after = self.b.create_block();
        self.b.ins().brif(boxed, ok, &[], after, &[]);
        self.switch_to(ok);
        match split {
            Some(split) => self.b.ins().jump(split, &[]),
            None => {
                self.push_boxed();
                if !result_kind.is_boxed() {
                    self.unbox_top(result_kind, pc + 1);
                }
                self.b.ins().jump(join, &[])
            }
        };
        self.terminated = true;
        self.switch_to(after);
        self.state.truncate(first);
        let settled = self
            .call("rt_direct_after", &[self.vm, status])
            .expect("a status");
        self.check_status(settled, pc, true);
        match split {
            Some(split) => self.b.ins().jump(split, &[]),
            None => {
                self.push_boxed();
                if !result_kind.is_boxed() {
                    self.unbox_top(result_kind, pc + 1);
                }
                self.b.ins().jump(join, &[])
            }
        };
        self.terminated = true;
        // the path through the interpreter's call
        self.switch_to(slow);
        self.state = saved;
        // the VM's path takes its arguments owned (decision AW5)
        self.retain_borrowed_arguments(pc, count);
        self.call_through_helper("rt_call", function, count, result_kind, pc);
        match split {
            Some(split) => {
                self.state.pop();
                self.b.ins().jump(split, &[])
            }
            None => self.b.ins().jump(join, &[]),
        };
        self.terminated = true;
        // the record of a pair callee taken apart into the pair's cells
        if let (Some(ty), Some(split)) = (pair, split) {
            self.switch_to(split);
            self.state.truncate(first);
            self.split_pair_top(ty, pc);
            self.b.ins().jump(join, &[]);
            self.terminated = true;
        }
        // every path leaves one operand of the result's kind
        self.switch_to(join);
    }

    /// Where a pair the call at `pc` leaves goes past the test of its
    /// `otherwise` (decision AW9, `otherwise_end`): the end of the
    /// `otherwise`, when the callee answers a pair the op there takes as it
    /// lies and the region the call ends is the innermost one open.
    fn pair_bypass(&self, function: usize, pc: usize) -> Option<usize> {
        pair_result(self.program, function)?;
        if !(self.pairs && pair_fused(self.code, pc)) {
            return None;
        }
        let end = otherwise_end(self.code, pc)?;
        let innermost = self.analysis.handlers[pc]
            .last()
            .map(|(target, _)| *target as usize);
        (innermost == Some(pc + 2)).then_some(end)
    }

    fn binary(&mut self, op: BinaryOp, pc: usize) {
        let n = self.state.len();
        let (left, right) = (self.state[n - 2], self.state[n - 1]);
        let result = abs_of_binary(op, left, right);
        match (left, right) {
            (Abs::Int, Abs::Int) if !result.is_boxed() => {
                let b = self.pop_int();
                let a = self.pop_int();
                self.int_binary(op, a, b, pc);
            }
            (Abs::Float, Abs::Float) if !result.is_boxed() => {
                let b = self.pop_float();
                let a = self.pop_float();
                self.float_binary(op, a, b, pc);
            }
            (Abs::Bool, Abs::Bool) if !result.is_boxed() => {
                let b = self.pop_bool();
                let a = self.pop_bool();
                let value = match op {
                    BinaryOp::Is => self.b.ins().icmp(IntCC::Equal, a, b),
                    BinaryOp::IsNot => self.b.ins().icmp(IntCC::NotEqual, a, b),
                    BinaryOp::And => self.b.ins().band(a, b),
                    BinaryOp::Or => self.b.ins().bor(a, b),
                    _ => unreachable!(),
                };
                self.push_bool(value);
            }
            _ if is_comparison(op) => self.compare_boxed(op, pc),
            _ => {
                let op_value = self.u8(binary_code(op));
                let pc_value = self.u32(pc as u32);
                let status = self
                    .helper_on_stack("rt_binary", 2, &[op_value, pc_value])
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, false);
                if result == Abs::Bool {
                    self.unbox_top(Abs::Bool, pc + 1);
                }
            }
        }
    }

    /// A comparison on boxed operands (decision AU13): the helper compares
    /// them where they lie, the ones a `Load`, a `Const` or `Nothing` just
    /// before pushed borrowed (`borrowed_operands`), and answers the
    /// Boolean into the out slot, from where it goes to a register; when
    /// the operands need the general path, its boxed answer on the stack
    /// is taken into the register as a typed call's is.
    fn compare_boxed(&mut self, op: BinaryOp, pc: usize) {
        let join = self.b.create_block();
        let texts = matches!(op, BinaryOp::Is | BinaryOp::IsNot) && {
            let n = self.state.len();
            let (left, right) = (self.state[n - 2], self.state[n - 1]);
            left.is_boxed()
                && right.is_boxed()
                && (is_text(self.program, left) || is_text(self.program, right))
        };
        if texts {
            self.compare_small_texts(op, join);
        }
        // two Integers of which at least one is boxed (decision AU45): a
        // small Integer in a register beside it, as a literal bound is
        let integers = {
            let n = self.state.len();
            let (left, right) = (self.state[n - 2], self.state[n - 1]);
            let typed = |abs: Abs| matches!(abs, Abs::Int) || is_integer(self.program, abs);
            (left.is_boxed() || right.is_boxed()) && typed(left) && typed(right)
        };
        if integers {
            self.compare_small_integers(op, join);
        }
        // two values of one variant type the checker typed (decision
        // AW13): compared in place by the type and the tag of their
        // blocks, the same tag of a variant with fields left to the helper
        let summed = if matches!(op, BinaryOp::Is | BinaryOp::IsNot) {
            let n = self.state.len();
            sum_compared(self.program, self.state[n - 2], self.state[n - 1])
        } else {
            None
        };
        if let Some(fielded) = summed {
            self.compare_sum_variants(op, pc, join, fielded);
        }
        let op_value = self.u8(binary_code(op));
        let mask_value = self.u32(self.masks[pc]);
        let pc_value = self.u32(pc as u32);
        let out = self.out_address();
        let status = self
            .helper_on_stack("rt_compare", 2, &[op_value, mask_value, pc_value, out])
            .expect("a status");
        let depth = self.state.len();
        let fast = self.b.create_block();
        let other = self.b.create_block();
        let answered = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, status, CONTINUE as i64);
        self.b.ins().brif(answered, fast, &[], other, &[]);
        self.b.seal_block(fast);
        self.b.seal_block(other);
        // the general path answered on the stack, or stopped the frame
        self.switch_to(other);
        self.push_boxed();
        let slow = self.b.create_block();
        let boxed =
            self.b
                .ins()
                .icmp_imm_s(IntCC::Equal, status, crate::native::runtime::BOXED as i64);
        let exit = self.exit;
        self.b.ins().brif(boxed, slow, &[], exit, &[status.into()]);
        self.b.seal_block(slow);
        self.switch_to(slow);
        self.unbox_top(Abs::Bool, pc + 1);
        self.b.ins().jump(join, &[]);
        // the helper answered into the out slot
        self.switch_to(fast);
        let value = self
            .b
            .ins()
            .load(types::I8, MemFlagsData::trusted(), out, 0);
        let var = self.bool_var(depth);
        self.b.def_var(var, value);
        self.b.ins().jump(join, &[]);
        self.b.seal_block(join);
        self.switch_to(join);
        self.state.truncate(depth);
        self.state.push(Abs::Bool);
    }

    /// `is` or `is not` on two operands of which the checker noted one a
    /// Text (decision AU26): when both are held in the value itself, their
    /// sixteen bytes past the tag are compared as two words, the answer
    /// goes to `join` as the Boolean of the depth the comparison leaves,
    /// and the operands are popped with nothing to release; anything else
    /// carries on to the general comparison in a block of its own.
    fn compare_small_texts(&mut self, op: BinaryOp, join: Block) {
        let flags = MemFlagsData::trusted();
        let height = self.height();
        let left = self.address_at(height - 2);
        let right = self.address_at(height - 1);
        let left_tag = self.tag_at(left, 0);
        let right_tag = self.tag_at(right, 0);
        let left_small = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, left_tag, TAG_SMALL_TEXT as i64);
        let right_small = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, right_tag, TAG_SMALL_TEXT as i64);
        let both = self.b.ins().band(left_small, right_small);
        let inline = self.b.create_block();
        let general = self.b.create_block();
        self.b.ins().brif(both, inline, &[], general, &[]);
        self.b.seal_block(inline);
        self.b.seal_block(general);
        self.switch_to(inline);
        let mut same = None;
        for offset in [PAYLOAD, PAYLOAD + 8] {
            let a = self.b.ins().load(types::I64, flags, left, offset);
            let b = self.b.ins().load(types::I64, flags, right, offset);
            let equal = self.b.ins().icmp(IntCC::Equal, a, b);
            same = Some(match same {
                Some(before) => self.b.ins().band(before, equal),
                None => equal,
            });
        }
        let same = same.expect("two words");
        let answer = match op {
            BinaryOp::Is => same,
            _ => self.b.ins().bxor_imm_u(same, 1),
        };
        let depth = self.state.len() - 2;
        self.store_height(height - 2);
        let var = self.bool_var(depth);
        self.b.def_var(var, answer);
        self.b.ins().jump(join, &[]);
        self.switch_to(general);
    }

    /// A comparison of two Integers of which at least one is boxed and
    /// the checker typed Integer, the other boxed too or a small Integer
    /// in a register (decision AU45): when the boxed ones are small, the
    /// payloads are compared in place with the op's condition, the Boolean
    /// goes to its register and the boxed operands are dropped, since a
    /// small Integer holds no count; a big Integer falls through to the
    /// helper, which has the big Integers.
    fn compare_small_integers(&mut self, op: BinaryOp, join: Block) {
        let flags = MemFlagsData::trusted();
        let n = self.state.len();
        let (left, right) = (self.state[n - 2], self.state[n - 1]);
        let height = self.height();
        // the right operand is the top slot when boxed, the left the slot
        // below it, or the top slot when the right is in a register
        let right_at = right.is_boxed().then(|| self.address_at(height - 1));
        let left_at = left
            .is_boxed()
            .then(|| self.address_at(height - 1 - usize::from(right.is_boxed())));
        let mut small = None;
        for at in [left_at, right_at].into_iter().flatten() {
            for (offset, tag) in [(0, TAG_INTEGER), (INT_TAG, INT_SMALL)] {
                let found = self.tag_at(at, offset);
                let fits = self.b.ins().icmp_imm_s(IntCC::Equal, found, tag as i64);
                small = Some(match small {
                    Some(before) => self.b.ins().band(before, fits),
                    None => fits,
                });
            }
        }
        let small = small.expect("a boxed operand");
        let inline = self.b.create_block();
        let general = self.b.create_block();
        self.b.ins().brif(small, inline, &[], general, &[]);
        self.b.seal_block(inline);
        self.b.seal_block(general);
        self.switch_to(inline);
        let a = match left_at {
            Some(at) => self.b.ins().load(types::I64, flags, at, INT_PAYLOAD),
            None => {
                let var = self.int_var(n - 2);
                self.b.use_var(var)
            }
        };
        let b = match right_at {
            Some(at) => self.b.ins().load(types::I64, flags, at, INT_PAYLOAD),
            None => {
                let var = self.int_var(n - 1);
                self.b.use_var(var)
            }
        };
        let answer = self.b.ins().icmp(comparison_cc(op), a, b);
        let dropped = usize::from(left.is_boxed()) + usize::from(right.is_boxed());
        self.store_height(height - dropped);
        let var = self.bool_var(n - 2);
        self.b.def_var(var, answer);
        self.b.ins().jump(join, &[]);
        self.switch_to(general);
    }

    /// `is` or `is not` on two operands the checker typed as one variant
    /// type without its own `equals` (decision AW13): when both lie as
    /// variants (a guarded one does not), the type and the tag their
    /// blocks hold are compared; different ones answer, the same tag
    /// answers when it names a variant without fields (`fielded` has a
    /// bit per tag that has them) and goes to the general comparison
    /// otherwise, with the operands untouched; an answer goes to `join`
    /// as the Boolean of the depth the comparison leaves, the operands not
    /// borrowed released in place.
    fn compare_sum_variants(&mut self, op: BinaryOp, pc: usize, join: Block, fielded: u64) {
        let flags = MemFlagsData::trusted();
        let height = self.height();
        let left = self.address_at(height - 2);
        let right = self.address_at(height - 1);
        let left_tag = self.tag_at(left, 0);
        let right_tag = self.tag_at(right, 0);
        let left_variant = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, left_tag, TAG_VARIANT as i64);
        let right_variant = self
            .b
            .ins()
            .icmp_imm_s(IntCC::Equal, right_tag, TAG_VARIANT as i64);
        let both = self.b.ins().band(left_variant, right_variant);
        let inline = self.b.create_block();
        let general = self.b.create_block();
        let answered = self.b.create_block();
        self.b.append_block_param(answered, types::I8);
        self.b.ins().brif(both, inline, &[], general, &[]);
        self.b.seal_block(inline);
        self.switch_to(inline);
        let left_block = self.b.ins().load(self.pointer, flags, left, PAYLOAD);
        let right_block = self.b.ins().load(self.pointer, flags, right, PAYLOAD);
        let left_ty = self.b.ins().load(types::I64, flags, left_block, RECORD_TY);
        let right_ty = self.b.ins().load(types::I64, flags, right_block, RECORD_TY);
        let same_ty = self.b.ins().icmp(IntCC::Equal, left_ty, right_ty);
        let left_word = self.b.ins().load(types::I64, flags, left_block, RECORD_TAG);
        let right_word = self
            .b
            .ins()
            .load(types::I64, flags, right_block, RECORD_TAG);
        let same_tag = self.b.ins().icmp(IntCC::Equal, left_word, right_word);
        let same = self.b.ins().band(same_ty, same_tag);
        let is = matches!(op, BinaryOp::Is);
        if fielded == 0 {
            let answer = if is {
                same
            } else {
                self.b.ins().bxor_imm_u(same, 1)
            };
            self.b.ins().jump(answered, &[answer.into()]);
        } else {
            let alike = self.b.create_block();
            let differ = self.b.create_block();
            let plain = self.b.create_block();
            self.b.ins().brif(same, alike, &[], differ, &[]);
            self.b.seal_block(alike);
            self.b.seal_block(differ);
            // the same tag: the helper compares the fields of a variant
            // that has them
            self.switch_to(alike);
            let mask = self.iconst(types::I64, fielded as i64);
            let shifted = self.b.ins().ushr(mask, left_word);
            let bit = self.b.ins().band_imm_s(shifted, 1);
            self.b.ins().brif(bit, general, &[], plain, &[]);
            self.b.seal_block(plain);
            self.switch_to(plain);
            let yes = self.u8(u8::from(is));
            self.b.ins().jump(answered, &[yes.into()]);
            self.switch_to(differ);
            let no = self.u8(u8::from(!is));
            self.b.ins().jump(answered, &[no.into()]);
        }
        self.b.seal_block(answered);
        self.b.seal_block(general);
        self.switch_to(answered);
        let answer = self.b.block_params(answered)[0];
        // an operand a `Load`, a `Const` or `Nothing` pushed borrowed holds
        // no reference of its own
        let mask = self.masks[pc];
        if mask & 1 == 0 {
            self.release(left);
        }
        if mask & 2 == 0 {
            self.release(right);
        }
        let depth = self.state.len() - 2;
        self.store_height(height - 2);
        let var = self.bool_var(depth);
        self.b.def_var(var, answer);
        self.b.ins().jump(join, &[]);
        self.switch_to(general);
    }

    /// Two small Integers in registers; an overflow hands the op to the
    /// interpreter, which has the big Integers.
    fn int_binary(&mut self, op: BinaryOp, a: IrValue, b: IrValue, pc: usize) {
        use BinaryOp::*;
        match op {
            Add | Subtract | Multiply => {
                let (value, overflow) = match op {
                    Add => self.b.ins().sadd_overflow(a, b),
                    Subtract => self.b.ins().ssub_overflow(a, b),
                    _ => self.b.ins().smul_overflow(a, b),
                };
                let fits = self.b.ins().icmp_imm_s(IntCC::Equal, overflow, 0);
                // the operands are back on the abstract stack for the
                // interpreter, which redoes the op
                self.push_int(a);
                self.push_int(b);
                self.deopt_unless(fits, pc);
                self.state.pop();
                self.state.pop();
                self.push_int(value);
            }
            Remainder => {
                let zero = self.b.ins().icmp_imm_s(IntCC::Equal, b, 0);
                let crash = self.b.create_block();
                let cont = self.b.create_block();
                self.b.ins().brif(zero, crash, &[], cont, &[]);
                self.b.set_cold_block(crash);
                self.switch_to(crash);
                let which = self.u32(0);
                let pc_value = self.u32(pc as u32);
                let status = self
                    .call("rt_crash_text", &[self.vm, which, pc_value])
                    .expect("a status");
                self.exit_with(status);
                self.switch_to(cont);
                // `i64::MIN remainder -1` is 0, as `checked_rem` makes it
                let minus_one = self.b.ins().icmp_imm_s(IntCC::Equal, b, -1);
                let one = self.iconst(types::I64, 1);
                let safe = self.b.ins().select(minus_one, one, b);
                let remainder = self.b.ins().srem(a, safe);
                let zero_value = self.iconst(types::I64, 0);
                let value = self.b.ins().select(minus_one, zero_value, remainder);
                self.push_int(value);
            }
            Power => {
                let out = self.out_address();
                let pc_value = self.u32(pc as u32);
                let status = self
                    .call("rt_int_power", &[self.vm, a, b, pc_value, out])
                    .expect("a status");
                let ok = self
                    .b
                    .ins()
                    .icmp_imm_s(IntCC::Equal, status, CONTINUE as i64);
                let cont = self.b.create_block();
                let other = self.b.create_block();
                self.b.ins().brif(ok, cont, &[], other, &[]);
                self.switch_to(other);
                let big = self
                    .b
                    .ins()
                    .icmp_imm_s(IntCC::Equal, status, FAILURE as i64);
                let deopt = self.b.create_block();
                let interrupt = self.b.create_block();
                self.b.ins().brif(big, deopt, &[], interrupt, &[]);
                self.switch_to(deopt);
                self.push_int(a);
                self.push_int(b);
                self.deopt_here(pc);
                self.state.pop();
                self.state.pop();
                self.switch_to(interrupt);
                self.exit_with(status);
                self.switch_to(cont);
                let value = self.out_read(types::I64);
                self.push_int(value);
            }
            Divide => {
                let which = self.u32(2);
                let pc_value = self.u32(pc as u32);
                let status = self
                    .call("rt_crash_text", &[self.vm, which, pc_value])
                    .expect("a status");
                self.exit_with(status);
                // nothing follows a crash; the state stays consistent for
                // the analysis's sake
                self.push_int(a);
            }
            Is | IsNot | IsLessThan | IsAtMost | IsGreaterThan | IsAtLeast => {
                let value = self.b.ins().icmp(comparison_cc(op), a, b);
                self.push_bool(value);
            }
            And | Or => unreachable!("boxed by the analysis"),
        }
    }

    fn float_binary(&mut self, op: BinaryOp, a: IrValue, b: IrValue, pc: usize) {
        use BinaryOp::*;
        match op {
            Is | IsNot | IsLessThan | IsAtMost | IsGreaterThan | IsAtLeast => {
                let cc = match op {
                    Is => FloatCC::Equal,
                    IsNot => FloatCC::NotEqual,
                    IsLessThan => FloatCC::LessThan,
                    IsAtMost => FloatCC::LessThanOrEqual,
                    IsGreaterThan => FloatCC::GreaterThan,
                    _ => FloatCC::GreaterThanOrEqual,
                };
                let value = self.b.ins().fcmp(cc, a, b);
                self.push_bool(value);
            }
            Add | Subtract | Multiply | Divide | Remainder | Power => {
                let out = self.out_address();
                let op_value = self.u8(binary_code(op));
                let pc_value = self.u32(pc as u32);
                let status = self
                    .call("rt_float_binary", &[self.vm, op_value, a, b, pc_value, out])
                    .expect("a status");
                self.check_status(status, pc, false);
                let value = self.out_read(types::F64);
                self.push_float(value);
            }
            And | Or => unreachable!("boxed by the analysis"),
        }
    }
}
