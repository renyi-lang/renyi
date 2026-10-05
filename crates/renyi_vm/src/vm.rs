//! The interpreter: frames over one value stack, re-entrant for callbacks
//! (ability methods, sorting keys, handlers). A fallible call that fails
//! leaves a `Failure` value; the frame's innermost handled region takes it,
//! or the frame's own result is the failure (examples, tests), or the program
//! crashes, since the checker has made sure a program handles every failure.

use std::cmp::Ordering;
use std::io::{BufRead, Write};
use std::rc::Rc;

use indexmap::IndexMap;
use renyi_check::effects::Capability;
use renyi_check::types::Ty;
use renyi_check::{AbilityId, FunctionId, TypeId};
use renyi_syntax::ast::BinaryOp;

use crate::bytecode::{CodeKind, Op};
use crate::compile::{CodeId, Program};
use crate::decimal::{Decimal, DecimalError};
use crate::integer::Int;
use crate::natives::{self, NativeFn};
use crate::types::TypeShape;
use crate::value::{take_list, take_map, Native, RangeValue, Value};

/// Why execution stopped before the program said so.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Interrupt {
    /// A crash: `crash with`, an unhandled failure, an arithmetic domain
    /// error, a construct the VM cannot run.
    Crash {
        message: String,
        /// `file:line` of the op that crashed, when known.
        location: Option<String>,
    },
    /// `environment.exit(code)`.
    Exit(i32),
}

impl Interrupt {
    pub fn crash(message: impl Into<String>) -> Interrupt {
        Interrupt::Crash {
            message: message.into(),
            location: None,
        }
    }
}

/// How a program is run: its arguments, its streams and its grant.
pub struct Options {
    pub arguments: Vec<String>,
    pub stdout: Box<dyn Write>,
    pub stderr: Box<dyn Write>,
    pub stdin: Box<dyn BufRead>,
    pub grant: Vec<Capability>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            arguments: Vec::new(),
            stdout: Box::new(std::io::stdout()),
            stderr: Box::new(std::io::stderr()),
            stdin: Box::new(std::io::BufReader::new(std::io::stdin())),
            grant: Vec::new(),
        }
    }
}

struct Frame {
    code: CodeId,
    pc: usize,
    /// The index of local 0 on the stack.
    base: usize,
    /// Open handled regions: the target and the stack height to unwind to.
    handlers: Vec<(usize, usize)>,
}

pub struct Vm<'p> {
    pub program: &'p Program,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    globals: Vec<Option<Value>>,
    natives: Vec<Option<NativeFn>>,
    pub stdout: Box<dyn Write>,
    pub stderr: Box<dyn Write>,
    pub stdin: Box<dyn BufRead>,
    pub arguments: Vec<String>,
    pub grant: Vec<Capability>,
    /// The context type of the next library call (`Op::ResultType`).
    expected: Option<Ty>,
    pub random_state: u64,
}

impl<'p> Vm<'p> {
    pub fn new(program: &'p Program, options: Options) -> Vm<'p> {
        let natives = program
            .function_metas
            .iter()
            .map(|meta| {
                if meta.is_library {
                    natives::lookup(&meta.module, &meta.name, meta.receiver.as_deref())
                } else {
                    None
                }
            })
            .collect();
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15)
            | 1;
        Vm {
            program,
            stack: Vec::new(),
            frames: Vec::new(),
            globals: vec![None; program.constants.len()],
            natives,
            stdout: options.stdout,
            stderr: options.stderr,
            stdin: options.stdin,
            arguments: options.arguments,
            grant: options.grant,
            expected: None,
            random_state: seed,
        }
    }

    /// The context type recorded for the library call now running.
    pub fn take_expected(&mut self) -> Option<Ty> {
        self.expected.take()
    }

    // ------------------------------------------------------------ entry points

    /// Run `main`: `Ok(None)` when it returns, `Ok(Some(error))` when it fails.
    pub fn run_main(&mut self) -> Result<Option<Value>, Interrupt> {
        let Some(main) = self.program.main else {
            return Err(Interrupt::crash("the program has no `main` function"));
        };
        let result = self.call_function(main, Vec::new())?;
        Ok(match result {
            Value::Failure(error) => Some((*error).clone()),
            _ => None,
        })
    }

    /// Call a declared function with its arguments; the result may be a
    /// `Failure`.
    pub fn call_function(&mut self, id: FunctionId, args: Vec<Value>) -> Result<Value, Interrupt> {
        if let Some(&code) = self.program.functions.get(&id) {
            return self.call_code(code, args);
        }
        if let Some(native) = self.natives.get(id).copied().flatten() {
            return native(self, args);
        }
        Err(Interrupt::crash(format!(
            "`{}` is not available in this build of the VM",
            self.qualified(id)
        )))
    }

