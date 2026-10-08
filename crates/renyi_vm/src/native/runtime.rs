//! The runtime helpers the generated code calls (decision AG1). Every op
//! the generated code does not do in registers is one of these functions,
//! which do exactly what the interpreter's arm for that op does, on the
//! VM's own stack: the generated code and the interpreter share one
//! semantics. A helper takes the VM, the frame's base and whatever the op
//! carries, and returns a status: `CONTINUE`, `FAILURE` (a `Failure` value
//! is on top of the stack: the generated code jumps to the innermost
//! handled region or leaves the frame as `settle` would), `INTERRUPT`
//! (`Vm::pending` holds a crash or an exit; the frame's pc names the op)
//! or `LEFT` (the frame returned: its result is on top of the caller's
//! stack).
//!
//! The helpers are `extern "C"` and take a raw pointer to the VM, since
//! the generated code holds no borrow; the VM is exclusively owned by the
//! call that entered the generated code, so the reference made here is
//! the only one alive.

use std::fmt::Write;
use std::rc::Rc;

use renyi_syntax::ast::BinaryOp;

use crate::bytecode::{CodeKind, GroupFold};
use crate::integer::Int;
use crate::native::infer::{Abs, SlotKind};
use crate::native::DeoptPoint;
use crate::natives;
use crate::value::{plain_all, Native, RangeValue, Value};
use crate::vm::{Interrupt, Vm};

pub const CONTINUE: i32 = 0;
pub const FAILURE: i32 = 1;
pub const INTERRUPT: i32 = 2;
pub const LEFT: i32 = 3;

/// What the generated function itself returns to the VM: the frame
/// returned, the frame is the interpreter's at the pc `rt_deopt` set, an
/// interrupt is pending, or (from an entry at a loop header) the frame is
/// as the interpreter left it, since a slot did not fit its register.
pub const RETURNED: i32 = 0;
pub const DEOPT: i32 = 1;
pub const STAY: i32 = 3;

/// What the body of a code object (`renyi_direct_*`, decision AR3) returns
/// to its caller, a generated caller or its own trampoline, beside the
/// bits of a typed result: the frame returned a typed result (the bits),
/// the frame left with a failure on the caller's stack, an interrupt is
/// pending, the frame is as the interpreter left it (an entry at a loop
/// header), the frame is the interpreter's now, or the frame returned a
/// boxed result, on the caller's stack.
pub const D_RETURNED: i32 = 0;
pub const D_FAILURE: i32 = 1;
pub const D_INTERRUPT: i32 = 2;
pub const D_STAY: i32 = 3;
pub const D_DEOPT: i32 = 4;
pub const D_BOXED: i32 = 5;

/// The number a kind travels as between the generated code and the
/// helpers that unbox a result by it.
pub fn kind_code(kind: Abs) -> u8 {
    match kind {
        Abs::Int => 0,
        Abs::Bool => 1,
        Abs::Float => 2,
        _ => 3,
    }
}

pub type VmPtr = *mut Vm<'static>;

/// The crash messages the generated code raises by number.
pub const CRASH_TEXTS: [&str; 4] = [
    "division by zero",
    "a range cannot step by 0",
    "`/` is not defined on Integers; use `quotient` or `to_decimal`",
    "the exponent must be between 0 and 1000000",
];

/// The binary operators by the number the generated code passes.
pub const BINARY_OPS: [BinaryOp; 14] = [
    BinaryOp::Is,
    BinaryOp::IsNot,
    BinaryOp::IsLessThan,
    BinaryOp::IsAtMost,
    BinaryOp::IsGreaterThan,
    BinaryOp::IsAtLeast,
    BinaryOp::And,
    BinaryOp::Or,
    BinaryOp::Add,
    BinaryOp::Subtract,
    BinaryOp::Multiply,
    BinaryOp::Divide,
    BinaryOp::Remainder,
    BinaryOp::Power,
];

pub fn binary_code(op: BinaryOp) -> u8 {
    BINARY_OPS
        .iter()
        .position(|candidate| *candidate == op)
        .expect("every operator is numbered") as u8
}

/// The group folds by number.
pub const FOLDS: [GroupFold; 4] = [
    GroupFold::Sum,
    GroupFold::First,
    GroupFold::Any,
    GroupFold::All,
];

pub fn fold_code(fold: GroupFold) -> u8 {
    FOLDS
        .iter()
        .position(|candidate| *candidate == fold)
        .expect("every fold is numbered") as u8
}

// SAFETY of every helper: the pointer is the VM that entered the generated
// code, which holds no other reference to it while the code runs.
macro_rules! vm {
    ($vm:expr) => {
        unsafe { &mut *$vm }
    };
}

/// A helper's `Result` as a status, with an interrupt parked in the VM.
fn status(vm: &mut Vm, result: Result<Value, Interrupt>) -> i32 {
    match result {
        Ok(value) => {
            let failed = value.is_failure();
            vm.stack.push(value);
            if failed {
                FAILURE
            } else {
                CONTINUE
            }
        }
        Err(interrupt) => {
            vm.pending = Some(interrupt);
            INTERRUPT
        }
    }
}

fn interrupt(vm: &mut Vm, interrupt: Interrupt) -> i32 {
    vm.pending = Some(interrupt);
    INTERRUPT
}

// ------------------------------------------------------------ the stack

pub(crate) unsafe extern "C" fn rt_push_const(vm: VmPtr, code: usize, index: u32) {
    let vm = vm!(vm);
    let value = vm.program.codes[code].constants[index as usize].clone();
    vm.stack.push(value);
}

pub(crate) unsafe extern "C" fn rt_push_nothing(vm: VmPtr) {
    vm!(vm).stack.push(Value::Nothing);
}

/// Box a value below `above` boxed operands (0: on top).
pub(crate) unsafe extern "C" fn rt_insert_int(vm: VmPtr, above: usize, value: i64) {
    let vm = vm!(vm);
    let at = vm.stack.len() - above;
    vm.stack.insert(at, Value::integer(value));
}

pub(crate) unsafe extern "C" fn rt_insert_bool(vm: VmPtr, above: usize, value: i8) {
    let vm = vm!(vm);
    let at = vm.stack.len() - above;
    vm.stack.insert(at, Value::Boolean(value != 0));
}

pub(crate) unsafe extern "C" fn rt_insert_float(vm: VmPtr, above: usize, value: f64) {
    let vm = vm!(vm);
    let at = vm.stack.len() - above;
    vm.stack.insert(at, Value::Float(value));
}

