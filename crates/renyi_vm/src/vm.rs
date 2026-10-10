//! The interpreter: frames over one value stack, re-entrant for callbacks
//! (ability methods, sorting keys, handlers). A fallible call that fails
//! leaves a `Failure` value; the frame's innermost handled region takes it,
//! or the frame's own result is the failure (examples, tests), or the program
//! crashes, since the checker has made sure a program handles every failure.
//!
//! The loop (`run_frames`, decision X3) keeps the running frame's code,
//! program counter and base in locals and writes them back only when the
//! frame changes or an interrupt needs the location; a call to a declared
//! function leaves the arguments on the stack as the callee's first
//! locals; the handled regions of every frame share one stack; a field
//! access remembers, per site, the index it found last.

use std::cmp::Ordering;
use std::io::{BufRead, Write};
use std::net::TcpListener;
use std::rc::Rc;

use renyi_check::effects::{self, Capability};
use renyi_check::types::Ty;
use renyi_check::{AbilityId, FunctionId, TypeId};
use renyi_syntax::ast::BinaryOp;

use crate::bytecode::{Code, CodeKind, Op};
use crate::compile::{CodeId, Program};
use crate::decimal::{Decimal, DecimalError};
use crate::extension::{Registry, Typed, TypedKind};
use crate::grant::{self, Counter, Guard, Narrowing};
use crate::integer::Int;
use crate::memory;
use crate::native::{self, Jit};
use crate::natives::json::{self, Json, Naming};
use crate::natives::time::instant_text;
use crate::natives::{self, NativeFn};
use crate::pinned::Pinned;
use crate::profile::Profile;
use crate::recording::{clip, redact, Call, Manifest, Outcome, Recording, Replay};
use crate::types::TypeShape;
use crate::value::{plain_all, plain_in_place, take_list, Native, RangeValue, Value};

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
    /// The watch of `renyi serve --watch` (decision AO1) found a new
    /// version ready between two requests: the run stops so that `main`
    /// can be run again on it, the listening socket left in the VM.
    Reload,
    /// The memory budget of the run (decision AP1) was exceeded: the
    /// limit, and the most bytes the run held above the level at its
    /// start.
    OverMemory { limit: u64, used: u64 },
}

impl Interrupt {
    pub fn crash(message: impl Into<String>) -> Interrupt {
        Interrupt::Crash {
            message: message.into(),
            location: None,
        }
    }
}

/// How a program is run: its arguments, its streams, what the command line
/// changes about its grant, and whether the run is recorded, replayed or
/// narrated.
pub struct Options {
    pub arguments: Vec<String>,
    pub stdout: Box<dyn Write>,
    pub stderr: Box<dyn Write>,
    pub stdin: Box<dyn BufRead>,
    /// `--deny`, `--allow-*` and `--at-most`.
    pub narrowing: Narrowing,
    /// Write a recording of every effect (`renyi record`).
    pub record: bool,
    /// Answer every effect from this recording (`renyi run --replay`).
    pub replay: Option<Recording>,
    /// The source revision a recording names.
    pub revision: Option<String>,
    /// Narrate the run on stderr (`--explain`).
    pub explain: bool,
    /// `renyi test --strict`: a recorded call a test never reaches fails it.
    pub strict: bool,
    /// `renyi test --refresh NAME`: run this test live and re-record its
    /// fixture.
    pub refresh: Option<String>,
    /// `server.serve` returns after this many requests; for tests of a
    /// server, which otherwise runs until the process stops.
    pub serve_limit: Option<usize>,
    /// Names whose values a recording replaces with a placeholder
    /// (`renyi record --redact NAME`).
    pub redact: Vec<String>,
    /// The known part of the run manifest (decision Q2): the toolchain,
    /// the source and the code hash; the rest is filled in when the run
    /// ends.
    pub manifest: Manifest,
    /// Write the console output during a replay (`renyi reproduce`).
    pub replay_output: bool,
    /// Count every operation, call and primitive call and sample where the
    /// time goes; the report goes to stderr when the run ends (`--profile`,
    /// decision X4).
    pub profile: bool,
    /// Run every function on the interpreter, never on generated machine
    /// code (`--interpret`, decision AG3); a narrated or profiled run does
    /// so by itself.
    pub interpret: bool,
    /// The image of the program (decision AS1): its machine code, loaded
    /// in place of compiling; built for this machine from this program.
    pub image: Option<crate::native::image::Image>,
    /// The extensions the toolchain is built with (decision AJ1): where
    /// the natives behind the program's library calls come from.
    pub registry: Registry,
    /// `renyi serve --watch` (decision AO1): asked by `server.serve`
    /// every half second while no request is waiting; `true` when a new
    /// version is ready, on which the run stops with `Interrupt::Reload`.
    pub watch: Option<Box<dyn FnMut() -> bool>>,
    /// The listening socket the previous run of `main` left (decision
    /// AO1), for `server.serve` to continue on when its port is the one
    /// asked for.
    pub listener: Option<TcpListener>,
    /// The memory budget of the run in bytes (decision AP1): the bytes it
    /// may hold above the level at its start, counted by the allocator
    /// of `memory`, checked at every call, primitive and loop turning;
    /// a budgeted run stays on the interpreter.
    pub memory: Option<u64>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            arguments: Vec::new(),
            stdout: Box::new(std::io::stdout()),
            stderr: Box::new(std::io::stderr()),
            stdin: Box::new(std::io::BufReader::new(std::io::stdin())),
            narrowing: Narrowing::default(),
            record: false,
            replay: None,
            revision: None,
            explain: false,
            strict: false,
            refresh: None,
            serve_limit: None,
            redact: Vec::new(),
            manifest: Manifest::default(),
            replay_output: false,
            profile: false,
            interpret: false,
            image: None,
            registry: Registry::standard(),
            watch: None,
            listener: None,
            memory: None,
        }
    }
}

/// A grant shared by the frames it is in force for.
pub type SharedGrant = Rc<Vec<Capability>>;

#[repr(C)]
pub(crate) struct Frame {
    pub(crate) code: CodeId,
    /// The next op to run; while the frame runs, the loop keeps it in a
    /// local and writes it back when the frame is left for a callee.
    pub(crate) pc: usize,
    /// The index of local 0 on the stack.
    pub(crate) base: usize,
    /// The height of `Vm::handlers` when the frame was entered: its own
    /// handled regions lie above.
    pub(crate) handler_base: usize,
    /// The effective grant of the function running here (decision Q1), by
    /// its index in `Vm::grants`.
    pub(crate) grant: u32,
}

/// What the generated code reaches through the VM pointer (decision AS1),
/// so that it holds no address of its own and an image of it loads
/// anywhere: how many generated frames are nested on the machine stack,
/// and the tables the JIT owns: the compiled body of every code object
/// (null while cold), the helpers in the order of `codegen::SIGNATURES`,
/// and the constants of every code object. Null and zero on an
/// interpreted run.
#[repr(C)]
pub(crate) struct NativeState {
    pub(crate) depth: usize,
    pub(crate) direct_table: *const *const u8,
    pub(crate) helpers: *const *const u8,
    pub(crate) constants: *const *const Value,
    /// The VM's count of ops run per code object (`Vm::hotness`), which
    /// the template tier's code adds to at its entries and its back
    /// edges (decision AU18).
    pub(crate) hotness: *mut u32,
}

impl Default for NativeState {
    fn default() -> NativeState {
        NativeState {
            depth: 0,
            direct_table: std::ptr::null(),
            helpers: std::ptr::null(),
            constants: std::ptr::null(),
            hotness: std::ptr::null_mut(),
        }
    }
}

/// What an `Op::Field` site saw last: a record's type (the tag is
/// `usize::MAX`) or a variant's type and tag, and the field's index in it.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct FieldSite {
    pub(crate) ty: TypeId,
    pub(crate) tag: usize,
    pub(crate) index: usize,
}

const EMPTY_SITE: FieldSite = FieldSite {
    ty: usize::MAX,
    tag: usize::MAX,
    index: 0,
};

/// What the loop does after a value was settled or a frame was left.
enum Flow {
    /// Carry on in the running frame.
    Go,
    /// The running frame changed (a frame was left, a handler took a
    /// failure): reload its state.
    Reload,
    /// The entry frame returned this.
    Done(Value),
}

// Decision AR4: the layout is fixed, so that the generated code finds the
// stack, the frames and the handlers by `offset_of!`.
#[repr(C)]
pub struct Vm<'p> {
    pub program: &'p Program,
    pub(crate) stack: Pinned<Value>,
    pub(crate) frames: Pinned<Frame>,
    /// The open handled regions of every frame, innermost last: the target
    /// and the stack height to unwind to; `Frame::handler_base` separates
    /// the frames.
    pub(crate) handlers: Pinned<(usize, usize)>,
    /// Per `Op::Field` site, what it saw last (decision X3).
    pub(crate) field_cache: Pinned<FieldSite>,
    /// The profiler of `--profile` (decision X4).
    profile: Option<Profile>,
    /// The arguments of the primitive being called, moved off the stack
    /// into a buffer that every call reuses (decision X3).
    pub(crate) scratch: Vec<Value>,
    globals: Vec<Option<Value>>,
    natives: Vec<Option<NativeFn>>,
    /// Per function, the typed entry of its native (decision AU1), which
    /// a call takes first when the function is pure.
    typed: Vec<Option<Typed>>,
    /// Per function, whether it is a native of this build that needs no
    /// capability: a call to it runs on the flat path (decision AU1).
    pure: Vec<bool>,
    /// Per function, its foreign binding once made (decision AF1).
    pub(crate) foreign: Vec<Option<natives::foreign::Bound>>,
    /// The libraries loaded for foreign modules, by the list of names that
    /// found them; a library is never unloaded while the VM lives.
    pub(crate) libraries: Vec<(Vec<String>, libloading::Library)>,
    /// The worker of the Python bridge, started at the first call into a
    /// Python module (decision AL2) and ended with the VM.
    pub(crate) python: Option<natives::python::Worker>,
    pub stdout: Box<dyn Write>,
    pub stderr: Box<dyn Write>,
    pub stdin: Box<dyn BufRead>,
    pub arguments: Vec<String>,
    /// The effective grant of the run, set by `begin_run`.
    pub grant: SharedGrant,
    /// Every grant a frame has run under, the run's own first: a frame
    /// names its grant by index, so that a call and a return move no
    /// reference count.
    grants: Vec<SharedGrant>,
    /// Per function, the enclosing grant its `needs` last narrowed and the
    /// result, both as indices into `grants`, so that a function called
    /// again from the same context does not intersect again.
    frame_grants: Vec<Option<(u32, u32)>>,
    /// Per code object, whether it belongs to a function whose `needs`
    /// narrow the grant; the other code objects run under their caller's.
    narrows: Vec<bool>,
    /// Per type, the first slot of its variants in `unit_variants` and
    /// how many it has.
    unit_base: Vec<(usize, usize)>,
    /// A variant without fields, once built: the same value every time
    /// (decision AG6).
    unit_variants: Vec<Option<Value>>,
    narrowing: Narrowing,
    counters: Vec<Counter>,
    /// Whether `begin_run` starts a recording.
    pub record: bool,
    revision: Option<String>,
    recording: Option<Recording>,
    pending_replay: Option<Recording>,
    replay: Option<Replay>,
    /// When the run began, in milliseconds since the epoch.
    started: i64,
    pub explain: bool,
    /// See `Options::serve_limit`.
    pub serve_limit: Option<usize>,
    redact: Vec<String>,
    manifest: Manifest,
    replay_output: bool,
    /// The context type of the next library call (`Op::ResultType`).
    pub(crate) expected: Option<Ty>,
    pub random_state: u64,
    /// The guards of the run (decision P3); the position of a guard is the
    /// origin bit a value carries.
    guards: Vec<Guard>,
    /// While a primitive runs: the origins of its arguments, which a
    /// function it calls back receives on its own arguments, and the
    /// origins of what the callbacks returned, which join the primitive's
    /// result.
    ambient: u64,
    gathered: u64,
    /// The machine code of the program's code objects (decision AG1);
    /// `None` on an interpreted run.
    pub(crate) native: Option<Box<Jit>>,
    /// What the generated code reads and writes through the VM pointer
    /// (decision AS1); the JIT's tables, when there is one.
    pub(crate) native_state: NativeState,
    /// Per code object, how many of its ops the interpreter has run: what
    /// makes it hot enough to compile (`native::HOT_FACTOR`).
    pub(crate) hotness: Vec<u32>,
    /// The interrupt a runtime helper of the generated code raised, on its
    /// way out through the generated frames.
    pub(crate) pending: Option<Interrupt>,
    /// See `Options::watch`.
    pub watch: Option<Box<dyn FnMut() -> bool>>,
    /// See `Options::listener`: the socket handed to this run, or the one
    /// `server.serve` leaves for the next.
    pub listener: Option<TcpListener>,
    /// See `Options::memory`.
    memory: Option<u64>,
    /// Whether this VM's budget is the one `memory` counts against.
    counting: bool,
}

