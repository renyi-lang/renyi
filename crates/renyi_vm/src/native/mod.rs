//! Machine code for the VM's bytecode (decision AG1): a code object is
//! compiled to native code through Cranelift inside the process once it
//! is hot (the interpreter has run `HOT_FACTOR` times its size in ops of
//! it), and runs in place of the interpreter's loop from then on: from
//! its next call, or, when the interpreter is inside one of its loops,
//! from the loop's next turn, through an entry at the loop header.
//! The bytecode file stays the contract between the front ends and the
//! VM; what the VM makes of it is its own business. The interpreter
//! remains the reference: narrated and profiled runs use it (decision
//! AG3), a frame whose typed assumptions fail at run time is handed to it
//! at the op in question (`rt_deopt`), and `--interpret` keeps everything
//! on it.
//!
//! - `infer.rs` settles the operand stack of a code object and decides
//!   which operands and slots stay unboxed in registers.
//! - `codegen.rs` translates the ops to Cranelift IR over that analysis.
//! - `runtime.rs` holds the helpers the generated code calls for
//!   everything it does not do in registers: each is the interpreter's
//!   arm for that op, on the VM's stack.

pub mod codegen;
pub mod infer;
pub mod runtime;

use std::collections::HashMap;

use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::Context;
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};

use crate::compile::{CodeId, Program};
use crate::vm::Vm;
use infer::{Abs, SlotKind};

