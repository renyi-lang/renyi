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
pub mod image;
pub mod infer;
pub mod runtime;

use std::io::{Read, Seek};

use cranelift_codegen::isa::OwnedTargetIsa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::Context;
use cranelift_frontend::FunctionBuilderContext;

use crate::compile::{CodeId, Program};
use crate::value::Value;
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
/// Decision AR5: a code object pays for its compilation (Cranelift's
/// work, about a quarter of a million instructions per op) only after
/// some ten thousand runs of each of its ops; 8000 is where the
/// compiler's self-check measured lowest (2000 before AR5).
pub const HOT_FACTOR: u32 = 8000;

/// Where the generated code hands a frame to the interpreter: what it
/// kept in registers at that op, so that `rt_deopt` can box it.
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// The JIT of one VM: Cranelift's target, the state of every code
/// object, the tables the generated code reaches through the VM
/// (decision AS1) and the executable memory the code is placed in.
pub struct Jit {
    isa: OwnedTargetIsa,
    ctx: Context,
    fctx: FunctionBuilderContext,
    states: Vec<State>,
    hot_factor: u32,
    /// Per code object, whether an entry at a loop header found the frame
    /// not fit for the registers: no further entry there.
    resume_refused: Vec<bool>,
    /// Per code object, the address of its body once compiled, else null:
    /// what a direct call site loads first (decision AR3), through
    /// `NativeState::direct_table`.
    direct_table: Box<[*const u8]>,
    /// The helpers in the order of `codegen::SIGNATURES`, which the
    /// generated code calls through `NativeState::helpers`.
    helpers: Box<[*const u8]>,
    /// Per code object, its constants, which the generated code reads
    /// through `NativeState::constants`.
    constants: Box<[*const Value]>,
    /// The executable memory of the compiled functions.
    code: CodeArena,
    /// Cranelift's optimisation level the code is compiled at (`none` or
    /// `speed`); an image records it.
    opt_level: String,
    /// How many code objects an image provided (decision AS1).
    pub loaded: usize,
    /// Whether the image's code section was mapped from its file
    /// (decision AT5) rather than copied.
    pub mapped: bool,
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
    pub placing: std::time::Duration,
    /// How often generated code handed a frame to the interpreter, and
    /// how many calls from generated code found no generated code to
    /// call and went through the interpreter.
    pub deopts_taken: usize,
    pub calls_cold: usize,
    /// How often a loop was entered from the interpreter, and how often
    /// such an entry was refused.
    pub resumes: usize,
    pub resumes_refused: usize,
}

/// The executable memory the generated code is placed in (decision
/// AS1), as `cranelift-jit` placed it: a function is written into fresh
/// pages, flushed from the instruction cache and made executable once,
/// and never written again.
#[derive(Default)]
pub struct CodeArena {
    pages: Vec<Pages>,
}