pub(crate) unsafe extern "C" fn rt_insert_range(
    vm: VmPtr,
    above: usize,
    from: i64,
    to: i64,
    by: i64,
) {
    let vm = vm!(vm);
    let at = vm.stack.len() - above;
    vm.stack.insert(at, range_value(from, to, by));
}

fn range_value(from: i64, to: i64, by: i64) -> Value {
    Value::Range(Rc::new(RangeValue {
        from: Int::Small(from),
        to: Int::Small(to),
        by: Int::Small(by),
    }))
}

pub(crate) unsafe extern "C" fn rt_global(vm: VmPtr, index: u32, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let result = vm.global(index as usize);
    status(vm, result)
}

pub(crate) unsafe extern "C" fn rt_load(vm: VmPtr, base: usize, slot: u32) {
    let vm = vm!(vm);
    let value = vm.stack[base + slot as usize].clone();
    vm.stack.push(value);
}

pub(crate) unsafe extern "C" fn rt_load_move(vm: VmPtr, base: usize, slot: u32) {
    let vm = vm!(vm);
    let value = std::mem::replace(&mut vm.stack[base + slot as usize], Value::Nothing);
    vm.stack.push(value);
}

pub(crate) unsafe extern "C" fn rt_store(vm: VmPtr, base: usize, slot: u32) {
    let vm = vm!(vm);
    let value = vm.pop();
    vm.stack[base + slot as usize] = value;
}

pub(crate) unsafe extern "C" fn rt_store_int(vm: VmPtr, base: usize, slot: u32, value: i64) {
    vm!(vm).stack[base + slot as usize] = Value::integer(value);
}

pub(crate) unsafe extern "C" fn rt_store_bool(vm: VmPtr, base: usize, slot: u32, value: i8) {
    vm!(vm).stack[base + slot as usize] = Value::Boolean(value != 0);
}

pub(crate) unsafe extern "C" fn rt_store_float(vm: VmPtr, base: usize, slot: u32, value: f64) {
    vm!(vm).stack[base + slot as usize] = Value::Float(value);
}

pub(crate) unsafe extern "C" fn rt_pop(vm: VmPtr) {
    vm!(vm).pop();
}

pub(crate) unsafe extern "C" fn rt_dup(vm: VmPtr) {
    let vm = vm!(vm);
    let top = vm.stack.last().cloned().unwrap_or(Value::Nothing);
    vm.stack.push(top);
}

/// Drop the operands above the frame's `boxed` boxed operands.
pub(crate) unsafe extern "C" fn rt_truncate(vm: VmPtr, base: usize, locals: u32, boxed: u32) {
    vm!(vm)
        .stack
        .truncate(base + locals as usize + boxed as usize);
}

/// Take a small plain Integer off the top into `out`; anything else stays
/// and the answer is 0.
pub(crate) unsafe extern "C" fn rt_take_int(vm: VmPtr, out: *mut i64) -> i8 {
    let vm = vm!(vm);
    match vm.stack.last() {
        Some(Value::Integer(Int::Small(value))) => {
            unsafe { *out = *value };
            vm.stack.pop();
            1
        }
        _ => 0,
    }
}

pub(crate) unsafe extern "C" fn rt_take_float(vm: VmPtr, out: *mut f64) -> i8 {
    let vm = vm!(vm);
    match vm.stack.last() {
        Some(Value::Float(value)) => {
            unsafe { *out = *value };
            vm.stack.pop();
            1
        }
        _ => 0,
    }
}

/// Pop a Boolean into `out`; a crash when the top is not one (the
/// interpreter's `pop_bool`).
pub(crate) unsafe extern "C" fn rt_take_bool(vm: VmPtr, pc: u32, out: *mut i8) -> i32 {
    let vm = vm!(vm);
    match vm.stack.pop() {
        Some(Value::Boolean(value)) => {
            unsafe { *out = value as i8 };
            CONTINUE
        }
        other => {
            vm.sync_pc(pc);
            interrupt(vm, Interrupt::crash(crate::vm::not_boolean(other)))
        }
    }
}

/// Read a parameter of the frame as a small plain Integer.
pub(crate) unsafe extern "C" fn rt_param_int(
    vm: VmPtr,
    base: usize,
    slot: u32,
    out: *mut i64,
) -> i8 {
    match &vm!(vm).stack[base + slot as usize] {
        Value::Integer(Int::Small(value)) => {
            unsafe { *out = *value };
            1
        }
        _ => 0,
    }
}

pub(crate) unsafe extern "C" fn rt_param_bool(
    vm: VmPtr,
    base: usize,
    slot: u32,
    out: *mut i8,
) -> i8 {
    match &vm!(vm).stack[base + slot as usize] {
        Value::Boolean(value) => {
            unsafe { *out = *value as i8 };
            1
        }
        _ => 0,
    }
}

pub(crate) unsafe extern "C" fn rt_param_float(
    vm: VmPtr,
    base: usize,
    slot: u32,
    out: *mut f64,
) -> i8 {
    match &vm!(vm).stack[base + slot as usize] {
        Value::Float(value) => {
            unsafe { *out = *value };
            1
        }
        _ => 0,
    }
}

// ------------------------------------------------------------ values

/// The entries at loop headers (decision AG3): a slot into the register
/// the code keeps it in, or `0` when its value does not fit. A slot not
/// yet assigned (`Nothing`) gives a zero, which the code assigns before
/// it reads it.
pub(crate) unsafe extern "C" fn rt_resume_int(
    vm: VmPtr,
    base: usize,
    slot: u32,
    out: *mut i64,
) -> i8 {
    match &vm!(vm).stack[base + slot as usize] {
        Value::Integer(Int::Small(value)) => {
            unsafe { *out = *value };
            1
        }
        Value::Nothing => {
            unsafe { *out = 0 };
            1
        }
        _ => 0,
    }
}

pub(crate) unsafe extern "C" fn rt_resume_bool(
    vm: VmPtr,
    base: usize,
    slot: u32,
    out: *mut i8,
) -> i8 {
    match &vm!(vm).stack[base + slot as usize] {
        Value::Boolean(value) => {
            unsafe { *out = *value as i8 };
            1
        }
        Value::Nothing => {
            unsafe { *out = 0 };
            1
        }
        _ => 0,
    }
}

pub(crate) unsafe extern "C" fn rt_resume_float(
    vm: VmPtr,
    base: usize,
    slot: u32,
    out: *mut f64,
) -> i8 {
    match &vm!(vm).stack[base + slot as usize] {
        Value::Float(value) => {
            unsafe { *out = *value };
            1
        }
        Value::Nothing => {
            unsafe { *out = 0.0 };
            1
        }
        _ => 0,
    }
}

