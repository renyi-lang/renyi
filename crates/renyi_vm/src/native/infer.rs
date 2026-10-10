//! Abstract interpretation of one code object before it is compiled to
//! machine code (decision AG1): the operand stack of the stack machine is
//! static (every op is reached with the same depth, by construction of the
//! compiler), so the depth, the representation of every operand and the
//! kind of every local slot are computed once here. An operand or a slot
//! that is always a small Integer, a Boolean or a Float is kept unboxed in
//! a machine register by the generated code; everything else stays a
//! `Value` on the VM's stack ("boxed"), where the interpreter's own
//! helpers operate on it, and the VM's stack holds exactly the boxed
//! operands, in order. A function the analysis cannot settle (an
//! inconsistent join, a slot used two ways) is left to the interpreter.

use renyi_check::types::Ty;
use renyi_syntax::ast::BinaryOp;

use crate::bytecode::{Code, CodeKind, Op};
use crate::compile::Program;
use crate::integer::Int;
use crate::types::TypeShape;
use crate::value::Value;

/// How an operand is held while the generated code runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Abs {
    /// Not yet known (the bottom of the lattice).
    Unset,
    /// A small Integer in an `i64` register.
    Int,
    /// A Boolean in a register.
    Bool,
    /// A Float in a register.
    Float,
    /// A range written as `from to by` whose three bounds are small
    /// Integers in registers: only `IterInit` consumes it unboxed.
    Range,
    /// Any value, on the VM's stack, with the index in the program's
    /// `result_types` of the type the checker noted for it when the op
    /// that pushed it was emitted under its expression (decision AU1,
    /// stage iv; `Code::types`), else `None`.
    Boxed(Option<u32>),
}

impl Abs {
    /// The least upper bound: equal kinds stay, `Unset` yields, two boxed
    /// values of different types are boxed without one, anything else is
    /// boxed.
    pub fn join(self, other: Abs) -> Abs {
        match (self, other) {
            (Abs::Unset, x) | (x, Abs::Unset) => x,
            (a, b) if a == b => a,
            _ => Abs::Boxed(None),
        }
    }

    pub fn is_boxed(self) -> bool {
        matches!(self, Abs::Boxed(_))
    }

    /// The type noted for a boxed value, when one was.
    pub fn type_index(self) -> Option<u32> {
        match self {
            Abs::Boxed(index) => index,
            _ => None,
        }
    }

    /// The kind without the type: what the generated code's blocks are
    /// keyed by, since the type changes no code at a landing or a hand-back.
    pub fn untyped(self) -> Abs {
        match self {
            Abs::Boxed(_) => Abs::Boxed(None),
            other => other,
        }
    }

    /// The byte an image writes for the kind (decision AS1).
    pub fn code(self) -> u8 {
        match self {
            Abs::Unset => 0,
            Abs::Int => 1,
            Abs::Bool => 2,
            Abs::Float => 3,
            Abs::Range => 4,
            Abs::Boxed(_) => 5,
        }
    }

    pub fn from_code(code: u8) -> Option<Abs> {
        Some(match code {
            0 => Abs::Unset,
            1 => Abs::Int,
            2 => Abs::Bool,
            3 => Abs::Float,
            4 => Abs::Range,
            5 => Abs::Boxed(None),
            _ => return None,
        })
    }
}

/// What a local slot holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotKind {
    Unset,
    Int,
    Bool,
    Float,
    /// A value on the VM's stack, in the slot itself, with the index in
    /// the program's `result_types` of its type when every store into the
    /// slot agreed on one (decision AU8).
    Boxed(Option<u32>),
    /// The operand stack's height at a loop's entry (`MarkStack`): unused
    /// by the generated code, whose stack is static, and reconstructed
    /// when the frame is handed back to the interpreter.
    Mark,
    /// A loop's iterator over a range whose bounds were unboxed: the
    /// position, the end and the step stay in registers.
    RangeIter,
    /// A loop's iterator over anything else, held by the interpreter's
    /// own iterator value in the slot.
    Iter,
    /// A `within` deadline, held in the slot by the interpreter's helper.
    Deadline,
}