    /// Run a code object to its end; the result may be a `Failure`.
    pub fn call_code(&mut self, code: CodeId, args: Vec<Value>) -> Result<Value, Interrupt> {
        self.push_frame(code, args);
        let depth = self.frames.len();
        self.execute(depth)
    }

    pub fn qualified(&self, id: FunctionId) -> String {
        let meta = &self.program.function_metas[id];
        format!("{}.{}", meta.module, meta.name)
    }

    fn push_frame(&mut self, code: CodeId, args: Vec<Value>) {
        let base = self.stack.len();
        let locals = self.program.codes[code].locals as usize;
        self.stack.extend(args);
        while self.stack.len() < base + locals {
            self.stack.push(Value::Nothing);
        }
        self.frames.push(Frame {
            code,
            pc: 0,
            base,
            handlers: Vec::new(),
        });
    }

    /// Run until the frame at `entry` (and every frame above it) has
    /// returned; the location of a crash is filled in here.
    fn execute(&mut self, entry: usize) -> Result<Value, Interrupt> {
        loop {
            match self.step(entry) {
                Ok(None) => {}
                Ok(Some(value)) => return Ok(value),
                Err(Interrupt::Crash {
                    message,
                    location: None,
                }) => {
                    let location = self.frames.last().and_then(|frame| {
                        let code = &self.program.codes[frame.code];
                        let span = code.spans.get(frame.pc.saturating_sub(1))?;
                        Some(self.program.location(code.module, *span))
                    });
                    // the entry frame and everything above it are abandoned
                    if let Some(frame) = self.frames.get(entry.saturating_sub(1)) {
                        self.stack.truncate(frame.base);
                    }
                    self.frames.truncate(entry.saturating_sub(1));
                    return Err(Interrupt::Crash { message, location });
                }
                Err(other) => return Err(other),
            }
        }
    }