/// The generated function of a code object: the VM, the frame's base and
/// the pc to enter at (`0`, or a loop header) in, the status out
/// (`runtime::RETURNED`, `DEOPT`, `INTERRUPT` or `STAY`).
pub type Entry = unsafe extern "C" fn(*mut Vm<'static>, usize, u32) -> i32;

/// How many generated frames may nest on the machine stack before a call
/// goes through the interpreter instead, which keeps its frames on the
/// heap: the generated code and its helpers use a few hundred bytes per
/// level, the interpreter's loop a few kilobytes when it is between two
/// generated levels, so this many levels fit a main thread's stack on
/// every platform with room to spare.
pub const DEPTH_LIMIT: usize = 200;

/// When a code object is compiled: once the interpreter has run this many
/// times its size in ops of it, so that the time it has had is a few
/// times what compiling it costs (compiling an op costs a few hundred
/// times running it on the interpreter), and code run once or twice (a
/// driver, a setup, a short loop) never pays for machine code it would
/// not use. `RENYI_NATIVE_HOT` overrides the factor, a development aid:
/// `0` compiles everything at its first call and enters every loop at its
/// first turn.
pub const HOT_FACTOR: u32 = 2000;

/// Where the generated code hands a frame to the interpreter: what it
/// kept in registers at that op, so that `rt_deopt` can box it.
#[derive(Clone, Debug)]
pub struct DeoptPoint {
    pub pc: u32,
    pub locals: u16,
    /// The abstract operand stack at the point.
    pub stack: Vec<Abs>,
    /// The kind of every slot.
    pub slots: Vec<SlotKind>,
    /// Per mark slot, the depth it marks.
    pub marks: Vec<Option<usize>>,
}

enum State {
    /// Not compiled yet: the interpreter runs it.
    Cold,
    Skipped,
    /// Compiled: the trampoline the VM calls, the body a generated caller
    /// calls directly (decision AR3), and the loop headers the function
    /// can be entered at.
    Ready {
        entry: Entry,
        direct: *const u8,
        headers: Vec<u32>,
    },
}

/// The JIT of one VM: the Cranelift module with every helper declared,
/// and the state of every code object.
pub struct Jit {
    module: Option<JITModule>,
    helpers: HashMap<&'static str, FuncId>,
    ctx: Context,
    fctx: FunctionBuilderContext,
    states: Vec<State>,
    hot_factor: u32,
    /// Per code object, whether an entry at a loop header found the frame
    /// not fit for the registers: no further entry there.
    resume_refused: Vec<bool>,
    /// How many generated frames are nested on the machine stack; the
    /// generated code reads and moves it in place around a direct call
    /// (decision AR3), by its address, which the box the JIT lives in
    /// keeps fixed.
    pub depth: usize,
    /// Per code object, the address of its body once compiled, else null:
    /// what a direct call site loads first (decision AR3).
    direct_table: Box<[*const u8]>,
    /// Per code object, its deopt points, numbered as the generated code
    /// names them.
    pub deopts: Vec<Vec<DeoptPoint>>,
    /// The counts and the time (for `RENYI_NATIVE_REPORT`, a development
    /// aid): code objects compiled and left to the interpreter, ops
    /// compiled, the compile time by phase.
    pub compiled: usize,
    pub skipped: usize,
    pub ops: usize,
    pub stats: codegen::Stats,
    pub finalizing: std::time::Duration,
    /// How often generated code handed a frame to the interpreter, how
    /// many calls generated code made, and how many of those found no
    /// generated code to call.
    pub deopts_taken: usize,
    pub calls: usize,
    pub calls_cold: usize,
    /// How often a loop was entered from the interpreter, and how often
    /// such an entry was refused.
    pub resumes: usize,
    pub resumes_refused: usize,
}

impl Jit {
    /// A JIT for the program, or `None` when the host is not a machine
    /// Cranelift generates code for (the interpreter then runs alone).
    pub fn new(program: &Program) -> Option<Jit> {
        let mut flags = settings::builder();
        flags.set("use_colocated_libcalls", "false").ok()?;
        flags.set("is_pic", "false").ok()?;
        // no optimisation: the generated code calls a helper for most ops,
        // and Cranelift's optimiser finds little in it for twice the time
        // (`RENYI_NATIVE_OPT=speed` compares, a development aid)
        let level = std::env::var("RENYI_NATIVE_OPT").unwrap_or_else(|_| "none".to_string());
        flags.set("opt_level", &level).ok()?;
        // the register allocator: Cranelift's backtracking one unless
        // `RENYI_NATIVE_REGALLOC` names the other (`single_pass`, quick to
        // compile, more spills; a development aid for the comparison)
        if let Ok(algorithm) = std::env::var("RENYI_NATIVE_REGALLOC") {
            flags.set("regalloc_algorithm", &algorithm).ok()?;
        }
        // the IR verifier is a development aid of Cranelift's, on by default
        // and a large part of the compile time; `RENYI_NATIVE_VERIFY` turns
        // it on when the generated IR is in question
        let verify = std::env::var_os("RENYI_NATIVE_VERIFY").is_some();
        flags
            .set("enable_verifier", if verify { "true" } else { "false" })
            .ok()?;
        let isa = cranelift_native::builder()
            .ok()?
            .finish(settings::Flags::new(flags))
            .ok()?;
        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        builder.symbols(runtime::HELPERS.iter().map(|(name, ptr)| (*name, *ptr)));
        let mut module = JITModule::new(builder);
        let mut helpers = HashMap::new();
        for (name, params, result) in codegen::SIGNATURES {
            let sig = codegen::signature(&module, params, *result);
            let id = module.declare_function(name, Linkage::Import, &sig).ok()?;
            helpers.insert(*name, id);
        }
        let ctx = module.make_context();
        let hot_factor = std::env::var("RENYI_NATIVE_HOT")
            .ok()
            .and_then(|text| text.parse().ok())
            .unwrap_or(HOT_FACTOR);
        Some(Jit {
            module: Some(module),
            helpers,
            ctx,
            fctx: FunctionBuilderContext::new(),
            states: program.codes.iter().map(|_| State::Cold).collect(),
            hot_factor,
            resume_refused: vec![false; program.codes.len()],
            depth: 0,
            direct_table: vec![std::ptr::null(); program.codes.len()].into_boxed_slice(),
            deopts: vec![Vec::new(); program.codes.len()],
            compiled: 0,
            skipped: 0,
            ops: 0,
            stats: codegen::Stats::default(),
            finalizing: std::time::Duration::ZERO,
            deopts_taken: 0,
            calls: 0,
            calls_cold: 0,
            resumes: 0,
            resumes_refused: 0,
        })
    }

    /// The counts and the time, one line; `hotness` is the VM's count of
    /// ops run per code object.
    pub fn report(&self, hotness: &[u32]) -> String {
        let cold = self
            .states
            .iter()
            .zip(hotness)
            .filter(|(state, ran)| matches!(state, State::Cold) && **ran > 0)
            .count();
        let stats = &self.stats;
        format!(
            "native: {} code objects compiled ({} ops, {} instructions in {} blocks) in {} ms (analysis {} ms, IR {} ms, cranelift {} ms, finalizing {} ms); {} left to the interpreter, {} called but cold",
            self.compiled,
            self.ops,
            stats.instructions,
            stats.blocks,
            (stats.analysis + stats.ir + stats.cranelift + self.finalizing).as_millis(),
            stats.analysis.as_millis(),
            stats.ir.as_millis(),
            stats.cranelift.as_millis(),
            self.finalizing.as_millis(),
            self.skipped,
            cold
        ) + &format!(
            "
native: {} deopts; {} calls from generated code, {} of them to the interpreter; {} loops entered from the interpreter, {} refused
{}",
            self.deopts_taken,
            self.calls,
            self.calls_cold,
            self.resumes,
            self.resumes_refused,
            cranelift_codegen::timing::take_current()
        )
    }

    /// The generated function of a code object for one more call of it,
    /// given how many of its ops the interpreter has run: compiled when
    /// that makes it hot; `None` while it is cold and when it stays with
    /// the interpreter.
    pub fn entry(&mut self, program: &Program, code: CodeId, hotness: u32) -> Option<Entry> {
        self.ready(program, code, hotness)
            .map(|(entry, _, _)| entry)
    }

    /// The body of a code object for a direct call (decision AR3), under
    /// the same rule as `entry`.
    pub fn direct(&mut self, program: &Program, code: CodeId, hotness: u32) -> Option<*const u8> {
        self.ready(program, code, hotness)
            .map(|(_, direct, _)| direct)
    }

    /// The generated function of a code object to enter at the loop
    /// header `pc`, when the code is hot, the function has that entry and
    /// no earlier entry there was refused.
    pub fn resume(
        &mut self,
        program: &Program,
        code: CodeId,
        pc: u32,
        hotness: u32,
    ) -> Option<Entry> {
        if self.resume_refused[code] {
            return None;
        }
        let (entry, _, headers) = self.ready(program, code, hotness)?;
        let found = headers.contains(&pc);
        self.resumes += found as usize;
        found.then_some(entry)
    }

    /// An entry at a loop header of the code found the frame not fit for
    /// the registers: no further entry there.
    pub fn refuse_resume(&mut self, code: CodeId) {
        self.resume_refused[code] = true;
        self.resumes_refused += 1;
    }

    /// Whether the interpreter has run enough of a code object of `size`
    /// ops to compile it.
    pub fn is_hot(&self, size: usize, hotness: u32) -> bool {
        hotness >= (size as u32).saturating_mul(self.hot_factor)
    }

    fn ready(
        &mut self,
        program: &Program,
        code: CodeId,
        hotness: u32,
    ) -> Option<(Entry, *const u8, &[u32])> {
        match &self.states[code] {
            State::Ready { .. } => {}
            State::Skipped => return None,
            State::Cold => {
                if !self.is_hot(program.codes[code].ops.len(), hotness) {
                    return None;
                }
                self.compile(program, code);
            }
        }
        match &self.states[code] {
            State::Ready {
                entry,
                direct,
                headers,
            } => Some((*entry, *direct, headers)),
            _ => None,
        }
    }

    /// Compile a code object; its state becomes `Ready` or `Skipped`.
    fn compile(&mut self, program: &Program, code: CodeId) {
        let addresses = codegen::Addresses {
            depth: &self.depth as *const usize as usize,
            direct_table: self.direct_table.as_ptr() as usize,
            calls: &self.calls as *const usize as usize,
        };
        let module = self.module.as_mut().expect("the module lives with the JIT");
        let compiled = codegen::compile(
            program,
            code,
            module,
            &self.helpers,
            &mut self.ctx,
            &mut self.fctx,
            addresses,
        );
        let (id, body, deopts, headers, stats) = match compiled {
            Ok(compiled) => compiled,
            Err(reason) => {
                if std::env::var_os("RENYI_NATIVE_REPORT").is_some() {
                    eprintln!(
                        "native: {} stays with the interpreter: {reason:?}",
                        program.codes[code].name
                    );
                }
                self.states[code] = State::Skipped;
                self.skipped += 1;
                return;
            }
        };
        self.stats.add(&stats);
        let defined = std::time::Instant::now();
        let finalized = module.finalize_definitions();
        self.finalizing += defined.elapsed();
        if finalized.is_err() {
            self.states[code] = State::Skipped;
            self.skipped += 1;
            return;
        }
        let pointer = module.get_finalized_function(id);
        // SAFETY: the function was defined with `entry_signature`, which is
        // the signature of `Entry`.
        let entry: Entry = unsafe { std::mem::transmute::<*const u8, Entry>(pointer) };
        let direct = module.get_finalized_function(body);
        self.direct_table[code] = direct;
        self.deopts[code] = deopts;
        self.states[code] = State::Ready {
            entry,
            direct,
            headers,
        };
        self.compiled += 1;
        self.ops += program.codes[code].ops.len();
    }
}

impl Drop for Jit {
    fn drop(&mut self) {
        if let Some(module) = self.module.take() {
            // SAFETY: no generated function outlives the VM that owns the
            // JIT; the entries are only ever called through it.
            unsafe { module.free_memory() };
        }
    }
}