/// A range iterator slot into its three registers (the next value, the
/// last one and the step), or `0` when the slot holds something else; a
/// slot not yet assigned gives a range that is over before it starts.
pub(crate) unsafe extern "C" fn rt_resume_range(
    vm: VmPtr,
    base: usize,
    slot: u32,
    out: *mut i64,
) -> i8 {
    let words = match &vm!(vm).stack[base + slot as usize] {
        Value::Native(native) => match &**native {
            Native::RangeIterator {
                current,
                to,
                by,
                done,
            } if !done.get() => [current.get(), *to, *by],
            _ => return 0,
        },
        Value::Nothing => [1, 0, 1],
        _ => return 0,
    };
    for (index, word) in words.into_iter().enumerate() {
        unsafe { *out.add(index) = word };
    }
    1
}

pub(crate) unsafe extern "C" fn rt_make_list(vm: VmPtr, count: u32) {
    let vm = vm!(vm);
    let (origins, items) = plain_all(vm.pop_n(count as usize));
    vm.stack.push(Value::list(items).guarded(origins));
}

pub(crate) unsafe extern "C" fn rt_make_map(vm: VmPtr, count: u32) {
    let vm = vm!(vm);
    vm.op_make_map(count as usize);
}

pub(crate) unsafe extern "C" fn rt_make_pair(vm: VmPtr) {
    let vm = vm!(vm);
    let right = vm.pop();
    let left = vm.pop();
    let origins = left.origins() | right.origins();
    vm.stack
        .push(Value::pair(left.into_plain(), right.into_plain()).guarded(origins));
}

pub(crate) unsafe extern "C" fn rt_make_range(vm: VmPtr, stepped: i8, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let result = vm.op_make_range(stepped != 0);
    status(vm, result)
}

pub(crate) unsafe extern "C" fn rt_construct(vm: VmPtr, ty: usize, fields: u32, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let (origins, fields) = plain_all(vm.pop_n(fields as usize));
    let result = vm.construct(ty, fields).map(|value| value.guarded(origins));
    status(vm, result)
}

pub(crate) unsafe extern "C" fn rt_construct_variant(
    vm: VmPtr,
    ty: usize,
    tag: u32,
    fields: u32,
    pc: u32,
) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let (origins, fields) = plain_all(vm.pop_n(fields as usize));
    let result = vm
        .construct_variant(ty, tag as usize, fields)
        .map(|value| value.guarded(origins));
    status(vm, result)
}

pub(crate) unsafe extern "C" fn rt_field(
    vm: VmPtr,
    code: usize,
    name: u32,
    site: u32,
    pc: u32,
) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let holder = vm.pop();
    let result = vm.op_field(code, name as usize, site as usize, &holder);
    status(vm, result)
}

/// `Op::LoadField`: the field read in the slot (`Vm::op_load_field`).
pub(crate) unsafe extern "C" fn rt_load_field(
    vm: VmPtr,
    base: usize,
    slot: u32,
    code: usize,
    name: u32,
    site: u32,
    pc: u32,
) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let result = vm.op_load_field(code, base + slot as usize, name as usize, site as usize);
    status(vm, result)
}

pub(crate) unsafe extern "C" fn rt_with(vm: VmPtr, count: u32, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let result = vm.op_with(count as usize);
    status(vm, result)
}

pub(crate) unsafe extern "C" fn rt_not(vm: VmPtr, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    match vm.pop_bool() {
        Ok(value) => {
            vm.stack.push(Value::Boolean(!value));
            CONTINUE
        }
        Err(error) => interrupt(vm, error),
    }
}

/// A binary operator on two boxed operands: the interpreter's arm.
pub(crate) unsafe extern "C" fn rt_binary(vm: VmPtr, op: u8, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let right = vm.pop();
    let left = vm.pop();
    let result = vm.binary_values(BINARY_OPS[op as usize], left, right);
    status(vm, result)
}

/// `+ - * / remainder power` on two Floats in registers; a result that is
/// not finite is a crash, as in the interpreter.
pub(crate) unsafe extern "C" fn rt_float_binary(
    vm: VmPtr,
    op: u8,
    left: f64,
    right: f64,
    pc: u32,
    out: *mut f64,
) -> i32 {
    let vm = vm!(vm);
    match crate::vm::arithmetic(
        BINARY_OPS[op as usize],
        Value::Float(left),
        Value::Float(right),
    ) {
        Ok(Value::Float(value)) => {
            unsafe { *out = value };
            CONTINUE
        }
        Ok(_) => {
            vm.sync_pc(pc);
            interrupt(vm, Interrupt::crash("a Float operation produced no Float"))
        }
        Err(error) => {
            vm.sync_pc(pc);
            interrupt(vm, error)
        }
    }
}

/// `power` on two small Integers: `CONTINUE` with the result when it fits a
/// machine word, `FAILURE` when it does not (the generated code hands the
/// op to the interpreter), a crash for a bad exponent.
pub(crate) unsafe extern "C" fn rt_int_power(
    vm: VmPtr,
    left: i64,
    right: i64,
    pc: u32,
    out: *mut i64,
) -> i32 {
    let vm = vm!(vm);
    match Int::Small(left).power(&Int::Small(right)) {
        Some(Int::Small(value)) => {
            unsafe { *out = value };
            CONTINUE
        }
        Some(Int::Big(_)) => FAILURE,
        None => {
            vm.sync_pc(pc);
            interrupt(vm, Interrupt::crash(CRASH_TEXTS[3]))
        }
    }
}

pub(crate) unsafe extern "C" fn rt_to_text(vm: VmPtr, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let value = vm.pop();
    let result = vm
        .to_text(&value)
        .map(|text| Value::text(text).guarded(value.origins()));
    status(vm, result)
}

pub(crate) unsafe extern "C" fn rt_concat(vm: VmPtr, count: u32, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let result = vm.op_concat(count as usize);
    status(vm, result)
}

pub(crate) unsafe extern "C" fn rt_is_variant(vm: VmPtr, tag: u32, out: *mut i8) {
    let vm = vm!(vm);
    let value = vm.pop();
    let fits = matches!(value.plain(), Value::Variant(v) if v.tag == tag as usize);
    unsafe { *out = fits as i8 };
}

pub(crate) unsafe extern "C" fn rt_is_nothing(vm: VmPtr, out: *mut i8) {
    let vm = vm!(vm);
    let value = vm.pop();
    unsafe { *out = value.is_nothing() as i8 };
}