impl Drop for Vm<'_> {
    fn drop(&mut self) {
        if self.counting {
            memory::end();
        }
    }
}

/// How the generated code calls a function (decision AU1): a declared
/// function or a primitive through `rt_call`; a pure primitive (no
/// `needs`, a native of this build) through the flat path; one with a
/// typed entry through `rt_call_typed`, its arguments borrowed and its
/// result in a register.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallKind {
    General,
    Pure,
    Typed(TypedKind),
}

/// Per function, the native of this build and its typed entry.
type Entries = Vec<Option<(NativeFn, Option<Typed>)>>;

fn entries_of(program: &Program, registry: &Registry) -> Entries {
    program
        .function_metas
        .iter()
        .map(|meta| {
            if meta.is_library {
                registry.lookup_entry(&meta.module, &meta.name, meta.receiver.as_deref())
            } else {
                None
            }
        })
        .collect()
}

/// Per function, whether it is a native of this build without `needs`.
fn pure_of(program: &Program, natives: &[Option<NativeFn>]) -> Vec<bool> {
    program
        .function_metas
        .iter()
        .zip(natives)
        .map(|(meta, native)| native.is_some() && meta.needs.is_empty())
        .collect()
}

fn kinds_of(pure: &[bool], typed: &[Option<Typed>]) -> Vec<CallKind> {
    pure.iter()
        .zip(typed)
        .map(|(pure, typed)| match (pure, typed) {
            (true, Some(typed)) => CallKind::Typed(typed.kind()),
            (true, None) => CallKind::Pure,
            (false, _) => CallKind::General,
        })
        .collect()
}

/// How the generated code calls every function of the program under this
/// registry (decision AU1): what `renyi build` compiles with.
pub fn call_kinds(program: &Program, registry: &Registry) -> Vec<CallKind> {
    let entries = entries_of(program, registry);
    let natives: Vec<Option<NativeFn>> = entries
        .iter()
        .map(|entry| entry.map(|(run, _)| run))
        .collect();
    let pure = pure_of(program, &natives);
    let typed: Vec<Option<Typed>> = entries
        .iter()
        .zip(&pure)
        .map(|(entry, pure)| {
            if *pure {
                entry.and_then(|(_, typed)| typed)
            } else {
                None
            }
        })
        .collect();
    kinds_of(&pure, &typed)
}

impl<'p> Vm<'p> {
    pub fn new(program: &'p Program, mut options: Options) -> Vm<'p> {
        let entries = entries_of(program, &options.registry);
        let natives: Vec<Option<NativeFn>> = entries
            .iter()
            .map(|entry| entry.map(|(run, _)| run))
            .collect();
        let pure = pure_of(program, &natives);
        let typed: Vec<Option<Typed>> = entries
            .iter()
            .zip(&pure)
            .map(|(entry, pure)| {
                if *pure {
                    entry.and_then(|(_, typed)| typed)
                } else {
                    None
                }
            })
            .collect();
        let calls = kinds_of(&pure, &typed);
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15)
            | 1;
        // a narrated, profiled or budgeted run is the interpreter's
        // (decisions AG3 and AP1)
        let mut native = if options.interpret
            || options.explain
            || options.profile
            || options.memory.is_some()
        {
            None
        } else {
            Jit::new(program, None, calls).map(Box::new)
        };
        if let (Some(jit), Some(image)) = (native.as_mut(), options.image.as_ref()) {
            if let Err(message) = jit.load_image(image) {
                let _ = writeln!(options.stderr, "renyi: the image was not loaded: {message}");
            }
        }
        let native_state = native
            .as_ref()
            .map(|jit| jit.state_pointers())
            .unwrap_or_default();
        let mut unit_base = Vec::with_capacity(program.types.metas.len());
        let mut units = 0;
        for meta in &program.types.metas {
            let count = match &meta.shape {
                TypeShape::Sum(variants) => variants.len(),
                _ => 0,
            };
            unit_base.push((units, count));
            units += count;
        }
        let mut vm = Vm {
            program,
            stack: Pinned::new(),
            frames: Pinned::new(),
            handlers: Pinned::new(),
            field_cache: vec![EMPTY_SITE; program.field_sites].into(),
            profile: options.profile.then(Profile::start),
            scratch: Vec::with_capacity(8),
            globals: vec![None; program.constants.len()],
            natives,
            typed,
            pure,
            foreign: vec![None; program.function_metas.len()],
            libraries: Vec::new(),
            python: None,
            stdout: options.stdout,
            stderr: options.stderr,
            stdin: options.stdin,
            arguments: options.arguments,
            grant: Rc::new(Vec::new()),
            grants: vec![Rc::new(Vec::new())],
            frame_grants: vec![None; program.function_metas.len()],
            narrows: program
                .codes
                .iter()
                .map(|code| {
                    code.function
                        .is_some_and(|function| !program.function_metas[function].needs.is_empty())
                })
                .collect(),
            unit_base,
            unit_variants: vec![None; units],
            narrowing: options.narrowing,
            counters: Vec::new(),
            record: options.record,
            revision: options.revision,
            recording: None,
            pending_replay: options.replay,
            replay: None,
            started: natives::now_millis(),
            explain: options.explain,
            serve_limit: options.serve_limit,
            redact: options.redact,
            manifest: options.manifest,
            replay_output: options.replay_output,
            expected: None,
            random_state: seed,
            guards: Vec::new(),
            ambient: 0,
            gathered: 0,
            native,
            native_state,
            hotness: vec![0; program.codes.len()],
            pending: None,
            watch: options.watch,
            listener: options.listener,
            memory: options.memory,
            counting: false,
        };
        // the template tier's code adds to the counts in place (decision
        // AU18); the vector is never resized after this
        vm.native_state.hotness = vm.hotness.as_mut_ptr();
        vm
    }

    // ------------------------------------------------------ grant and record

    /// Start a run under a declared grant: the effective grant with its
    /// budget counters, the clock, a fresh recording when recording, and
    /// the replay the options carried, checked against the grant.
    pub fn begin_run(&mut self, declared: &[Capability], program_name: &str) -> Result<(), String> {
        let grant = grant::effective(declared, &self.narrowing);
        self.grant = Rc::new(grant.capabilities);
        self.grants.clear();
        self.grants.push(self.grant.clone());
        self.frame_grants.iter_mut().for_each(|slot| *slot = None);
        self.counters = grant.counters;
        self.guards = grant::guards(declared);
        if let Some(sandbox) = &self.narrowing.sandbox {
            self.guards.extend(grant::guards(sandbox));
        }
        if self.guards.len() > 64 {
            return Err(format!(
                "the grant carries {} guarded capabilities (`only to`); the VM tracks at most 64 (open item R6-5)",
                self.guards.len()
            ));
        }
        self.started = natives::now_millis();
        if let Some(limit) = self.memory {
            if !memory::installed() {
                return Err("the memory budget needs the counting allocator: a binary declares `renyi::Allocator` as its global allocator".to_string());
            }
            if self.counting {
                memory::end();
            }
            if !memory::begin(limit) {
                return Err(
                    "a memory budget is in force already; one run at a time counts its memory"
                        .to_string(),
                );
            }
            self.counting = true;
        }
        self.replay = None;
        self.recording = self.record.then(|| {
            let mut recording = Recording::new(
                program_name,
                self.revision.clone(),
                instant_text(self.started),
                self.grant.iter().map(grant::spell).collect(),
            );
            recording.manifest = Manifest {
                arguments: self.arguments.clone(),
                ..self.manifest.clone()
            };
            recording
        });
        match self.pending_replay.take() {
            Some(recording) => self.replay_with(recording),
            None => Ok(()),
        }
    }

    /// Answer every effect of the run from a recording, once the grant the
    /// recording names and then each recorded call are checked against the
    /// grant (decision Y2).
    pub fn replay_with(&mut self, recording: Recording) -> Result<(), String> {
        for spelled in &recording.grant {
            let capability = grant::parse_capability(spelled)
                .map_err(|detail| format!("the recording's grant: {detail}"))?;
            if !effects::covered(&self.grant, &capability, true) {
                return Err(format!(
                    "the recording's grant names {}, which the grant {} does not cover",
                    capability.spelling(),
                    grant_text(&self.grant)
                ));
            }
        }
        for call in &recording.calls {
            let capability = grant::parse_capability(&call.capability)
                .map_err(|detail| format!("the recording's call #{}: {detail}", call.sequence))?;
            if !effects::covered(&self.grant, &capability, true) {
                return Err(format!(
                    "the recording's call #{} uses {}, which the grant {} does not cover",
                    call.sequence,
                    call.capability,
                    grant_text(&self.grant)
                ));
            }
        }
        self.replay = Some(Replay::new(recording));
        Ok(())
    }

    /// End a replay: the recorded calls the run never reached, described.
    pub fn end_replay(&mut self) -> Vec<String> {
        match self.replay.take() {
            Some(replay) => replay
                .unused()
                .iter()
                .map(|call| format!("#{} {}", call.sequence, call.describe()))
                .collect(),
            None => Vec::new(),
        }
    }

