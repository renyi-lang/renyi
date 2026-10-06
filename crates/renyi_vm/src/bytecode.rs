//! The instruction set of the stack machine and the code object of one
//! function, test, constant, example or refinement. Locals live on the value
//! stack below the operands of the frame; jumps are absolute indices into
//! `ops`.

use renyi_check::{AbilityId, FunctionId, ModuleId, TypeId};
use renyi_syntax::ast::BinaryOp;
use renyi_syntax::Span;

use crate::value::Value;

#[derive(Clone, Debug)]
pub enum Op {
    /// Push `constants[index]`.
    Const(u32),
    Nothing,
    /// Push a global constant's value (a top-level `let`).
    Global(u32),
    Load(u16),
    /// Take the local out, leaving `Nothing`: the last use of a binding, so
    /// that a collection is updated in place (decision O1).
    LoadMove(u16),
    Store(u16),
    Pop,
    Dup,
    MakeList(u16),
    /// `2n` operands: key, value, ...
    MakeMap(u16),
    MakePair,
    /// `from`, `to` and, when `stepped`, `by`.
    MakeRange {
        stepped: bool,
    },
    /// A record from its fields in declaration order, or a refined subtype
    /// from one value; a refinement that does not hold leaves a
    /// `Failure(ConstraintViolation)`.
    Construct {
        ty: TypeId,
        fields: u16,
    },
    ConstructVariant {
        ty: TypeId,
        tag: u16,
        fields: u16,
    },
    /// A field by name (`constants[name]`) of a record, a variant, a pair or
    /// a native value.
    Field(u32),
    /// `base`, then `fields` pairs of (name constant, value): the updated copy.
    With(u16),
    Call {
        function: FunctionId,
        args: u16,
    },
    /// An ability method on the receiver, which is the first argument.
    CallAbility {
        ability: AbilityId,
        method: u16,
        args: u16,
    },
    /// A function value, then its arguments.
    CallValue(u16),
    /// The context type of the next library call: `result_types[index]` of
    /// the program (`json.parse` decodes by it).
    ResultType(u32),
    Not,
    Binary(BinaryOp),
    /// Replace the top by its `ToText` rendering.
    ToText,
    /// Join `n` texts.
    Concat(u16),
    Jump(u32),
    /// Pop a Boolean; jump when false.
    JumpIfFalse(u32),
    /// Pop a Boolean; jump when true.
    JumpIfTrue(u32),
    /// Peek; jump when the top is `Nothing` or a `Failure` (an `otherwise`
    /// on a `maybe` value).
    JumpIfAbsent(u32),
    /// Peek; jump when the top is a `Failure` (an `otherwise` on a fallible
    /// call, whose success may be `Nothing` when it returns no value).
    JumpIfFailure(u32),
    /// Open a handled region: a failure inside it unwinds the operand stack
    /// to its height here, pushes the `Failure` and jumps to the target.
    PushHandler(u32),
    PopHandler,
    Return,
    ReturnNothing,
    /// Fail the function with the error on top of the stack (a `Failure` on
    /// top is passed on as it is).
    Fail,
    /// Crash with the text on top of the stack.
    Crash,
    /// Pop a value; push whether it is a variant with the tag.
    IsVariant(u16),
    IsNothing,
    IsFailure,
    /// Pop a value; push whether it has the runtime type.
    IsType(TypeId),
    /// Pop a record, variant, pair or list and push its parts in order.
    Unpack(u16),
    /// Pop a `Failure` and push its error.
    UnwrapFailure,
    /// Pop the collection into an iterator in the slot.
    IterInit(u16),
    /// Push the next item of the iterator in the slot, or jump when it is
    /// exhausted.
    IterNext {
        slot: u16,
        exit: u32,
    },
    /// Pop an item and a list; push the list with the item appended.
    ListPush,
    /// Pop an item, a key and a map; push the map with the item appended to
    /// the key's list.
    GroupInsert,
    /// Pop a list of pairs (key, item); push the items stably sorted by key.
    SortByKey {
        descending: bool,
    },
    /// Pop a duration; store a deadline in the slot.
    Deadline(u16),
    /// Fail with `TimedOut` when the deadline in the slot has passed.
    CheckDeadline(u16),
    /// `check` in a test: pop a Boolean; when false, fail the test with the
    /// text constant as the message.
    Check(u32),
}

/// Why a code object exists: its callers decide what to do with the result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeKind {
    Function,
    Test,
    Constant,
    /// An `example:` line: the call's value or failure is compared outside.
    Example,
    /// A refinement predicate over the fields; returns a Boolean.
    Refinement,
}

#[derive(Clone, Debug)]
pub struct Code {
    pub name: String,
    pub module: ModuleId,
    pub kind: CodeKind,
    /// The declared function this is the body of, for narrated runs.
    pub function: Option<FunctionId>,
    /// Parameters first, then the other locals.
    pub params: u16,
    pub locals: u16,
    pub ops: Vec<Op>,
    /// The source span of each op, for crash messages.
    pub spans: Vec<Span>,
    pub constants: Vec<Value>,
}

impl Code {
    pub fn new(name: impl Into<String>, module: ModuleId, kind: CodeKind) -> Code {
        Code {
            name: name.into(),
            module,
            kind,
            function: None,
            params: 0,
            locals: 0,
            ops: Vec::new(),
            spans: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn emit(&mut self, op: Op, span: Span) -> usize {
        self.ops.push(op);
        self.spans.push(span);
        self.ops.len() - 1
    }

    /// Add a constant, reusing an equal one of the same kind.
    pub fn constant(&mut self, value: Value) -> u32 {
        if let Some(index) = self.constants.iter().position(|c| {
            std::mem::discriminant(c) == std::mem::discriminant(&value) && *c == value
        }) {
            return index as u32;
        }
        self.constants.push(value);
        (self.constants.len() - 1) as u32
    }

    pub fn here(&self) -> u32 {
        self.ops.len() as u32
    }

    /// Point a jump emitted earlier at the current position.
    pub fn patch(&mut self, at: usize) {
        let target = self.here();
        self.patch_to(at, target);
    }

    pub fn patch_to(&mut self, at: usize, target: u32) {
        match &mut self.ops[at] {
            Op::Jump(t)
            | Op::JumpIfFalse(t)
            | Op::JumpIfTrue(t)
            | Op::JumpIfAbsent(t)
            | Op::JumpIfFailure(t)
            | Op::PushHandler(t) => *t = target,
            Op::IterNext { exit, .. } => *exit = target,
            other => panic!("not a jump: {other:?}"),
        }
    }
}