pub(crate) unsafe extern "C" fn rt_is_failure(vm: VmPtr, out: *mut i8) {
    let vm = vm!(vm);
    let value = vm.pop();
    unsafe { *out = value.is_failure() as i8 };
}

pub(crate) unsafe extern "C" fn rt_is_type(vm: VmPtr, ty: usize, out: *mut i8) {
    let vm = vm!(vm);
    let value = vm.pop();
    let fits = vm.has_type(value.plain(), ty);
    unsafe { *out = fits as i8 };
}

pub(crate) unsafe extern "C" fn rt_unpack(vm: VmPtr, count: u32, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let value = vm.pop();
    let origins = value.origins();
    match vm.unpack(value.into_plain(), count as usize) {
        Ok(parts) => {
            vm.stack
                .extend(parts.into_iter().map(|part| part.guarded(origins)));
            CONTINUE
        }
        Err(error) => interrupt(vm, error),
    }
}

pub(crate) unsafe extern "C" fn rt_unwrap_failure(vm: VmPtr) {
    let vm = vm!(vm);
    let value = vm.pop();
    match value {
        Value::Failure(error) => vm.stack.push((*error).clone()),
        other => vm.stack.push(other),
    }
}

// ------------------------------------------------------------ calls

/// A call to a declared function or a library primitive with its
/// arguments on top of the stack: the result replaces them.
pub(crate) unsafe extern "C" fn rt_call(vm: VmPtr, function: usize, args: u32, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let result = vm.call_from_stack(function, args as usize);
    status(vm, result)
}

pub(crate) unsafe extern "C" fn rt_call_ability(
    vm: VmPtr,
    ability: usize,
    method: u32,
    args: u32,
    pc: u32,
) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let count = args as usize;
    let result = match vm.ability_target(ability, method as usize, count) {
        Some(function) => vm.call_from_stack(function, count),
        None => {
            let args = vm.pop_n(count);
            vm.derived_ability(ability, method as usize, args)
        }
    };
    status(vm, result)
}

pub(crate) unsafe extern "C" fn rt_call_value(vm: VmPtr, args: u32, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let count = args as usize;
    let Some(at) = vm.stack.len().checked_sub(count + 1) else {
        return interrupt(vm, Interrupt::crash("a call without its function value"));
    };
    let result = match vm.stack.remove(at) {
        Value::Function(id) => vm.call_from_stack(id, count),
        other => Err(Interrupt::crash(format!(
            "cannot call {}",
            other.kind_name()
        ))),
    };
    status(vm, result)
}

/// The body of a code object for a direct call (decision AR3): its address
/// when the callee is compiled, or hot enough to be compiled now, and the
/// machine stack has room; else null, and the caller goes through
/// `rt_call`.
pub(crate) unsafe extern "C" fn rt_direct_entry(vm: VmPtr, code: usize) -> *const u8 {
    vm!(vm).direct_entry_of(code)
}

/// The frame of a direct call (`Vm::push_frame_direct`): the boxed
/// arguments lie on the stack, `mask` says which parameters they are, and
/// the frame's base comes back.
pub(crate) unsafe extern "C" fn rt_direct_frame(vm: VmPtr, code: usize, mask: u64) -> usize {
    vm!(vm).push_frame_direct(code, mask)
}

/// After a direct call whose callee returned neither a typed result nor a
/// boxed one (which the generated code handles itself): a failure is on
/// the stack (`FAILURE`), the frame was handed to the interpreter and is
/// run to its end here, or an interrupt is pending and the callee's frame
/// goes.
pub(crate) unsafe extern "C" fn rt_direct_after(vm: VmPtr, status: i32) -> i32 {
    let vm = vm!(vm);
    match status {
        D_BOXED => CONTINUE,
        D_FAILURE => FAILURE,
        D_DEOPT => {
            let entry = vm.frames.len();
            let result = vm.execute(entry);
            self::status(vm, result)
        }
        _ => {
            let pending = vm.take_pending();
            let entry = vm.frames.len();
            let abandoned = vm.abandon(entry, pending);
            interrupt(vm, abandoned)
        }
    }
}

pub(crate) unsafe extern "C" fn rt_result_type(vm: VmPtr, index: u32) {
    let vm = vm!(vm);
    vm.expected = vm.program.result_types.get(index as usize).cloned();
}

// ------------------------------------------------------------ leaving

/// Return the value on top of the stack from the frame.
pub(crate) unsafe extern "C" fn rt_return(vm: VmPtr) {
    let vm = vm!(vm);
    let value = vm.pop();
    vm.leave_frame(value);
}

pub(crate) unsafe extern "C" fn rt_return_int(vm: VmPtr, value: i64) {
    vm!(vm).leave_frame(Value::integer(value));
}

pub(crate) unsafe extern "C" fn rt_return_bool(vm: VmPtr, value: i8) {
    vm!(vm).leave_frame(Value::Boolean(value != 0));
}

pub(crate) unsafe extern "C" fn rt_return_float(vm: VmPtr, value: f64) {
    vm!(vm).leave_frame(Value::Float(value));
}

pub(crate) unsafe extern "C" fn rt_return_nothing(vm: VmPtr) {
    vm!(vm).leave_frame(Value::Nothing);
}

/// A typed result leaves the frame without a push: the value goes back
/// in a register (decision AR3).
pub(crate) unsafe extern "C" fn rt_leave_typed(vm: VmPtr) {
    let vm = vm!(vm);
    let left = vm.frames.pop().expect("a frame");
    vm.stack.truncate(left.base);
    vm.handlers.truncate(left.handler_base);
}

/// The value on top returned from a frame whose declared result is typed:
/// its bits go to `out` and the frame leaves without a push
/// (`D_RETURNED`); a value that does not fit the kind (a big Integer, a
/// guarded value) or a failure leaves with the frame on the caller's
/// stack instead (`D_BOXED`, `D_FAILURE`).
pub(crate) unsafe extern "C" fn rt_leave_unbox(vm: VmPtr, kind: u8, out: *mut i64) -> i32 {
    let vm = vm!(vm);
    let value = vm.pop();
    let bits = match (kind, &value) {
        (0, Value::Integer(Int::Small(small))) => Some(*small),
        (1, Value::Boolean(flag)) => Some(*flag as i64),
        (2, Value::Float(float)) => Some(float.to_bits() as i64),
        _ => None,
    };
    match bits {
        Some(bits) => {
            unsafe { *out = bits };
            let left = vm.frames.pop().expect("a frame");
            vm.stack.truncate(left.base);
            vm.handlers.truncate(left.handler_base);
            D_RETURNED
        }
        None => {
            let failed = value.is_failure();
            vm.leave_frame(value);
            if failed {
                D_FAILURE
            } else {
                D_BOXED
            }
        }
    }
}