impl SlotKind {
    /// The byte an image writes for the kind (decision AS1).
    pub fn code(self) -> u8 {
        match self {
            SlotKind::Unset => 0,
            SlotKind::Int => 1,
            SlotKind::Bool => 2,
            SlotKind::Float => 3,
            SlotKind::Boxed(_) => 4,
            SlotKind::Mark => 5,
            SlotKind::RangeIter => 6,
            SlotKind::Iter => 7,
            SlotKind::Deadline => 8,
        }
    }

    pub fn from_code(code: u8) -> Option<SlotKind> {
        Some(match code {
            0 => SlotKind::Unset,
            1 => SlotKind::Int,
            2 => SlotKind::Bool,
            3 => SlotKind::Float,
            4 => SlotKind::Boxed(None),
            5 => SlotKind::Mark,
            6 => SlotKind::RangeIter,
            7 => SlotKind::Iter,
            8 => SlotKind::Deadline,
            _ => return None,
        })
    }

    /// Whether the slot holds a value the generated code keeps in a
    /// register.
    pub fn is_unboxed_value(self) -> bool {
        matches!(self, SlotKind::Int | SlotKind::Bool | SlotKind::Float)
    }
}

/// The analysis of one code object.
#[derive(Clone, Debug)]
pub struct Analysis {
    /// The abstract stack on entry to every op; `None` for an op no path
    /// reaches.
    pub entry: Vec<Option<Vec<Abs>>>,
    /// The kind of every local slot.
    pub slots: Vec<SlotKind>,
    /// The handled regions open on entry to every op, innermost last:
    /// the handler's target and the abstract depth below the region.
    pub handlers: Vec<Vec<(u32, usize)>>,
    /// Per `MarkStack` slot, the abstract depth it marks.
    pub marks: Vec<Option<usize>>,
    /// Whether the op is the target of a jump or follows a terminator: the
    /// start of a basic block.
    pub block_starts: Vec<bool>,
    /// Per local slot, the index in the program's `result_types` of the
    /// variable's type: from the annotations of the ops that load it
    /// (decision AU1, stage iv), else from what is stored into it, a
    /// parameter's declared type among them (decision AU8); `None` when
    /// nothing says or two sources disagree.
    pub slot_types: Vec<Option<u32>>,
}

/// How many boxed operands lie on the VM's stack in a state.
pub fn boxed_depth(state: &[Abs]) -> usize {
    state.iter().filter(|abs| abs.is_boxed()).count()
}

/// The representation a declared type gives a value that is known to have
/// it: a small Integer, a Boolean or a Float when it may be unboxed, else
/// boxed. The checker's types are exact, but a value with an Integer type
/// may be big or guarded at run time, so the generated code checks before
/// it unboxes and hands the frame to the interpreter when the check fails.
pub fn abs_of_type(program: &Program, ty: &Ty) -> Abs {
    let b = &program.builtins;
    match ty {
        Ty::App(id, args) if args.is_empty() => {
            if *id == b.integer {
                Abs::Int
            } else if *id == b.boolean {
                Abs::Bool
            } else if *id == b.float {
                Abs::Float
            } else {
                // a refined subtype's values are its base's (`type Part is
                // Integer where ...` holds plain Integers)
                match &program.types.meta(*id).shape {
                    crate::types::TypeShape::Subtype { base } => abs_of_type(program, base),
                    _ => Abs::Boxed(None),
                }
            }
        }
        _ => Abs::Boxed(None),
    }
}

/// Whether the checker noted the boxed operand as a Text: the generated
/// code compares two texts held in the value itself in place (decision
/// AU26).
pub fn is_text(program: &Program, abs: Abs) -> bool {
    let Abs::Boxed(Some(index)) = abs else {
        return false;
    };
    matches!(
        program.result_types.get(index as usize),
        Some(Ty::App(id, args)) if *id == program.builtins.text && args.is_empty()
    )
}

/// The representation of a declared parameter from the type as the
/// checker spells it (`FunctionMeta::param_types`).
pub fn abs_of_spelling(spelling: &str) -> Abs {
    match spelling {
        "Integer" => Abs::Int,
        "Boolean" => Abs::Bool,
        "Float" => Abs::Float,
        _ => Abs::Boxed(None),
    }
}

