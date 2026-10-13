//! Abstract interpretation of one code object before it is compiled to
//! machine code (decision AG1): the operand stack of the stack machine is
//! static (every op is reached with the same depth, by construction of the
//! compiler), so the depth, the representation of every operand and the
//! kind of every local slot are computed once here. An operand or a slot
//! that is always a small Integer, a Boolean or a Float is kept unboxed in
//! a machine register by the generated code; any other operand is either
//! held as the three words of its `Value` in registers (decision AW22:
//! what a load, a constant, a field read or a loop's item pushes, as long
//! as nothing above it goes on the stack) or a `Value` on the VM's stack
//! ("boxed"), where the interpreter's own helpers operate on it, and the
//! VM's stack holds exactly the boxed operands, in order. A function the
//! analysis cannot settle (an inconsistent join, a slot used two ways) is
//! left to the interpreter.

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
    /// A record of two fields that lies in two cells on the VM's stack,
    /// the first field below the second, with the record's type (decision
    /// AW9): what a call to a function whose declared result is such a
    /// record leaves, and a load of a slot that holds one, when the op
    /// after takes the pair as it lies (`pair_fused`: a `Store`, a `Field`
    /// or a `Return`, for a call past the test of an `otherwise` that only
    /// a failure reaches); before any other op the pair becomes its record.
    Pair(u32),
    /// Any value, on the VM's stack, with the index in the program's
    /// `result_types` of the type the checker noted for it when the op
    /// that pushed it was emitted under its expression (decision AU1,
    /// stage iv; `Code::types`), else `None`.
    Boxed(Option<u32>),
    /// Any value held as its three words in registers (decision AW22,
    /// stage B2 of road (B)): the tag word and the two payload words of
    /// the 24-byte `Value`, as a `Load` of a boxed slot reads them. It
    /// lies nowhere on the VM's stack: a helper that reads the operand
    /// finds it written there first (`box_position`), a landing and an
    /// exit find every held operand written back, and a hand-back
    /// carries the words in its buffer. With the type the checker noted,
    /// as `Boxed` carries it. A held operand never lies under an operand
    /// on the stack: an op whose result goes on the stack writes every
    /// held operand back first, so that the write is three stores at the
    /// top. What a load of a boxed slot, a boxed constant, `Nothing`, a
    /// module constant, a field read, a loop's item and a `Dup` of one
    /// push, when the tier holds values (`analyse`'s `held`).
    Held(Option<u32>),
}

impl Abs {
    /// The least upper bound: equal kinds stay, `Unset` yields, two held
    /// values of different types are held without one (decision AW22),
    /// two boxed values of different types are boxed without one,
    /// anything else is boxed on the stack.
    pub fn join(self, other: Abs) -> Abs {
        match (self, other) {
            (Abs::Unset, x) | (x, Abs::Unset) => x,
            (a, b) if a == b => a,
            (Abs::Held(_), Abs::Held(_)) => Abs::Held(None),
            _ => Abs::Boxed(None),
        }
    }

    /// Whether the operand lies on the VM's stack as one value, where a
    /// helper reads it.
    pub fn is_boxed(self) -> bool {
        matches!(self, Abs::Boxed(_))
    }

    /// Whether the operand is held as its three words in registers
    /// (decision AW22).
    pub fn is_held(self) -> bool {
        matches!(self, Abs::Held(_))
    }

    /// Whether the operand is a value of any type, on the stack or held,
    /// and not a scalar in a register.
    pub fn is_value(self) -> bool {
        matches!(self, Abs::Boxed(_) | Abs::Held(_))
    }

    /// The kind with a held value written back to the stack (decision
    /// AW22): what a landing and an exit leave, and what an op whose
    /// result goes on the stack leaves below it.
    pub fn spilled(self) -> Abs {
        match self {
            Abs::Held(ty) => Abs::Boxed(ty),
            other => other,
        }
    }

    /// How many cells of the VM's stack the operand takes: one boxed, two
    /// for a pair (decision AW9), none in a register.
    pub fn cells(self) -> usize {
        match self {
            Abs::Boxed(_) => 1,
            Abs::Pair(_) => 2,
            _ => 0,
        }
    }