/// The value on top returned from a frame whose result is boxed: the
/// frame leaves with it on the caller's stack.
pub(crate) unsafe extern "C" fn rt_leave_boxed(vm: VmPtr) -> i32 {
    let vm = vm!(vm);
    let value = vm.pop();
    let failed = value.is_failure();
    vm.leave_frame(value);
    if failed {
        D_FAILURE
    } else {
        D_BOXED
    }
}

/// After a helper left the frame (`LEFT`): whether what it left on the
/// caller's stack is a failure.
pub(crate) unsafe extern "C" fn rt_left_status(vm: VmPtr) -> i32 {
    if matches!(vm!(vm).stack.last(), Some(Value::Failure(_))) {
        D_FAILURE
    } else {
        D_BOXED
    }
}

/// `fail` with the error on top: the frame leaves with the failure.
pub(crate) unsafe extern "C" fn rt_fail(vm: VmPtr, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let error = vm.pop();
    let failure = match error {
        Value::Failure(_) => error,
        Value::Nothing => {
            return interrupt(
                vm,
                Interrupt::crash("`otherwise fail` after a value that was nothing, not a failure"),
            )
        }
        other => Value::failure(other),
    };
    vm.leave_frame(failure);
    LEFT
}

pub(crate) unsafe extern "C" fn rt_crash(vm: VmPtr, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let message = vm.pop();
    let error = match vm.text_for_console(&message, "a crash message") {
        Ok(text) => Interrupt::crash(text),
        Err(error) => error,
    };
    interrupt(vm, error)
}

pub(crate) unsafe extern "C" fn rt_crash_text(vm: VmPtr, which: u32, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    interrupt(vm, Interrupt::crash(CRASH_TEXTS[which as usize]))
}

/// `check` in a test found its condition false: the test fails.
pub(crate) unsafe extern "C" fn rt_check_failed(vm: VmPtr, code: usize, message: u32) -> i32 {
    let vm = vm!(vm);
    let text = vm.program.codes[code].constants[message as usize]
        .as_text()
        .unwrap_or("")
        .to_string();
    vm.leave_frame(Value::failure(Value::text(format!("check failed: {text}"))));
    LEFT
}

/// A failure on top of the stack that no handled region of the frame
/// takes: the frame's own failure when it may fail, else a crash (the
/// interpreter's `settle`).
pub(crate) unsafe extern "C" fn rt_unhandled(vm: VmPtr, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let value = vm.pop();
    let frame = vm.frames.last().expect("a frame");
    let code = &vm.program.codes[frame.code];
    let leaves = match code.kind {
        CodeKind::Example | CodeKind::Test | CodeKind::Refinement | CodeKind::Constant => true,
        CodeKind::Function => code
            .function
            .is_some_and(|id| !vm.program.function_metas[id].fails.is_empty()),
    };
    if leaves {
        vm.leave_frame(value);
        return LEFT;
    }
    let error = match &value {
        Value::Failure(error) => (**error).clone(),
        _ => Value::Nothing,
    };
    let crash = match vm.text_for_console(&error, "an unhandled failure") {
        Ok(shown) => Interrupt::crash(format!("unhandled failure: {shown}")),
        Err(error) => error,
    };
    interrupt(vm, crash)
}

/// The failure on top of the stack lands on the floor of a handled region:
/// the operands above the frame's `boxed` boxed operands go.
pub(crate) unsafe extern "C" fn rt_settle_handler(vm: VmPtr, base: usize, locals: u32, boxed: u32) {
    let vm = vm!(vm);
    let failure = vm.pop();
    vm.stack.truncate(base + locals as usize + boxed as usize);
    vm.stack.push(failure);
}

// ------------------------------------------------------------ loops

pub(crate) unsafe extern "C" fn rt_iter_init(vm: VmPtr, base: usize, slot: u32, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let source = vm.pop();
    match vm.iterator_of(source) {
        Ok(iterator) => {
            vm.stack[base + slot as usize] = iterator;
            CONTINUE
        }
        Err(error) => interrupt(vm, error),
    }
}

/// Push the iterator's next item and answer 1, or answer 0 when it is
/// exhausted.
pub(crate) unsafe extern "C" fn rt_iter_next(vm: VmPtr, base: usize, slot: u32) -> i8 {
    let vm = vm!(vm);
    match vm.iterator_next(base + slot as usize) {
        Some(item) => {
            vm.stack.push(item);
            1
        }
        None => 0,
    }
}

pub(crate) unsafe extern "C" fn rt_list_push(vm: VmPtr, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let result = vm.op_list_push();
    status(vm, result)
}

pub(crate) unsafe extern "C" fn rt_group_insert(vm: VmPtr, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let result = vm.op_group_insert();
    status(vm, result)
}

pub(crate) unsafe extern "C" fn rt_group_fold(vm: VmPtr, fold: u8, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let result = vm.op_group_fold(FOLDS[fold as usize]);
    status(vm, result)
}

pub(crate) unsafe extern "C" fn rt_sort_by_key(vm: VmPtr, descending: i8, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let result = vm.op_sort_by_key(descending != 0);
    status(vm, result)
}

pub(crate) unsafe extern "C" fn rt_deadline(vm: VmPtr, base: usize, slot: u32, pc: u32) -> i32 {
    let vm = vm!(vm);
    vm.sync_pc(pc);
    let limit = match vm.pop().into_plain() {
        Value::Duration(ms) => ms,
        other => {
            return interrupt(
                vm,
                Interrupt::crash(format!(
                    "`within` needs a Duration, found {}",
                    other.kind_name()
                )),
            )
        }
    };
    vm.stack[base + slot as usize] = Value::Native(Rc::new(Native::Deadline(
        natives::now_millis() + limit,
        limit,
    )));
    CONTINUE
}

/// `FAILURE` with `TimedOut` on top when the deadline in the slot has
/// passed.
pub(crate) unsafe extern "C" fn rt_check_deadline(vm: VmPtr, base: usize, slot: u32) -> i32 {
    let vm = vm!(vm);
    match vm.expired_deadline(base + slot as usize) {
        Some(limit) => {
            let error = Value::record(vm.program.builtins.timed_out, vec![Value::Duration(limit)]);
            vm.stack.push(Value::failure(error));
            FAILURE
        }
        None => CONTINUE,
    }
}

// ------------------------------------------------------------ deopt

