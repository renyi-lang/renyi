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

use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    types, AbiParam, Block, FuncRef, InstBuilder, Signature, StackSlot, StackSlotData,
    StackSlotKind, Type, UserFuncName, Value as IrValue,
};
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::JITModule;
use cranelift_module::{FuncId, Linkage, Module};
use renyi_syntax::ast::BinaryOp;

use crate::bytecode::{Code, Op};
use crate::compile::Program;
use crate::integer::Int;
use crate::native::infer::{
    abs_of_binary, abs_of_constant, abs_of_field, abs_of_result, analyse, boxed_depth, Abs,
    Analysis, SlotKind,
};
use crate::native::runtime::{
    binary_code, fold_code, CONTINUE, DEOPT, FAILURE, INTERRUPT, LEFT, RETURNED, STAY,
};
use crate::native::DeoptPoint;
use crate::value::Value;

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
    ("rt_return_int", "pq", 'v'),
    ("rt_return_bool", "pb", 'v'),
    ("rt_return_float", "pf", 'v'),
    ("rt_return_nothing", "p", 'v'),
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
pub fn signature(module: &JITModule, params: &str, result: char) -> Signature {
    let pointer = module.target_config().pointer_type();
    let mut sig = module.make_signature();
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
        other => panic!("no result letter {other}"),
    }
    sig
}

/// The signature of a generated function: the VM, the frame's base and
/// the pc to enter at (`0`, or a loop header), the status it leaves with.
pub fn entry_signature(module: &JITModule) -> Signature {
    let pointer = module.target_config().pointer_type();
    let mut sig = module.make_signature();
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
pub type Compiled = (FuncId, Vec<DeoptPoint>, Vec<u32>, Stats);

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
    module: &'a mut JITModule,
    /// Every helper by name, and the ones this function has called so
    /// far as its own references, declared on first use.
    helper_ids: &'a HashMap<&'static str, FuncId>,
    helpers: HashMap<&'static str, FuncRef>,
    vm: IrValue,
    base: IrValue,
    /// The pc to enter at: `0`, or one of `headers`.
    pc_param: IrValue,
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
    state: Vec<Abs>,
    /// Whether the current block has been ended by a terminator.
    terminated: bool,
}