/// The kind each parameter of a declared function is passed as (decision
/// AR3): what the analysis seeds its slot with, by the declared type, and
/// holds it to. The type's index in the program's table, when the file
/// carries it (decision AU8), decides and types a boxed parameter; the
/// spelling decides otherwise.
pub fn abs_of_params(program: &Program, function: usize, params: usize) -> Vec<Abs> {
    let meta = &program.function_metas[function];
    (0..params)
        .map(
            |index| match meta.param_type_indices.get(index).copied().flatten() {
                Some(ty) => match abs_of_type(program, &program.result_types[ty as usize]) {
                    Abs::Boxed(_) => Abs::Boxed(Some(ty)),
                    kind => kind,
                },
                None => meta
                    .param_types
                    .get(index)
                    .map(|spelling| abs_of_spelling(spelling))
                    .unwrap_or(Abs::Boxed(None)),
            },
        )
        .collect()
}

/// The representation of a constant of the code's table.
pub fn abs_of_constant(value: &Value) -> Abs {
    match value {
        Value::Integer(Int::Small(_)) => Abs::Int,
        Value::Boolean(_) => Abs::Bool,
        Value::Float(_) => Abs::Float,
        _ => Abs::Boxed(None),
    }
}

/// The representation of what a call to the function leaves on the stack.
pub fn abs_of_result(program: &Program, function: usize) -> Abs {
    match &program.function_metas[function].returns {
        Some(ty) => abs_of_type(program, ty),
        None => Abs::Boxed(None),
    }
}

/// The representation of a field read by name, when every record or
/// variant field of that name is a scalar of one kind; the holder's type
/// is not known statically, so the generated code checks the value it
/// finds and hands the frame to the interpreter when it is something else
/// (a pair's `left`, a row's `line`).
pub fn abs_of_field(program: &Program, name: &str) -> Abs {
    let mut found = Abs::Unset;
    for meta in &program.types.metas {
        let fields: Vec<&crate::types::FieldMeta> = match &meta.shape {
            TypeShape::Record(fields) => fields.iter().collect(),
            TypeShape::Sum(variants) => variants.iter().flat_map(|v| v.fields.iter()).collect(),
            _ => Vec::new(),
        };
        for field in fields {
            if field.name == name {
                found = found.join(abs_of_type(program, &field.ty));
                if found.is_boxed() {
                    return Abs::Boxed(None);
                }
            }
        }
    }
    if found == Abs::Unset {
        Abs::Boxed(None)
    } else {
        found
    }
}

/// The representation of what a field read pushes: by the type the checker
/// noted for the expression when the op carries one (decision AU8), else
/// by the field's name across every type.
pub fn abs_of_field_at(program: &Program, code: &Code, pc: usize, name: &str) -> Abs {
    match code.types.get(pc).copied().flatten() {
        Some(index) => abs_of_type(program, &program.result_types[index as usize]),
        None => abs_of_field(program, name),
    }
}

/// The type and the index of a field of a record whose type the checker
/// noted (decision AU8): the generated code checks the holder's type
/// against the first and reads the field at the second in place, with no
/// cache. `None` for anything but a record type with the field.
pub fn static_field(program: &Program, ty: u32, name: &str) -> Option<(usize, usize)> {
    match program.result_types.get(ty as usize)? {
        Ty::App(id, _) => match &program.types.metas.get(*id)?.shape {
            TypeShape::Record(fields) => {
                Some((*id, fields.iter().position(|field| field.name == name)?))
            }
            _ => None,
        },
        _ => None,
    }
}

/// The result of a binary operator on two operands of these representations.
pub fn abs_of_binary(op: BinaryOp, left: Abs, right: Abs) -> Abs {
    use BinaryOp::*;
    match (left, right) {
        (Abs::Int, Abs::Int) => match op {
            Add | Subtract | Multiply | Remainder | Power => Abs::Int,
            Is | IsNot | IsLessThan | IsAtMost | IsGreaterThan | IsAtLeast => Abs::Bool,
            // `/` on Integers is a crash in the interpreter
            Divide | And | Or => Abs::Boxed(None),
        },
        (Abs::Float, Abs::Float) => match op {
            Add | Subtract | Multiply | Divide | Remainder | Power => Abs::Float,
            Is | IsNot | IsLessThan | IsAtMost | IsGreaterThan | IsAtLeast => Abs::Bool,
            And | Or => Abs::Boxed(None),
        },
        (Abs::Bool, Abs::Bool) => match op {
            Is | IsNot | And | Or => Abs::Bool,
            _ => Abs::Boxed(None),
        },
        _ => match op {
            // a comparison always yields a Boolean, whatever the operands
            Is | IsNot | IsLessThan | IsAtMost | IsGreaterThan | IsAtLeast => Abs::Bool,
            _ => Abs::Boxed(None),
        },
    }
}