/// Hand the frame back to the interpreter at a point of the code: the
/// values the generated code kept in registers (in `values`, in the order
/// the point lists them) are boxed into their slots and onto the operand
/// stack, among the boxed operands already there, and the frame's pc is
/// set to the point.
pub(crate) unsafe extern "C" fn rt_deopt(
    vm: VmPtr,
    base: usize,
    code: usize,
    point: u32,
    values: *const u64,
) {
    let vm = vm!(vm);
    let jit = vm
        .native
        .as_mut()
        .expect("the generated code runs under a JIT");
    jit.deopts_taken += 1;
    let point: &DeoptPoint = &jit.deopts[code][point as usize];
    let locals = point.locals as usize;
    let mut next = 0usize;
    let mut word = || {
        let value = unsafe { *values.add(next) };
        next += 1;
        value
    };
    // the typed locals, in slot order
    for (slot, kind) in point.slots.iter().enumerate() {
        match kind {
            SlotKind::Int => vm.stack[base + slot] = Value::integer(word() as i64),
            SlotKind::Bool => vm.stack[base + slot] = Value::Boolean(word() != 0),
            SlotKind::Float => vm.stack[base + slot] = Value::Float(f64::from_bits(word())),
            SlotKind::RangeIter => {
                let current = word() as i64;
                let to = word() as i64;
                let by = word() as i64;
                vm.stack[base + slot] = Value::Native(Rc::new(Native::RangeIterator {
                    current: std::cell::Cell::new(current),
                    to,
                    by,
                    done: std::cell::Cell::new(false),
                }));
            }
            SlotKind::Mark => {
                let depth = point.marks[slot].unwrap_or(0);
                vm.stack[base + slot] = Value::integer((base + locals + depth) as i64);
            }
            _ => {}
        }
    }
    // the operand stack: the boxed operands in order, the typed ones from
    // the registers
    let boxed: Vec<Value> = vm.stack.drain(base + locals..).collect();
    let mut boxed = boxed.into_iter();
    for abs in &point.stack {
        let value = match abs {
            Abs::Boxed | Abs::Unset => boxed.next().unwrap_or(Value::Nothing),
            Abs::Int => Value::integer(word() as i64),
            Abs::Bool => Value::Boolean(word() != 0),
            Abs::Float => Value::Float(f64::from_bits(word())),
            Abs::Range => {
                let from = word() as i64;
                let to = word() as i64;
                let by = word() as i64;
                range_value(from, to, by)
            }
        };
        vm.stack.push(value);
    }
    if let Some(frame) = vm.frames.last_mut() {
        frame.pc = point.pc as usize;
    }
}

