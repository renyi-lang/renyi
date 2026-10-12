//! Extensions (decisions AJ1 and AK1 to AK4): native functions registered
//! with the VM under declaration files and built into a `renyi` binary.
//! The standard library is the first extension (`natives::standard`);
//! another is a Rust crate that holds an [`Extension`] value, and a binary
//! that carries it is `renyi::main_with(vec![EXTENSION])`. A registered
//! native is checked, recorded, replayed and narrated like a library
//! primitive: the boundary of `vm.rs` does not know which extension a
//! function came from.
//!
//! A declaration file and its natives meet by name (AK2): a [`Native`]
//! names its module, its function and the type of the function's first
//! parameter as the checker spells it, in full (`List of Integer`) or by
//! its head (`List`), and [`Registry::verify`] holds the table and the
//! declarations equal both ways. A native sees what the standard
//! library's natives see (AK3): the VM, the plain values of its arguments
//! and the helpers of [`crate::natives`]. It declares its capabilities
//! from the kinds of reference section 11 and adds none (AK4).
//!
//! A native may carry a typed entry (decision AU1): a second Rust
//! function from a fixed family of signatures, which reads its arguments
//! where they lie and answers a Boolean, an Integer, a Float or a value,
//! or declines (`None`), whereupon the native itself runs through the
//! boundary. The generated code calls it with the arguments borrowed and
//! takes a scalar result in a register, so a call to a pure primitive
//! costs no more than the primitive's own work.
//!
//! A native may carry an in-place entry too (decision AW11): the
//! generated code of both tiers and the interpreter call it directly, by
//! its address in a table the VM keeps per function, on the arguments
//! where they lie on the stack, and it answers in the receiver's place,
//! the other arguments consumed: no scratch vector, no `Result`, no
//! status and no push on the way. [`in_place!`] makes one from a typed
//! entry (the arguments read where they lie) or from a function that
//! takes its arguments (`fn(&mut [Value], u32) -> Option<Value>`), which
//! updates a collection in place when nothing else holds it.

use std::collections::HashMap;

use renyi_check::Library;
use renyi_syntax::parse_declarations;

use crate::natives::NativeFn;
use crate::value::Value;
use crate::vm::Vm;

/// The typed entry of a native (decision AU1): a function over the plain
/// arguments, borrowed, that answers in one of four kinds or declines
/// with `None`, which sends the call through the native itself and the
/// boundary. A typed entry must answer exactly what the native answers
/// whenever it answers; it declines on a guarded argument (the result
/// would need the guard's origins), on an Integer past the machine word,
/// and wherever the native would fail or crash. Its kind must agree with
/// the declared result type: `Bool` for `Boolean`, `Int` for `Integer`,
/// `Float` for `Float`, `Value` for anything else (`Registry::verify`
/// checks). A `Value` entry never answers a `Failure`.
#[derive(Clone, Copy)]
pub enum Typed {
    Bool(fn(&[Value]) -> Option<bool>),
    Int(fn(&[Value]) -> Option<i64>),
    Float(fn(&[Value]) -> Option<f64>),
    Value(fn(&[Value]) -> Option<Value>),
}

/// The kind of a typed entry's result: what the generated code expects
/// back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypedKind {
    Bool,
    Int,
    Float,
    Value,
}

impl Typed {
    pub fn kind(self) -> TypedKind {
        match self {
            Typed::Bool(_) => TypedKind::Bool,
            Typed::Int(_) => TypedKind::Int,
            Typed::Float(_) => TypedKind::Float,
            Typed::Value(_) => TypedKind::Value,
        }
    }
}