/// Why a code object stays with the interpreter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    /// Two paths reach an op with different depths, or the analysis does
    /// not converge.
    DepthMismatch(usize),
    /// A slot is used as a value and as a mark, an iterator or a deadline.
    SlotConflict(u16),
    StackUnderflow(usize),
    /// `UnwindStack` of a slot no `MarkStack` set, or set at two depths.
    MissingMark(u16),
}

/// An edge out of an op: the pc it goes to, the operand stack it carries
/// there, and the handlers in force there.
type Edge = (usize, Vec<Abs>, Vec<(u32, usize)>);

/// Analyse a code object: `Err` when its stack cannot be settled.
pub fn analyse(program: &Program, code: &Code) -> Result<Analysis, Rejection> {
    let count = code.ops.len();
    let mut slots = vec![SlotKind::Unset; code.locals as usize];
    // a parameter starts as its declared type says (`abs_of_params`, the
    // kind a direct caller passes it as); the parameters of a test, an
    // example, a constant or a refinement are boxed
    let declared = match (code.function, code.kind) {
        (Some(function), CodeKind::Function) => {
            abs_of_params(program, function, code.params as usize)
        }
        _ => vec![Abs::Boxed(None); code.params as usize],
    };
    for (slot, abs) in slots.iter_mut().zip(declared) {
        *slot = slot_of_abs(abs);
    }
    let mut entry: Vec<Option<Vec<Abs>>> = vec![None; count + 1];
    let mut handlers: Vec<Vec<(u32, usize)>> = vec![Vec::new(); count + 1];
    let mut marks: Vec<Option<usize>> = vec![None; code.locals as usize];
    let mut block_starts = vec![false; count + 1];
    block_starts[0] = true;
    entry[0] = Some(Vec::new());
    // a variable's type, from the ops that load its slot
    let mut slot_types: Vec<Option<Option<u32>>> = vec![None; code.locals as usize];
    for (pc, op) in code.ops.iter().enumerate() {
        if let Op::Load(slot) | Op::LoadMove(slot) = op {
            let noted = code.types.get(pc).copied().flatten();
            let known = &mut slot_types[*slot as usize];
            *known = match (*known, noted) {
                (None, noted) => Some(noted),
                (Some(Some(a)), Some(b)) if a == b => Some(Some(a)),
                _ => Some(None),
            };
        }
    }
    let slot_types: Vec<Option<u32>> = slot_types.into_iter().map(|t| t.flatten()).collect();
    // the fixed point: every op is revisited while its entry state or a
    // slot it reads keeps changing; a slot nothing stores into is boxed
    // (it holds `Nothing`), decided once the rest has settled
    let mut rounds = 0;
    loop {
        let mut changed = false;
        rounds += 1;
        if rounds > 64 {
            return Err(Rejection::DepthMismatch(0));
        }
        for pc in 0..count {
            let Some(state) = entry[pc].clone() else {
                continue;
            };
            let open = handlers[pc].clone();
            let mut stack = state;
            let mut open_after = open.clone();
            let op = &code.ops[pc];
            // the successors of this op with the state they receive
            let mut flows: Vec<Edge> = Vec::new();
            let mut falls_through = true;
            macro_rules! pop {
                () => {
                    match stack.pop() {
                        Some(abs) => abs,
                        None => return Err(Rejection::StackUnderflow(pc)),
                    }
                };
            }
            macro_rules! pop_n {
                ($n:expr) => {
                    for _ in 0..$n {
                        pop!();
                    }
                };
            }
            // a failure inside a handled region flows to its handler with
            // the failure on top of the region's floor
            macro_rules! may_fail {
                () => {
                    if let Some((target, depth)) = open.last() {
                        let mut handler_state: Vec<Abs> = stack[..*depth].to_vec();
                        handler_state.push(Abs::Boxed(None));
                        let mut outer = open.clone();
                        outer.pop();
                        flows.push((*target as usize, handler_state, outer));
                    }
                };
            }
            // a boxed value pushed by an op emitted under its own expression
            // carries the type the checker noted for it
            let note = |abs: Abs| -> Abs {
                match abs {
                    Abs::Boxed(None) if op.pushes_its_expression() => {
                        Abs::Boxed(code.types.get(pc).copied().flatten())
                    }
                    other => other,
                }
            };
            macro_rules! store {
                ($slot:expr, $kind:expr) => {{
                    let slot = $slot as usize;
                    let kind = $kind;
                    let joined =
                        join_slot(slots[slot], kind).ok_or(Rejection::SlotConflict($slot))?;
                    if joined != slots[slot] {
                        slots[slot] = joined;
                        changed = true;
                    }
                }};
            }
            match op {
                Op::Const(index) => {
                    stack.push(note(abs_of_constant(&code.constants[*index as usize])))
                }
                Op::Nothing | Op::Global(_) => stack.push(note(Abs::Boxed(None))),
                Op::Load(slot) | Op::LoadMove(slot) => {
                    let kind = slots[*slot as usize];
                    if !matches!(
                        kind,
                        SlotKind::Unset
                            | SlotKind::Int
                            | SlotKind::Bool
                            | SlotKind::Float
                            | SlotKind::Boxed(_)
                    ) {
                        return Err(Rejection::SlotConflict(*slot));
                    }
                    stack.push(note(abs_of_slot(kind)));
                }
                Op::LoadField { slot, name, .. } => {
                    if !matches!(slots[*slot as usize], SlotKind::Unset | SlotKind::Boxed(_)) {
                        return Err(Rejection::SlotConflict(*slot));
                    }
                    let name = code.constants[*name as usize].as_text().unwrap_or("");
                    stack.push(note(abs_of_field_at(program, code, pc, name)));
                }
                Op::Store(slot) => {
                    let abs = pop!();
                    store!(*slot, slot_of_abs(abs));
                }
                Op::Pop => {
                    pop!();
                }
                Op::Dup => {
                    let top = *stack.last().ok_or(Rejection::StackUnderflow(pc))?;
                    stack.push(top);
                }
                Op::MakeList(n) => {
                    pop_n!(*n);
                    stack.push(note(Abs::Boxed(None)));
                }
                Op::MakeMap(n) => {
                    pop_n!(2 * *n);
                    stack.push(note(Abs::Boxed(None)));
                }
                Op::MakePair => {
                    pop_n!(2);
                    stack.push(note(Abs::Boxed(None)));
                }
                Op::MakeRange { stepped } => {
                    let by = if *stepped { pop!() } else { Abs::Int };
                    let to = pop!();
                    let from = pop!();
                    if from == Abs::Int && to == Abs::Int && by == Abs::Int {
                        stack.push(Abs::Range);
                    } else {
                        stack.push(note(Abs::Boxed(None)));
                    }
                }
                Op::Construct { fields, .. } => {
                    pop_n!(*fields);
                    stack.push(note(Abs::Boxed(None)));
                    may_fail!();
                }
                Op::ConstructVariant { fields, .. } => {
                    pop_n!(*fields);
                    stack.push(note(Abs::Boxed(None)));
                    may_fail!();
                }
                Op::Field { name, .. } => {
                    pop!();
                    let name = code.constants[*name as usize].as_text().unwrap_or("");
                    stack.push(note(abs_of_field_at(program, code, pc, name)));
                }
                Op::With(n) => {
                    pop_n!(2 * *n + 1);
                    stack.push(note(Abs::Boxed(None)));
                    may_fail!();
                }
                Op::WithSlot { slot, fields } => {
                    if !matches!(slots[*slot as usize], SlotKind::Unset | SlotKind::Boxed(_)) {
                        return Err(Rejection::SlotConflict(*slot));
                    }
                    pop_n!(2 * *fields);
                    stack.push(note(Abs::Boxed(None)));
                    may_fail!();
                }
                Op::TakeField { slot, name, .. } => {
                    if !matches!(slots[*slot as usize], SlotKind::Unset | SlotKind::Boxed(_)) {
                        return Err(Rejection::SlotConflict(*slot));
                    }
                    let name = code.constants[*name as usize].as_text().unwrap_or("");
                    stack.push(note(abs_of_field_at(program, code, pc, name)));
                }
                Op::Call { function, args } => {
                    pop_n!(*args);
                    stack.push(note(abs_of_result(program, *function)));
                    may_fail!();
                }
                Op::CallAbility { args, .. } => {
                    pop_n!(*args);
                    stack.push(note(Abs::Boxed(None)));
                    may_fail!();
                }
                Op::CallValue(n) => {
                    pop_n!(*n + 1);
                    stack.push(note(Abs::Boxed(None)));
                    may_fail!();
                }
                Op::ResultType(_) => {}
                Op::Not => {
                    pop!();
                    stack.push(Abs::Bool);
                }
                Op::Binary(op) => {
                    let right = pop!();
                    let left = pop!();
                    stack.push(note(abs_of_binary(*op, left, right)));
                }
                Op::ToText => {
                    pop!();
                    stack.push(Abs::Boxed(None));
                }
                Op::Concat(n) => {
                    pop_n!(*n);
                    stack.push(note(Abs::Boxed(None)));
                }
                Op::Jump(target) => {
                    flows.push((*target as usize, stack.clone(), open_after.clone()));
                    falls_through = false;
                }
                Op::JumpIfFalse(target) | Op::JumpIfTrue(target) => {
                    pop!();
                    flows.push((*target as usize, stack.clone(), open_after.clone()));
                }
                Op::JumpIfAbsent(target) | Op::JumpIfFailure(target) => {
                    // the value stays, boxed, and may be a failure or nothing
                    let top = pop!();
                    stack.push(match top {
                        Abs::Boxed(noted) => Abs::Boxed(noted),
                        _ => Abs::Boxed(None),
                    });
                    flows.push((*target as usize, stack.clone(), open_after.clone()));
                }
                Op::PushHandler(target) => {
                    open_after.push((*target, stack.len()));
                }
                Op::PopHandler => {
                    open_after.pop();
                }
                Op::Return | Op::Fail | Op::Crash => {
                    pop!();
                    falls_through = false;
                }
                Op::ReturnNothing => falls_through = false,
                Op::IsVariant(_) | Op::IsNothing | Op::IsFailure | Op::IsType(_) => {
                    pop!();
                    stack.push(Abs::Bool);
                }
                Op::Unpack(n) => {
                    pop!();
                    for _ in 0..*n {
                        stack.push(Abs::Boxed(None));
                    }
                }
                Op::UnwrapFailure => {
                    pop!();
                    stack.push(Abs::Boxed(None));
                }
                Op::IterInit(slot) => {
                    let source = pop!();
                    let kind = if source == Abs::Range {
                        SlotKind::RangeIter
                    } else {
                        SlotKind::Iter
                    };
                    store!(*slot, kind);
                }
                Op::IterNext { slot, exit } => {
                    flows.push((*exit as usize, stack.clone(), open_after.clone()));
                    let item = match slots[*slot as usize] {
                        SlotKind::RangeIter => Abs::Int,
                        _ => Abs::Boxed(None),
                    };
                    stack.push(item);
                }
                Op::ListPush => {
                    pop_n!(2);
                    stack.push(Abs::Boxed(None));
                }
                Op::GroupInsert | Op::GroupFold(_) => {
                    pop_n!(3);
                    stack.push(Abs::Boxed(None));
                }
                Op::SortByKey { .. } => {
                    pop!();
                    stack.push(Abs::Boxed(None));
                }
                Op::Deadline(slot) => {
                    pop!();
                    store!(*slot, SlotKind::Deadline);
                }
                Op::CheckDeadline(_) => may_fail!(),
                Op::MarkStack(slot) => {
                    store!(*slot, SlotKind::Mark);
                    let depth = stack.len();
                    let index = *slot as usize;
                    match marks[index] {
                        Some(known) if known != depth => return Err(Rejection::MissingMark(*slot)),
                        Some(_) => {}
                        None => {
                            marks[index] = Some(depth);
                            changed = true;
                        }
                    }
                }
                Op::UnwindStack(slot) => {
                    let depth = marks[*slot as usize].ok_or(Rejection::MissingMark(*slot))?;
                    if depth > stack.len() {
                        return Err(Rejection::DepthMismatch(pc));
                    }
                    stack.truncate(depth);
                }
                Op::Check(_) => {
                    pop!();
                }
            }
            if falls_through {
                flows.push((pc + 1, stack, open_after));
            }
            for (target, state, open) in flows {
                if target > count {
                    return Err(Rejection::DepthMismatch(pc));
                }
                if target != pc + 1 {
                    block_starts[target] = true;
                }
                let merged = match &entry[target] {
                    None => Some(state),
                    Some(known) => {
                        if known.len() != state.len() {
                            return Err(Rejection::DepthMismatch(target));
                        }
                        let joined: Vec<Abs> =
                            known.iter().zip(&state).map(|(a, b)| a.join(*b)).collect();
                        if joined != *known {
                            Some(joined)
                        } else {
                            None
                        }
                    }
                };
                if let Some(state) = merged {
                    entry[target] = Some(state);
                    handlers[target] = open;
                    changed = true;
                } else if handlers[target] != open {
                    // the same point reached with different regions open:
                    // the compiler never does this
                    return Err(Rejection::DepthMismatch(target));
                }
            }
            if !falls_through && pc + 1 < count {
                block_starts[pc + 1] = true;
            }
        }
        if changed {
            continue;
        }
        let mut settled = false;
        for slot in &mut slots {
            if *slot == SlotKind::Unset {
                *slot = SlotKind::Boxed(None);
                settled = true;
            }
        }
        if !settled {
            break;
        }
    }
    // a parameter keeps the kind its declared type gives it (decision
    // AR3): a direct call passes it so, and a function that stores
    // another kind into the slot stays with the interpreter
    if let (Some(function), CodeKind::Function) = (code.function, code.kind) {
        let declared = abs_of_params(program, function, code.params as usize);
        for (index, kind) in declared.iter().enumerate() {
            if !kind.is_boxed() && slots[index] != slot_of_abs(*kind) {
                return Err(Rejection::SlotConflict(index as u16));
            }
        }
    }
    entry.truncate(count);
    handlers.truncate(count);
    block_starts.truncate(count);
    // a variable's type: what its loads say, else what was stored into it
    let slot_types: Vec<Option<u32>> = slot_types
        .iter()
        .zip(&slots)
        .map(|(hint, kind)| {
            hint.or(match kind {
                SlotKind::Boxed(ty) => *ty,
                _ => None,
            })
        })
        .collect();
    Ok(Analysis {
        entry,
        slots,
        handlers,
        marks,
        block_starts,
        slot_types,
    })
}