impl Vm<'_> {
    /// The frame's pc as the interpreter would have it while the op at
    /// `pc` runs: one past it, which is what locates a crash.
    #[inline]
    pub(crate) fn sync_pc(&mut self, pc: u32) {
        if let Some(frame) = self.frames.last_mut() {
            frame.pc = pc as usize + 1;
        }
    }

    /// Pop the running frame with its result onto the caller's stack (the
    /// interpreter's `leave!` without the settling, which the caller does).
    pub(crate) fn leave_frame(&mut self, value: Value) {
        let left = self.frames.pop().expect("a frame");
        self.stack.truncate(left.base);
        self.handlers.truncate(left.handler_base);
        self.stack.push(value);
    }

    /// A call with the arguments on top of the stack, as the interpreter's
    /// `call!` makes it: a declared function runs in a new frame (native
    /// or interpreted), a primitive runs in Rust.
    pub(crate) fn call_from_stack(
        &mut self,
        function: usize,
        count: usize,
    ) -> Result<Value, Interrupt> {
        let program = self.program;
        if let Some(callee) = program.function_codes.get(function).copied().flatten() {
            self.push_frame_in_place(callee, count);
            let entry = self.frames.len();
            if let Some(jit) = self.native.as_mut() {
                jit.calls += 1;
            }
            return self.run_top_frame(entry);
        }
        let mut args = std::mem::take(&mut self.scratch);
        let at = self.stack.len().saturating_sub(count);
        args.extend(self.stack.drain(at..));
        let result = self.call_native(function, &mut args);
        args.clear();
        self.scratch = args;
        result
    }

    pub(crate) fn op_make_map(&mut self, count: usize) {
        let (origins, items) = plain_all(self.pop_n(2 * count));
        let mut items = items.into_iter();
        let mut map = indexmap::IndexMap::with_capacity(count);
        while let (Some(key), Some(value)) = (items.next(), items.next()) {
            map.insert(key, value);
        }
        self.stack.push(Value::Map(Rc::new(map)).guarded(origins));
    }

    pub(crate) fn op_make_range(&mut self, stepped: bool) -> Result<Value, Interrupt> {
        let by = if stepped {
            self.pop()
        } else {
            Value::integer(1)
        };
        let to = self.pop();
        let from = self.pop();
        let origins = from.origins() | to.origins() | by.origins();
        match (from.into_plain(), to.into_plain(), by.into_plain()) {
            (Value::Integer(from), Value::Integer(to), Value::Integer(by)) => {
                Ok(Value::Range(Rc::new(RangeValue { from, to, by })).guarded(origins))
            }
            _ => Err(Interrupt::crash("a range needs Integer bounds")),
        }
    }

    pub(crate) fn op_field(
        &mut self,
        code: usize,
        name: usize,
        site: usize,
        holder: &Value,
    ) -> Result<Value, Interrupt> {
        let name = self.program.codes[code].constants[name]
            .as_text()
            .unwrap_or("");
        Ok(self
            .field_at(site, holder.plain(), name)?
            .guarded(holder.origins()))
    }

    pub(crate) fn op_with(&mut self, count: usize) -> Result<Value, Interrupt> {
        let mut updates = Vec::with_capacity(count);
        let mut origins = 0;
        for _ in 0..count {
            let value = self.pop();
            let name = self.pop();
            origins |= value.origins();
            updates.push((name, value.into_plain()));
        }
        let holder = self.pop();
        origins |= holder.origins();
        Ok(self.with(holder.into_plain(), updates)?.guarded(origins))
    }

    /// The pieces are joined where they lie on the stack, an Integer
    /// written straight into the text, and the stack cut below them after.
    pub(crate) fn op_concat(&mut self, count: usize) -> Result<Value, Interrupt> {
        let at = self.stack.len().saturating_sub(count);
        let mut origins = 0;
        let mut length = 0;
        for piece in &self.stack[at..] {
            origins |= piece.origins();
            length += match piece.plain() {
                Value::Text(part) => part.len(),
                _ => 20,
            };
        }
        let mut text = String::with_capacity(length);
        for index in at..self.stack.len() {
            match self.stack[index].plain() {
                Value::Text(part) => text.push_str(part),
                Value::Integer(value) => {
                    let _ = write!(text, "{value}");
                }
                _ => {
                    let piece = self.stack[index].clone();
                    text.push_str(&self.render(piece.plain(), false)?);
                }
            }
        }
        self.stack.truncate(at);
        Ok(Value::text(text).guarded(origins))
    }

    /// The loop iterator over a collection value (the interpreter's
    /// `IterInit`).
    pub(crate) fn iterator_of(&mut self, source: Value) -> Result<Value, Interrupt> {
        let program = self.program;
        let origins = source.origins();
        let plain = source.into_plain();
        let plain = match plain.type_id().and_then(|ty| program.specials[ty].to_list) {
            Some(function) => self.call_function(function, vec![plain])?.into_plain(),
            None => plain,
        };
        // a range of small Integers iterates without being listed, in the
        // shape the generated code keeps (decision AG3)
        if origins == 0 {
            if let Value::Range(range) = &plain {
                if let (Int::Small(from), Int::Small(to), Int::Small(by)) =
                    (&range.from, &range.to, &range.by)
                {
                    if *by != 0 {
                        return Ok(Value::Native(Rc::new(Native::RangeIterator {
                            current: std::cell::Cell::new(*from),
                            to: *to,
                            by: *by,
                            done: std::cell::Cell::new(false),
                        })));
                    }
                }
            }
        }
        let mut items = self.iterate(plain)?;
        if origins != 0 {
            items = Rc::new(
                items
                    .iter()
                    .map(|item| item.clone().guarded(origins))
                    .collect(),
            );
        }
        Ok(Value::Native(Rc::new(Native::Iterator(
            std::cell::RefCell::new((items, 0)),
        ))))
    }

    /// The next item of the iterator in the stack slot.
    pub(crate) fn iterator_next(&mut self, index: usize) -> Option<Value> {
        match &self.stack[index] {
            Value::Native(native) => match &**native {
                Native::Iterator(state) => {
                    let mut state = state.borrow_mut();
                    let (items, position) = &mut *state;
                    let item = items.get(*position).cloned();
                    *position += 1;
                    item
                }
                Native::RangeIterator {
                    current,
                    to,
                    by,
                    done,
                } => {
                    if done.get() {
                        return None;
                    }
                    let value = current.get();
                    let past = if *by > 0 { value > *to } else { value < *to };
                    if past {
                        done.set(true);
                        return None;
                    }
                    match value.checked_add(*by) {
                        Some(next) => current.set(next),
                        None => done.set(true),
                    }
                    Some(Value::integer(value))
                }
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn op_list_push(&mut self) -> Result<Value, Interrupt> {
        let item = self.pop();
        let list = self.pop();
        let origins = item.origins() | list.origins();
        match list.into_plain() {
            Value::List(mut list) => {
                Rc::make_mut(&mut list).push(item.into_plain());
                Ok(Value::List(list).guarded(origins))
            }
            other => Err(Interrupt::crash(format!(
                "cannot collect into {}",
                other.kind_name()
            ))),
        }
    }

    pub(crate) fn op_group_insert(&mut self) -> Result<Value, Interrupt> {
        let item = self.pop();
        let key = self.pop();
        let map = self.pop();
        let origins = item.origins() | key.origins() | map.origins();
        match map.into_plain() {
            Value::Map(map) => {
                let mut map = crate::value::take_map(map);
                let group = map
                    .entry(key.into_plain())
                    .or_insert_with(|| Value::list(Vec::new()));
                if let Value::List(list) = group {
                    Rc::make_mut(list).push(item.into_plain());
                }
                Ok(Value::Map(Rc::new(map)).guarded(origins))
            }
            other => Err(Interrupt::crash(format!(
                "cannot group into {}",
                other.kind_name()
            ))),
        }
    }

    pub(crate) fn op_group_fold(&mut self, fold: GroupFold) -> Result<Value, Interrupt> {
        let value = self.pop();
        let key = self.pop();
        let map = self.pop();
        let origins = value.origins() | key.origins() | map.origins();
        let (value, key) = (value.into_plain(), key.into_plain());
        match map.into_plain() {
            Value::Map(map) => {
                let mut map = crate::value::take_map(map);
                let folded = match (fold, map.get(&key)) {
                    (_, None) => value,
                    (GroupFold::Sum, Some(current)) => {
                        self.binary_values(BinaryOp::Add, current.clone(), value)?
                    }
                    (GroupFold::First, Some(current)) => current.clone(),
                    (GroupFold::Any, Some(current)) => Value::Boolean(
                        matches!(current, Value::Boolean(true))
                            || matches!(value, Value::Boolean(true)),
                    ),
                    (GroupFold::All, Some(current)) => Value::Boolean(
                        matches!(current, Value::Boolean(true))
                            && matches!(value, Value::Boolean(true)),
                    ),
                };
                map.insert(key, folded);
                Ok(Value::Map(Rc::new(map)).guarded(origins))
            }
            other => Err(Interrupt::crash(format!(
                "cannot group into {}",
                other.kind_name()
            ))),
        }
    }

    pub(crate) fn op_sort_by_key(&mut self, descending: bool) -> Result<Value, Interrupt> {
        let list = self.pop();
        let origins = list.origins();
        let pairs = match list.into_plain() {
            Value::List(list) => crate::value::take_list(list),
            other => {
                return Err(Interrupt::crash(format!(
                    "cannot sort {}",
                    other.kind_name()
                )))
            }
        };
        let mut keyed: Vec<(Value, Value)> = pairs
            .into_iter()
            .map(|pair| match pair {
                Value::Pair(pair) => (pair.0.clone(), pair.1.clone()),
                other => (Value::Nothing, other),
            })
            .collect();
        self.sort_by_key(&mut keyed, descending)?;
        let items: Vec<Value> = keyed.into_iter().map(|(_, item)| item).collect();
        Ok(Value::list(items).guarded(origins))
    }

    /// The limit of an expired `within` deadline in the slot, when it has
    /// passed.
    pub(crate) fn expired_deadline(&self, index: usize) -> Option<i64> {
        match &self.stack[index] {
            Value::Native(native) => match &**native {
                Native::Deadline(at, limit) if natives::now_millis() > *at => Some(*limit),
                _ => None,
            },
            _ => None,
        }
    }

    /// The `Binary` arm of the interpreter on two values: the fast paths
    /// of decision X3, the guard wrappers, then the general operator.
    pub(crate) fn binary_values(
        &mut self,
        op: BinaryOp,
        left: Value,
        right: Value,
    ) -> Result<Value, Interrupt> {
        let fast = match (&left, &right) {
            (Value::Integer(Int::Small(a)), Value::Integer(Int::Small(b))) => {
                crate::vm::small_binary(op, *a, *b)
            }
            (Value::Text(a), Value::Text(b)) => crate::vm::text_binary(op, a, b),
            (Value::Boolean(a), Value::Boolean(b)) => crate::vm::boolean_binary(op, *a, *b),
            _ => None,
        };
        match fast {
            Some(value) => Ok(value),
            None if left.is_guarded() || right.is_guarded() => {
                let origins = left.origins() | right.origins();
                Ok(self
                    .binary(op, left.into_plain(), right.into_plain())?
                    .guarded(origins))
            }
            None => self.binary(op, left, right),
        }
    }
}

/// Whether the top of the stack is `Nothing` or a `Failure` (an
/// `otherwise` on a `maybe` value); the value stays.
pub(crate) unsafe extern "C" fn rt_top_is_absent(vm: VmPtr) -> i8 {
    matches!(
        vm!(vm).stack.last(),
        Some(Value::Nothing) | Some(Value::Failure(_))
    ) as i8
}

/// Whether the top of the stack is a `Failure`; the value stays.
pub(crate) unsafe extern "C" fn rt_top_is_failure(vm: VmPtr) -> i8 {
    matches!(vm!(vm).stack.last(), Some(Value::Failure(_))) as i8
}

/// Every helper by name with its address, for the JIT's symbol table;
/// `codegen::SIGNATURES` gives each its signature.
pub const HELPERS: &[(&str, *const u8)] = &[
    ("rt_top_is_absent", rt_top_is_absent as *const u8),
    ("rt_top_is_failure", rt_top_is_failure as *const u8),
    ("rt_push_const", rt_push_const as *const u8),
    ("rt_push_nothing", rt_push_nothing as *const u8),
    ("rt_insert_int", rt_insert_int as *const u8),
    ("rt_insert_bool", rt_insert_bool as *const u8),
    ("rt_insert_float", rt_insert_float as *const u8),
    ("rt_insert_range", rt_insert_range as *const u8),
    ("rt_global", rt_global as *const u8),
    ("rt_load", rt_load as *const u8),
    ("rt_load_move", rt_load_move as *const u8),
    ("rt_store", rt_store as *const u8),
    ("rt_store_int", rt_store_int as *const u8),
    ("rt_store_bool", rt_store_bool as *const u8),
    ("rt_store_float", rt_store_float as *const u8),
    ("rt_pop", rt_pop as *const u8),
    ("rt_dup", rt_dup as *const u8),
    ("rt_truncate", rt_truncate as *const u8),
    ("rt_take_int", rt_take_int as *const u8),
    ("rt_take_float", rt_take_float as *const u8),
    ("rt_take_bool", rt_take_bool as *const u8),
    ("rt_param_int", rt_param_int as *const u8),
    ("rt_param_bool", rt_param_bool as *const u8),
    ("rt_param_float", rt_param_float as *const u8),
    ("rt_resume_int", rt_resume_int as *const u8),
    ("rt_resume_bool", rt_resume_bool as *const u8),
    ("rt_resume_float", rt_resume_float as *const u8),
    ("rt_resume_range", rt_resume_range as *const u8),
    ("rt_make_list", rt_make_list as *const u8),
    ("rt_make_map", rt_make_map as *const u8),
    ("rt_make_pair", rt_make_pair as *const u8),
    ("rt_make_range", rt_make_range as *const u8),
    ("rt_construct", rt_construct as *const u8),
    ("rt_construct_variant", rt_construct_variant as *const u8),
    ("rt_field", rt_field as *const u8),
    ("rt_load_field", rt_load_field as *const u8),
    ("rt_with", rt_with as *const u8),
    ("rt_not", rt_not as *const u8),
    ("rt_binary", rt_binary as *const u8),
    ("rt_float_binary", rt_float_binary as *const u8),
    ("rt_int_power", rt_int_power as *const u8),
    ("rt_to_text", rt_to_text as *const u8),
    ("rt_concat", rt_concat as *const u8),
    ("rt_is_variant", rt_is_variant as *const u8),
    ("rt_is_nothing", rt_is_nothing as *const u8),
    ("rt_is_failure", rt_is_failure as *const u8),
    ("rt_is_type", rt_is_type as *const u8),
    ("rt_unpack", rt_unpack as *const u8),
    ("rt_unwrap_failure", rt_unwrap_failure as *const u8),
    ("rt_call", rt_call as *const u8),
    ("rt_call_ability", rt_call_ability as *const u8),
    ("rt_call_value", rt_call_value as *const u8),
    ("rt_result_type", rt_result_type as *const u8),
    ("rt_return", rt_return as *const u8),
    ("rt_return_int", rt_return_int as *const u8),
    ("rt_return_bool", rt_return_bool as *const u8),
    ("rt_return_float", rt_return_float as *const u8),
    ("rt_return_nothing", rt_return_nothing as *const u8),
    ("rt_direct_entry", rt_direct_entry as *const u8),
    ("rt_direct_frame", rt_direct_frame as *const u8),
    ("rt_direct_after", rt_direct_after as *const u8),
    ("rt_leave_typed", rt_leave_typed as *const u8),
    ("rt_leave_unbox", rt_leave_unbox as *const u8),
    ("rt_leave_boxed", rt_leave_boxed as *const u8),
    ("rt_left_status", rt_left_status as *const u8),
    ("rt_fail", rt_fail as *const u8),
    ("rt_crash", rt_crash as *const u8),
    ("rt_crash_text", rt_crash_text as *const u8),
    ("rt_check_failed", rt_check_failed as *const u8),
    ("rt_unhandled", rt_unhandled as *const u8),
    ("rt_settle_handler", rt_settle_handler as *const u8),
    ("rt_iter_init", rt_iter_init as *const u8),
    ("rt_iter_next", rt_iter_next as *const u8),
    ("rt_list_push", rt_list_push as *const u8),
    ("rt_group_insert", rt_group_insert as *const u8),
    ("rt_group_fold", rt_group_fold as *const u8),
    ("rt_sort_by_key", rt_sort_by_key as *const u8),
    ("rt_deadline", rt_deadline as *const u8),
    ("rt_check_deadline", rt_check_deadline as *const u8),
    ("rt_deopt", rt_deopt as *const u8),
];