    // ------------------------------------------------------------ the stack

    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap_or(Value::Nothing)
    }

    fn pop_n(&mut self, count: usize) -> Vec<Value> {
        let at = self.stack.len().saturating_sub(count);
        self.stack.split_off(at)
    }

    fn pop_bool(&mut self) -> Result<bool, Interrupt> {
        match self.pop() {
            Value::Boolean(value) => Ok(value),
            other => Err(Interrupt::crash(format!(
                "expected a Boolean, found {}",
                other.kind_name()
            ))),
        }
    }

    fn local_index(&self, slot: u16) -> usize {
        self.frames.last().map(|f| f.base).unwrap_or(0) + slot as usize
    }

    /// A call result or a constructed value lands on the stack, or, when it
    /// is a `Failure`, in the innermost handled region; without one the
    /// frame's own result is the failure (examples, tests) or the program
    /// crashes. `Some` tells `execute` to return.
    fn settle(&mut self, value: Value, entry: usize) -> Result<Option<Value>, Interrupt> {
        if !value.is_failure() {
            self.stack.push(value);
            return Ok(None);
        }
        let frame = self.frames.last_mut().expect("a frame");
        if let Some((target, height)) = frame.handlers.pop() {
            self.stack.truncate(height);
            self.stack.push(value);
            frame.pc = target;
            return Ok(None);
        }
        match self.program.codes[frame.code].kind {
            CodeKind::Example | CodeKind::Test | CodeKind::Refinement | CodeKind::Constant => {
                self.leave_frame(value, entry)
            }
            CodeKind::Function => {
                let error = match &value {
                    Value::Failure(error) => (**error).clone(),
                    _ => Value::Nothing,
                };
                let shown = self.to_text(&error)?;
                Err(Interrupt::crash(format!("unhandled failure: {shown}")))
            }
        }
    }

    /// Pop the frame with its result: the caller's operand, or the value
    /// `execute` returns when the frame was its entry. A `Failure` result is
    /// settled in the caller.
    fn leave_frame(&mut self, value: Value, entry: usize) -> Result<Option<Value>, Interrupt> {
        let frame = self.frames.pop().expect("a frame");
        self.stack.truncate(frame.base);
        if self.frames.len() < entry {
            return Ok(Some(value));
        }
        self.settle(value, entry)
    }

    // ------------------------------------------------------------ one op

    fn step(&mut self, entry: usize) -> Result<Option<Value>, Interrupt> {
        let program = self.program;
        let (code_id, pc) = {
            let frame = self.frames.last_mut().expect("a frame");
            let pc = frame.pc;
            frame.pc += 1;
            (frame.code, pc)
        };
        let code = &program.codes[code_id];
        let Some(op) = code.ops.get(pc) else {
            return self.leave_frame(Value::Nothing, entry);
        };
        match op {
            Op::Const(index) => self.stack.push(code.constants[*index as usize].clone()),
            Op::Nothing => self.stack.push(Value::Nothing),
            Op::Global(index) => {
                let value = self.global(*index as usize)?;
                self.stack.push(value);
            }
            Op::Load(slot) => {
                let index = self.local_index(*slot);
                self.stack.push(self.stack[index].clone());
            }
            Op::LoadMove(slot) => {
                let index = self.local_index(*slot);
                let value = std::mem::replace(&mut self.stack[index], Value::Nothing);
                self.stack.push(value);
            }
            Op::Store(slot) => {
                let value = self.pop();
                let index = self.local_index(*slot);
                self.stack[index] = value;
            }
            Op::Pop => {
                self.pop();
            }
            Op::Dup => {
                let top = self.stack.last().cloned().unwrap_or(Value::Nothing);
                self.stack.push(top);
            }
            Op::MakeList(count) => {
                let items = self.pop_n(*count as usize);
                self.stack.push(Value::list(items));
            }
            Op::MakeMap(count) => {
                let mut items = self.pop_n(2 * *count as usize).into_iter();
                let mut map = IndexMap::with_capacity(*count as usize);
                while let (Some(key), Some(value)) = (items.next(), items.next()) {
                    map.insert(key, value);
                }
                self.stack.push(Value::Map(Rc::new(map)));
            }
            Op::MakePair => {
                let right = self.pop();
                let left = self.pop();
                self.stack.push(Value::pair(left, right));
            }
            Op::MakeRange { stepped } => {
                let by = if *stepped {
                    self.pop()
                } else {
                    Value::integer(1)
                };
                let to = self.pop();
                let from = self.pop();
                match (from, to, by) {
                    (Value::Integer(from), Value::Integer(to), Value::Integer(by)) => {
                        self.stack
                            .push(Value::Range(Rc::new(RangeValue { from, to, by })));
                    }
                    _ => return Err(Interrupt::crash("a range needs Integer bounds")),
                }
            }
            Op::Construct { ty, fields } => {
                let fields = self.pop_n(*fields as usize);
                let value = self.construct(*ty, fields)?;
                return self.settle(value, entry);
            }
            Op::ConstructVariant { ty, tag, fields } => {
                let fields = self.pop_n(*fields as usize);
                let value = self.construct_variant(*ty, *tag as usize, fields)?;
                return self.settle(value, entry);
            }
            Op::Field(name) => {
                let base = self.pop();
                let name = code.constants[*name as usize].as_text().unwrap_or("");
                let value = self.field(&base, name)?;
                self.stack.push(value);
            }
            Op::With(count) => {
                let mut updates = Vec::with_capacity(*count as usize);
                for _ in 0..*count {
                    let value = self.pop();
                    let name = self.pop();
                    updates.push((name, value));
                }
                let base = self.pop();
                let value = self.with(base, updates)?;
                self.stack.push(value);
            }
            Op::Call { function, args } => {
                let args = self.pop_n(*args as usize);
                return self.invoke(*function, args, entry);
            }
            Op::CallAbility {
                ability,
                method,
                args,
            } => {
                let args = self.pop_n(*args as usize);
                return self.call_ability(*ability, *method as usize, args, entry);
            }
            Op::CallValue(count) => {
                let args = self.pop_n(*count as usize);
                let callee = self.pop();
                match callee {
                    Value::Function(id) => return self.invoke(id, args, entry),
                    other => {
                        return Err(Interrupt::crash(format!(
                            "cannot call {}",
                            other.kind_name()
                        )))
                    }
                }
            }
            Op::ResultType(index) => {
                self.expected = program.result_types.get(*index as usize).cloned();
            }
            Op::Not => {
                let value = self.pop_bool()?;
                self.stack.push(Value::Boolean(!value));
            }
            Op::Binary(op) => {
                let right = self.pop();
                let left = self.pop();
                let value = self.binary(*op, left, right)?;
                self.stack.push(value);
            }
            Op::ToText => {
                let value = self.pop();
                let text = self.to_text(&value)?;
                self.stack.push(Value::text(text));
            }
            Op::Concat(count) => {
                let pieces = self.pop_n(*count as usize);
                let mut text = String::new();
                for piece in &pieces {
                    match piece {
                        Value::Text(part) => text.push_str(part),
                        other => text.push_str(&self.render(other, false)?),
                    }
                }
                self.stack.push(Value::text(text));
            }
            Op::Jump(target) => self.jump(*target),
            Op::JumpIfFalse(target) => {
                if !self.pop_bool()? {
                    self.jump(*target);
                }
            }
            Op::JumpIfTrue(target) => {
                if self.pop_bool()? {
                    self.jump(*target);
                }
            }
            Op::JumpIfAbsent(target) => {
                if matches!(
                    self.stack.last(),
                    Some(Value::Nothing) | Some(Value::Failure(_))
                ) {
                    self.jump(*target);
                }
            }
            Op::PushHandler(target) => {
                let height = self.stack.len();
                let frame = self.frames.last_mut().expect("a frame");
                frame.handlers.push((*target as usize, height));
            }
            Op::PopHandler => {
                let frame = self.frames.last_mut().expect("a frame");
                frame.handlers.pop();
            }
            Op::Return => {
                let value = self.pop();
                return self.leave_frame(value, entry);
            }
            Op::ReturnNothing => return self.leave_frame(Value::Nothing, entry),
            Op::Fail => {
                let error = self.pop();
                let failure = match error {
                    Value::Failure(_) => error,
                    Value::Nothing => {
                        return Err(Interrupt::crash(
                            "`otherwise fail` after a value that was nothing, not a failure",
                        ))
                    }
                    other => Value::failure(other),
                };
                return self.leave_frame(failure, entry);
            }
            Op::Crash => {
                let message = self.pop();
                let text = self.to_text(&message)?;
                return Err(Interrupt::crash(text));
            }
            Op::IsVariant(tag) => {
                let value = self.pop();
                let fits = matches!(&value, Value::Variant(v) if v.tag == *tag as usize);
                self.stack.push(Value::Boolean(fits));
            }
            Op::IsNothing => {
                let value = self.pop();
                self.stack.push(Value::Boolean(value.is_nothing()));
            }
            Op::IsFailure => {
                let value = self.pop();
                self.stack.push(Value::Boolean(value.is_failure()));
            }
            Op::IsType(ty) => {
                let value = self.pop();
                let fits = self.has_type(&value, *ty);
                self.stack.push(Value::Boolean(fits));
            }
            Op::Unpack(count) => {
                let value = self.pop();
                let parts = self.unpack(value, *count as usize)?;
                self.stack.extend(parts);
            }
            Op::UnwrapFailure => {
                let value = self.pop();
                match value {
                    Value::Failure(error) => self.stack.push((*error).clone()),
                    other => self.stack.push(other),
                }
            }
            Op::IterInit(slot) => {
                let source = self.pop();
                let items = self.iterate(source)?;
                let index = self.local_index(*slot);
                self.stack[index] = Value::Native(Rc::new(Native::Iterator(
                    std::cell::RefCell::new((items, 0)),
                )));
            }
            Op::IterNext { slot, exit } => {
                let index = self.local_index(*slot);
                let next = match &self.stack[index] {
                    Value::Native(native) => match &**native {
                        Native::Iterator(state) => {
                            let mut state = state.borrow_mut();
                            let (items, position) = &mut *state;
                            let item = items.get(*position).cloned();
                            *position += 1;
                            item
                        }
                        _ => None,
                    },
                    _ => None,
                };
                match next {
                    Some(item) => self.stack.push(item),
                    None => self.jump(*exit),
                }
            }
            Op::ListPush => {
                let item = self.pop();
                match self.pop() {
                    Value::List(mut list) => {
                        Rc::make_mut(&mut list).push(item);
                        self.stack.push(Value::List(list));
                    }
                    other => {
                        return Err(Interrupt::crash(format!(
                            "cannot collect into {}",
                            other.kind_name()
                        )))
                    }
                }
            }
            Op::GroupInsert => {
                let item = self.pop();
                let key = self.pop();
                match self.pop() {
                    Value::Map(map) => {
                        let mut map = take_map(map);
                        let group = map.entry(key).or_insert_with(|| Value::list(Vec::new()));
                        if let Value::List(list) = group {
                            Rc::make_mut(list).push(item);
                        }
                        self.stack.push(Value::Map(Rc::new(map)));
                    }
                    other => {
                        return Err(Interrupt::crash(format!(
                            "cannot group into {}",
                            other.kind_name()
                        )))
                    }
                }
            }
            Op::SortByKey { descending } => {
                let pairs = match self.pop() {
                    Value::List(list) => take_list(list),
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
                self.sort_by_key(&mut keyed, *descending)?;
                let items: Vec<Value> = keyed.into_iter().map(|(_, item)| item).collect();
                self.stack.push(Value::list(items));
            }
            Op::Deadline(slot) => {
                let limit = match self.pop() {
                    Value::Duration(ms) => ms,
                    other => {
                        return Err(Interrupt::crash(format!(
                            "`within` needs a Duration, found {}",
                            other.kind_name()
                        )))
                    }
                };
                let index = self.local_index(*slot);
                self.stack[index] = Value::Native(Rc::new(Native::Deadline(
                    natives::now_millis() + limit,
                    limit,
                )));
            }
            Op::CheckDeadline(slot) => {
                let index = self.local_index(*slot);
                let expired = match &self.stack[index] {
                    Value::Native(native) => match &**native {
                        Native::Deadline(at, limit) if natives::now_millis() > *at => Some(*limit),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(limit) = expired {
                    let error =
                        Value::record(program.builtins.timed_out, vec![Value::Duration(limit)]);
                    return self.settle(Value::failure(error), entry);
                }
            }
            Op::Check(message) => {
                if !self.pop_bool()? {
                    let text = code.constants[*message as usize]
                        .as_text()
                        .unwrap_or("")
                        .to_string();
                    let failure = Value::failure(Value::text(format!("check failed: {text}")));
                    return self.leave_frame(failure, entry);
                }
            }
        }
        Ok(None)
    }

    fn jump(&mut self, target: u32) {
        self.frames.last_mut().expect("a frame").pc = target as usize;
    }

    // ------------------------------------------------------------ calls

    /// Call a function from an op: a declared body gets a frame, a library
    /// primitive runs in Rust and its result is settled.
    fn invoke(
        &mut self,
        function: FunctionId,
        args: Vec<Value>,
        entry: usize,
    ) -> Result<Option<Value>, Interrupt> {
        if let Some(&code) = self.program.functions.get(&function) {
            self.push_frame(code, args);
            return Ok(None);
        }
        if let Some(native) = self.natives.get(function).copied().flatten() {
            let value = native(self, args)?;
            return self.settle(value, entry);
        }
        Err(Interrupt::crash(format!(
            "`{}` is not available in this build of the VM",
            self.qualified(function)
        )))
    }

    fn call_ability(
        &mut self,
        ability: AbilityId,
        method: usize,
        args: Vec<Value>,
        entry: usize,
    ) -> Result<Option<Value>, Interrupt> {
        let program = self.program;
        let (ability_name, method_name) = match program.abilities.get(ability) {
            Some((name, methods)) => (
                name.as_str(),
                methods.get(method).map(String::as_str).unwrap_or(""),
            ),
            None => ("?", ""),
        };
        let receiver = args.first().cloned().unwrap_or(Value::Nothing);
        if let Some(ty) = receiver.type_id() {
            if let Some(function) = program.types.implementation(ability, ty, method_name) {
                return self.invoke(function, args, entry);
            }
        }
        // the derived abilities
        let b = &program.builtins;
        let value = if ability == b.to_text {
            Value::text(self.to_text(&receiver)?)
        } else if ability == b.compare {
            let other = args.get(1).cloned().unwrap_or(Value::Nothing);
            let ordering = self.compare(&receiver, &other)?;
            self.ordering_value(ordering)
        } else if ability == b.equal {
            let other = args.get(1).cloned().unwrap_or(Value::Nothing);
            Value::Boolean(receiver == other)
        } else if ability == b.hash {
            Value::integer(hash_of(&receiver))
        } else {
            return Err(Interrupt::crash(format!(
                "{} has no implementation of `{ability_name}.{method_name}`",
                self.describe(&receiver)
            )));
        };
        self.settle(value, entry)
    }

    pub fn ordering_value(&self, ordering: Ordering) -> Value {
        let tag = match ordering {
            Ordering::Less => 0,
            Ordering::Equal => 1,
            Ordering::Greater => 2,
        };
        Value::variant(self.program.builtins.ordering, tag, Vec::new())
    }

    fn global(&mut self, index: usize) -> Result<Value, Interrupt> {
        if let Some(value) = &self.globals[index] {
            return Ok(value.clone());
        }
        let meta = &self.program.constants[index];
        let value = self.call_code(meta.code, Vec::new())?;
        if let Value::Failure(error) = &value {
            let shown = self.to_text(error)?;
            return Err(Interrupt::crash(format!(
                "the constant `{}` could not be built: {shown}",
                meta.name
            )));
        }
        self.globals[index] = Some(value.clone());
        Ok(value)
    }

    // ------------------------------------------------------------ values

    /// A record from its fields, or a refined subtype from one value; a
    /// refinement that does not hold gives a `Failure(ConstraintViolation)`.
    pub fn construct(&mut self, ty: TypeId, fields: Vec<Value>) -> Result<Value, Interrupt> {
        let meta = self.program.types.meta(ty);
        match &meta.shape {
            TypeShape::Subtype { .. } => {
                let value = fields.into_iter().next().unwrap_or(Value::Nothing);
                for &code in &meta.refinements {
                    if !self.holds(code, vec![value.clone()])? {
                        return Ok(self.violation(ty, code));
                    }
                }
                Ok(value)
            }
            TypeShape::Record(field_metas) => {
                for field_meta in field_metas {
                    if let Some(index) = field_meta.refinement {
                        let code = meta.refinements[index];
                        if !self.holds(code, fields.clone())? {
                            return Ok(self.violation(ty, code));
                        }
                    }
                }
                Ok(Value::record(ty, fields))
            }
            _ => Ok(Value::record(ty, fields)),
        }
    }

    pub fn construct_variant(
        &mut self,
        ty: TypeId,
        tag: usize,
        fields: Vec<Value>,
    ) -> Result<Value, Interrupt> {
        let meta = self.program.types.meta(ty);
        if let TypeShape::Sum(variants) = &meta.shape {
            if let Some(variant) = variants.get(tag) {
                for field_meta in &variant.fields {
                    if let Some(index) = field_meta.refinement {
                        let code = meta.refinements[index];
                        if !self.holds(code, fields.clone())? {
                            return Ok(self.violation(ty, code));
                        }
                    }
                }
            }
        }
        Ok(Value::variant(ty, tag, fields))
    }

    fn holds(&mut self, code: CodeId, fields: Vec<Value>) -> Result<bool, Interrupt> {
        match self.call_code(code, fields)? {
            Value::Boolean(holds) => Ok(holds),
            Value::Failure(error) => {
                let shown = self.to_text(&error)?;
                Err(Interrupt::crash(format!("a refinement failed: {shown}")))
            }
            other => Err(Interrupt::crash(format!(
                "a refinement produced {}, not a Boolean",
                other.kind_name()
            ))),
        }
    }

    fn violation(&self, ty: TypeId, code: CodeId) -> Value {
        let type_name = self.program.types.meta(ty).name.clone();
        let detail = self.program.codes[code]
            .constants
            .first()
            .and_then(Value::as_text)
            .unwrap_or("its condition")
            .to_string();
        Value::failure(Value::record(
            self.program.builtins.constraint_violation,
            vec![Value::text(type_name), Value::text(detail)],
        ))
    }

    fn field(&mut self, base: &Value, name: &str) -> Result<Value, Interrupt> {
        let types = &self.program.types;
        let found = match base {
            Value::Record(record) => types
                .field_index(record.ty, name)
                .and_then(|index| record.fields.get(index).cloned()),
            Value::Variant(variant) => types
                .variant_field_index(variant.ty, variant.tag, name)
                .and_then(|index| variant.fields.get(index).cloned()),
            Value::Pair(pair) => match name {
                "left" => Some(pair.0.clone()),
                "right" => Some(pair.1.clone()),
                _ => None,
            },
            Value::Native(native) => match &**native {
                Native::CsvRow { line, .. } if name == "line" => Some(Value::integer(*line)),
                _ => None,
            },
            _ => None,
        };
        found.ok_or_else(|| {
            Interrupt::crash(format!("{} has no field `{name}`", self.describe(base)))
        })
    }

    fn with(&mut self, base: Value, updates: Vec<(Value, Value)>) -> Result<Value, Interrupt> {
        let Value::Record(record) = base else {
            return Err(Interrupt::crash(format!(
                "`with` needs a record, found {}",
                base.kind_name()
            )));
        };
        let mut record = Rc::try_unwrap(record).unwrap_or_else(|shared| (*shared).clone());
        for (name, value) in updates {
            let name = name.as_text().unwrap_or("").to_string();
            match self.program.types.field_index(record.ty, &name) {
                Some(index) => record.fields[index] = value,
                None => {
                    return Err(Interrupt::crash(format!(
                        "`{}` has no field `{name}`",
                        self.program.types.meta(record.ty).name
                    )))
                }
            }
        }
        Ok(Value::Record(Rc::new(record)))
    }

    fn unpack(&self, value: Value, count: usize) -> Result<Vec<Value>, Interrupt> {
        let parts = match value {
            Value::Pair(pair) => vec![pair.0.clone(), pair.1.clone()],
            Value::List(items) => take_list(items),
            Value::Record(record) => record.fields.clone(),
            Value::Variant(variant) => variant.fields.clone(),
            other => {
                return Err(Interrupt::crash(format!(
                    "cannot take {count} parts out of {}",
                    other.kind_name()
                )))
            }
        };
        if parts.len() != count {
            return Err(Interrupt::crash(format!(
                "expected {count} parts, found {}",
                parts.len()
            )));
        }
        Ok(parts)
    }

    /// The items of a collection, for a loop.
    pub fn iterate(&self, source: Value) -> Result<Vec<Value>, Interrupt> {
        Ok(match source {
            Value::List(items) => take_list(items),
            Value::Set(items) => items.iter().cloned().collect(),
            Value::Map(entries) => entries
                .iter()
                .map(|(key, value)| Value::pair(key.clone(), value.clone()))
                .collect(),
            Value::Range(range) => range_items(&range)?,
            Value::Text(text) => text.chars().map(|c| Value::text(c.to_string())).collect(),
            other => {
                return Err(Interrupt::crash(format!(
                    "cannot loop over {}",
                    other.kind_name()
                )))
            }
        })
    }

    /// Whether a value has a declared type: records and variants by their
    /// type, base values by kind, a subtype by its base.
    pub fn has_type(&self, value: &Value, ty: TypeId) -> bool {
        if let Some(id) = value.type_id() {
            return id == ty;
        }
        if let TypeShape::Subtype { base } = &self.program.types.meta(ty).shape {
            if let Ty::App(base_id, _) = base {
                return self.has_type(value, *base_id);
            }
            return false;
        }
        let b = &self.program.builtins;
        match value {
            Value::Integer(_) => ty == b.integer,
            Value::Decimal(_) => ty == b.decimal,
            Value::Float(_) => ty == b.float,
            Value::Boolean(_) => ty == b.boolean,
            Value::Text(_) => ty == b.text,
            Value::Bytes(_) => ty == b.bytes,
            Value::List(_) => ty == b.list,
            Value::Map(_) => ty == b.map,
            Value::Set(_) => ty == b.set,
            Value::Range(_) => ty == b.range,
            Value::Pair(_) => ty == b.pair,
            Value::Duration(_) => ty == b.duration,
            Value::Instant(_) => self.program.types.meta(ty).name == "Instant",
            _ => false,
        }
    }

    pub fn describe(&self, value: &Value) -> String {
        match value.type_id() {
            Some(ty) => format!("a `{}`", self.program.types.meta(ty).name),
            None => format!("a {}", value.kind_name()),
        }
    }

    // ------------------------------------------------------------ operators

    fn binary(&mut self, op: BinaryOp, left: Value, right: Value) -> Result<Value, Interrupt> {
        Ok(match op {
            BinaryOp::Is => Value::Boolean(left == right),
            BinaryOp::IsNot => Value::Boolean(left != right),
            BinaryOp::IsLessThan => Value::Boolean(self.compare(&left, &right)? == Ordering::Less),
            BinaryOp::IsAtMost => Value::Boolean(self.compare(&left, &right)? != Ordering::Greater),
            BinaryOp::IsGreaterThan => {
                Value::Boolean(self.compare(&left, &right)? == Ordering::Greater)
            }
            BinaryOp::IsAtLeast => Value::Boolean(self.compare(&left, &right)? != Ordering::Less),
            BinaryOp::And => {
                Value::Boolean(left.as_bool().unwrap_or(false) && right.as_bool().unwrap_or(false))
            }
            BinaryOp::Or => {
                Value::Boolean(left.as_bool().unwrap_or(false) || right.as_bool().unwrap_or(false))
            }
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder
            | BinaryOp::Power => arithmetic(op, left, right)?,
        })
    }

    /// Stable sort of (key, item) pairs by key.
    pub fn sort_by_key(
        &mut self,
        items: &mut [(Value, Value)],
        descending: bool,
    ) -> Result<(), Interrupt> {
        let mut error = None;
        items.sort_by(|a, b| {
            if error.is_some() {
                return Ordering::Equal;
            }
            match self.compare(&a.0, &b.0) {
                Ok(ordering) => {
                    if descending {
                        ordering.reverse()
                    } else {
                        ordering
                    }
                }
                Err(interrupt) => {
                    error = Some(interrupt);
                    Ordering::Equal
                }
            }
        });
        match error {
            Some(interrupt) => Err(interrupt),
            None => Ok(()),
        }
    }

    /// Stable sort of values by their own order.
    pub fn sort_values(&mut self, items: &mut [Value]) -> Result<(), Interrupt> {
        let mut error = None;
        items.sort_by(|a, b| {
            if error.is_some() {
                return Ordering::Equal;
            }
            match self.compare(a, b) {
                Ok(ordering) => ordering,
                Err(interrupt) => {
                    error = Some(interrupt);
                    Ordering::Equal
                }
            }
        });
        match error {
            Some(interrupt) => Err(interrupt),
            None => Ok(()),
        }
    }
}

/// The items of a range in order.
pub fn range_items(range: &RangeValue) -> Result<Vec<Value>, Interrupt> {
    if range.by.is_zero() {
        return Err(Interrupt::crash("a range cannot step by 0"));
    }
    let mut items = Vec::new();
    let mut current = range.from.clone();
    let ascending = !range.by.is_negative();
    loop {
        let ordering = current.compare(&range.to);
        let done = if ascending {
            ordering == Ordering::Greater
        } else {
            ordering == Ordering::Less
        };
        if done {
            break;
        }
        items.push(Value::Integer(current.clone()));
        current = current.add(&range.by);
    }
    Ok(items)
}

pub fn hash_of(value: &Value) -> i64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish() as i64
}

fn decimal_error(error: DecimalError) -> Interrupt {
    Interrupt::crash(match error {
        DecimalError::DivisionByZero => "division by zero".to_string(),
        DecimalError::Overflow => "the Decimal result is out of range".to_string(),
        DecimalError::InvalidExponent => "the Decimal exponent is out of range".to_string(),
    })
}

/// `+ - * / remainder power` on two numbers of one type; mixed types are
/// promoted Integer to Decimal to Float as a safety net.
pub fn arithmetic(op: BinaryOp, left: Value, right: Value) -> Result<Value, Interrupt> {
    match (left, right) {
        (Value::Integer(a), Value::Integer(b)) => integer_arithmetic(op, &a, &b),
        (Value::Decimal(a), Value::Decimal(b)) => decimal_arithmetic(op, &a, &b),
        (Value::Float(a), Value::Float(b)) => float_arithmetic(op, a, b),
        (Value::Decimal(a), Value::Integer(b)) => {
            if op == BinaryOp::Power {
                return Ok(Value::Decimal(a.power(&b).map_err(decimal_error)?));
            }
            decimal_arithmetic(op, &a, &Decimal::from_int(&b))
        }
        (Value::Integer(a), Value::Decimal(b)) => {
            decimal_arithmetic(op, &Decimal::from_int(&a), &b)
        }
        (Value::Float(a), Value::Integer(b)) => float_arithmetic(op, a, b.to_f64()),
        (Value::Integer(a), Value::Float(b)) => float_arithmetic(op, a.to_f64(), b),
        (Value::Float(a), Value::Decimal(b)) => float_arithmetic(op, a, b.to_f64()),
        (Value::Decimal(a), Value::Float(b)) => float_arithmetic(op, a.to_f64(), b),
        (left, right) => Err(Interrupt::crash(format!(
            "cannot apply `{}` to {} and {}",
            op.spelling(),
            left.kind_name(),
            right.kind_name()
        ))),
    }
}

fn integer_arithmetic(op: BinaryOp, a: &Int, b: &Int) -> Result<Value, Interrupt> {
    Ok(Value::Integer(match op {
        BinaryOp::Add => a.add(b),
        BinaryOp::Subtract => a.subtract(b),
        BinaryOp::Multiply => a.multiply(b),
        BinaryOp::Divide => {
            return Err(Interrupt::crash(
                "`/` is not defined on Integers; use `quotient` or `to_decimal`",
            ))
        }
        BinaryOp::Remainder => a
            .remainder(b)
            .ok_or_else(|| Interrupt::crash("division by zero"))?,
        BinaryOp::Power => a
            .power(b)
            .ok_or_else(|| Interrupt::crash("the exponent must be between 0 and 1000000"))?,
        _ => unreachable!(),
    }))
}

fn decimal_arithmetic(op: BinaryOp, a: &Decimal, b: &Decimal) -> Result<Value, Interrupt> {
    let result = match op {
        BinaryOp::Add => a.add(b),
        BinaryOp::Subtract => a.subtract(b),
        BinaryOp::Multiply => a.multiply(b),
        BinaryOp::Divide => a.divide(b),
        BinaryOp::Remainder => {
            let quotient = a.divide(b).map_err(decimal_error)?;
            let whole = Decimal::from_int(&quotient.truncated());
            let product = b.multiply(&whole).map_err(decimal_error)?;
            a.subtract(&product)
        }
        BinaryOp::Power => {
            let exponent = b.truncated();
            if Decimal::from_int(&exponent) != *b {
                return Err(Interrupt::crash(
                    "a Decimal exponent must be a whole number",
                ));
            }
            a.power(&exponent)
        }
        _ => unreachable!(),
    };
    result.map(Value::Decimal).map_err(decimal_error)
}

fn float_arithmetic(op: BinaryOp, a: f64, b: f64) -> Result<Value, Interrupt> {
    Ok(Value::Float(match op {
        BinaryOp::Add => a + b,
        BinaryOp::Subtract => a - b,
        BinaryOp::Multiply => a * b,
        BinaryOp::Divide => {
            if b == 0.0 {
                return Err(Interrupt::crash("division by zero"));
            }
            a / b
        }
        BinaryOp::Remainder => {
            if b == 0.0 {
                return Err(Interrupt::crash("division by zero"));
            }
            a % b
        }
        BinaryOp::Power => a.powf(b),
        _ => unreachable!(),
    }))
}