/// The slot kind a stored value gives its slot.
fn slot_of_abs(abs: Abs) -> SlotKind {
    match abs {
        Abs::Unset => SlotKind::Unset,
        Abs::Int => SlotKind::Int,
        Abs::Bool => SlotKind::Bool,
        Abs::Float => SlotKind::Float,
        Abs::Range => SlotKind::Boxed(None),
        Abs::Boxed(ty) => SlotKind::Boxed(ty),
    }
}

/// What a load of the slot pushes.
pub fn abs_of_slot(kind: SlotKind) -> Abs {
    match kind {
        SlotKind::Unset => Abs::Unset,
        SlotKind::Int => Abs::Int,
        SlotKind::Bool => Abs::Bool,
        SlotKind::Float => Abs::Float,
        SlotKind::Boxed(ty) => Abs::Boxed(ty),
        SlotKind::Mark | SlotKind::RangeIter | SlotKind::Iter | SlotKind::Deadline => {
            Abs::Boxed(None)
        }
    }
}

/// The join of a slot's kind with a new use: value kinds join as operands
/// do; a mark, an iterator or a deadline slot is used that one way only.
fn join_slot(known: SlotKind, new: SlotKind) -> Option<SlotKind> {
    use SlotKind::*;
    Some(match (known, new) {
        (Unset, x) | (x, Unset) => x,
        (a, b) if a == b => a,
        (Boxed(_), Boxed(_)) => Boxed(None),
        (Int | Bool | Float | Boxed(_), Int | Bool | Float | Boxed(_)) => Boxed(None),
        _ => return None,
    })
}