    /// The recording of the run so far, ending it.
    pub fn take_recording(&mut self) -> Option<Recording> {
        self.recording.take()
    }

    /// The budget counters and the clock they count against, for a host
    /// that runs several calls as one run (decision AP1).
    pub fn budgets(&self) -> (Vec<Counter>, i64) {
        (self.counters.clone(), self.started)
    }

    /// Continue the budgets of an earlier run under the same grant: its
    /// counters and its clock.
    pub fn resume_budgets(&mut self, counters: Vec<Counter>, started: i64) {
        self.counters = counters;
        self.started = started;
    }

    /// Whether the memory budget is spent: the test alone, for a fast
    /// path that builds the interrupt elsewhere (decision AU5).
    #[inline]
    pub(crate) fn memory_is_over(&self) -> bool {
        self.counting && memory::over()
    }

    /// The memory budget at a safe point (decision AP1): exceeded since
    /// the run began, it stops the run.
    #[inline]
    pub(crate) fn memory_check(&self) -> Result<(), Interrupt> {
        if self.counting && memory::over() {
            return Err(Interrupt::OverMemory {
                limit: self.memory.unwrap_or(0),
                used: memory::peak(),
            });
        }
        Ok(())
    }

    /// The grant in force where the next primitive call happens: the
    /// innermost frame's, or the run's before any frame.
    pub fn effective_grant(&self) -> &[Capability] {
        match self.frames.last() {
            Some(frame) => self.grants[frame.grant as usize].as_slice(),
            None => self.grant.as_slice(),
        }
    }

    /// The grant of a new frame, as its index in `grants`: the enclosing
    /// one, narrowed by the `needs` of the function the code belongs to
    /// (decision Q1), remembered per function while the enclosing grant
    /// stays the same; a code object that narrows nothing takes its
    /// caller's index.
    #[inline]
    pub(crate) fn frame_grant(&mut self, code: CodeId) -> u32 {
        let enclosing = match self.frames.last() {
            Some(frame) => frame.grant,
            None => 0,
        };
        if !self.narrows[code] {
            return enclosing;
        }
        let program = self.program;
        let function = program.codes[code]
            .function
            .expect("a code object that narrows the grant belongs to a function");
        if let Some((parent, narrowed)) = self.frame_grants[function] {
            if parent == enclosing {
                return narrowed;
            }
        }
        let needs = &program.function_metas[function].needs;
        let narrowed = grant::within(&self.grants[enclosing as usize], needs);
        let index = match self.grants.iter().position(|known| **known == narrowed) {
            Some(index) => index,
            None => {
                self.grants.push(Rc::new(narrowed));
                self.grants.len() - 1
            }
        } as u32;
        self.frame_grants[function] = Some((enclosing, index));
        index
    }

    /// The innermost running function whose `needs` narrowed the grant,
    /// named in a denial.
    fn grant_owner(&self) -> Option<String> {
        let program = self.program;
        self.frames.iter().rev().find_map(|frame| {
            let function = program.codes[frame.code].function?;
            (!program.function_metas[function].needs.is_empty()).then(|| self.label(function))
        })
    }

    fn unavailable(&self, function: FunctionId) -> Interrupt {
        Interrupt::crash(format!(
            "`{}` is not available in this build of the VM",
            self.qualified(function)
        ))
    }

    /// A primitive with the arguments on top of the stack, through the
    /// boundary: the arguments move into the scratch buffer, which a
    /// primitive calling back leaves to the nested call, and the result
    /// comes back to the caller.
    pub(crate) fn call_scratch(
        &mut self,
        function: FunctionId,
        count: usize,
    ) -> Result<Value, Interrupt> {
        let mut args = std::mem::take(&mut self.scratch);
        let at = self.stack.len().saturating_sub(count);
        self.stack.drain_into(at, &mut args);
        let result = self.call_native(function, &mut args);
        args.clear();
        self.scratch = args;
        result
    }

    /// A pure primitive (no `needs`) with the arguments on top of the
    /// stack, on the flat path of decision AU1: none of the boundary's
    /// checks applies to it, so the arguments move to the scratch buffer,
    /// the native runs on them plain, and its result carries their
    /// origins; what `call_primitive` does for such a call, without the
    /// layers.
    pub(crate) fn call_pure_from_stack(
        &mut self,
        function: FunctionId,
        count: usize,
    ) -> Result<Value, Interrupt> {
        let Some(native) = self.natives.get(function).copied().flatten() else {
            return self.call_scratch(function, count);
        };
        self.memory_check()?;
        let mut args = std::mem::take(&mut self.scratch);
        let at = self.stack.len().saturating_sub(count);
        self.stack.drain_into(at, &mut args);
        let origins = plain_in_place(&mut args);
        let result = self.run_native(native, origins, &mut args);
        args.clear();
        self.scratch = args;
        if let Some(profile) = &mut self.profile {
            profile.primitive(function);
        }
        result
    }

    /// A primitive's typed entry (decision AU1) on the `count` arguments
    /// where they lie on top of the stack: `Some` with the answer, the
    /// arguments popped, when the entry answers; `None`, the stack as it
    /// was, when it declines (a guarded argument, a big Integer, a case
    /// the native fails or crashes on) and the native itself must run.
    pub(crate) fn call_typed_in_place(
        &mut self,
        function: FunctionId,
        typed: Typed,
        count: usize,
    ) -> Option<Result<Value, Interrupt>> {
        let at = self.stack.len().saturating_sub(count);
        let args = &self.stack.as_slice()[at..];
        let value = match typed {
            Typed::Bool(answer) => answer(args).map(Value::Boolean),
            Typed::Int(answer) => answer(args).map(Value::integer),
            Typed::Float(answer) => answer(args).map(Value::Float),
            Typed::Value(answer) => answer(args),
        }?;
        self.stack.truncate(at);
        if let Some(profile) = &mut self.profile {
            profile.primitive(function);
        }
        Some(self.memory_check().map(|()| value))
    }

    /// The typed entry of a pure function's native, if any (decision AU1).
    pub(crate) fn typed_of(&self, function: FunctionId) -> Option<Typed> {
        self.typed.get(function).copied().flatten()
    }

    /// A library primitive, counted and sampled when profiling.
    pub(crate) fn call_native(
        &mut self,
        function: FunctionId,
        args: &mut [Value],
    ) -> Result<Value, Interrupt> {
        if self.profile.is_none() {
            return self.call_primitive(function, args);
        }
        let result = self.call_primitive(function, args);
        if let Some(profile) = &mut self.profile {
            profile.primitive(function);
        }
        result
    }

    /// A library primitive. A pure one runs. One under a capability is
    /// checked against the grant, the guards and the budgets, then runs
    /// live and is recorded, or is answered from the replay: this is the
    /// boundary of `06-runtime-guarantees.md` section 4. A primitive sees
    /// plain values; its result carries the origins of its arguments and,
    /// under a guarded capability, the guard's own origin (section 3).
    fn call_primitive(
        &mut self,
        function: FunctionId,
        args: &mut [Value],
    ) -> Result<Value, Interrupt> {
        self.memory_check()?;
        let program = self.program;
        let native = self.natives.get(function).copied().flatten();
        let meta = &program.function_metas[function];
        let origins = plain_in_place(args);
        let Some(effect) = grant::effect_of(meta, args) else {
            return self.run_primitive(function, native, origins, args);
        };
        if !effects::covered(self.effective_grant(), &effect, true) {
            self.expected = None;
            return self.denied(function, &effect, args);
        }
        // `filesystem.copy` and `move` write their target too
        if let Some(target) = grant::target_effect_of(meta, args) {
            if !effects::covered(self.effective_grant(), &target, true) {
                self.expected = None;
                return self.denied(function, &target, args);
            }
        }
        if let Some(guard) = self.blocking_guard(origins, &effect) {
            self.expected = None;
            return self.refused(function, guard, &effect);
        }
        let origins = origins | self.origins_through(&effect);
        let arguments = self.encode_arguments(function, args)?;
        if self.replay.is_some() {
            // the context type decodes the recorded result
            let expected = self.expected.take();
            let value = self.replay_call(function, &effect, arguments, args, expected)?;
            // `renyi reproduce` shows the console output of the replayed run
            if self.replay_output && meta.module == "std.console" && meta.name.starts_with("print")
            {
                if let Some(native) = native {
                    native(self, args)?;
                }
            }
            return Ok(value.guarded(origins));
        }
        let at_ms = natives::now_millis() - self.started;
        if let Some(budget) = self.exhausted(&effect, at_ms) {
            self.expected = None;
            return self.over_budget(function, &effect, &budget, args);
        }
        let shown_args = self.explain.then(|| args.to_vec());
        // a live primitive takes the context type itself (`sqlite.query`)
        let result = self.run_primitive(function, native, origins, args);
        self.expected = None;
        let duration_ms = natives::now_millis() - self.started - at_ms;
        let value = match &result {
            Ok(value) => value.clone(),
            // an exit is recorded as a call that returned nothing
            Err(Interrupt::Exit(_)) => Value::Nothing,
            Err(_) => return result,
        };
        if let Some(shown_args) = shown_args {
            self.narrate_effect(&effect, function, &shown_args, &value, Some(duration_ms));
        }
        if self.recording.is_some() {
            let outcome = self.encode_outcome(&value)?;
            let primitive = self.qualified(function);
            let mut call = Call {
                sequence: 0,
                capability: effect.spelling(),
                primitive,
                arguments,
                outcome,
                duration_ms: Some(duration_ms),
                at_ms,
            };
            redact(&self.redact, &mut call);
            if let Some(recording) = &mut self.recording {
                recording.push(call);
            }
        }
        result
    }

    /// Run a primitive on plain arguments. A function it calls back
    /// receives the arguments' origins on its own arguments, and the result
    /// carries those origins together with the origins of what the
    /// callbacks returned.
    /// A primitive: a native of this build, a foreign function bound
    /// through its library (decision AF1) or a function of a Python module
    /// run through the bridge (decision AL2); a function with none of them
    /// is not available.
    fn run_primitive(
        &mut self,
        function: FunctionId,
        native: Option<NativeFn>,
        origins: u64,
        args: &mut [Value],
    ) -> Result<Value, Interrupt> {
        if let Some(native) = native {
            return self.run_native(native, origins, args);
        }
        if self.program.function_metas[function].foreign.is_some() {
            return natives::foreign::run(self, function, origins, args);
        }
        if self.program.function_metas[function].python.is_some() {
            return natives::python::run(self, function, origins, args);
        }
        Err(self.unavailable(function))
    }

    fn run_native(
        &mut self,
        native: NativeFn,
        origins: u64,
        args: &mut [Value],
    ) -> Result<Value, Interrupt> {
        let (ambient, gathered) = (self.ambient, self.gathered);
        self.ambient = origins;
        self.gathered = 0;
        let result = native(self, args);
        let collected = self.gathered;
        self.ambient = ambient;
        self.gathered = gathered;
        Ok(result?.guarded(origins | collected))
    }