/// Executable pages the arena holds: ones it wrote, or an image's code
/// section mapped from its file (decision AT5).
enum Pages {
    Owned(#[allow(dead_code)] region::Allocation),
    Mapped(#[allow(dead_code)] memmap2::Mmap),
}

impl CodeArena {
    /// An image's code section mapped executable from its file, `len`
    /// bytes at `offset` (a multiple of the page size); its address. The
    /// system may refuse (a `noexec` mount), and the caller then reads
    /// and places the section instead.
    pub fn map_section(
        &mut self,
        file: &std::fs::File,
        offset: u64,
        len: usize,
    ) -> Result<*const u8, String> {
        // SAFETY: the mapping is private and read-only, of a file that is
        // the image a run was asked for; the bytes are the machine code
        // `renyi build` wrote, checked by the image's header to be this
        // VM's.
        let mapping = unsafe {
            memmap2::MmapOptions::new()
                .offset(offset)
                .len(len.max(1))
                .map_exec(file)
        }
        .map_err(|error| error.to_string())?;
        let pointer = mapping.as_ptr();
        self.pages.push(Pages::Mapped(mapping));
        Ok(pointer)
    }

    /// The bytes placed, executable; the address they run at.
    pub fn place(&mut self, bytes: &[u8]) -> Result<*const u8, String> {
        let mut allocation = region::alloc(bytes.len().max(1), region::Protection::READ_WRITE)
            .map_err(|error| error.to_string())?;
        let pointer = allocation.as_mut_ptr::<u8>();
        // SAFETY: the allocation holds at least the bytes and is writable
        // until it is made executable below; the flush and the protection
        // are what the instruction cache and the loader need, in the order
        // cranelift-jit used.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer, bytes.len());
            wasmtime_internal_jit_icache_coherence::clear_cache(
                pointer as *const std::ffi::c_void,
                bytes.len(),
            )
            .map_err(|error| error.to_string())?;
            region::protect(pointer, allocation.len(), region::Protection::READ_EXECUTE)
                .map_err(|error| error.to_string())?;
        }
        wasmtime_internal_jit_icache_coherence::pipeline_flush_mt()
            .map_err(|error| error.to_string())?;
        self.pages.push(Pages::Owned(allocation));
        Ok(pointer as *const u8)
    }
}

impl Jit {
    /// A JIT for the program, or `None` when the host is not a machine
    /// Cranelift generates code for (the interpreter then runs alone).
    pub fn new(program: &Program, opt_level: Option<&str>) -> Option<Jit> {
        // no optimisation unless asked (`renyi build --opt speed`, or
        // `RENYI_NATIVE_OPT=speed` as a development aid): the generated
        // code calls a helper for most ops, and Cranelift's optimiser
        // found little in it for twice the time
        let level = opt_level
            .map(str::to_string)
            .or_else(|| std::env::var("RENYI_NATIVE_OPT").ok())
            .unwrap_or_else(|| "none".to_string());
        let isa = Jit::isa(&level)?;
        let hot_factor = std::env::var("RENYI_NATIVE_HOT")
            .ok()
            .and_then(|text| text.parse().ok())
            .unwrap_or(HOT_FACTOR);
        // the helpers in the order the generated code indexes them
        let helpers: Box<[*const u8]> = codegen::SIGNATURES
            .iter()
            .map(|(name, _, _)| {
                runtime::HELPERS
                    .iter()
                    .find(|(known, _)| known == name)
                    .map(|(_, address)| *address)
                    .unwrap_or_else(|| panic!("no helper {name}"))
            })
            .collect();
        let constants: Box<[*const Value]> = program
            .codes
            .iter()
            .map(|code| code.constants.as_ptr())
            .collect();
        Some(Jit {
            isa,
            ctx: Context::new(),
            fctx: FunctionBuilderContext::new(),
            states: program.codes.iter().map(|_| State::Cold).collect(),
            hot_factor,
            resume_refused: vec![false; program.codes.len()],
            direct_table: vec![std::ptr::null(); program.codes.len()].into_boxed_slice(),
            helpers,
            constants,
            code: CodeArena::default(),
            opt_level: level,
            loaded: 0,
            mapped: false,
            deopts: vec![Vec::new(); program.codes.len()],
            compiled: 0,
            skipped: 0,
            ops: 0,
            stats: codegen::Stats::default(),
            placing: std::time::Duration::ZERO,
            deopts_taken: 0,
            calls_cold: 0,
            resumes: 0,
            resumes_refused: 0,
        })
    }

    /// Cranelift's target for this host at the optimisation level, or
    /// `None` when Cranelift generates no code for this machine.
    fn isa(opt_level: &str) -> Option<OwnedTargetIsa> {
        let mut flags = settings::builder();
        flags.set("use_colocated_libcalls", "false").ok()?;
        flags.set("is_pic", "false").ok()?;
        flags.set("opt_level", opt_level).ok()?;
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
        cranelift_native::builder()
            .ok()?
            .finish(settings::Flags::new(flags))
            .ok()
    }

    /// The target an image built here names (decision AS1), or `None`
    /// when Cranelift generates no code for this machine.
    pub fn host_target() -> Option<String> {
        Jit::isa("none").map(|isa| image::target_of(&*isa))
    }

    /// The target this JIT generates code for.
    pub fn target(&self) -> String {
        image::target_of(&*self.isa)
    }

    pub fn opt_level(&self) -> &str {
        &self.opt_level
    }

    /// Every code object compiled, for an image (decision AS1): the
    /// machine code of each, or `None` for one the analysis left to the
    /// interpreter. Nothing is placed in executable memory.
    pub fn compile_everything(&mut self, program: &Program) -> Vec<Option<codegen::Compiled>> {
        (0..program.codes.len())
            .map(|code| match self.compile_code(program, code) {
                Ok(compiled) => {
                    self.compiled += 1;
                    self.ops += program.codes[code].ops.len();
                    Some(compiled)
                }
                Err(_) => {
                    self.skipped += 1;
                    None
                }
            })
            .collect()
    }