/// What the generated code calls for an in-place entry (decision AW11):
/// the VM, the address of the call's first argument on the stack, the
/// count of the arguments, the bits of those the caller still holds (the
/// first argument bit 0) and the call's pc. On `CONTINUE` or `FAILURE`
/// the answer is in the first argument's place and the arguments are
/// consumed (an argument the caller holds is left as it was, the others
/// are released); on `DECLINED` nothing was touched and the call goes
/// the general way; on `INTERRUPT` the answer is in place and the
/// interrupt is parked in the VM. [`in_place!`] writes one.
pub type InPlaceShim = unsafe extern "C" fn(*mut Vm<'static>, *mut Value, u32, u32, u32) -> i32;

/// The in-place entry of a native (decision AW11): the function the
/// generated code and the interpreter call directly, and whether it may
/// take an argument the call owns (to update a collection in place) or
/// only reads its arguments, as a typed entry does. The entry must answer
/// exactly what the native answers whenever it answers, and decline
/// wherever a typed entry would: on a guarded argument, on an Integer
/// past the machine word, and wherever the native would fail or crash;
/// a declared `Integer`, `Boolean` or `Float` result is answered as a
/// small Integer, a Boolean or a Float. An entry that takes must leave
/// every argument as it was when it declines, and must copy, not update,
/// an argument whose bit is set in the mask it is given, since the caller
/// still holds that value.
#[derive(Clone, Copy)]
pub struct InPlace {
    pub shim: InPlaceShim,
    pub takes: bool,
}

/// An in-place entry (decision AW11) from a typed entry, `in_place!(Int:
/// list_length_typed)` for the kinds `Bool`, `Int`, `Float` and `Value`,
/// or from a function that takes its arguments, `in_place!(takes
/// list_append_in_place)` for a `fn(&mut [Value], u32) -> Option<Value>`
/// that is given the mask of the arguments the caller still holds (the
/// first argument bit 0) and may take the others with
/// [`crate::natives::take_or_clone`].
#[macro_export]
macro_rules! in_place {
    ($kind:ident : $answer:path) => {{
        unsafe extern "C" fn shim(
            vm: *mut $crate::Vm<'static>,
            at: *mut $crate::Value,
            count: u32,
            borrowed: u32,
            pc: u32,
        ) -> i32 {
            // SAFETY: the generated code's call, as `in_place_typed` says
            unsafe {
                $crate::extension::in_place_typed(
                    vm,
                    at,
                    count,
                    borrowed,
                    pc,
                    $crate::Typed::$kind($answer),
                )
            }
        }
        $crate::InPlace { shim, takes: false }
    }};
    (takes $answer:path) => {{
        unsafe extern "C" fn shim(
            vm: *mut $crate::Vm<'static>,
            at: *mut $crate::Value,
            count: u32,
            borrowed: u32,
            pc: u32,
        ) -> i32 {
            // SAFETY: the generated code's call, as `in_place_taking` says
            unsafe { $crate::extension::in_place_taking(vm, at, count, borrowed, pc, $answer) }
        }
        $crate::InPlace { shim, takes: true }
    }};
}

/// The body of an in-place entry made from a typed entry (`in_place!`,
/// decision AW11): the arguments read where they lie, the answer boxed in
/// the first argument's place. Inlined into every shim, so that the match
/// on the kind is settled at compile time. No guard is tested: a typed
/// entry answers on plain values alone (a guarded value carries its
/// origins outermost and matches no plain pattern), which is what
/// `rt_call_typed` relies on too. A scalar answer (`Bool`, `Int`,
/// `Float`) retains nothing, and the arguments released only lower the
/// memory in use, so a scalar entry tests no budget and answers
/// `CONTINUE` or `DECLINED` alone, as AU34's lean helpers did; the
/// generated code tests nothing else after it. A `Value` answer goes
/// through `answered_in_place`.
///
/// # Safety
///
/// `vm` is the VM running the call; `at` points at its `count` arguments
/// on the VM's stack, `count` at least 1 and at most 32 (the answer needs
/// a place; the mask has 32 bits), which nothing reads again but through
/// the entry.
#[doc(hidden)]
#[inline(always)]
pub unsafe fn in_place_typed(
    vm: *mut Vm<'static>,
    at: *mut Value,
    count: u32,
    borrowed: u32,
    pc: u32,
    typed: Typed,
) -> i32 {
    use crate::native::runtime::{CONTINUE, DECLINED};
    // SAFETY: the caller's.
    let args = unsafe { std::slice::from_raw_parts(at, count as usize) };
    let value = match typed {
        Typed::Bool(answer) => match answer(args) {
            Some(answer) => Value::Boolean(answer),
            None => return DECLINED,
        },
        Typed::Int(answer) => match answer(args) {
            Some(answer) => Value::integer(answer),
            None => return DECLINED,
        },
        Typed::Float(answer) => match answer(args) {
            Some(answer) => Value::Float(answer),
            None => return DECLINED,
        },
        Typed::Value(answer) => {
            return match answer(args) {
                // SAFETY: the caller's.
                Some(value) => unsafe {
                    answered_in_place(&mut *vm, at, count, borrowed, pc, value)
                },
                None => DECLINED,
            };
        }
    };
    // SAFETY: the caller's: the arguments are dead after, and the first
    // was released or is the caller's, either way not to be dropped here.
    unsafe {
        release_owned(at, count, borrowed);
        std::ptr::write(at, value);
    }
    CONTINUE
}

/// The body of an in-place entry that takes its arguments (`in_place!`,
/// decision AW11). Under a provenance guard it declines: the native must
/// run through the boundary, which lifts the arguments' origins onto what
/// it builds. It may fail or find the budget spent, so the generated code
/// tests its status as a `Value` entry's.
///
/// # Safety
///
/// As `in_place_typed`.
#[doc(hidden)]
#[inline(always)]
pub unsafe fn in_place_taking(
    vm: *mut Vm<'static>,
    at: *mut Value,
    count: u32,
    borrowed: u32,
    pc: u32,
    answer: fn(&mut [Value], u32) -> Option<Value>,
) -> i32 {
    use crate::native::runtime::DECLINED;
    // SAFETY: the caller's.
    let vm = unsafe { &mut *vm };
    if vm.guarding {
        return DECLINED;
    }
    // SAFETY: the caller's.
    let args = unsafe { std::slice::from_raw_parts_mut(at, count as usize) };
    match answer(args, borrowed) {
        // SAFETY: the caller's.
        Some(value) => unsafe { answered_in_place(vm, at, count, borrowed, pc, value) },
        None => DECLINED,
    }
}

/// An in-place entry's value answer put in place: the arguments the
/// caller does not hold released where they lie, the answer written over
/// the first, and the memory budget checked after, as a typed entry's
/// answer is.
///
/// # Safety
///
/// As `in_place_typed`; the arguments are dead after.
#[inline(always)]
unsafe fn answered_in_place(
    vm: &mut Vm,
    at: *mut Value,
    count: u32,
    borrowed: u32,
    pc: u32,
    value: Value,
) -> i32 {
    use crate::native::runtime::{CONTINUE, FAILURE};
    let failed = value.is_failure();
    // SAFETY: the caller's: the arguments are dead after, and the first
    // was released or is the caller's, either way not to be dropped here.
    unsafe {
        release_owned(at, count, borrowed);
        std::ptr::write(at, value);
    }
    if vm.memory_is_over() {
        return crate::native::runtime::over_memory(vm, pc);
    }
    if failed {
        FAILURE
    } else {
        CONTINUE
    }
}

/// The arguments the call owns released where they lie: every one of the
/// `count` at `at` whose bit `borrowed` does not set (one the entry took
/// is `Nothing` there, a tag test). A loop over the bits set, not over
/// the arguments: nothing to do costs one test.
///
/// # Safety
///
/// As `in_place_typed`; the arguments are dead after.
#[inline(always)]
unsafe fn release_owned(at: *mut Value, count: u32, borrowed: u32) {
    let mut owned = !borrowed & ((1u64 << count) - 1) as u32;
    while owned != 0 {
        let index = owned.trailing_zeros();
        // SAFETY: the caller's: the argument is dead after.
        unsafe { crate::value::release_in_place(at.add(index as usize)) };
        owned &= owned - 1;
    }
}

/// A native function of an extension: which declared function it
/// implements, and the Rust function.
#[derive(Clone, Copy)]
pub struct Native {
    /// The module of the declaration file: `std.json`, `python.pandas`.
    pub module: &'static str,
    /// The function's name in the declaration file.
    pub name: &'static str,
    /// The type of the function's first parameter as the checker spells
    /// it (`Text`, `List of Integer`) or its head (`List`); `None` for a
    /// function matched by its name alone, whatever its first parameter.
    pub receiver: Option<&'static str>,
    pub run: NativeFn,
    /// The typed entry (decision AU1), when the native has one.
    pub typed: Option<Typed>,
    /// The in-place entry (decision AW11), when the native has one.
    pub in_place: Option<InPlace>,
}

impl Native {
    /// A function matched by its name alone:
    /// `Native::function("std.json", "parse", parse)`.
    pub const fn function(module: &'static str, name: &'static str, run: NativeFn) -> Native {
        Native {
            module,
            name,
            receiver: None,
            run,
            typed: None,
            in_place: None,
        }
    }

    /// The native with a typed entry (decision AU1):
    /// `Native::method("std.prelude", "contains", "Text", text_contains)
    /// .with_typed(Typed::Bool(text_contains_typed))`.
    pub const fn with_typed(self, typed: Typed) -> Native {
        Native {
            module: self.module,
            name: self.name,
            receiver: self.receiver,
            run: self.run,
            typed: Some(typed),
            in_place: self.in_place,
        }
    }

    /// The native with an in-place entry (decision AW11):
    /// `Native::method("std.prelude", "append", "List", list_append)
    /// .with_in_place(in_place!(takes list_append_in_place))`.
    pub const fn with_in_place(self, in_place: InPlace) -> Native {
        Native {
            module: self.module,
            name: self.name,
            receiver: self.receiver,
            run: self.run,
            typed: self.typed,
            in_place: Some(in_place),
        }
    }

    /// A function matched by its name and the type of its first parameter,
    /// a method most often:
    /// `Native::method("std.prelude", "length", "Text", text_length)`.
    pub const fn method(
        module: &'static str,
        name: &'static str,
        receiver: &'static str,
        run: NativeFn,
    ) -> Native {
        Native {
            module,
            name,
            receiver: Some(receiver),
            run,
            typed: None,
            in_place: None,
        }
    }
}

/// What an extension brings: its declaration files, one per module, and
/// its natives. The fields are static so that an extension is a constant
/// of its crate.
#[derive(Clone, Copy)]
pub struct Extension {
    pub name: &'static str,
    pub version: &'static str,
    /// Each module's name and the text of its declaration file, in the
    /// order they are declared: a module before one that imports it.
    pub modules: &'static [(&'static str, &'static str)],
    pub natives: &'static [Native],
}

/// The natives registered under one module and name, in registration
/// order: the receiver, the function, its typed entry and its in-place
/// entry.
type Entries = Vec<(
    Option<&'static str>,
    NativeFn,
    Option<Typed>,
    Option<InPlace>,
)>;

/// What a lookup finds for a declared function: the native, its typed
/// entry and its in-place entry.
pub type Found = (NativeFn, Option<Typed>, Option<InPlace>);

/// The extensions a toolchain is built with, the standard library first,
/// and the natives they register, by module, name and receiver.
#[derive(Clone)]
pub struct Registry {
    extensions: Vec<Extension>,
    /// Per module and function name, the entries in registration order.
    table: HashMap<(&'static str, &'static str), Entries>,
}

impl Registry {
    /// The standard library alone: what the official binary carries.
    pub fn standard() -> Registry {
        let mut registry = Registry {
            extensions: Vec::new(),
            table: HashMap::new(),
        };
        registry.add(crate::natives::standard());
        registry
    }

    /// Register an extension after those already registered. Nothing is
    /// checked here: `verify` is.
    pub fn add(&mut self, extension: Extension) {
        for native in extension.natives {
            self.table
                .entry((native.module, native.name))
                .or_default()
                .push((native.receiver, native.run, native.typed, native.in_place));
        }
        self.extensions.push(extension);
    }

    /// `add`, for a chain: `Registry::standard().with(EXTENSION)`.
    pub fn with(mut self, extension: Extension) -> Registry {
        self.add(extension);
        self
    }

    /// Every extension, the standard library first.
    pub fn extensions(&self) -> &[Extension] {
        &self.extensions
    }

    /// `name version` of every extension but the standard library, in
    /// registration order: what the run manifest names (decision AK1).
    pub fn extras(&self) -> Vec<String> {
        self.extensions
            .iter()
            .skip(1)
            .map(|extension| format!("{} {}", extension.name, extension.version))
            .collect()
    }

    /// The declaration files of every extension, in order, for the
    /// checker.
    pub fn library(&self) -> Library {
        let mut library = Library::empty();
        for extension in &self.extensions {
            for (name, source) in extension.modules {
                library.add(name, source);
            }
        }
        library
    }

    /// The native behind a declared function: the entry whose receiver is
    /// the type's full spelling, else its head, else the entry without a
    /// receiver.
    pub fn lookup(&self, module: &str, name: &str, receiver: Option<&str>) -> Option<NativeFn> {
        self.lookup_entry(module, name, receiver)
            .map(|(run, _, _)| run)
    }

    /// `lookup`, with the typed entry (decision AU1) and the in-place
    /// entry (decision AW11) of the native found.
    pub fn lookup_entry(&self, module: &str, name: &str, receiver: Option<&str>) -> Option<Found> {
        let entries = self.table.get(&(module, name))?;
        let find = |wanted: Option<&str>| {
            entries
                .iter()
                .find(|(entry, _, _, _)| *entry == wanted)
                .map(|(_, run, typed, in_place)| (*run, *typed, *in_place))
        };
        let head = receiver.map(|receiver| receiver.split(' ').next().unwrap_or(receiver));
        receiver
            .and_then(|full| find(Some(full)))
            .or_else(|| head.and_then(|head| find(Some(head))))
            .or_else(|| find(None))
    }

    /// The table against the declarations, both ways (decision AK2):
    /// every declaration file parses and declares the module it is
    /// registered under, no module is declared twice, the declarations
    /// check, every declared function has a native, and every native
    /// implements a declared function. The first problem of the files
    /// stops the check; the mismatches of the table are all reported, one
    /// per line. A binary whose extensions fail this does not start; the
    /// standard library passes it in a test.
    pub fn verify(&self) -> Result<(), String> {
        let mut owners: HashMap<&'static str, &'static str> = HashMap::new();
        for extension in &self.extensions {
            for (module, source) in extension.modules {
                if let Some(other) = owners.insert(module, extension.name) {
                    return Err(format!(
                        "the module `{module}` is declared by the extensions `{other}` and `{}`",
                        extension.name
                    ));
                }
                let parsed = parse_declarations(source);
                if let Some(problem) = parsed.diagnostics.iter().find(|d| d.is_error()) {
                    return Err(format!(
                        "extension `{}`: the declaration file of `{module}` does not parse: {}",
                        extension.name, problem.message
                    ));
                }
                let declared: Vec<&str> =
                    parsed.module.name.iter().map(|n| n.text.as_str()).collect();
                let declared = declared.join(".");
                if declared != *module {
                    return Err(format!(
                        "extension `{}`: the declaration file registered as `{module}` declares `module {declared}`",
                        extension.name
                    ));
                }
            }
        }
        let owner = |module: &str| owners.get(module).copied().unwrap_or("?");
        let mut world = self.library().world();
        world.resolve_all();
        let mut problems = Vec::new();
        for (id, diagnostic) in &world.diagnostics {
            if diagnostic.is_error() {
                let module = &world.modules[*id].name;
                return Err(format!(
                    "extension `{}`: `{module}`: {}",
                    owner(module),
                    diagnostic.message
                ));
            }
        }
        for info in &world.functions {
            if !info.is_library {
                continue;
            }
            let module = &world.modules[info.module].name;
            let receiver = info.params.first().map(|(_, ty)| world.show(ty));
            let on = receiver
                .as_deref()
                .map(|receiver| format!(" on `{receiver}`"))
                .unwrap_or_default();
            match self.lookup_entry(module, &info.name, receiver.as_deref()) {
                None => problems.push(format!(
                    "extension `{}`: `{module}.{}`{on} is declared but has no native",
                    owner(module),
                    info.name
                )),
                Some((_, typed, in_place)) if typed.is_some() || in_place.is_some() => {
                    // a typed entry and an in-place entry skip the
                    // boundary, so the function must need nothing, and a
                    // typed entry answers in the kind of the declared
                    // result (decision AU1)
                    if !info.needs.is_empty() {
                        let needs: Vec<String> =
                            info.needs.iter().map(|need| need.spelling()).collect();
                        let what = if typed.is_some() {
                            "a typed entry"
                        } else {
                            "an in-place entry"
                        };
                        problems.push(format!(
                            "extension `{}`: `{module}.{}`{on} has {what} but needs `{}`; {what} skips the boundary",
                            owner(module),
                            info.name,
                            needs.join(", ")
                        ));
                    }
                    let Some(typed) = typed else {
                        continue;
                    };
                    let returns = info
                        .returns
                        .as_ref()
                        .map(|ty| world.show(ty))
                        .unwrap_or_else(|| "nothing".to_string());
                    let expected = match returns.as_str() {
                        "Boolean" => TypedKind::Bool,
                        "Integer" => TypedKind::Int,
                        "Float" => TypedKind::Float,
                        _ => TypedKind::Value,
                    };
                    if typed.kind() != expected {
                        let answers = match typed.kind() {
                            TypedKind::Bool => "a Boolean",
                            TypedKind::Int => "an Integer",
                            TypedKind::Float => "a Float",
                            TypedKind::Value => "a value",
                        };
                        problems.push(format!(
                            "extension `{}`: the typed entry of `{module}.{}`{on} answers {answers} where the declaration returns `{returns}`",
                            owner(module),
                            info.name
                        ));
                    }
                }
                Some(_) => {}
            }
        }
        for extension in &self.extensions {
            for native in extension.natives {
                let implemented = world.functions.iter().any(|info| {
                    info.is_library
                        && world.modules[info.module].name == native.module
                        && info.name == native.name
                        && match native.receiver {
                            None => true,
                            Some(wanted) => info
                                .params
                                .first()
                                .map(|(_, ty)| world.show(ty))
                                .is_some_and(|full| {
                                    full == wanted || full.split(' ').next() == Some(wanted)
                                }),
                        }
                });
                if !implemented {
                    let on = native
                        .receiver
                        .map(|receiver| format!(" on `{receiver}`"))
                        .unwrap_or_default();
                    problems.push(format!(
                        "extension `{}`: the native `{}.{}`{on} implements no declared function",
                        extension.name, native.module, native.name
                    ));
                }
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems.join("\n"))
        }
    }
}