    /// The guard that stops a value with these origins from reaching the
    /// effect: the first whose sinks do not cover it.
    fn blocking_guard(&self, origins: u64, effect: &Capability) -> Option<usize> {
        if origins == 0 {
            return None;
        }
        (0..self.guards.len())
            .find(|&index| origins & (1u64 << index) != 0 && !self.guards[index].admits(effect))
    }

    /// The origin bits of what enters through the effect: every guard whose
    /// capability covers it.
    fn origins_through(&self, effect: &Capability) -> u64 {
        self.guards
            .iter()
            .enumerate()
            .filter(|(_, guard)| guard.capability.covers(effect, true))
            .fold(0, |bits, (index, _)| bits | (1u64 << index))
    }

    /// Why a value with these origins may not reach the effect (decision
    /// P3), when a guard stops it: what would leave, where it came from,
    /// where it was going and where it may go.
    pub fn refusal(&self, origins: u64, effect: &Capability, what: &str) -> Option<String> {
        let guard = &self.guards[self.blocking_guard(origins, effect)?];
        let sinks: Vec<String> = guard.sinks.iter().map(Capability::spelling).collect();
        Some(format!(
            "{what} would send a value from {} to {}; the guard allows only {}",
            guard.capability.spelling(),
            effect.spelling(),
            sinks.join(" or ")
        ))
    }

    /// A call that would send a guarded value past its sinks: the `Guarded`
    /// error of the prelude when the primitive can fail, else a crash.
    fn refused(
        &mut self,
        function: FunctionId,
        guard: usize,
        effect: &Capability,
    ) -> Result<Value, Interrupt> {
        let what = format!("`{}`", self.qualified(function));
        let message = self
            .refusal(1u64 << guard, effect, &what)
            .unwrap_or_default();
        if self.program.function_metas[function].fails.is_empty() {
            return Err(Interrupt::crash(message));
        }
        let origin = self.guards[guard].capability.spelling();
        Ok(Value::failure(Value::record(
            self.program.builtins.guarded,
            vec![Value::text(origin), Value::text(effect.spelling())],
        )))
    }

    /// Text that leaves the program through the console outside a
    /// primitive call: a crash message, the failure of `main` or of a test.
    /// Refused when a guard on its origins does not list `console`.
    pub fn text_for_console(&mut self, value: &Value, what: &str) -> Result<String, Interrupt> {
        if let Some(message) = self.refusal(value.origins(), &console_capability(), what) {
            return Err(Interrupt::crash(message));
        }
        self.to_text(value)
    }

    /// Whether the budgets covering an effect admit a call at `now`; the
    /// call is counted when they do, and the exhausted budget is named
    /// when they do not.
    fn exhausted(&mut self, effect: &Capability, now: i64) -> Option<String> {
        let mut covering = Vec::new();
        for (index, counter) in self.counters.iter_mut().enumerate() {
            if counter.capability.covers(effect, true) {
                if !counter.fits(now) {
                    return Some(counter.spelling());
                }
                covering.push(index);
            }
        }
        for index in covering {
            self.counters[index].note(now);
        }
        None
    }

    /// The variant of a function's declared failure type that reports a
    /// refusal of the boundary, for the library's modules and an
    /// extension's alike (decision AK4): `PermissionDenied`,
    /// `HostNotAllowed` or `ProgramNotAllowed` for a denied call,
    /// `OverBudget` for a call past a budget, each with one field, the
    /// scope of the effect, or the first argument when the capability
    /// carries no scope (`serve` names its port). `None` when the
    /// function cannot fail or no failure type has such a variant: the
    /// call crashes then.
    fn boundary_failure(
        &self,
        function: FunctionId,
        names: &[&str],
        effect: &Capability,
        args: &[Value],
    ) -> Option<Value> {
        let meta = &self.program.function_metas[function];
        let field = match &effect.scope {
            Some(scope) => Value::text(scope.clone()),
            None => args.first().cloned().unwrap_or(Value::Nothing),
        };
        for ty in &meta.fails {
            let Ty::App(id, _) = ty else {
                continue;
            };
            let TypeShape::Sum(variants) = &self.program.types.meta(*id).shape else {
                continue;
            };
            for name in names {
                let found = variants
                    .iter()
                    .position(|variant| variant.name == *name && variant.fields.len() == 1);
                if let Some(tag) = found {
                    return Some(Value::failure(Value::variant(*id, tag, vec![field])));
                }
            }
        }
        None
    }

    /// A call outside the grant: the module's error when the primitive can
    /// fail (decision J11), else a crash naming the function whose `needs`
    /// narrowed the grant (decision Y1).
    fn denied(
        &mut self,
        function: FunctionId,
        effect: &Capability,
        args: &[Value],
    ) -> Result<Value, Interrupt> {
        let names = ["PermissionDenied", "HostNotAllowed", "ProgramNotAllowed"];
        match self.boundary_failure(function, &names, effect, args) {
            Some(failure) => Ok(failure),
            None => Err(Interrupt::crash(format!(
                "`{}` needs {}, which the grant {}{} does not allow",
                self.qualified(function),
                effect.spelling(),
                grant_text(self.effective_grant()),
                self.grant_owner()
                    .map(|owner| format!(" of `{owner}`"))
                    .unwrap_or_default()
            ))),
        }
    }

    /// A call past a budget (decision P2): the module's `OverBudget`, else
    /// a crash.
    fn over_budget(
        &mut self,
        function: FunctionId,
        effect: &Capability,
        budget: &str,
        args: &[Value],
    ) -> Result<Value, Interrupt> {
        match self.boundary_failure(function, &["OverBudget"], effect, args) {
            Some(failure) => Ok(failure),
            None => Err(Interrupt::crash(format!(
                "`{}` exceeds the budget `{budget}`",
                self.qualified(function)
            ))),
        }
    }

    fn encode_arguments(
        &mut self,
        function: FunctionId,
        args: &[Value],
    ) -> Result<Vec<(String, Json)>, Interrupt> {
        let program = self.program;
        let meta = &program.function_metas[function];
        let mut out = Vec::with_capacity(args.len());
        for (name, value) in meta.params.iter().zip(args) {
            let json = match value {
                Value::Function(id) => Json::Text(format!("function {}", self.qualified(*id))),
                Value::Native(native) => native_json(native),
                other => json::encode(self, other, Naming::Exact)?,
            };
            out.push((name.clone(), json));
        }
        Ok(out)
    }

    fn encode_outcome(&mut self, value: &Value) -> Result<Outcome, Interrupt> {
        Ok(match value {
            Value::Failure(error) => Outcome::Failure(json::encode(self, error, Naming::Exact)?),
            Value::Native(native) => Outcome::Success(native_json(native)),
            other => Outcome::Success(json::encode(self, other, Naming::Exact)?),
        })
    }

    /// The recorded answer to a call: the entry with the same primitive and
    /// arguments, its outcome decoded by the primitive's declared types.
    fn replay_call(
        &mut self,
        function: FunctionId,
        effect: &Capability,
        arguments: Vec<(String, Json)>,
        args: &[Value],
        expected: Option<Ty>,
    ) -> Result<Value, Interrupt> {
        let primitive = self.qualified(function);
        let replay = self.replay.as_mut().expect("a replay");
        let index = replay
            .take(&primitive, &arguments)
            .map_err(Interrupt::crash)?;
        let call = replay.recording.calls[index].clone();
        if let Some(budget) = self.exhausted(effect, call.at_ms) {
            return Err(Interrupt::crash(format!(
                "the recording exceeds the budget `{budget}` at call #{}",
                call.sequence
            )));
        }
        let value = match &call.outcome {
            Outcome::Success(_) if primitive == "std.environment.exit" => {
                let code = args.first().and_then(Value::as_i64).unwrap_or(0) as i32;
                return Err(Interrupt::Exit(code));
            }
            Outcome::Success(json) => self.decode_result(function, json, expected)?,
            Outcome::Failure(json) => Value::failure(self.decode_error(function, json)?),
        };
        if self.explain {
            self.narrate_effect(effect, function, args, &value, None);
        }
        Ok(value)
    }

    fn decode_result(
        &mut self,
        function: FunctionId,
        json: &Json,
        expected: Option<Ty>,
    ) -> Result<Value, Interrupt> {
        let program = self.program;
        // a connection has no file on a replay: a placeholder that keeps
        // the path, so that later calls match the recording
        if let Some(path) = connection_path(json) {
            return Ok(Value::Native(Rc::new(Native::Connection {
                connection: std::cell::RefCell::new(None),
                path,
            })));
        }
        let ty = expected
            .or_else(|| program.function_metas[function].returns.clone())
            .unwrap_or(Ty::Unit);
        match json::decode(self, json, &ty, "$", Naming::Exact)? {
            Ok(value) => Ok(value),
            Err(error) => {
                let shown = self.to_text(&error)?;
                Err(Interrupt::crash(format!(
                    "the recorded result of `{}` does not fit its type: {shown}",
                    self.qualified(function)
                )))
            }
        }
    }

    fn decode_error(&mut self, function: FunctionId, json: &Json) -> Result<Value, Interrupt> {
        let program = self.program;
        let mut reasons = Vec::new();
        for ty in &program.function_metas[function].fails {
            match json::decode(self, json, ty, "$", Naming::Exact)? {
                Ok(value) => return Ok(value),
                Err(error) => reasons.push(self.to_text(&error)?),
            }
        }
        Err(Interrupt::crash(format!(
            "the recorded failure of `{}` fits none of its error types: {}",
            self.qualified(function),
            reasons.join("; ")
        )))
    }

    // ------------------------------------------------------------ narration

    /// How many declared functions are running: the indentation of a
    /// narrated line.
    fn function_depth(&self) -> usize {
        let program = self.program;
        self.frames
            .iter()
            .filter(|frame| program.codes[frame.code].kind == CodeKind::Function)
            .count()
    }

    fn narrate(&mut self, depth: usize, line: &str) {
        let _ = writeln!(self.stderr, "{}{line}", "  ".repeat(depth));
    }

    fn label(&self, function: FunctionId) -> String {
        if self.program.main == Some(function) {
            "main".to_string()
        } else {
            self.qualified(function)
        }
    }

    fn shown(&mut self, value: &Value) -> String {
        let text = self.render(value, true).unwrap_or_else(|_| "?".to_string());
        clip(&text, 100)
    }

    /// `Purpose. (module.name, param: value)` on entering a declared
    /// function; a function without a purpose is narrated by its name.
    fn narrate_entry(&mut self, code: CodeId, base: usize) {
        let program = self.program;
        let code = &program.codes[code];
        let (CodeKind::Function, Some(id)) = (code.kind, code.function) else {
            return;
        };
        let meta = &program.function_metas[id];
        let depth = self.function_depth() - 1;
        let head = meta.purpose.clone().unwrap_or_else(|| meta.name.clone());
        let mut line = format!("{head} ({}", self.label(id));
        let args = self.stack[base..base + code.params as usize].to_vec();
        for (name, value) in meta.params.iter().zip(&args) {
            let shown = self.shown(value);
            line.push_str(&format!(", {name}: {shown}"));
        }
        line.push(')');
        self.narrate(depth, &line);
    }