    /// The code of an image placed and made ready (decision AS1): every
    /// code object the image holds runs as machine code from its first
    /// call, the others stay with the interpreter. The image must have
    /// been built for this machine (`Header::mismatch`) from the same
    /// program.
    pub fn load_image(&mut self, image: &image::Image) -> Result<(), String> {
        if image.codes.len() != self.states.len() {
            return Err(format!(
                "the image holds {} code objects, the program {}",
                image.codes.len(),
                self.states.len()
            ));
        }
        let started = std::time::Instant::now();
        // the section mapped from the file (decision AT5), or placed as
        // compiled code is when the image is in memory or the system
        // refuses an executable mapping of the file
        let mapped = match &image.mapped {
            Some(mapped) => self
                .code
                .map_section(&mapped.file, mapped.offset, image.section_len)
                .ok(),
            None => None,
        };
        let base = match mapped {
            Some(base) => {
                self.mapped = true;
                base
            }
            None if image.mapped.is_some() => {
                let mapped = image
                    .mapped
                    .as_ref()
                    .expect("a file to read the section from");
                let mut section = vec![0u8; image.section_len];
                let mut reader = &mapped.file;
                reader
                    .seek(std::io::SeekFrom::Start(mapped.offset))
                    .and_then(|_| reader.read_exact(&mut section))
                    .map_err(|error| format!("cannot read the code section: {error}"))?;
                self.code.place(&section)?
            }
            None => self.code.place(&image.section)?,
        };
        self.placing += started.elapsed();
        for (index, code) in image.codes.iter().enumerate() {
            match code {
                None => {
                    self.states[index] = State::Skipped;
                    self.skipped += 1;
                }
                Some(code) => {
                    let body = base.wrapping_add(code.body.offset as usize);
                    let entry = base.wrapping_add(code.trampoline.offset as usize);
                    // SAFETY: the trampoline was compiled with
                    // `entry_signature`, which is the signature of `Entry`.
                    let entry: Entry = unsafe { std::mem::transmute::<*const u8, Entry>(entry) };
                    self.direct_table[index] = body;
                    self.deopts[index] = code.deopts.clone();
                    self.states[index] = State::Ready {
                        entry,
                        direct: body,
                        headers: code.headers.clone(),
                    };
                    self.loaded += 1;
                }
            }
        }
        Ok(())
    }

    /// What the VM keeps for the generated code to reach (decision AS1):
    /// the tables' addresses, which the boxes they live in keep fixed.
    pub(crate) fn state_pointers(&self) -> crate::vm::NativeState {
        crate::vm::NativeState {
            depth: 0,
            direct_table: self.direct_table.as_ptr(),
            helpers: self.helpers.as_ptr(),
            constants: self.constants.as_ptr(),
        }
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
        let loaded = if self.loaded > 0 {
            format!(
                "{} code objects loaded from the image ({}); ",
                self.loaded,
                if self.mapped {
                    "the section mapped from the file"
                } else {
                    "the section copied"
                }
            )
        } else {
            String::new()
        };
        format!(
            "native: {loaded}{} code objects compiled ({} ops, {} instructions in {} blocks) in {} ms (analysis {} ms, IR {} ms, cranelift {} ms, placing {} ms); {} left to the interpreter, {} called but cold",
            self.compiled,
            self.ops,
            stats.instructions,
            stats.blocks,
            (stats.analysis + stats.ir + stats.cranelift + self.placing).as_millis(),
            stats.analysis.as_millis(),
            stats.ir.as_millis(),
            stats.cranelift.as_millis(),
            self.placing.as_millis(),
            self.skipped,
            cold
        ) + &format!(
            "
native: {} deopts; {} calls from generated code went to the interpreter; {} loops entered from the interpreter, {} refused
{}",
            self.deopts_taken,
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

    /// The machine code of a code object, with its statistics counted;
    /// the reason when it stays with the interpreter, reported under
    /// `RENYI_NATIVE_REPORT`.
    fn compile_code(
        &mut self,
        program: &Program,
        code: CodeId,
    ) -> Result<codegen::Compiled, codegen::Skipped> {
        let compiled = codegen::compile(program, code, &*self.isa, &mut self.ctx, &mut self.fctx);
        match compiled {
            Ok(compiled) => {
                self.stats.add(&compiled.stats);
                Ok(compiled)
            }
            Err(reason) => {
                if std::env::var_os("RENYI_NATIVE_REPORT").is_some() {
                    eprintln!(
                        "native: {} stays with the interpreter: {reason:?}",
                        program.codes[code].name
                    );
                }
                Err(reason)
            }
        }
    }

    /// Compile a code object; its state becomes `Ready` or `Skipped`.
    fn compile(&mut self, program: &Program, code: CodeId) {
        let compiled = match self.compile_code(program, code) {
            Ok(compiled) => compiled,
            Err(_) => {
                self.states[code] = State::Skipped;
                self.skipped += 1;
                return;
            }
        };
        let started = std::time::Instant::now();
        let placed = self.code.place(&compiled.body).and_then(|body| {
            self.code
                .place(&compiled.trampoline)
                .map(|entry| (body, entry))
        });
        self.placing += started.elapsed();
        let (body, entry) = match placed {
            Ok(placed) => placed,
            Err(_) => {
                self.states[code] = State::Skipped;
                self.skipped += 1;
                return;
            }
        };
        // SAFETY: the trampoline was compiled with `entry_signature`, which
        // is the signature of `Entry`.
        let entry: Entry = unsafe { std::mem::transmute::<*const u8, Entry>(entry) };
        self.direct_table[code] = body;
        self.deopts[code] = compiled.deopts;
        self.states[code] = State::Ready {
            entry,
            direct: body,
            headers: compiled.headers,
        };
        self.compiled += 1;
        self.ops += program.codes[code].ops.len();
    }
}