    /// The type noted for a boxed value, when one was.
    pub fn type_index(self) -> Option<u32> {
        match self {
            Abs::Boxed(index) | Abs::Held(index) => index,
            _ => None,
        }
    }

    /// The kind without the type: what the generated code's blocks are
    /// keyed by, since the type changes no code at a landing or a hand-back.
    pub fn untyped(self) -> Abs {
        match self {
            Abs::Boxed(_) => Abs::Boxed(None),
            Abs::Held(_) => Abs::Held(None),
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
            Abs::Pair(_) => 6,
            Abs::Held(_) => 7,
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
            // the type follows the kind in an image
            6 => Abs::Pair(0),
            7 => Abs::Held(None),
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
    /// A record of two fields held as its fields (decision AW9): the
    /// first in the slot, the second in a cell of the frame past the code
    /// object's locals (the k-th pair slot's in the k-th), both `Nothing`
    /// once the pair is moved out or taken apart; with the record's type.
    Pair(u32),
    /// The operand stack's height at a loop's entry (`MarkStack`): unused
    /// by the generated code, whose stack is static, and reconstructed
    /// when the frame is handed back to the interpreter.
    Mark,
    /// A loop's iterator over a range whose bounds were unboxed: the
    /// position, the end and the step stay in registers.
    RangeIter,
    /// A loop's iterator over anything else: the source as a list in the
    /// slot (or a range iterator value), the position an Integer in the
    /// slot after it (decision AW14).
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
            SlotKind::Pair(_) => 9,
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
            // the type follows the kind in an image
            9 => SlotKind::Pair(0),
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

/// How many cells of the VM's stack the operands of a state take: one
/// per boxed operand, two per pair (decision AW9).
pub fn boxed_depth(state: &[Abs]) -> usize {
    state.iter().map(|abs| abs.cells()).sum()
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
    let Some(index) = abs.type_index() else {
        return false;
    };
    matches!(
        program.result_types.get(index as usize),
        Some(Ty::App(id, args)) if *id == program.builtins.text && args.is_empty()
    )
}

/// Whether the checker noted the boxed operand as an Integer: the
/// generated code compares two small Integers in place (decision AU45).
pub fn is_integer(program: &Program, abs: Abs) -> bool {
    let Some(index) = abs.type_index() else {
        return false;
    };
    matches!(
        program.result_types.get(index as usize),
        Some(Ty::App(id, args)) if *id == program.builtins.integer && args.is_empty()
    )
}

/// Whether the checker noted both boxed operands as one variant type
/// without its own `equals` and with at most 64 variants (decision AW13),
/// and then which of its tags name a variant with fields, one bit per
/// tag: the generated code compares two such values in place by the type
/// and the tag their blocks hold, and settles the answer there unless the
/// tags are the same and name a variant with fields, whose values the
/// helper compares.
pub fn sum_compared(program: &Program, left: Abs, right: Abs) -> Option<u64> {
    let type_of = |abs: Abs| {
        let index = abs.type_index()?;
        match program.result_types.get(index as usize)? {
            Ty::App(id, args) if args.is_empty() => Some(*id),
            _ => None,
        }
    };
    let id = type_of(left)?;
    if type_of(right)? != id
        || program
            .specials
            .get(id)
            .is_some_and(|special| special.equals.is_some())
    {
        return None;
    }
    let TypeShape::Sum(variants) = &program.types.metas.get(id)?.shape else {
        return None;
    };
    if variants.len() > 64 {
        return None;
    }
    Some(
        variants
            .iter()
            .enumerate()
            .filter(|(_, variant)| !variant.fields.is_empty())
            .fold(0, |fielded, (tag, _)| fielded | 1 << tag),
    )
}

/// The representation the checker's note gives a boxed operand: a small
/// Integer, a Boolean or a Float when it noted such a type (a refined
/// subtype's base among them, as `abs_of_type` reads it), else boxed. A
/// direct call takes such an operand of a typed parameter from the stack
/// at the site, after a test of its tags (decision AW3).
pub fn noted_kind(program: &Program, abs: Abs) -> Abs {
    match abs.type_index() {
        Some(index) => match program.result_types.get(index as usize) {
            Some(ty) => abs_of_type(program, ty),
            None => Abs::Boxed(None),
        },
        None => Abs::Boxed(None),
    }
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

/// The type of the record of two fields a call to the function leaves in
/// two cells (decision AW9): the declared result is a record of two
/// fields without refinements and not `Date`, and the function has a code
/// object (its body returns the pair to a direct caller and the record to
/// a trampoline's); `None` for every other function.
pub fn pair_result(program: &Program, function: usize) -> Option<u32> {
    program.function_codes.get(function).copied().flatten()?;
    let Some(Ty::App(id, _)) = &program.function_metas[function].returns else {
        return None;
    };
    let meta = program.types.metas.get(*id)?;
    let TypeShape::Record(fields) = &meta.shape else {
        return None;
    };
    let fits = fields.len() == 2 && meta.refinements.is_empty() && Some(*id) != program.date;
    fits.then_some(*id as u32)
}

/// Whether the op after `pc` takes a pair as it lies (decision AW9): a
/// `Store`, a `Field` or a `Return`, for a call that ends the region of an
/// `otherwise` (`otherwise_end`) the op where the region's success path
/// lands. Before any other op the pair becomes its record at once, so that
/// a pair never lies under another operand.
pub fn pair_fused(code: &Code, pc: usize) -> bool {
    let next = otherwise_end(code, pc).unwrap_or(pc + 1);
    matches!(
        code.ops().get(next),
        Some(Op::Store(_) | Op::Field { .. } | Op::Return)
    )
}

/// Where the success path of an `otherwise` lands when the call at `pc`
/// ends its handled region (decision AW9): the compiler closes the region
/// with `PopHandler`, tests the value with `JumpIfFailure` or
/// `JumpIfAbsent` to the fallback right after the next op, a `Jump` to the
/// end. A pair the call leaves takes that jump at once, past the test,
/// which only a failure then reaches (`handler_only`).
pub fn otherwise_end(code: &Code, pc: usize) -> Option<usize> {
    if !matches!(code.ops().get(pc), Some(Op::Call { .. })) {
        return None;
    }
    match (
        code.ops().get(pc + 1),
        code.ops().get(pc + 2),
        code.ops().get(pc + 3),
    ) {
        (
            Some(Op::PopHandler),
            Some(Op::JumpIfFailure(fallback) | Op::JumpIfAbsent(fallback)),
            Some(Op::Jump(end)),
        ) if *fallback as usize == pc + 4 => Some(*end as usize),
        _ => None,
    }
}

/// Whether the op at `pc`, the test of an `otherwise`, is reached through
/// its handler alone (decision AW9): the op before it is the region's
/// `PopHandler` and no path reaches that, the call before having taken its
/// pair past the test. The value on top is then a failure, and the test's
/// jump is taken.
pub fn handler_only(code: &Code, entry: &[Option<Vec<Abs>>], pc: usize) -> bool {
    pc > 0
        && matches!(code.ops()[pc - 1], Op::PopHandler)
        && matches!(entry.get(pc - 1), Some(None))
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
    match code.types().get(pc).copied().flatten() {
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

/// The type and the packed field indices of a `with` whose base the
/// checker typed (decision AW8): the op's annotation names the record's
/// type, a record without refinements and not `Date`; the name of each
/// update is the text the `Const` at its depth pushed, found through the
/// analysis's states (every op between that `Const` and the `with` lies
/// deeper); the indices eight bits each, the first update lowest, and
/// the count in the top byte, so at most seven updates. `None` when the
/// op carries no such type, a name is not found or names no field, or
/// there are more updates than fit: the general helper then.
pub fn update_by_index(
    program: &Program,
    code: &Code,
    analysis: &Analysis,
    pc: usize,
    count: usize,
) -> Option<(usize, u64)> {
    if count > 7 {
        return None;
    }
    let noted = code.types().get(pc).copied().flatten()?;
    let Ty::App(id, _) = program.result_types.get(noted as usize)? else {
        return None;
    };
    let meta = program.types.metas.get(*id)?;
    let TypeShape::Record(fields) = &meta.shape else {
        return None;
    };
    if !meta.refinements.is_empty() || Some(*id) == program.date {
        return None;
    }
    let first = analysis
        .entry
        .get(pc)?
        .as_ref()?
        .len()
        .checked_sub(2 * count)?;
    let mut packed = (count as u64) << 56;
    for update in 0..count {
        let depth = first + 2 * update;
        let at = (0..pc).rev().find(|&at| {
            analysis.entry[at]
                .as_ref()
                .is_some_and(|state| state.len() == depth)
        })?;
        let Op::Const(constant) = code.ops()[at] else {
            return None;
        };
        let name = code.constants[constant as usize].as_text()?;
        let field = fields.iter().position(|field| field.name == name)?;
        if field > 0xff {
            return None;
        }
        packed |= (field as u64) << (8 * update);
    }
    Some((*id, packed))
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
/// `globals` says which module constants are already evaluated to a
/// small Integer, by index (decision AU53): such a constant is an Int
/// operand, which the generated code pushes as a literal; an empty
/// slice says none is known. `pairs` says whether a record of two fields
/// is held as a pair (decision AW9): the Cranelift tier holds one so in a
/// code object's own ops; the template tier and an expanded body do not.
/// `held` says whether a value a load, a constant, a field read or a
/// loop's item pushes is held as its three words in registers (decision
/// AW22): the Cranelift tier holds them; the template tier does not.
pub fn analyse(
    program: &Program,
    code: &Code,
    globals: &[Option<i64>],
    pairs: bool,
    held: bool,
) -> Result<Analysis, Rejection> {
    let count = code.ops().len();
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
    for (pc, op) in code.ops().iter().enumerate() {
        if let Op::Load(slot) | Op::LoadMove(slot) = op {
            let noted = code.types().get(pc).copied().flatten();
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
    let mut cleaned = false;
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
            let op = &code.ops()[pc];
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
                        // a landing finds every held operand written back
                        // to the stack (decision AW22)
                        let mut handler_state: Vec<Abs> =
                            stack[..*depth].iter().map(|abs| abs.spilled()).collect();
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
                        Abs::Boxed(code.types().get(pc).copied().flatten())
                    }
                    other => other,
                }
            };
            // a value held as its words where the tier holds one (decision
            // AW22): what a load of a boxed slot, a boxed constant,
            // `Nothing`, a module constant, a field read or a loop's item
            // pushes
            let hold = |abs: Abs| -> Abs {
                match abs {
                    Abs::Boxed(ty) if held => Abs::Held(ty),
                    other => other,
                }
            };
            // a value pushed on the VM's stack lies above every held
            // operand, which is written back first (decision AW22)
            macro_rules! push {
                ($abs:expr) => {{
                    let pushed: Abs = $abs;
                    if pushed.is_boxed() || matches!(pushed, Abs::Pair(_)) {
                        for below in stack.iter_mut() {
                            *below = below.spilled();
                        }
                    }
                    stack.push(pushed);
                }};
            }
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
                    push!(hold(note(abs_of_constant(
                        &code.constants[*index as usize]
                    ))))
                }
                Op::Nothing => push!(hold(note(Abs::Boxed(None)))),
                Op::Global(index) => match globals.get(*index as usize) {
                    Some(Some(_)) => push!(Abs::Int),
                    _ => push!(hold(note(Abs::Boxed(None)))),
                },
                Op::Load(slot) | Op::LoadMove(slot) => {
                    let kind = slots[*slot as usize];
                    if !matches!(
                        kind,
                        SlotKind::Unset
                            | SlotKind::Int
                            | SlotKind::Bool
                            | SlotKind::Float
                            | SlotKind::Boxed(_)
                            | SlotKind::Pair(_)
                    ) {
                        return Err(Rejection::SlotConflict(*slot));
                    }
                    // a pair slot's pair as it lies for the op after, else
                    // its record (decision AW9)
                    push!(match kind {
                        SlotKind::Pair(ty) if pair_fused(code, pc) => Abs::Pair(ty),
                        SlotKind::Pair(_) => note(Abs::Boxed(None)),
                        kind => hold(note(abs_of_slot(kind))),
                    });
                }
                Op::LoadField { slot, name, .. } => {
                    if !matches!(
                        slots[*slot as usize],
                        SlotKind::Unset | SlotKind::Boxed(_) | SlotKind::Pair(_)
                    ) {
                        return Err(Rejection::SlotConflict(*slot));
                    }
                    let name = code.constants[*name as usize].as_text().unwrap_or("");
                    push!(hold(note(abs_of_field_at(program, code, pc, name))));
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
                    push!(top);
                }
                Op::MakeList(n) => {
                    pop_n!(*n);
                    push!(note(Abs::Boxed(None)));
                }
                Op::MakeMap(n) => {
                    pop_n!(2 * *n);
                    push!(note(Abs::Boxed(None)));
                }
                Op::MakePair => {
                    pop_n!(2);
                    push!(note(Abs::Boxed(None)));
                }
                Op::MakeRange { stepped } => {
                    let by = if *stepped { pop!() } else { Abs::Int };
                    let to = pop!();
                    let from = pop!();
                    if from == Abs::Int && to == Abs::Int && by == Abs::Int {
                        push!(Abs::Range);
                    } else {
                        push!(note(Abs::Boxed(None)));
                    }
                }
                Op::Construct { fields, .. } => {
                    pop_n!(*fields);
                    push!(note(Abs::Boxed(None)));
                    may_fail!();
                }
                Op::ConstructVariant { fields, .. } => {
                    pop_n!(*fields);
                    push!(note(Abs::Boxed(None)));
                    may_fail!();
                }
                Op::Field { name, .. } => {
                    pop!();
                    let name = code.constants[*name as usize].as_text().unwrap_or("");
                    push!(hold(note(abs_of_field_at(program, code, pc, name))));
                }
                Op::With(n) => {
                    pop_n!(2 * *n + 1);
                    push!(note(Abs::Boxed(None)));
                    may_fail!();
                }
                Op::WithSlot { slot, fields } => {
                    // a pair slot updated in place holds its record instead
                    // (decision AW9): boxed from here on
                    if let SlotKind::Pair(_) = slots[*slot as usize] {
                        store!(*slot, SlotKind::Boxed(None));
                    }
                    if !matches!(slots[*slot as usize], SlotKind::Unset | SlotKind::Boxed(_)) {
                        return Err(Rejection::SlotConflict(*slot));
                    }
                    pop_n!(2 * *fields);
                    push!(note(Abs::Boxed(None)));
                    may_fail!();
                }
                Op::TakeField { slot, name, .. } => {
                    if !matches!(
                        slots[*slot as usize],
                        SlotKind::Unset | SlotKind::Boxed(_) | SlotKind::Pair(_)
                    ) {
                        return Err(Rejection::SlotConflict(*slot));
                    }
                    let name = code.constants[*name as usize].as_text().unwrap_or("");
                    push!(hold(note(abs_of_field_at(program, code, pc, name))));
                }
                Op::Call { function, args } => {
                    pop_n!(*args);
                    // a pair callee's two cells as they lie for the op
                    // after, else its record (decision AW9)
                    let pair =
                        pair_result(program, *function).filter(|_| pairs && pair_fused(code, pc));
                    push!(match pair {
                        Some(ty) => Abs::Pair(ty),
                        None => note(abs_of_result(program, *function)),
                    });
                    may_fail!();
                    // the pair past the test of the `otherwise` whose region
                    // the call ends: the region closed, the success path
                    // straight to the end, the test left to the failures
                    if let (Some(_), Some(end)) = (pair, otherwise_end(code, pc)) {
                        if open.last().map(|(target, _)| *target as usize) == Some(pc + 2) {
                            open_after.pop();
                            flows.push((end, stack.clone(), open_after.clone()));
                            falls_through = false;
                        }
                    }
                }
                Op::CallAbility { args, .. } => {
                    pop_n!(*args);
                    push!(note(Abs::Boxed(None)));
                    may_fail!();
                }
                Op::CallValue(n) => {
                    pop_n!(*n + 1);
                    push!(note(Abs::Boxed(None)));
                    may_fail!();
                }
                Op::ResultType(_) => {}
                Op::Not => {
                    pop!();
                    push!(Abs::Bool);
                }
                Op::Binary(op) => {
                    let right = pop!();
                    let left = pop!();
                    push!(note(abs_of_binary(*op, left, right)));
                }
                Op::ToText => {
                    pop!();
                    push!(Abs::Boxed(None));
                }
                Op::Concat(n) => {
                    pop_n!(*n);
                    push!(note(Abs::Boxed(None)));
                }
                Op::Jump(target) => {
                    flows.push((*target as usize, stack.clone(), open_after.clone()));
                    falls_through = false;
                }
                Op::JumpIfFalse(target) | Op::JumpIfTrue(target) => {
                    pop!();
                    flows.push((*target as usize, stack.clone(), open_after.clone()));
                }
                Op::JumpByTag { table, .. } => {
                    // the arms by the tag (decision AW15), the stack as it is
                    for target in &code.tables()[*table as usize] {
                        flows.push((*target as usize, stack.clone(), open_after.clone()));
                    }
                }
                Op::JumpIfAbsent(target) | Op::JumpIfFailure(target) => {
                    // the value stays, boxed, and may be a failure or nothing
                    let top = pop!();
                    push!(match top {
                        Abs::Boxed(noted) => Abs::Boxed(noted),
                        Abs::Held(noted) => Abs::Held(noted),
                        _ => Abs::Boxed(None),
                    });
                    flows.push((*target as usize, stack.clone(), open_after.clone()));
                    // reached through its handler alone (decision AW9): a
                    // failure on top, the jump taken
                    if handler_only(code, &entry, pc) {
                        falls_through = false;
                    }
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
                    push!(Abs::Bool);
                }
                Op::Unpack(n) => {
                    pop!();
                    for _ in 0..*n {
                        push!(Abs::Boxed(None));
                    }
                }
                Op::UnwrapFailure => {
                    pop!();
                    push!(Abs::Boxed(None));
                }
                Op::IterInit(slot) => {
                    let source = pop!();
                    let kind = if source == Abs::Range {
                        SlotKind::RangeIter
                    } else {
                        SlotKind::Iter
                    };
                    store!(*slot, kind);
                    // the position, an Integer in the slot after (decision
                    // AW14)
                    store!(*slot + 1, SlotKind::Int);
                }
                Op::IterNext { slot, exit } => {
                    flows.push((*exit as usize, stack.clone(), open_after.clone()));
                    let item = match slots[*slot as usize] {
                        SlotKind::RangeIter => Abs::Int,
                        _ => hold(Abs::Boxed(None)),
                    };
                    push!(item);
                }
                Op::ListPush => {
                    pop_n!(2);
                    push!(Abs::Boxed(None));
                }
                Op::GroupInsert | Op::GroupFold(_) => {
                    pop_n!(3);
                    push!(Abs::Boxed(None));
                }
                Op::SortByKey { .. } => {
                    pop!();
                    push!(Abs::Boxed(None));
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
        if cleaned {
            break;
        }
        let mut settled = false;
        for slot in &mut slots {
            if *slot == SlotKind::Unset {
                *slot = SlotKind::Boxed(None);
                settled = true;
            }
        }
        if settled {
            continue;
        }
        // the slot kinds are final: every entry state computed again from
        // them (decision AW22), so that no state carries what an earlier
        // round's slot kinds gave an op: a load of a slot that was an
        // Integer before it was boxed pushed a register then and pushes
        // a held value now, and their join, a value on the stack, is
        // what the generated code, which pushes by the final kinds, would
        // not have; the clean run changes no slot, its states lying under
        // the ones that settled them
        cleaned = true;
        rounds = 0;
        entry = vec![None; count + 1];
        entry[0] = Some(Vec::new());
        handlers = vec![Vec::new(); count + 1];
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

/// Per op, the local slots that may hold a counted block on entry to it
/// (decision AW4), which is what the frame's leave releases: a boxed
/// parameter from the entry; a boxed slot from a store of anything but a
/// plain scalar until the move out of it (a `LoadMove` the code moves,
/// not one a typed call or a comparison borrows, which keeps the slot's
/// value) or the update in place (`WithSlot`, which moves the record out);
/// a slot of another kind (an iterator, a deadline, a mark) always; a
/// typed slot never; a pair slot (decision AW9) from the store of a pair
/// until the move out of it or the take of a field, which empties both
/// cells. A failure flows to its handler with the state before
/// the op, so that a slot the op would have emptied stays counted there.
/// An op the analysis left without a state (the `PopHandler` and the
/// `Jump` of an `otherwise` region whose call takes its pair straight to
/// the region's end, decision AW9) passes the state on as its kind says,
/// so that the ops after the region and a loop's back edge are reached;
/// where no path reaches an op, every slot counts. The interpreter
/// writes the slots as these ops say too, so a frame it hands over at a
/// loop header fits the state there.
pub fn owned_slots(code: &Code, analysis: &Analysis, borrowed: &[bool]) -> Vec<Vec<bool>> {
    let count = code.ops().len();
    let locals = code.locals as usize;
    let boxed =
        |slot: usize| matches!(analysis.slots[slot], SlotKind::Boxed(_) | SlotKind::Pair(_));
    let start: Vec<bool> = (0..locals)
        .map(|slot| match analysis.slots[slot] {
            SlotKind::Int | SlotKind::Bool | SlotKind::Float | SlotKind::Pair(_) => false,
            SlotKind::Boxed(_) => slot < code.params as usize,
            _ => true,
        })
        .collect();
    let mut owned: Vec<Option<Vec<bool>>> = vec![None; count + 1];
    owned[0] = Some(start);
    let mut changed = true;
    while changed {
        changed = false;
        for pc in 0..count {
            let Some(state) = owned[pc].clone() else {
                continue;
            };
            let entry = analysis.entry[pc].as_deref();
            let op = &code.ops()[pc];
            let mut after = state.clone();
            match op {
                Op::Store(slot) => {
                    let slot = *slot as usize;
                    if boxed(slot) {
                        after[slot] = !matches!(
                            entry.and_then(|state| state.last()),
                            Some(Abs::Int | Abs::Bool | Abs::Float)
                        );
                    }
                }
                Op::LoadMove(slot) if !borrowed[pc] => {
                    let slot = *slot as usize;
                    if boxed(slot) {
                        after[slot] = false;
                    }
                }
                Op::WithSlot { slot, .. } => {
                    let slot = *slot as usize;
                    if boxed(slot) {
                        after[slot] = false;
                    }
                }
                Op::TakeField { slot, .. } => {
                    // a pair slot taken apart at its last read holds
                    // `Nothing` in both cells (decision AW9)
                    let slot = *slot as usize;
                    if let SlotKind::Pair(_) = analysis.slots[slot] {
                        after[slot] = false;
                    }
                }
                _ => {}
            }
            let mut flows: Vec<(usize, &Vec<bool>)> = Vec::new();
            let may_fail = matches!(
                op,
                Op::Construct { .. }
                    | Op::ConstructVariant { .. }
                    | Op::With(_)
                    | Op::WithSlot { .. }
                    | Op::Call { .. }
                    | Op::CallAbility { .. }
                    | Op::CallValue(_)
                    | Op::CheckDeadline(_)
            );
            if may_fail {
                if let Some((target, _)) = analysis.handlers[pc].last() {
                    flows.push((*target as usize, &state));
                }
            }
            let mut falls_through = true;
            match op {
                Op::Jump(target) => {
                    flows.push((*target as usize, &after));
                    falls_through = false;
                }
                Op::JumpIfFalse(target)
                | Op::JumpIfTrue(target)
                | Op::JumpIfAbsent(target)
                | Op::JumpIfFailure(target) => flows.push((*target as usize, &after)),
                Op::IterNext { exit, .. } => flows.push((*exit as usize, &after)),
                Op::JumpByTag { table, .. } => {
                    for target in &code.tables()[*table as usize] {
                        flows.push((*target as usize, &after));
                    }
                }
                Op::Return | Op::ReturnNothing | Op::Fail | Op::Crash => falls_through = false,
                _ => {}
            }
            if falls_through {
                flows.push((pc + 1, &after));
            }
            for (target, flow) in flows {
                match &mut owned[target] {
                    None => {
                        owned[target] = Some(flow.clone());
                        changed = true;
                    }
                    Some(known) => {
                        for (bit, more) in known.iter_mut().zip(flow) {
                            if *more && !*bit {
                                *bit = true;
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
    }
    owned.truncate(count);
    owned
        .into_iter()
        .map(|state| state.unwrap_or_else(|| vec![true; locals]))
        .collect()
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
        Abs::Held(ty) => SlotKind::Boxed(ty),
        Abs::Pair(ty) => SlotKind::Pair(ty),
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
        SlotKind::Pair(ty) => Abs::Pair(ty),
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
        // a pair slot stored into with anything else holds records
        // (decision AW9)
        (Int | Bool | Float | Boxed(_) | Pair(_), Int | Bool | Float | Boxed(_) | Pair(_)) => {
            Boxed(None)
        }
        _ => return None,
    })
}