/// Compile one code object into the module; the function is defined but
/// not finalized.
pub fn compile(
    program: &Program,
    code_id: usize,
    module: &mut JITModule,
    helper_ids: &HashMap<&'static str, FuncId>,
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
    let sig = entry_signature(module);
    let name = format!("renyi_code_{code_id}");
    let id = module
        .declare_function(&name, Linkage::Local, &sig)
        .map_err(|error| Skipped::Codegen(error.to_string()))?;
    module.clear_context(ctx);
    ctx.func.signature = sig;
    ctx.func.name = UserFuncName::user(0, id.as_u32());
    let target = module.target_config();
    let pointer = target.pointer_type();
    let started = Instant::now();
    let deopts = {
        let mut builder = FunctionBuilder::new(&mut ctx.func, fctx);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        let vm = builder.block_params(entry)[0];
        let base = builder.block_params(entry)[1];
        let pc_param = builder.block_params(entry)[2];
        let exit = builder.create_block();
        builder.append_block_param(exit, types::I32);
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
            module,
            helper_ids,
            helpers: HashMap::new(),
            vm,
            base,
            pc_param,
            headers: headers.clone(),
            positions: Vec::new(),
            slots: vec![SlotVars::None; code.locals as usize],
            blocks: HashMap::new(),
            exit,
            landings: Vec::new(),
            out_slot,
            deopt_slot,
            deopts: Vec::new(),
            state: Vec::new(),
            terminated: false,
        };
        gen.declare_slots();
        gen.prologue();
        gen.body();
        gen.epilogue();
        gen.b.seal_all_blocks();
        let deopts = std::mem::take(&mut gen.deopts);
        gen.b.finalize(target);
        deopts
    };
    stats.ir = started.elapsed();
    stats.instructions = ctx.func.dfg.num_insts();
    stats.blocks = ctx.func.layout.blocks().count();
    let started = Instant::now();
    module
        .define_function(id, ctx)
        .map_err(|error| Skipped::Codegen(format!("{error:?}")))?;
    stats.cranelift = started.elapsed();
    let headers = headers.into_iter().map(|header| header as u32).collect();
    Ok((id, deopts, headers, stats))
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
        let func = match self.helpers.get(name) {
            Some(func) => *func,
            None => {
                let id = *self
                    .helper_ids
                    .get(name)
                    .unwrap_or_else(|| panic!("no helper {name}"));
                let func = self.module.declare_func_in_func(id, self.b.func);
                self.helpers.insert(name, func);
                func
            }
        };
        let inst = self.b.ins().call(func, args);
        self.b.inst_results(inst).first().copied()
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

    fn return_status(&mut self, status: i32) {
        let value = self.iconst(types::I32, status as i64);
        self.b.ins().return_(&[value]);
        self.terminated = true;
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
        let above_value = self.usize(above);
        match self.state[depth] {
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
        self.return_status(DEOPT);
    }

    /// Continue when `ok` (an `i8`), else hand the frame to the
    /// interpreter at `pc` with the current state.
    fn deopt_unless(&mut self, ok: IrValue, pc: usize) {
        let cont = self.b.create_block();
        let deopt = self.b.create_block();
        self.b.ins().brif(ok, cont, &[], deopt, &[]);
        self.b.set_cold_block(deopt);
        self.b.seal_block(cont);
        self.b.seal_block(deopt);
        self.switch_to(deopt);
        self.deopt_here(pc);
        self.switch_to(cont);
    }

    /// Take the boxed value on top of the stack into a register of the
    /// kind, or hand the frame to the interpreter at `pc` (the value stays
    /// boxed on the stack, where the interpreter expects it).
    fn unbox_top(&mut self, kind: Abs, pc: usize) {
        let out = self.out_address();
        let depth = self.state.len() - 1;
        match kind {
            Abs::Int => {
                let ok = self
                    .call("rt_take_int", &[self.vm, out])
                    .expect("an answer");
                self.deopt_unless(ok, pc);
                let value = self.out_read(types::I64);
                let var = self.int_var(depth);
                self.b.def_var(var, value);
            }
            Abs::Float => {
                let ok = self
                    .call("rt_take_float", &[self.vm, out])
                    .expect("an answer");
                self.deopt_unless(ok, pc);
                let value = self.out_read(types::F64);
                let var = self.float_var(depth);
                self.b.def_var(var, value);
            }
            Abs::Bool => {
                // a Boolean is never big or guarded: the take cannot fail
                let pc_value = self.u32(pc as u32);
                let status = self
                    .call("rt_take_bool", &[self.vm, pc_value, out])
                    .expect("a status");
                self.check_status(status, pc, false);
                let value = self.out_read(types::I8);
                let var = self.bool_var(depth);
                self.b.def_var(var, value);
            }
            _ => return,
        }
        self.state[depth] = kind;
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
                let out = self.out_address();
                let pc_value = self.u32(pc as u32);
                let status = self
                    .call("rt_take_bool", &[self.vm, pc_value, out])
                    .expect("a status");
                self.check_status(status, pc, false);
                self.out_read(types::I8)
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

    // ------------------------------------------------------------ the body

    fn prologue(&mut self) {
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
        // the typed parameters, unboxed under a check; a check that fails
        // leaves the frame to the interpreter, which starts at pc 0
        let out = self.out_address();
        for slot in 0..self.code.params as usize {
            let kind = self.analysis.slots[slot];
            let (helper, ty) = match kind {
                SlotKind::Int => ("rt_param_int", types::I64),
                SlotKind::Bool => ("rt_param_bool", types::I8),
                SlotKind::Float => ("rt_param_float", types::F64),
                _ => continue,
            };
            let slot_value = self.u32(slot as u32);
            let ok = self
                .call(helper, &[self.vm, self.base, slot_value, out])
                .expect("an answer");
            self.leave_unless(ok, DEOPT);
            let value = self.out_read(ty);
            let var = self.slot_var(slot);
            self.b.def_var(var, value);
        }
        self.jump_to(0);
        for (header, block) in resumes {
            self.switch_to(block);
            self.resume_at(header);
        }
    }

    /// Continue when `ok` (an `i8`), else return the status to the VM with
    /// the frame untouched.
    fn leave_unless(&mut self, ok: IrValue, status: i32) {
        let cont = self.b.create_block();
        let leave = self.b.create_block();
        self.b.ins().brif(ok, cont, &[], leave, &[]);
        self.b.set_cold_block(leave);
        self.b.seal_block(cont);
        self.b.seal_block(leave);
        self.switch_to(leave);
        self.return_status(status);
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
                    self.leave_unless(ok, STAY);
                    let value = self.out_read(ty);
                    let var = self.slot_var(slot);
                    self.b.def_var(var, value);
                }
                SlotKind::RangeIter => {
                    let ok = self
                        .call("rt_resume_range", &[self.vm, self.base, slot_value, out])
                        .expect("an answer");
                    self.leave_unless(ok, STAY);
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
        self.emit_landings();
        // the end of the code: an implicit `return nothing`
        let count = self.code.ops.len();
        if let Some(block) = self.blocks.get(&count).copied() {
            self.switch_to(block);
            self.call("rt_return_nothing", &[self.vm]);
            self.return_status(RETURNED);
        }
        // the exit: `LEFT` is a return, anything else an interrupt
        let exit = self.exit;
        self.switch_to(exit);
        let status = self.b.block_params(exit)[0];
        let left = self.b.ins().icmp_imm_s(IntCC::Equal, status, LEFT as i64);
        let returned = self.iconst(types::I32, RETURNED as i64);
        let interrupted = self.iconst(types::I32, INTERRUPT as i64);
        let result = self.b.ins().select(left, returned, interrupted);
        self.b.ins().return_(&[result]);
        self.terminated = true;
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
                        let index = self.u32(*index);
                        self.call("rt_push_const", &[self.vm, code_value, index]);
                        self.push_boxed();
                    }
                }
            }
            Op::Nothing => {
                self.call("rt_push_nothing", &[self.vm]);
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
                        let helper = if matches!(self.code.ops[pc], Op::LoadMove(_)) {
                            "rt_load_move"
                        } else {
                            "rt_load"
                        };
                        let slot_value = self.u32(*slot as u32);
                        self.call(helper, &[self.vm, self.base, slot_value]);
                        self.push_boxed();
                    }
                }
            }
            Op::Store(slot) => {
                let index = *slot as usize;
                let slot_value = self.u32(*slot as u32);
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
                    (SlotKind::Boxed, Abs::Int) => {
                        let value = self.pop_int();
                        self.call("rt_store_int", &[self.vm, self.base, slot_value, value]);
                    }
                    (SlotKind::Boxed, Abs::Bool) => {
                        let value = self.pop_bool();
                        self.call("rt_store_bool", &[self.vm, self.base, slot_value, value]);
                    }
                    (SlotKind::Boxed, Abs::Float) => {
                        let value = self.pop_float();
                        self.call("rt_store_float", &[self.vm, self.base, slot_value, value]);
                    }
                    _ => {
                        self.box_top(1);
                        self.call("rt_store", &[self.vm, self.base, slot_value]);
                        self.state.pop();
                    }
                }
            }
            Op::Pop => {
                if self.state.last().is_some_and(|abs| abs.is_boxed()) {
                    self.call("rt_pop", &[self.vm]);
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
                        self.call("rt_dup", &[self.vm]);
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
            Op::With(count) => {
                let count_value = self.u32(*count as u32);
                let status = self
                    .helper_on_stack("rt_with", 2 * *count as usize + 1, &[count_value, pc_value])
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, true);
            }
            Op::Call { function, args } => {
                let function_value = self.usize(*function);
                let args_value = self.u32(*args as u32);
                let status = self
                    .helper_on_stack(
                        "rt_call",
                        *args as usize,
                        &[function_value, args_value, pc_value],
                    )
                    .expect("a status");
                self.push_boxed();
                self.check_status(status, pc, true);
                let kind = abs_of_result(self.program, *function);
                if !kind.is_boxed() {
                    self.unbox_top(kind, pc + 1);
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
                    let helper = if matches!(self.code.ops[pc], Op::JumpIfAbsent(_)) {
                        "rt_top_is_absent"
                    } else {
                        "rt_top_is_failure"
                    };
                    let taken = self.call(helper, &[self.vm]).expect("an answer");
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
                match self.state[depth] {
                    Abs::Int => {
                        let value = self.pop_int();
                        self.call("rt_return_int", &[self.vm, value]);
                    }
                    Abs::Bool => {
                        let value = self.pop_bool();
                        self.call("rt_return_bool", &[self.vm, value]);
                    }
                    Abs::Float => {
                        let value = self.pop_float();
                        self.call("rt_return_float", &[self.vm, value]);
                    }
                    _ => {
                        self.box_top(1);
                        self.state.pop();
                        self.call("rt_return", &[self.vm]);
                    }
                }
                self.return_status(RETURNED);
            }
            Op::ReturnNothing => {
                self.call("rt_return_nothing", &[self.vm]);
                self.return_status(RETURNED);
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
            Op::IsNothing => {
                let out = self.out_address();
                self.helper_on_stack("rt_is_nothing", 1, &[out]);
                let value = self.out_read(types::I8);
                self.push_bool(value);
            }
            Op::IsFailure => {
                let out = self.out_address();
                self.helper_on_stack("rt_is_failure", 1, &[out]);
                let value = self.out_read(types::I8);
                self.push_bool(value);
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