    /// `-> value` on leaving a declared function.
    fn narrate_exit(&mut self, code: CodeId, value: &Value) {
        let program = self.program;
        let code = &program.codes[code];
        if code.kind != CodeKind::Function || code.function.is_none() {
            return;
        }
        let line = match value {
            Value::Nothing => return,
            Value::Failure(error) => format!("-> failed with {}", self.shown(error)),
            other => format!("-> {}", self.shown(other)),
        };
        let depth = self.function_depth() + 1;
        self.narrate(depth, &line);
    }

    /// One effect: `console "text"`, or the capability, the primitive with
    /// its arguments, and the result.
    fn narrate_effect(
        &mut self,
        effect: &Capability,
        function: FunctionId,
        args: &[Value],
        result: &Value,
        duration_ms: Option<i64>,
    ) {
        let program = self.program;
        let meta = &program.function_metas[function];
        let depth = self.function_depth();
        let mut line = match (meta.module.as_str(), meta.name.as_str()) {
            ("std.console", "print" | "print_error") => {
                let text = args.first().map(|arg| self.shown(arg)).unwrap_or_default();
                format!("console {text}")
            }
            _ => {
                let mut line = format!("{} {}(", effect.path.join("."), meta.name);
                for (index, (name, value)) in meta.params.iter().zip(args).enumerate() {
                    if index > 0 {
                        line.push_str(", ");
                    }
                    let shown = self.shown(value);
                    line.push_str(&format!("{name}: {shown}"));
                }
                line.push(')');
                line
            }
        };
        match result {
            Value::Nothing => {}
            Value::Failure(error) => {
                let shown = self.shown(error);
                line.push_str(&format!(" -> failed with {shown}"));
            }
            other => {
                let shown = self.shown(other);
                line.push_str(&format!(" -> {shown}"));
            }
        }
        if let Some(ms) = duration_ms {
            if ms > 0 {
                line.push_str(&format!(", {ms} ms"));
            }
        }
        self.narrate(depth, &line);
    }

    /// The context type recorded for the library call now running.
    pub fn take_expected(&mut self) -> Option<Ty> {
        self.expected.take()
    }

