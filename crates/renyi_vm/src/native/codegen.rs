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
use crate::integer::Int;
use crate::native::infer::{
    abs_of_binary, abs_of_constant, abs_of_field, abs_of_params, abs_of_result, abs_of_slot,
    analyse, boxed_depth, Abs, Analysis, SlotKind,
};
use crate::native::runtime::{
    binary_code, fold_code, CONTINUE, DEOPT, D_BOXED, D_DEOPT, D_FAILURE, D_INTERRUPT, D_RETURNED,
    D_STAY, FAILURE, LEFT, RETURNED,
};
use crate::native::{DeoptPoint, DEPTH_LIMIT};
use crate::value::layout::{
    INT_PAYLOAD, INT_SMALL, INT_TAG, PAYLOAD, RC_VALUE, RECORD_FIELDS, RECORD_TAG, RECORD_TY, SIZE,
    TAG_BOOLEAN, TAG_FAILURE, TAG_FLOAT, TAG_INTEGER, TAG_NOTHING, TAG_RECORD,
};
use crate::value::Value;
use crate::vm::{FieldSite, Frame, NativeState, Vm};

/// Where the VM keeps its stack (decision AR4): the generated code reads
/// the pinned vector's pointer, length and capacity there.
fn stack_offset() -> i64 {
    std::mem::offset_of!(Vm<'static>, stack) as i64
}

/// Where the VM keeps its frames and its handlers, and the layout of a
/// frame record (decision AR4): a direct call pushes the callee's frame
/// in place and a return pops it.
fn frames_offset() -> i64 {
    std::mem::offset_of!(Vm<'static>, frames) as i64
}

fn handlers_offset() -> i64 {
    std::mem::offset_of!(Vm<'static>, handlers) as i64
}

const FRAME_SIZE: usize = std::mem::size_of::<Frame>();
const FRAME_CODE: i32 = std::mem::offset_of!(Frame, code) as i32;
const FRAME_PC: i32 = std::mem::offset_of!(Frame, pc) as i32;
const FRAME_BASE: i32 = std::mem::offset_of!(Frame, base) as i32;
const FRAME_HANDLER_BASE: i32 = std::mem::offset_of!(Frame, handler_base) as i32;
const FRAME_GRANT: i32 = std::mem::offset_of!(Frame, grant) as i32;

/// Where the VM keeps the cache of the field sites, and an entry's layout
/// (decision AR4): a field load reads the entry and the holder in place.
fn field_cache_offset() -> i64 {
    std::mem::offset_of!(Vm<'static>, field_cache) as i64
}

const SITE_SIZE: usize = std::mem::size_of::<FieldSite>();
const SITE_TY: i32 = std::mem::offset_of!(FieldSite, ty) as i32;
const SITE_TAG: i32 = std::mem::offset_of!(FieldSite, tag) as i32;
const SITE_INDEX: i32 = std::mem::offset_of!(FieldSite, index) as i32;

/// How many values a return releases one by one before it calls the
/// helper that cuts the stack instead.
const RELEASES_INLINE: usize = 4;

/// The helpers that cannot move the stack's vector (they grow nothing
/// and run no program code), after which the frame pointer needs no
/// reload (decision AT6; a helper not listed is reloaded after, which is
/// always sound). The list is kept short on purpose: a helper that runs
/// program code, as most do, may grow the stack through the frames it
/// pushes.
const STACK_SAFE: &[&str] = &[
    "rt_drop_at",
    "rt_retain_at",
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
fn native_state_offset() -> i64 {
    std::mem::offset_of!(Vm<'static>, native_state) as i64
}

const STATE_DEPTH: i32 = std::mem::offset_of!(NativeState, depth) as i32;
const STATE_DIRECT: i32 = std::mem::offset_of!(NativeState, direct_table) as i32;
const STATE_HELPERS: i32 = std::mem::offset_of!(NativeState, helpers) as i32;
const STATE_CONSTANTS: i32 = std::mem::offset_of!(NativeState, constants) as i32;

/// The position of a helper in `SIGNATURES`, which is its index in the
/// VM's table of helpers.
fn helper_index(name: &str) -> usize {
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
    ("rt_retain_at", "p", 'v'),
    ("rt_room", "pz", 'v'),
    ("rt_grow_frames", "p", 'v'),
    ("rt_frame_grant", "pz", 'i'),
    ("rt_dup", "p", 'v'),
    ("rt_truncate", "pzww", 'v'),
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
    ("rt_field", "pzwww", 'i'),
    ("rt_load_field", "pzwzwww", 'i'),
    ("rt_with", "pww", 'i'),
    ("rt_not", "pw", 'i'),
    ("rt_binary", "puw", 'i'),
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
/// caller wants it on the stack), then the typed parameters in slot order;
/// the status (`runtime::D_*`) and the bits of a typed result come back.
pub fn direct_signature(isa: &dyn TargetIsa, kinds: &[Abs]) -> Signature {
    let pointer = isa.pointer_type();
    let mut sig = Signature::new(isa.default_call_conv());
    sig.params.push(AbiParam::new(pointer));
    sig.params.push(AbiParam::new(pointer));
    sig.params.push(AbiParam::new(types::I32));
    sig.params.push(AbiParam::new(types::I8));
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
    code: &'a Code,
    analysis: &'a Analysis,
    b: FunctionBuilder<'b>,
    pointer: Type,
    isa: &'a dyn TargetIsa,
    /// The VM's table of helpers, read once at the entry: every helper is
    /// called through it (decision AS1).
    helper_table: IrValue,
    /// The signatures imported so far, by helper name.
    helper_sigs: HashMap<&'static str, SigRef>,
    vm: IrValue,
    base: IrValue,
    /// The pc to enter at: `0`, or one of `headers`.
    pc_param: IrValue,
    /// Whether a direct call entered the frame (an `i8`): a typed result
    /// then leaves in a register, else on the stack.
    direct: IrValue,
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
}

/// Compile one code object (decision AS1): its body and its trampoline
/// as machine code that holds no address, with the deopt points and the
/// loop headers the VM keeps beside them.
pub fn compile(
    program: &Program,
    code_id: usize,
    isa: &dyn TargetIsa,
    ctx: &mut Context,
    fctx: &mut FunctionBuilderContext,
) -> Result<Compiled, Skipped> {
    let code = &program.codes[code_id];
    let mut stats = Stats::default();
    let started = Instant::now();
    let analysis = analyse(program, code).map_err(Skipped::Analysis)?;
    stats.analysis = started.elapsed();
    let mut headers: Vec<usize> = Vec::new();
    for (pc, op) in code.ops.iter().enumerate() {
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
    let result_kind = match (code.kind, code.function) {
        (CodeKind::Function, Some(function)) => abs_of_result(program, function),
        _ => Abs::Boxed,
    };
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
        let typed_params: Vec<(usize, IrValue)> = kinds
            .iter()
            .enumerate()
            .filter(|(_, kind)| !kind.is_boxed())
            .zip(builder.block_params(entry)[4..].iter())
            .map(|((slot, _), value)| (slot, *value))
            .collect();
        let exit = builder.create_block();
        builder.append_block_param(exit, types::I32);
        let base24 = builder.ins().imul_imm_s(base, SIZE as i64);
        let frame_var = builder.declare_var(pointer);
        // the VM's table of helpers, read once (decision AS1)
        let helper_table = builder.ins().load(
            pointer,
            MemFlagsData::trusted(),
            vm,
            (native_state_offset() + STATE_HELPERS as i64) as i32,
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
        let words = (code.locals as usize + max_depth) * 3 + 1;
        let deopt_slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            (words * 8) as u32,
            3,
        ));
        let mut gen = Gen {
            program,
            code_id,
            code,
            analysis: &analysis,
            b: builder,
            pointer,
            isa,
            helper_table,
            helper_sigs: HashMap::new(),
            vm,
            base,
            pc_param,
            direct,
            typed_params,
            base24,
            frame_var,
            max_depth,
            result_kind,
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
    let body = machine_code(ctx, isa)?;
    let trampoline = trampoline(isa, ctx, fctx, code_id, &kinds)?;
    stats.cranelift = started.elapsed();
    let headers = headers.into_iter().map(|header| header as u32).collect();
    Ok(Compiled {
        body,
        trampoline,
        deopts,
        headers,
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
        let start = b.create_block();
        let resume = b.create_block();
        let finish = b.create_block();
        b.append_block_param(finish, types::I32);
        b.append_block_param(finish, types::I64);
        let at_start = b.ins().icmp_imm_s(IntCC::Equal, pc, 0);
        b.ins().brif(at_start, start, &[], resume, &[]);
        // the start: the typed parameters from the frame's slots
        b.switch_to_block(start);
        let mut args = vec![vm, base, pc, zero8];
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
        let mut args = vec![vm, base, pc, zero8];
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
        let below: Vec<Abs> = match handler {
            Some((_, depth)) => self.state[..depth].to_vec(),
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
                    let locals = self.u32(self.code.locals as u32);
                    let floor = self.u32(floor);
                    self.call("rt_settle_handler", &[self.vm, self.base, locals, floor]);
                    self.state.push(Abs::Boxed);
                    self.jump_to(target as usize);
                }
                None => {
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
            self.state[depth] = Abs::Boxed;
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
            Abs::Boxed | Abs::Unset => return,
        }
        self.state[depth] = Abs::Boxed;
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
            locals: self.code.locals,
            stack: self.state.clone(),
            slots: self.analysis.slots.clone(),
            marks: self.analysis.marks.clone(),
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
                Abs::Boxed | Abs::Unset => {}
            }
        }
        let code = self.usize(self.code_id);
        let point = self.u32(index);
        let buffer = self.b.ins().stack_addr(self.pointer, self.deopt_slot, 0);
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
        let known = self
            .deopt_blocks
            .iter()
            .find(|((at, state), _)| *at == pc && *state == self.state);
        if let Some((_, block)) = known {
            return *block;
        }
        let block = self.b.create_block();
        self.b.set_cold_block(block);
        self.deopt_blocks.push(((pc, self.state.clone()), block));
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

    fn push_boxed(&mut self) {
        self.state.push(Abs::Boxed);
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
    /// state: the locals, then the boxed operands (a typed operand is in
    /// a register). The stack's length is `base` plus this at every op,
    /// so the generated code computes it and never loads it (decision
    /// AT4).
    fn height(&self) -> usize {
        self.code.locals as usize + boxed_depth(&self.state)
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

    /// One more reference to the value at the address: what `Value::clone`
    /// does to the count, through the helper (decision AT6: the sequence
    /// AR4 generated in place, the tag's bit in `RC_TAGS`, the big-integer
    /// case, two payload loads and a select, measured larger and slower
    /// than the call).
    fn retain(&mut self, at: IrValue) {
        self.call("rt_retain_at", &[at]);
    }

    /// One reference fewer to the value at the address: what dropping a
    /// `Value` does, through the helper, which frees the last (decision
    /// AT6). The slot is dead after.
    fn release(&mut self, at: IrValue) {
        self.call("rt_drop_at", &[at]);
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
    /// written `Nothing` by the callee's prologue, decision AT2), and the
    /// frame record carries the grant of decision Q1:
    /// the caller's, unless the callee narrows it, when the helper
    /// computes it. The new frame's base is returned.
    fn push_frame_inline(&mut self, callee: usize, mask: u64) -> IrValue {
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
        // the grant first: it is read from the caller's frame, on top
        let frames = self.frames_vector();
        let frames_len = self.b.ins().load(pointer, flags, frames, 8);
        let grant = if narrows {
            let callee_value = self.usize(callee);
            self.call("rt_frame_grant", &[self.vm, callee_value])
                .expect("a grant")
        } else {
            let frames_ptr = self.b.ins().load(pointer, flags, frames, 0);
            let top = self.b.ins().iadd_imm_s(frames_len, -1);
            let record = self.frame_address(frames_ptr, top);
            self.b.ins().load(types::I32, flags, record, FRAME_GRANT)
        };
        // room on the stack for the locals; the callee's base is this
        // frame's height less the boxed arguments (decision AT4)
        let stack = self.stack_vector();
        let callee_height = self.height() - boxed;
        let base = self.length_at(callee_height);
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
        self.b.ins().store(flags, needed, stack, 8);
        // the frame record
        let frames_cap = self.b.ins().load(pointer, flags, frames, 16);
        let full = self.b.ins().icmp(IntCC::Equal, frames_len, frames_cap);
        let grow = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(full, grow, &[], cont, &[]);
        self.b.set_cold_block(grow);
        self.switch_to(grow);
        self.call("rt_grow_frames", &[self.vm]);
        self.b.ins().jump(cont, &[]);
        self.switch_to(cont);
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
        base
    }

    /// The frame left in place (decision AR4): every local that may hold
    /// a reference and the `operands` boxed operands above the locals are
    /// released, the stack is cut to the base, the handlers to the frame's
    /// height, and the frame is popped. A typed local holds `Nothing` or
    /// a plain value and needs no release.
    fn leave_frame_inline(&mut self, operands: usize) {
        let flags = MemFlagsData::trusted();
        let pointer = self.pointer;
        let locals = self.code.locals as usize;
        let holders: Vec<usize> = (0..locals)
            .filter(|slot| {
                !matches!(
                    self.analysis.slots[*slot],
                    SlotKind::Int | SlotKind::Bool | SlotKind::Float
                )
            })
            .collect();
        if holders.len() + operands <= RELEASES_INLINE {
            for slot in holders {
                let at = self.address_at(slot);
                self.release(at);
            }
            for operand in 0..operands {
                let at = self.address_at(locals + operand);
                self.release(at);
            }
            self.store_height(0);
        } else {
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

    /// The boxed result on top of the stack returned (decision AR4): it
    /// moves out, the frame is left, and it goes where the frame's first
    /// local was; the status is `D_FAILURE` for a failure, else
    /// `D_BOXED`.
    fn leave_boxed_inline(&mut self, operands: usize) -> IrValue {
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
        self.leave_frame_inline(operands);
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
        let room = (self.code.locals as usize + self.max_depth + 1) as i64;
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
        // value, with nothing to release)
        let params = self.code.params as usize;
        let locals = self.code.locals as usize;
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
    /// interpreter carries on with the frame as it is.
    fn resume_at(&mut self, header: usize) {
        let out = self.out_address();
        let analysis = self.analysis;
        for (slot, &kind) in analysis.slots.iter().enumerate() {
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
            debug_assert_eq!(
                Some(&self.state),
                self.analysis.entry[pc].as_ref(),
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
                        self.push_copy(from, true);
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
                let index = self.u32(*index);
                let status = self
                    .call("rt_global", &[self.vm, index, pc_value])
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, false);
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
                    _ => {
                        // the slot's value pushed in place: a copy with one
                        // more reference, or the move that leaves `Nothing`
                        let moving = matches!(self.code.ops[pc], Op::LoadMove(_));
                        let from = self.address_at(index);
                        let to = self.top_address();
                        self.copy_value(from, to);
                        if moving {
                            let nothing = self.iconst(types::I32, TAG_NOTHING as i64);
                            self.b
                                .ins()
                                .istore8(MemFlagsData::trusted(), nothing, from, 0);
                        } else {
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
                let top = *self.state.last().expect("an operand");
                match (self.analysis.slots[index], top) {
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
                    (SlotKind::Boxed, Abs::Int | Abs::Bool | Abs::Float) => {
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
                        // the top copied with one more reference
                        let from = self.last_address();
                        let to = self.top_address();
                        self.copy_value(from, to);
                        self.retain(to);
                        let height = self.height() + 1;
                        self.store_height(height);
                        self.push_boxed();
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
                let ty_value = self.usize(*ty);
                let fields_value = self.u32(*fields as u32);
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
            Op::ConstructVariant { ty, tag, fields } => {
                let ty_value = self.usize(*ty);
                let tag_value = self.u32(*tag as u32);
                let fields_value = self.u32(*fields as u32);
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
                let name_value = self.u32(*name);
                let site_value = self.u32(*site);
                let status = self
                    .helper_on_stack(
                        "rt_field",
                        1,
                        &[code_value, name_value, site_value, pc_value],
                    )
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, false);
                let field_name = self.code.constants[*name as usize]
                    .as_text()
                    .unwrap_or("")
                    .to_string();
                let kind = abs_of_field(self.program, &field_name);
                if !kind.is_boxed() {
                    self.unbox_top(kind, pc + 1);
                }
            }
            Op::LoadField { slot, name, site } => {
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
                let ty = self
                    .b
                    .ins()
                    .load(types::I64, flags, rc, RC_VALUE + RECORD_TY);
                let holder_tag = self
                    .b
                    .ins()
                    .load(types::I64, flags, rc, RC_VALUE + RECORD_TAG);
                let fields_ptr = self
                    .b
                    .ins()
                    .load(pointer, flags, rc, RC_VALUE + RECORD_FIELDS);
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
                self.push_boxed();
                let field_name = self.code.constants[*name as usize]
                    .as_text()
                    .unwrap_or("")
                    .to_string();
                let kind = abs_of_field(self.program, &field_name);
                if !kind.is_boxed() {
                    self.unbox_top(kind, pc + 1);
                }
            }
            Op::With(count) => {
                let count_value = self.u32(*count as u32);
                let status = self
                    .helper_on_stack("rt_with", 2 * *count as usize + 1, &[count_value, pc_value])
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, true);
            }
            Op::Call { function, args } => {
                let count = *args as usize;
                let result_kind = abs_of_result(self.program, *function);
                let callee = self
                    .program
                    .function_codes
                    .get(*function)
                    .copied()
                    .flatten();
                let kinds = callee.map(|callee| {
                    let params = self.program.codes[callee].params as usize;
                    abs_of_params(self.program, *function, params)
                });
                let first = self.state.len() - count;
                // a direct call (decision AR3) when the callee has a code
                // object and every typed parameter has a typed operand here
                let direct = match &kinds {
                    Some(kinds) => {
                        kinds.len() == count
                            && count <= 64
                            && kinds.iter().enumerate().all(|(index, kind)| {
                                kind.is_boxed() || self.state[first + index] == *kind
                            })
                    }
                    None => false,
                };
                match (direct, callee, kinds) {
                    (true, Some(callee), Some(kinds)) => {
                        self.call_direct(callee, *function, &kinds, result_kind, pc);
                    }
                    _ => self.call_through_helper(*function, count, result_kind, pc),
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
                if self.state[depth].is_boxed() {
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
                        self.leave_frame_inline(operands);
                        let bits = self.payload_bits(top, value);
                        let status = self.iconst(types::I32, D_RETURNED as i64);
                        self.return_with(status, bits);
                        self.switch_to(on_stack);
                        self.leave_frame_inline(operands);
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
                        self.leave_frame_inline(operands);
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
                        self.leave_frame_inline(operands);
                        let status = self.iconst(types::I32, D_RETURNED as i64);
                        self.return_with(status, bits);
                        self.switch_to(stays);
                        let status = self.leave_boxed_inline(operands);
                        let zero = self.iconst(types::I64, 0);
                        self.return_with(status, zero);
                        self.switch_to(on_stack);
                        let status = self.leave_boxed_inline(operands);
                        let zero = self.iconst(types::I64, 0);
                        self.return_with(status, zero);
                    }
                    _ => {
                        self.box_top(1);
                        self.state.pop();
                        let operands = boxed_depth(&self.state);
                        let status = self.leave_boxed_inline(operands);
                        let zero = self.iconst(types::I64, 0);
                        self.return_with(status, zero);
                    }
                }
            }
            Op::ReturnNothing => {
                let operands = boxed_depth(&self.state);
                self.leave_frame_inline(operands);
                self.push_nothing_at(0);
                self.return_direct(D_BOXED);
            }
            Op::Fail => {
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
                let out = self.out_address();
                let tag_value = self.u32(*tag as u32);
                self.helper_on_stack("rt_is_variant", 1, &[tag_value, out]);
                let value = self.out_read(types::I8);
                self.push_bool(value);
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
                    let slot_value = self.u32(*slot as u32);
                    let has = self
                        .call("rt_iter_next", &[self.vm, self.base, slot_value])
                        .expect("an answer");
                    let (exit_block, fill_exit) = self.edge_to(*exit as usize);
                    let body = self.b.create_block();
                    self.b.ins().brif(has, body, &[], exit_block, &[]);
                    self.terminated = true;
                    if fill_exit {
                        self.fill_edge(exit_block, *exit as usize);
                    }
                    self.switch_to(body);
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
                    let locals = self.u32(self.code.locals as u32);
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

    /// A call through `rt_call`: the arguments boxed on the stack, the
    /// result boxed, and unboxed when the declared result is typed.
    fn call_through_helper(&mut self, function: usize, count: usize, result_kind: Abs, pc: usize) {
        let pc_value = self.u32(pc as u32);
        let function_value = self.usize(function);
        let args_value = self.u32(count as u32);
        let status = self
            .helper_on_stack("rt_call", count, &[function_value, args_value, pc_value])
            .expect("a status");
        self.push_boxed();
        self.check_status(status, pc, true);
        if !result_kind.is_boxed() {
            self.unbox_top(result_kind, pc + 1);
        }
    }

    /// A direct call (decision AR3): the callee's body called with its
    /// typed arguments in registers and the boxed ones on the stack. The
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
        let base = self.push_frame_inline(callee, mask);
        let pc0 = self.u32(0);
        let one = self.u8(1);
        let mut arguments = vec![self.vm, base, pc0, one];
        for (index, kind) in kinds.iter().enumerate() {
            let depth = first + index;
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
        let rest = self.b.create_block();
        if result_kind.is_boxed() {
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
        self.push_boxed();
        if !result_kind.is_boxed() {
            self.unbox_top(result_kind, pc + 1);
        }
        self.b.ins().jump(join, &[]);
        self.terminated = true;
        self.switch_to(after);
        self.state.truncate(first);
        let settled = self
            .call("rt_direct_after", &[self.vm, status])
            .expect("a status");
        self.check_status(settled, pc, true);
        self.push_boxed();
        if !result_kind.is_boxed() {
            self.unbox_top(result_kind, pc + 1);
        }
        self.b.ins().jump(join, &[]);
        self.terminated = true;
        // the path through the interpreter's call
        self.switch_to(slow);
        self.state = saved;
        self.call_through_helper(function, count, result_kind, pc);
        self.b.ins().jump(join, &[]);
        self.terminated = true;
        // every path leaves one operand of the result's kind
        self.switch_to(join);
    }

    fn binary(&mut self, op: BinaryOp, pc: usize) {
        let n = self.state.len();
        let (left, right) = (self.state[n - 2], self.state[n - 1]);
        let result = abs_of_binary(op, left, right);
        match (left, right) {
            (Abs::Int, Abs::Int) if result != Abs::Boxed => {
                let b = self.pop_int();
                let a = self.pop_int();
                self.int_binary(op, a, b, pc);
            }
            (Abs::Float, Abs::Float) if result != Abs::Boxed => {
                let b = self.pop_float();
                let a = self.pop_float();
                self.float_binary(op, a, b, pc);
            }
            (Abs::Bool, Abs::Bool) if result != Abs::Boxed => {
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
                let cc = match op {
                    Is => IntCC::Equal,
                    IsNot => IntCC::NotEqual,
                    IsLessThan => IntCC::SignedLessThan,
                    IsAtMost => IntCC::SignedLessThanOrEqual,
                    IsGreaterThan => IntCC::SignedGreaterThan,
                    _ => IntCC::SignedGreaterThanOrEqual,
                };
                let value = self.b.ins().icmp(cc, a, b);
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