    /// The profiler's report on the standard error, when the run was
    /// profiled (decision X4).
    pub fn report_profile(&mut self) {
        if let Some(profile) = self.profile.take() {
            let report = profile.render(self.program);
            let _ = self.stderr.write_all(report.as_bytes());
        }
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
    /// `Failure`. Called back from a primitive, the function receives the
    /// origins of the primitive's arguments, and what it returns joins the
    /// primitive's result (decision P3).
    pub fn call_function(&mut self, id: FunctionId, args: Vec<Value>) -> Result<Value, Interrupt> {
        let ambient = self.ambient;
        let mut args: Vec<Value> = if ambient == 0 {
            args
        } else {
            args.into_iter().map(|arg| arg.guarded(ambient)).collect()
        };
        let result = match self.program.function_codes.get(id).copied().flatten() {
            Some(code) => self.call_code(code, args)?,
            None => self.call_native(id, &mut args)?,
        };
        self.gathered |= result.origins();
        Ok(result)
    }

    /// Run a code object to its end; the result may be a `Failure`.
    pub fn call_code(&mut self, code: CodeId, args: Vec<Value>) -> Result<Value, Interrupt> {
        self.push_frame(code, args);
        let depth = self.frames.len();
        self.run_top_frame(depth)
    }

    pub fn qualified(&self, id: FunctionId) -> String {
        let meta = &self.program.function_metas[id];
        format!("{}.{}", meta.module, meta.name)
    }

    fn push_frame(&mut self, code: CodeId, args: Vec<Value>) {
        let count = args.len();
        self.stack.extend(args);
        self.push_frame_in_place(code, count);
    }

    /// Enter a code object whose `count` arguments are the top of the
    /// stack: they stay there as its first locals (decision X3).
    #[inline]
    pub(crate) fn push_frame_in_place(&mut self, code: CodeId, count: usize) {
        let grant = self.frame_grant(code);
        let base = self
            .stack
            .len()
            .checked_sub(count)
            .expect("the arguments are on the stack");
        let locals = self.program.codes[code].locals as usize;
        if self.stack.len() < base + locals {
            self.stack.resize(base + locals, Value::Nothing);
        }
        self.frames.push(Frame {
            code,
            pc: 0,
            base,
            handler_base: self.handlers.len(),
            grant,
        });
        if let Some(profile) = &mut self.profile {
            profile.call(code);
        }
        if self.explain {
            self.narrate_entry(code, base);
        }
    }

    /// Run until the frame at `entry` (and every frame above it) has
    /// returned. An interrupt abandons the entry frame and everything
    /// above it; a crash gets its location from the frame that was
    /// running.
    pub(crate) fn execute(&mut self, entry: usize) -> Result<Value, Interrupt> {
        match self.run_frames(entry) {
            Ok(value) => Ok(value),
            Err(interrupt) => Err(self.abandon(entry, interrupt)),
        }
    }

    /// Abandon the entry frame and everything above it after an
    /// interrupt, which gets its location from the frame that was running.
    pub(crate) fn abandon(&mut self, entry: usize, interrupt: Interrupt) -> Interrupt {
        let interrupt = match interrupt {
            Interrupt::Crash {
                message,
                location: None,
            } => {
                let location = self.frames.last().and_then(|frame| {
                    let code = &self.program.codes[frame.code];
                    let span = code.spans.get(frame.pc.saturating_sub(1))?;
                    Some(self.program.location(code.module, *span))
                });
                Interrupt::Crash { message, location }
            }
            other => other,
        };
        if let Some(frame) = self.frames.get(entry.saturating_sub(1)) {
            self.stack.truncate(frame.base);
            self.handlers.truncate(frame.handler_base);
        }
        self.frames.truncate(entry.saturating_sub(1));
        interrupt
    }

    /// Run the frame on top to its end: on its machine code when it has
    /// some (decision AG1), else on the interpreter, which also takes over
    /// a frame the machine code hands back.
    pub(crate) fn run_top_frame(&mut self, entry: usize) -> Result<Value, Interrupt> {
        let function = self.native_entry_of_top();
        if function.is_none() {
            if let Some(jit) = self.native.as_mut() {
                jit.calls_cold += 1;
            }
        }
        if let Some(function) = function {
            let base = self.frames[entry - 1].base;
            match self.run_generated(function, base, 0) {
                native::runtime::RETURNED => return Ok(self.pop()),
                native::runtime::DEOPT | native::runtime::STAY => {}
                _ => {
                    let interrupt = self.take_pending();
                    return Err(self.abandon(entry, interrupt));
                }
            }
        }
        self.execute(entry)
    }

    /// The generated function of the frame on top, when the run uses
    /// machine code, the code has some and the machine stack has room.
    #[inline]
    pub(crate) fn native_entry_of_top(&mut self) -> Option<native::Entry> {
        if self.native_state.depth >= native::DEPTH_LIMIT {
            return None;
        }
        let code = self.frames.last()?.code;
        let program = self.program;
        let hotness = self.hotness[code];
        self.native.as_mut()?.entry(program, code, hotness)
    }

    /// The generated function of the frame on top to enter at the loop
    /// header `pc` (decision AG3), when the code is hot and has the entry.
    #[inline]
    pub(crate) fn native_resume_of_top(&mut self, pc: u32) -> Option<native::Entry> {
        if self.native_state.depth >= native::DEPTH_LIMIT {
            return None;
        }
        let code = self.frames.last()?.code;
        let program = self.program;
        let hotness = self.hotness[code];
        self.native.as_mut()?.resume(program, code, pc, hotness)
    }

    /// Call a generated function on the frame on top, at its start or at
    /// the loop header `pc`.
    #[inline]
    pub(crate) fn run_generated(&mut self, function: native::Entry, base: usize, pc: u32) -> i32 {
        self.native_state.depth += 1;
        // SAFETY: the generated function takes the VM it was made for, the
        // base of the frame on top, which `push_frame_in_place` made, and
        // one of the entry pcs it was made with.
        let vm: *mut Vm<'_> = self;
        let status = unsafe { function(vm.cast::<Vm<'static>>(), base, pc) };
        self.native_state.depth -= 1;
        status
    }

    pub(crate) fn take_pending(&mut self) -> Interrupt {
        self.pending
            .take()
            .unwrap_or_else(|| Interrupt::crash("the generated code left without a reason"))
    }

    // ------------------------------------------------------------ the stack

    pub(crate) fn pop(&mut self) -> Value {
        self.stack.pop().unwrap_or(Value::Nothing)
    }

    pub(crate) fn pop_n(&mut self, count: usize) -> Vec<Value> {
        let at = self.stack.len().saturating_sub(count);
        self.stack.split_off(at)
    }

    /// A Boolean never carries origins (`Value::guarded`), so no wrapper
    /// needs stripping here.
    pub(crate) fn pop_bool(&mut self) -> Result<bool, Interrupt> {
        match self.stack.pop() {
            Some(Value::Boolean(value)) => Ok(value),
            other => Err(Interrupt::crash(not_boolean(other))),
        }
    }

    /// A call result or a constructed value lands on the stack, or, when it
    /// is a `Failure`, in the innermost handled region of the running
    /// frame; without one the frame's own result is the failure (examples,
    /// tests) or the program crashes.
    fn settle(&mut self, value: Value, entry: usize) -> Result<Flow, Interrupt> {
        if !value.is_failure() {
            self.stack.push(value);
            return Ok(Flow::Go);
        }
        let frame = self.frames.last_mut().expect("a frame");
        if self.handlers.len() > frame.handler_base {
            let (target, height) = self.handlers.pop().expect("a handler");
            self.stack.truncate(height);
            self.stack.push(value);
            frame.pc = target;
            return Ok(Flow::Reload);
        }
        let code = &self.program.codes[frame.code];
        match code.kind {
            CodeKind::Example | CodeKind::Test | CodeKind::Refinement | CodeKind::Constant => {
                self.leave(value, entry)
            }
            CodeKind::Function => {
                // nothing in the body handles it: the function's own failure
                // when it declares failures (an expired `within` deadline,
                // decision J12, whose `TimedOut` the checker made it
                // declare), else a crash
                let declares = code
                    .function
                    .is_some_and(|id| !self.program.function_metas[id].fails.is_empty());
                if declares {
                    return self.leave(value, entry);
                }
                let error = match &value {
                    Value::Failure(error) => (**error).clone(),
                    _ => Value::Nothing,
                };
                let shown = self.text_for_console(&error, "an unhandled failure")?;
                Err(Interrupt::crash(format!("unhandled failure: {shown}")))
            }
        }
    }

    /// Pop the running frame with its result: the value `execute` returns
    /// when the frame was its entry, else the caller's operand, settled
    /// there (a `Failure` looks for the caller's handler).
    fn leave(&mut self, value: Value, entry: usize) -> Result<Flow, Interrupt> {
        let frame = self.frames.pop().expect("a frame");
        self.stack.truncate(frame.base);
        self.handlers.truncate(frame.handler_base);
        if self.explain {
            self.narrate_exit(frame.code, &value);
        }
        if self.frames.len() < entry {
            return Ok(Flow::Done(value));
        }
        match self.settle(value, entry)? {
            Flow::Go => Ok(Flow::Reload),
            other => Ok(other),
        }
    }

    // ------------------------------------------------------------ the loop

    /// The interpreter loop (decision X3). The running frame's code, pc and
    /// base live in locals: a call writes the pc back and loads the
    /// callee's state, a return or a handler reloads the frame on top, and
    /// an interrupt writes the pc back so that the crash is located.
    fn run_frames(&mut self, entry: usize) -> Result<Value, Interrupt> {
        let program = self.program;
        let mut frame = self.frames.len() - 1;
        let mut code_id = self.frames[frame].code;
        let mut code: &Code = &program.codes[code_id];
        let mut pc = self.frames[frame].pc;
        let mut base = self.frames[frame].base;
        // the ops run in the running frame since it became the running one,
        // added to the code's hotness when another frame takes over
        let mut ran: u32 = 0;
        // the frame on top becomes the running one
        macro_rules! reload {
            () => {{
                self.hotness[code_id] = self.hotness[code_id].saturating_add(ran);
                ran = 0;
                frame = self.frames.len() - 1;
                let running = &self.frames[frame];
                code_id = running.code;
                code = &program.codes[code_id];
                pc = running.pc;
                base = running.base;
            }};
        }
        // the pc back into the frame, when it still exists (a failure
        // settled after a return has left it)
        macro_rules! sync {
            () => {
                if let Some(running) = self.frames.get_mut(frame) {
                    running.pc = pc;
                }
            };
        }
        // an interrupt leaves with the pc written back
        macro_rules! try_op {
            ($result:expr) => {
                match $result {
                    Ok(value) => value,
                    Err(interrupt) => {
                        sync!();
                        return Err(interrupt);
                    }
                }
            };
        }
        macro_rules! crash {
            ($message:expr) => {{
                sync!();
                return Err(Interrupt::crash($message));
            }};
        }
        // what `settle` and `leave` say: carry on, reload, or return
        macro_rules! flow {
            ($flow:expr) => {
                match try_op!($flow) {
                    Flow::Go => {}
                    Flow::Reload => reload!(),
                    Flow::Done(value) => return Ok(value),
                }
            };
        }
        // a value lands on the stack; a failure looks for a handler
        macro_rules! settle {
            ($value:expr) => {{
                let value = $value;
                if value.is_failure() {
                    flow!(self.settle(value, entry));
                } else {
                    self.stack.push(value);
                }
            }};
        }
        // the running frame returns the value: to `execute` when the frame
        // was its entry, else onto its caller's stack
        macro_rules! leave {
            ($value:expr) => {{
                let value = $value;
                self.hotness[code_id] = self.hotness[code_id].saturating_add(ran);
                ran = 0;
                let left = self.frames.pop().expect("a frame");
                self.stack.truncate(left.base);
                self.handlers.truncate(left.handler_base);
                if self.explain {
                    self.narrate_exit(left.code, &value);
                }
                if self.frames.len() < entry {
                    return Ok(value);
                }
                if value.is_failure() {
                    match try_op!(self.settle(value, entry)) {
                        Flow::Done(value) => return Ok(value),
                        _ => reload!(),
                    }
                } else {
                    self.stack.push(value);
                    reload!();
                }
            }};
        }
        // a declared function gets a frame over the arguments in place; a
        // primitive runs in Rust and its result is settled
        macro_rules! call {
            ($function:expr, $count:expr) => {{
                try_op!(self.memory_check());
                if let Some(callee) = program.function_codes.get($function).copied().flatten() {
                    self.frames[frame].pc = pc;
                    self.push_frame_in_place(callee, $count);
                    // the callee's machine code runs it to its end, like a
                    // primitive would, unless it hands the frame back
                    match self.native_entry_of_top() {
                        Some(function) => {
                            let callee_base = self.frames[self.frames.len() - 1].base;
                            match self.run_generated(function, callee_base, 0) {
                                native::runtime::RETURNED => {
                                    let value = self.pop();
                                    settle!(value);
                                }
                                native::runtime::DEOPT | native::runtime::STAY => reload!(),
                                _ => return Err(self.take_pending()),
                            }
                        }
                        None => reload!(),
                    }
                } else if let Some(typed) = self.typed.get($function).copied().flatten() {
                    // a typed entry answers on the arguments where they lie
                    // (decision AU1), or declines
                    let result = match self.call_typed_in_place($function, typed, $count) {
                        Some(result) => result,
                        None => self.call_scratch($function, $count),
                    };
                    settle!(try_op!(result));
                } else if self.pure.get($function).copied().unwrap_or(false) {
                    let result = self.call_pure_from_stack($function, $count);
                    settle!(try_op!(result));
                } else {
                    let result = self.call_scratch($function, $count);
                    settle!(try_op!(result));
                }
            }};
        }
        loop {
            let Some(op) = code.ops.get(pc) else {
                leave!(Value::Nothing);
                continue;
            };
            pc += 1;
            ran += 1;
            if let Some(profile) = &mut self.profile {
                profile.op(code_id, op);
            }
            match op {
                Op::Const(index) => self.stack.push(code.constants[*index as usize].clone()),
                Op::Nothing => self.stack.push(Value::Nothing),
                Op::Global(index) => {
                    let value = try_op!(self.global(*index as usize));
                    self.stack.push(value);
                }
                Op::Load(slot) => {
                    let value = self.stack[base + *slot as usize].clone();
                    self.stack.push(value);
                }
                Op::LoadMove(slot) => {
                    let value =
                        std::mem::replace(&mut self.stack[base + *slot as usize], Value::Nothing);
                    self.stack.push(value);
                }
                Op::Store(slot) => {
                    let value = self.pop();
                    self.stack[base + *slot as usize] = value;
                }
                Op::Pop => {
                    self.pop();
                }
                Op::Dup => {
                    let top = self.stack.last().cloned().unwrap_or(Value::Nothing);
                    self.stack.push(top);
                }
                // every operation strips its operands' guard wrappers and tags
                // its result with the union of their origins (decision P3)
                Op::MakeList(count) => {
                    let (origins, items) = plain_all(self.pop_n(*count as usize));
                    self.stack.push(Value::list(items).guarded(origins));
                }
                Op::MakeMap(count) => self.op_make_map(*count as usize),
                Op::MakePair => {
                    let right = self.pop();
                    let left = self.pop();
                    let origins = left.origins() | right.origins();
                    self.stack
                        .push(Value::pair(left.into_plain(), right.into_plain()).guarded(origins));
                }
                Op::MakeRange { stepped } => {
                    let value = try_op!(self.op_make_range(*stepped));
                    self.stack.push(value);
                }
                Op::Construct { ty, fields } => {
                    let (origins, fields) = plain_all(self.pop_n(*fields as usize));
                    settle!(try_op!(self.construct(*ty, fields)).guarded(origins));
                }
                Op::ConstructVariant { ty, tag, fields } => {
                    let (origins, fields) = plain_all(self.pop_n(*fields as usize));
                    settle!(try_op!(self.construct_variant(*ty, *tag as usize, fields))
                        .guarded(origins));
                }
                Op::Field { name, site } => {
                    let holder = self.pop();
                    let value =
                        try_op!(self.op_field(code_id, *name as usize, *site as usize, &holder));
                    self.stack.push(value);
                }
                Op::LoadField { slot, name, site } => {
                    let value = try_op!(self.op_load_field(
                        code_id,
                        base + *slot as usize,
                        *name as usize,
                        *site as usize
                    ));
                    self.stack.push(value);
                }
                Op::TakeField { slot, name, site } => {
                    let value = try_op!(self.op_take_field(
                        code_id,
                        base + *slot as usize,
                        *name as usize,
                        *site as usize
                    ));
                    self.stack.push(value);
                }
                // a refined field is checked again: the copy may be a
                // `Failure(ConstraintViolation)`, like a construction
                Op::With(count) => settle!(try_op!(self.op_with(*count as usize))),
                Op::WithSlot { slot, fields } => {
                    settle!(try_op!(
                        self.op_with_slot(base + *slot as usize, *fields as usize)
                    ))
                }
                Op::Call { function, args } => call!(*function, *args as usize),
                Op::CallAbility {
                    ability,
                    method,
                    args,
                } => {
                    let count = *args as usize;
                    match self.ability_target(*ability, *method as usize, count) {
                        Some(function) => call!(function, count),
                        None => {
                            let args = self.pop_n(count);
                            settle!(try_op!(self.derived_ability(
                                *ability,
                                *method as usize,
                                args
                            )));
                        }
                    }
                }
                Op::CallValue(count) => {
                    let count = *count as usize;
                    let Some(at) = self.stack.len().checked_sub(count + 1) else {
                        crash!("a call without its function value");
                    };
                    match self.stack.remove(at) {
                        Value::Function(id) => call!(id, count),
                        other => crash!(format!("cannot call {}", other.kind_name())),
                    }
                }
                Op::ResultType(index) => {
                    self.expected = program.result_types.get(*index as usize).cloned();
                }
                Op::Not => {
                    let value = try_op!(self.pop_bool());
                    self.stack.push(Value::Boolean(!value));
                }
                Op::Binary(op) => {
                    let right = self.pop();
                    let left = self.pop();
                    let value = try_op!(self.binary_values(*op, left, right));
                    self.stack.push(value);
                }
                Op::ToText => {
                    let value = self.pop();
                    let text = try_op!(self.to_text(&value));
                    self.stack.push(Value::text(text).guarded(value.origins()));
                }
                Op::Concat(count) => {
                    let value = try_op!(self.op_concat(*count as usize));
                    self.stack.push(value);
                }
                Op::Jump(target) => {
                    let target = *target as usize;
                    if target < pc {
                        // a jump backwards: a safe point of the memory
                        // budget, and once the code is hot, the loop
                        // carries on as machine code (decision AG3)
                        try_op!(self.memory_check());
                        self.hotness[code_id] = self.hotness[code_id].saturating_add(ran);
                        ran = 0;
                        let hot = self
                            .native
                            .as_ref()
                            .is_some_and(|jit| jit.is_hot(code.ops.len(), self.hotness[code_id]));
                        if hot {
                            self.frames[frame].pc = target;
                            if let Some(function) = self.native_resume_of_top(target as u32) {
                                match self.run_generated(function, base, target as u32) {
                                    native::runtime::RETURNED => {
                                        let value = self.pop();
                                        if self.frames.len() < entry {
                                            return Ok(value);
                                        }
                                        if value.is_failure() {
                                            match try_op!(self.settle(value, entry)) {
                                                Flow::Done(value) => return Ok(value),
                                                _ => reload!(),
                                            }
                                        } else {
                                            self.stack.push(value);
                                            reload!();
                                        }
                                        continue;
                                    }
                                    native::runtime::DEOPT => {
                                        reload!();
                                        continue;
                                    }
                                    native::runtime::STAY => {
                                        if let Some(jit) = self.native.as_mut() {
                                            jit.refuse_resume(code_id);
                                        }
                                    }
                                    _ => return Err(self.take_pending()),
                                }
                            }
                        }
                    }
                    pc = target;
                }
                Op::JumpIfFalse(target) => match self.stack.pop() {
                    Some(Value::Boolean(condition)) => {
                        if !condition {
                            pc = *target as usize;
                        }
                    }
                    other => crash!(not_boolean(other)),
                },
                Op::JumpIfTrue(target) => match self.stack.pop() {
                    Some(Value::Boolean(condition)) => {
                        if condition {
                            pc = *target as usize;
                        }
                    }
                    other => crash!(not_boolean(other)),
                },
                Op::JumpIfAbsent(target) => {
                    if matches!(
                        self.stack.last(),
                        Some(Value::Nothing) | Some(Value::Failure(_))
                    ) {
                        pc = *target as usize;
                    }
                }
                Op::JumpIfFailure(target) => {
                    if matches!(self.stack.last(), Some(Value::Failure(_))) {
                        pc = *target as usize;
                    }
                }
                Op::PushHandler(target) => {
                    let height = self.stack.len();
                    self.handlers.push((*target as usize, height));
                }
                Op::PopHandler => {
                    self.handlers.pop();
                }
                Op::Return => {
                    let value = self.pop();
                    leave!(value);
                }
                Op::ReturnNothing => leave!(Value::Nothing),
                Op::Fail => {
                    let error = self.pop();
                    let failure = match error {
                        Value::Failure(_) => error,
                        Value::Nothing => {
                            crash!("`otherwise fail` after a value that was nothing, not a failure")
                        }
                        other => Value::failure(other),
                    };
                    leave!(failure);
                }
                Op::Crash => {
                    let message = self.pop();
                    let text = try_op!(self.text_for_console(&message, "a crash message"));
                    crash!(text);
                }
                Op::IsVariant(tag) => {
                    let value = self.pop();
                    let fits = matches!(value.plain(), Value::Variant(v) if v.tag == *tag as usize);
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
                    let fits = self.has_type(value.plain(), *ty);
                    self.stack.push(Value::Boolean(fits));
                }
                Op::Unpack(count) => {
                    let value = self.pop();
                    let origins = value.origins();
                    let parts = try_op!(self.unpack(value.into_plain(), *count as usize));
                    self.stack
                        .extend(parts.into_iter().map(|part| part.guarded(origins)));
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
                    let iterator = try_op!(self.iterator_of(source));
                    self.stack[base + *slot as usize] = iterator;
                }
                Op::IterNext { slot, exit } => match self.iterator_next(base + *slot as usize) {
                    Some(item) => self.stack.push(item),
                    None => pc = *exit as usize,
                },
                Op::ListPush => {
                    let value = try_op!(self.op_list_push());
                    self.stack.push(value);
                }
                Op::GroupInsert => {
                    let value = try_op!(self.op_group_insert());
                    self.stack.push(value);
                }
                Op::GroupFold(fold) => {
                    let value = try_op!(self.op_group_fold(*fold));
                    self.stack.push(value);
                }
                Op::SortByKey { descending } => {
                    let value = try_op!(self.op_sort_by_key(*descending));
                    self.stack.push(value);
                }
                Op::Deadline(slot) => {
                    let limit = match self.pop().into_plain() {
                        Value::Duration(ms) => ms,
                        other => crash!(format!(
                            "`within` needs a Duration, found {}",
                            other.kind_name()
                        )),
                    };
                    self.stack[base + *slot as usize] = Value::Native(Rc::new(Native::Deadline(
                        natives::now_millis() + limit,
                        limit,
                    )));
                }
                Op::MarkStack(slot) => {
                    let height = self.stack.len() as i64;
                    self.stack[base + *slot as usize] = Value::integer(height);
                }
                Op::UnwindStack(slot) => {
                    let height = match &self.stack[base + *slot as usize] {
                        Value::Integer(Int::Small(height)) => *height as usize,
                        _ => crash!("the loop's stack mark is missing".to_string()),
                    };
                    self.stack.truncate(height);
                }
                Op::CheckDeadline(slot) => {
                    if let Some(limit) = self.expired_deadline(base + *slot as usize) {
                        let error =
                            Value::record(program.builtins.timed_out, vec![Value::Duration(limit)]);
                        settle!(Value::failure(error));
                    }
                }
                Op::Check(message) => {
                    if !try_op!(self.pop_bool()) {
                        let text = code.constants[*message as usize]
                            .as_text()
                            .unwrap_or("")
                            .to_string();
                        let failure = Value::failure(Value::text(format!("check failed: {text}")));
                        leave!(failure);
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------ calls

    /// The explicit implementation an ability call resolves to, by the
    /// type of the receiver: the first of the `count` arguments on the
    /// stack.
    pub(crate) fn ability_target(
        &self,
        ability: AbilityId,
        method: usize,
        count: usize,
    ) -> Option<FunctionId> {
        let program = self.program;
        let receiver = self.stack.get(self.stack.len().checked_sub(count)?)?;
        // a base value dispatches by its declared type: a list is `Iterable`
        // through the prelude's implementation (decision AB1)
        let ty = receiver
            .type_id()
            .or_else(|| self.builtin_type_id(receiver))?;
        let method_name = program.abilities.get(ability)?.1.get(method)?;
        program.types.implementation(ability, ty, method_name)
    }

    /// The declared type of a base value (a number, a text, a collection);
    /// `None` for a record or a variant, which carry their type, and for a
    /// value without a prelude type.
    fn builtin_type_id(&self, value: &Value) -> Option<TypeId> {
        let b = &self.program.builtins;
        Some(match value.plain() {
            Value::Integer(_) => b.integer,
            Value::Decimal(_) => b.decimal,
            Value::Float(_) => b.float,
            Value::Boolean(_) => b.boolean,
            Value::Text(_) => b.text,
            Value::Bytes(_) => b.bytes,
            Value::List(_) => b.list,
            Value::Map(_) => b.map,
            Value::Set(_) => b.set,
            Value::Range(_) => b.range,
            Value::Pair(_) => b.pair,
            Value::Duration(_) => b.duration,
            _ => return None,
        })
    }

    /// An ability call without an explicit implementation: the derived
    /// abilities, whose results carry the operands' origins.
    pub(crate) fn derived_ability(
        &mut self,
        ability: AbilityId,
        method: usize,
        args: Vec<Value>,
    ) -> Result<Value, Interrupt> {
        let program = self.program;
        let receiver = args.first().cloned().unwrap_or(Value::Nothing);
        let origins = args.iter().fold(0, |all, arg| all | arg.origins());
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
            let (ability_name, method_name) = match program.abilities.get(ability) {
                Some((name, methods)) => (
                    name.as_str(),
                    methods.get(method).map(String::as_str).unwrap_or(""),
                ),
                None => ("?", ""),
            };
            return Err(Interrupt::crash(format!(
                "{} has no implementation of `{ability_name}.{method_name}`",
                self.describe(&receiver)
            )));
        };
        Ok(value.guarded(origins))
    }

    pub fn ordering_value(&self, ordering: Ordering) -> Value {
        let tag = match ordering {
            Ordering::Less => 0,
            Ordering::Equal => 1,
            Ordering::Greater => 2,
        };
        Value::variant(self.program.builtins.ordering, tag, Vec::new())
    }

    pub(crate) fn global(&mut self, index: usize) -> Result<Value, Interrupt> {
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
            TypeShape::Record(_) => self.refined_record(ty, fields),
            _ => Ok(Value::record(ty, fields)),
        }
    }

    /// A record whose fields satisfy every refinement of the type, or the
    /// `Failure(ConstraintViolation)` of the first that does not.
    fn refined_record(&mut self, ty: TypeId, fields: Vec<Value>) -> Result<Value, Interrupt> {
        if let Some(violation) = self.refinement_violation(ty, &fields)? {
            return Ok(violation);
        }
        Ok(Value::record(ty, fields))
    }

    /// The `Failure(ConstraintViolation)` of the first refinement of the
    /// type that the fields do not satisfy, when there is one.
    fn refinement_violation(
        &mut self,
        ty: TypeId,
        fields: &[Value],
    ) -> Result<Option<Value>, Interrupt> {
        let meta = self.program.types.meta(ty);
        if let (false, TypeShape::Record(field_metas)) = (meta.refinements.is_empty(), &meta.shape)
        {
            for field_meta in field_metas {
                if let Some(index) = field_meta.refinement {
                    let code = meta.refinements[index];
                    if !self.holds(code, fields.to_vec())? {
                        return Ok(Some(self.violation(ty, code)));
                    }
                }
            }
        }
        // a `Date` also needs a day its month has (library sketch, section 4)
        if Some(ty) == self.program.date {
            if let [year, month, day] = fields {
                let parts = (
                    crate::natives::small(year),
                    crate::natives::small(month),
                    crate::natives::small(day),
                );
                if let (Ok(year), Ok(month), Ok(day)) = parts {
                    let length = crate::natives::time::days_in_month(year, month);
                    if (1..=12).contains(&month) && day > length {
                        return Ok(Some(Value::failure(Value::record(
                            self.program.builtins.constraint_violation,
                            vec![
                                Value::text("Date"),
                                Value::text(format!("the month has {length} days")),
                            ],
                        ))));
                    }
                }
            }
        }
        Ok(None)
    }

    pub fn construct_variant(
        &mut self,
        ty: TypeId,
        tag: usize,
        fields: Vec<Value>,
    ) -> Result<Value, Interrupt> {
        // a variant without fields is the same value every time: built
        // once per type and tag (decision AG6)
        if fields.is_empty() {
            let (base, count) = self.unit_base[ty];
            if tag < count {
                if let Some(value) = &self.unit_variants[base + tag] {
                    return Ok(value.clone());
                }
                let value = Value::variant(ty, tag, fields);
                self.unit_variants[base + tag] = Some(value.clone());
                return Ok(value);
            }
        }
        let meta = self.program.types.meta(ty);
        if let (false, TypeShape::Sum(variants)) = (meta.refinements.is_empty(), &meta.shape) {
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

    /// A field of a record or a variant through the site's cache, which a
    /// miss fills (decision X3); the other holders go by name.
    pub(crate) fn field_at(
        &mut self,
        site: usize,
        holder: &Value,
        name: &str,
    ) -> Result<Value, Interrupt> {
        let (ty, tag, fields) = match holder {
            Value::Record(record) => (record.ty, usize::MAX, &record.fields),
            Value::Variant(variant) => (variant.ty, variant.tag, &variant.fields),
            other => return self.field(other, name),
        };
        let cached = self.field_cache[site];
        if cached.ty == ty && cached.tag == tag {
            if let Some(value) = fields.get(cached.index) {
                return Ok(value.clone());
            }
        }
        let types = &self.program.types;
        let index = if tag == usize::MAX {
            types.field_index(ty, name)
        } else {
            types.variant_field_index(ty, tag, name)
        };
        match index.and_then(|index| Some((index, fields.get(index)?.clone()))) {
            Some((index, value)) => {
                self.field_cache[site] = FieldSite { ty, tag, index };
                Ok(value)
            }
            None => Err(Interrupt::crash(format!(
                "{} has no field `{name}`",
                self.describe(holder)
            ))),
        }
    }

    /// `Op::LoadField`: the field `name` of the value in the slot at `at`,
    /// read there when the site has seen the holder's type (decision AG6,
    /// as an op since AR2), else through `op_field` on a clone of the
    /// holder.
    pub(crate) fn op_load_field(
        &mut self,
        code: usize,
        at: usize,
        name: usize,
        site: usize,
    ) -> Result<Value, Interrupt> {
        let cached = self.field_cache[site];
        let found = match &self.stack[at] {
            Value::Record(record) if cached.ty == record.ty && cached.tag == usize::MAX => {
                record.fields.get(cached.index)
            }
            Value::Variant(variant) if cached.ty == variant.ty && cached.tag == variant.tag => {
                variant.fields.get(cached.index)
            }
            _ => None,
        };
        if let Some(value) = found {
            return Ok(value.clone());
        }
        let holder = self.stack[at].clone();
        self.op_field(code, name, site, &holder)
    }

    /// `Op::TakeField`: the field `name` of the record in the slot at `at`,
    /// moved out when the record is held once, `Nothing` left in its place
    /// until the `WithSlot` that follows fills it (decision AU11); read as
    /// `op_load_field` reads it when the record is shared, or when the site
    /// has not seen the holder's type yet.
    pub(crate) fn op_take_field(
        &mut self,
        code: usize,
        at: usize,
        name: usize,
        site: usize,
    ) -> Result<Value, Interrupt> {
        let cached = self.field_cache[site];
        if let Value::Record(record) = &mut self.stack[at] {
            if cached.ty == record.ty && cached.tag == usize::MAX {
                let found = match Rc::get_mut(record) {
                    Some(unique) => unique
                        .fields
                        .get_mut(cached.index)
                        .map(|field| std::mem::replace(field, Value::Nothing)),
                    None => record.fields.get(cached.index).cloned(),
                };
                if let Some(value) = found {
                    return Ok(value);
                }
            }
        }
        self.op_load_field(code, at, name, site)
    }

    /// A field by name of a pair or a native value (a record or a variant
    /// goes through `field_at`).
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

    pub(crate) fn with(
        &mut self,
        base: Value,
        updates: Vec<(Value, Value)>,
    ) -> Result<Value, Interrupt> {
        let Value::Record(mut record) = base else {
            return Err(Interrupt::crash(format!(
                "`with` needs a record, found {}",
                base.kind_name()
            )));
        };
        {
            // in place when the record is held once (decision AU11), else
            // in a copy; the allocation stays either way
            let record = Rc::make_mut(&mut record);
            for (name, value) in updates {
                let name = name.as_text().unwrap_or("");
                match self.program.types.field_index(record.ty, name) {
                    Some(index) => record.fields[index] = value,
                    None => {
                        return Err(Interrupt::crash(format!(
                            "`{}` has no field `{name}`",
                            self.program.types.meta(record.ty).name
                        )))
                    }
                }
            }
        }
        // the refinements hold for every value of the type, the copy included
        if let Some(violation) = self.refinement_violation(record.ty, &record.fields)? {
            return Ok(violation);
        }
        Ok(Value::Record(record))
    }

    pub(crate) fn unpack(&self, value: Value, count: usize) -> Result<Vec<Value>, Interrupt> {
        let parts = match value {
            Value::Pair(pair) => vec![pair.0.clone(), pair.1.clone()],
            Value::List(items) => take_list(items),
            Value::Record(record) => record.fields.to_vec(),
            Value::Variant(variant) => variant.fields.to_vec(),
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

    /// The items of a collection, for a loop: a list itself, the others
    /// as a list; the caller tags them with the collection's origins.
    pub fn iterate(&self, source: Value) -> Result<Rc<Vec<Value>>, Interrupt> {
        Ok(match source.into_plain() {
            Value::List(items) => items,
            Value::Set(items) => Rc::new(items.iter().cloned().collect()),
            Value::Map(entries) => Rc::new(
                entries
                    .iter()
                    .map(|(key, value)| Value::pair(key.clone(), value.clone()))
                    .collect(),
            ),
            Value::Range(range) => Rc::new(range_items(&range)?),
            Value::Text(text) => Rc::new(natives::prelude::characters(&text)),
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
        let value = value.plain();
        if let Some(id) = value.type_id() {
            return id == ty;
        }
        if let TypeShape::Subtype { base } = &self.program.types.meta(ty).shape {
            if let Ty::App(base_id, _) = base {
                return self.has_type(value, *base_id);
            }
            return false;
        }
        match self.builtin_type_id(value) {
            Some(id) => id == ty,
            None => {
                matches!(value, Value::Instant(_)) && self.program.types.meta(ty).name == "Instant"
            }
        }
    }

    pub fn describe(&self, value: &Value) -> String {
        match value.type_id() {
            Some(ty) => format!("a `{}`", self.program.types.meta(ty).name),
            None => format!("a {}", value.kind_name()),
        }
    }

    // ------------------------------------------------------------ operators

    /// `is` and `is not`: a declared `equals` of the left value's type
    /// (decision K1, the `Equal` ability), else the derived structural
    /// equality, which sets, maps and the library's searches always use.
    pub fn equal(&mut self, left: &Value, right: &Value) -> Result<bool, Interrupt> {
        if let Some(ty) = left.type_id() {
            if let Some(function) = self.program.specials[ty].equals {
                return match self
                    .call_function(function, vec![left.clone(), right.clone()])?
                    .into_plain()
                {
                    Value::Boolean(value) => Ok(value),
                    other => Err(Interrupt::crash(format!(
                        "`equals` produced {}, not a Boolean",
                        other.kind_name()
                    ))),
                };
            }
        }
        Ok(left == right)
    }

    pub(crate) fn binary(
        &mut self,
        op: BinaryOp,
        left: Value,
        right: Value,
    ) -> Result<Value, Interrupt> {
        Ok(match op {
            BinaryOp::Is => Value::Boolean(self.equal(&left, &right)?),
            BinaryOp::IsNot => Value::Boolean(!self.equal(&left, &right)?),
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

/// The crash of an operation that needed a Boolean.
pub(crate) fn not_boolean(found: Option<Value>) -> String {
    format!(
        "expected a Boolean, found {}",
        found.unwrap_or(Value::Nothing).kind_name()
    )
}

/// An operator on two small Integers without the general path: `None` on
/// overflow and for the operators the general path defines (decision X3).
/// The three fast paths are inlined wherever they are called: with a
/// second caller in `rt_compare` (decision AU13) the compiler stopped
/// inlining them into the interpreter's `binary_values`, which cost the
/// interpreter one percent of its instructions on the self-check.
#[inline(always)]
pub(crate) fn small_binary(op: BinaryOp, a: i64, b: i64) -> Option<Value> {
    Some(match op {
        BinaryOp::Is => Value::Boolean(a == b),
        BinaryOp::IsNot => Value::Boolean(a != b),
        BinaryOp::IsLessThan => Value::Boolean(a < b),
        BinaryOp::IsAtMost => Value::Boolean(a <= b),
        BinaryOp::IsGreaterThan => Value::Boolean(a > b),
        BinaryOp::IsAtLeast => Value::Boolean(a >= b),
        BinaryOp::Add => Value::integer(a.checked_add(b)?),
        BinaryOp::Subtract => Value::integer(a.checked_sub(b)?),
        BinaryOp::Multiply => Value::integer(a.checked_mul(b)?),
        // the remainder stays on the fast path too (decision AG1); a zero
        // divisor and `i64::MIN remainder -1` take the general one
        BinaryOp::Remainder if b != 0 && b != -1 => Value::integer(a % b),
        _ => return None,
    })
}

/// The comparisons of two texts without the general path.
#[inline(always)]
pub(crate) fn text_binary(op: BinaryOp, a: &str, b: &str) -> Option<Value> {
    Some(match op {
        BinaryOp::Is => Value::Boolean(a == b),
        BinaryOp::IsNot => Value::Boolean(a != b),
        BinaryOp::IsLessThan => Value::Boolean(a < b),
        BinaryOp::IsAtMost => Value::Boolean(a <= b),
        BinaryOp::IsGreaterThan => Value::Boolean(a > b),
        BinaryOp::IsAtLeast => Value::Boolean(a >= b),
        _ => return None,
    })
}

/// `is`, `is not`, `and` and `or` on two Booleans without the general path.
#[inline(always)]
pub(crate) fn boolean_binary(op: BinaryOp, a: bool, b: bool) -> Option<Value> {
    Some(match op {
        BinaryOp::Is => Value::Boolean(a == b),
        BinaryOp::IsNot => Value::Boolean(a != b),
        BinaryOp::And => Value::Boolean(a && b),
        BinaryOp::Or => Value::Boolean(a || b),
        _ => return None,
    })
}

/// The capability a crash message or a reported failure leaves through.
fn console_capability() -> Capability {
    Capability {
        path: vec!["console".to_string()],
        scope: None,
        budget: None,
        only_to: Vec::new(),
    }
}

/// A grant spelled for a message.
fn grant_text(grant: &[Capability]) -> String {
    if grant.is_empty() {
        return "(nothing)".to_string();
    }
    grant
        .iter()
        .map(grant::spell)
        .collect::<Vec<_>>()
        .join(", ")
}

/// A native value in a recording: a connection by its path, the rest by
/// kind.
fn native_json(native: &Native) -> Json {
    match native {
        Native::Connection { path, .. } => Json::Text(format!("<connection {path}>")),
        Native::CsvRow { .. } => Json::Text("<row>".to_string()),
        Native::Iterator(_) | Native::RangeIterator { .. } => Json::Text("<iterator>".to_string()),
        Native::Deadline(..) => Json::Text("<deadline>".to_string()),
    }
}

fn connection_path(json: &Json) -> Option<String> {
    match json {
        Json::Text(text) => text
            .strip_prefix("<connection ")
            .and_then(|rest| rest.strip_suffix('>'))
            .map(str::to_string),
        _ => None,
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
                return Ok(Value::decimal(a.power(&b).map_err(decimal_error)?));
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
    result.map(Value::decimal).map_err(decimal_error)
}

/// A `Float` never holds infinity or NaN (library sketch section 1): a result
/// that is not finite is a crash.
fn float_arithmetic(op: BinaryOp, a: f64, b: f64) -> Result<Value, Interrupt> {
    let result = match op {
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
    };
    finite_float(result)
}

/// The Float as a value, or the crash for infinity and NaN.
pub fn finite_float(value: f64) -> Result<Value, Interrupt> {
    if value.is_nan() {
        return Err(Interrupt::crash("the Float result is not a number"));
    }
    if value.is_infinite() {
        return Err(Interrupt::crash("the Float result is out of range"));
    }
    Ok(Value::Float(value))
}
