//! The `renyi` command: `check` (lexer, parser, type and effect checker and
//! layout diagnostics, text or JSON), `format` (canonical layout, in place or
//! `--check`), `tokens` (a token dump), `parse` (a syntax tree dump, as Rust
//! debug output or as JSON for tools), `index` (the project map, text or
//! JSON), `run` (check, then execute `main` on the VM, optionally replaying
//! a recording or narrating the run), `record` (run and write a recording of
//! every effect with the run manifest in its header), `reproduce` (replay a
//! recording under its manifest and compare), `test` (every `example:`
//! line and `test` block, with `replays` tests answered from their
//! recordings), `compile` (check, then write the program as a bytecode
//! file, which `run`, `record`, `test` and `reproduce` load in place of
//! the source when the path ends in `.ryc`, decision Z4), `build` (the
//! bytecode with the machine code of every function, an image the same
//! commands load in place of the source when the path ends in `.ryi`,
//! decision AS1), `mcp` (the
//! toolchain served to an agent host over standard input and output, in
//! `mcp.rs`), `lsp` (the language server for an editor, in `lsp.rs`) and
//! `serve` (`run` for a service; with `--watch`, reloaded between requests
//! when its files change, in `serve.rs`). `run`, `record` and `serve`
//! take `--sandbox <grant.json>`, a grant narrower than `main` declares
//! (decision AP1, in `sandbox.rs`).
//!
//! The crate is a library too (decision AK1): [`main_with`] is the whole
//! program of a binary built with extensions, and `src/main.rs`, the
//! official binary, calls it with none. An extension is an [`Extension`]
//! value of `renyi_vm` (decision AJ1; the guide is `docs/extensions.md`).
//! The embedding API (decision AP1; the guide is `docs/embedding.md`) is
//! [`Sandbox`]: a host loads a module with a [`Grant`] and calls its
//! public functions; [`Allocator`] is the global allocator a binary
//! declares, which counts for the memory budget.

mod bind;
mod exe;
mod lsp;
mod maps;
mod mcp;
mod packages;
mod sandbox;
mod serve;

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
use std::sync::OnceLock;

use renyi_check::effects::Capability;
use renyi_check::Library;
use renyi_syntax::diagnostics::{render_json, render_text};
use renyi_syntax::layout::check_layout;
use renyi_syntax::{format, lex, module_to_json, parse, parse_declarations, SourceFile, TokenKind};
use renyi_vm::grant::{parse_capability, Unit};
use renyi_vm::native::image::{self, Image};
use renyi_vm::recording::Dependency;
use renyi_vm::{file, Manifest};

pub use renyi_vm::memory::Counting;
pub use renyi_vm::{Extension, Native, Registry, Value};
pub use sandbox::{bytes_text, CallError, Function, Grant, Sandbox};

/// The allocator of a `renyi` binary (decisions X6 and AP1): mimalloc,
/// since the VM allocates a record, a list, a text or a frame's locals
/// at a time and the system allocator of Windows is slow at that,
/// wrapped in the counting of `renyi_vm::memory`, which the memory
/// budget of a sandbox needs. A binary declares it in one line,
/// `#[global_allocator] static ALLOCATOR: renyi::Allocator =
/// renyi::Allocator;`, as the official binary does; a host with an
/// allocator of its own wraps that one in [`Counting`] instead.
pub struct Allocator;

static COUNTING: Counting<mimalloc::MiMalloc> = Counting(mimalloc::MiMalloc);

unsafe impl std::alloc::GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        COUNTING.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        COUNTING.alloc_zeroed(layout)
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: std::alloc::Layout) {
        COUNTING.dealloc(pointer, layout)
    }

    unsafe fn realloc(
        &self,
        pointer: *mut u8,
        layout: std::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        COUNTING.realloc(pointer, layout, new_size)
    }
}

const USAGE: &str = "usage:
  renyi check [--json] [--strict] <file.ry>...
                                      report diagnostics (exit 1 when any error; --strict: a call
                                      to a deprecated definition is an error)
  renyi format [--check] <file.ry>... rewrite files in canonical layout (--check: report only)
  renyi tokens <file.ry>              dump the token stream
  renyi parse [--json] [--declarations] <file.ry>
                                      dump the syntax tree (--json: for tools; --declarations: a
                                      library declaration file, whose functions have no bodies)
  renyi index [--json] [path]         the project map of a directory or a file with its imports
  renyi index --budgets [path]        every value of the map over its budget (exit 0 either way)
  renyi index --diff <base> [--json] [path]
                                      what changed since a saved map (a file `renyi index --json` wrote)
                                      or a git revision: per definition, what it reaches, the version bump
  renyi tools [path]                  the manifest of every function with `expose as tool`: JSON Schema
                                      from the parameters, the description from `purpose:`, the
                                      permissions from `needs`
  renyi run [option...] <file.ry> [argument...]
                                      check the program, then run its `main` (exit 1 when it fails, 2 on a crash)
  renyi run --manifest [option...] <file.ry> [argument...]
                                      also print the run manifest on stderr: toolchain, extensions, code
                                      hash, grant, arguments, environment variables read, outcome, output hash
  renyi record [--to <file.json>] [option...] <file.ry> [argument...]
                                      run `main` and write a recording of its effects, the manifest in its
                                      header (default: <name>.recording.json)
  renyi serve [--watch] [option...] <file.ry> [argument...]
                                      `run` for a service; --watch: compile the program again when a file
                                      of its project changed and, when it checks clean, run `main` again
                                      between two requests with the listening socket kept open
  renyi reproduce <file.json> [<file.ry>]
                                      replay a recording under its manifest and compare the outcome and the
                                      output byte for byte (exit 1 when they differ)
  renyi test [--strict] [--refresh <name> [--redact <name>]] [--explain] [--interpret] <file.ry>...
                                      run every `example:` line and `test` block (exit 1 when any fails)
  renyi compile [--to <file.ryc>] <file.ry>
                                      check the program and write its bytecode (default: <name>.ryc);
                                      run, record, test and reproduce load a .ryc in place of a .ry
  renyi build [--exe] [--to <file>] [--opt speed|none] <file.ry>
                                      check the program, compile every function to machine code for this
                                      machine (Cranelift's `speed` level unless --opt none) and write the
                                      image (default: <name>.ryi); run, record, test and reproduce load a
                                      .ryi in place of a .ry and compile nothing; --exe: a self-contained
                                      executable instead (default: <name>), this binary with the image in
                                      it, which runs the program with its command line as the arguments
  renyi add <name> [<version>]        a dependency from the registry renyi.json names: choose the
                                      versions, fetch and verify the packages, print the effects of
                                      the package added, write renyi.json and renyi.lock.json
  renyi update [--accept-effects]     every dependency to the highest version its requirement allows;
                                      a version whose effects widen is refused without the flag
  renyi audit                         every dependency's effects against the `main` functions of the
                                      project, and the capabilities no dependency uses
  renyi fetch                         the locked packages from the registry, every hash verified
  renyi publish [--to <directory>]    the project into a directory registry as a new version
  renyi bind <header.h> --module <name> --library <name>[,<name>...] [--to <directory>]
                                      a C header as a foreign module: the declaration file <name>.ry
                                      and the module's entry in renyi.json (printed when there is none)
  renyi bind --python <package> [--module <name>] [--to <directory>]
                                      a Python package as a Python module, through the interpreter:
                                      the declaration file <name>.ry (the package's name when --module
                                      is left out) and the entry in renyi.json (printed when none)
  renyi mcp [path]                    serve the toolchain to an agent host over standard input and
                                      output (Model Context Protocol), for the directory given
  renyi lsp                           serve the language server to an editor over standard input and
                                      output (Language Server Protocol): diagnostics, hover,
                                      definition, document symbols
  renyi version                      the toolchain's version, then one line per extension built in
options of run and record:
  --explain                           narrate the run on stderr: purposes, arguments, results, effects
  --profile                           count every operation, call and primitive call and sample where the
                                      time goes; the report on stderr when the run ends
  --interpret                         run on the interpreter alone, never on the machine code the VM
                                      generates (also `test`); a narrated or profiled run does so by itself
  --replay <file.json>                run only: answer every effect from the recording; nothing is written or sent
  --deny <capability>                 refuse to start when any function needs the capability
  --allow-host <host>                 narrow network.http to one host
  --allow-read <path>                 narrow filesystem.read to a path prefix
  --allow-write <path>                narrow filesystem.write to a path prefix
  --at-most <capability>=<count>/<unit>
                                      add a budget; the unit is second, minute, hour, day or run
  --redact <name>                     record and test --refresh: replace the argument, header or
                                      environment variable of that name with a placeholder
  --sandbox <grant.json>              run, record and serve: a grant narrower than `main` declares, as
                                      {\"grant\": \"console, network.http(\\\"host\\\") at most 60 per minute\",
                                      \"memory\": \"256 megabytes\"}: the program's grant intersected with it,
                                      its budgets and guards added, the bytes held above the start bounded";

/// What this binary was built with (decision AK1): the standard library
/// and the extensions `main_with` was given, set once before any command
/// runs; a process has one toolchain, as it has one version.
static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// The registry of this binary: the standard library alone when
/// `main_with` was not called, as in the crate's own tests.
pub(crate) fn registry() -> &'static Registry {
    REGISTRY.get_or_init(Registry::standard)
}

/// The declaration files this binary checks against.
pub(crate) fn library() -> Library {
    registry().library()
}

/// The whole program of a `renyi` binary: the commands of the module
/// documentation over the standard library and the extensions given
/// (decision AK1). A binary with extensions verifies them first (decision
/// AK2) and does not start when one fails; the message names the
/// function declared without a native or implemented without a
/// declaration.
pub fn main_with(extensions: Vec<Extension>) -> ExitCode {
    let verify = !extensions.is_empty();
    let mut registry = Registry::standard();
    for extension in extensions {
        registry.add(extension);
    }
    if verify {
        if let Err(problem) = registry.verify() {
            eprintln!("renyi: {problem}");
            return ExitCode::FAILURE;
        }
    }
    if REGISTRY.set(registry).is_err() {
        eprintln!("renyi: `main_with` was called twice");
        return ExitCode::FAILURE;
    }
    // the machine code of decision AG1 nests calls on the machine stack: a
    // thread with room for them, since the main thread's stack is small on
    // Windows (the size is reserved, not committed)
    let embedded = exe::embedded_image();
    let worker = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || match embedded {
            Some(image) => run_embedded(image),
            None => dispatch(),
        })
        .expect("a thread for the command");
    worker.join().unwrap_or(ExitCode::FAILURE)
}

/// The image a self-contained executable carries (decision AS4), run as
/// `renyi run <image> <arguments>` runs it: the whole command line is the
/// program's arguments, and the executable's name stands for the path.
fn run_embedded(embedded: (std::fs::File, u64, usize)) -> ExitCode {
    let name = std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "program".to_string());
    let (file, offset, length) = embedded;
    let (program, image) = match Image::open_at(file, offset, length)
        .map_err(|detail| format!("{name}: {detail}"))
        .and_then(|loaded| check_image(loaded, &name))
    {
        Ok(loaded) => loaded,
        Err(message) => {
            eprintln!("renyi: {message}");
            return ExitCode::FAILURE;
        }
    };
    let mut rest: Vec<String> = vec![name.clone()];
    rest.extend(std::env::args().skip(1));
    let hash = image.code_hash.clone();
    run_loaded(
        &name,
        program,
        Hashed::Given(hash),
        Some(image),
        Flags::default(),
        None,
        &rest,
        false,
    )
}

fn dispatch() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") => check(&args[1..]),
        Some("format") => format_command(&args[1..]),
        Some("tokens") => tokens(&args[1..]),
        Some("parse") => parse_command(&args[1..]),
        Some("index") => index_command(&args[1..]),
        Some("tools") => tools_command(&args[1..]),
        Some("run") => run_command(&args[1..], false),
        Some("record") => run_command(&args[1..], true),
        Some("serve") => serve::serve_command(&args[1..]),
        Some("reproduce") => reproduce_command(&args[1..]),
        Some("test") => test_command(&args[1..]),
        Some("compile") => compile_command(&args[1..]),
        Some("build") => build_command(&args[1..]),
        Some("add") => packages::add_command(&args[1..]),
        Some("update") => packages::update_command(&args[1..]),
        Some("audit") => packages::audit_command(&args[1..]),
        Some("fetch") => packages::fetch_command(&args[1..]),
        Some("publish") => packages::publish_command(&args[1..]),
        Some("bind") => bind::bind_command(&args[1..]),
        Some("mcp") => mcp::serve(&args[1..]),
        Some("lsp") => lsp::serve(&args[1..]),
        Some("version") | Some("--version") => {
            println!("renyi {}", env!("CARGO_PKG_VERSION"));
            for extension in registry().extras() {
                println!("extension {extension}");
            }
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// A source file; the error names the path.
pub(crate) fn read_source(path: &str) -> Result<SourceFile, String> {
    std::fs::read_to_string(path)
        .map(|text| SourceFile::new(path, text))
        .map_err(|error| format!("cannot read {path}: {error}"))
}

fn load(path: &str) -> Result<SourceFile, ExitCode> {
    read_source(path).map_err(|message| {
        eprintln!("renyi: {message}");
        ExitCode::FAILURE
    })
}

/// Every diagnostic of a file: the parser's, the checker's when the file
/// parses, and the layout's, in source order.
pub(crate) fn diagnose(file: &SourceFile) -> Vec<renyi_syntax::Diagnostic> {
    // a foreign module (decision AF1) or a Python module (decision AL1) of
    // the project declares: no bodies
    let file = renyi_check::tagged(file.clone());
    let parsed = if file.foreign.is_some() || file.python.is_some() {
        parse_declarations(&file.text)
    } else {
        parse(&file.text)
    };
    let mut diagnostics = parsed.diagnostics;
    if !diagnostics.iter().any(|diagnostic| diagnostic.is_error()) {
        diagnostics.extend(renyi_check::check_file_in(&library(), &file));
    }
    diagnostics.extend(check_layout(&file));
    diagnostics.sort_by_key(|diagnostic| diagnostic.span.start);
    diagnostics
}

fn check(args: &[String]) -> ExitCode {
    let json = args.iter().any(|arg| arg == "--json");
    let strict = args.iter().any(|arg| arg == "--strict");
    if let Some(unknown) = args
        .iter()
        .find(|arg| arg.starts_with("--") && *arg != "--json" && *arg != "--strict")
    {
        eprintln!("renyi: unknown option `{unknown}`");
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    let files: Vec<&String> = args.iter().filter(|arg| !arg.starts_with("--")).collect();
    if files.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    let mut failed = false;
    for path in files {
        let file = match load(path) {
            Ok(file) => file,
            Err(code) => return code,
        };
        let mut diagnostics = diagnose(&file);
        if strict {
            // decision C8c, tier 2: a call to a deprecated definition is an error
            for diagnostic in &mut diagnostics {
                if diagnostic.code == "deprecated" {
                    diagnostic.severity = renyi_syntax::diagnostics::Severity::Error;
                }
            }
        }
        failed |= diagnostics.iter().any(|diagnostic| diagnostic.is_error());
        if json {
            print!("{}", render_json(&file, &diagnostics));
        } else {
            print!("{}", render_text(&file, &diagnostics));
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn tokens(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let file = match load(path) {
        Ok(file) => file,
        Err(code) => return code,
    };
    let lexed = lex(&file.text);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for token in &lexed.tokens {
        let position = file.position(token.span.start);
        let shown = match &token.kind {
            TokenKind::Newline => String::new(),
            TokenKind::Text { .. }
            | TokenKind::RawText(_)
            | TokenKind::ClauseText(_)
            | TokenKind::Comment => {
                format!("{:?}", token.text(&file.text))
            }
            _ => token.text(&file.text).to_string(),
        };
        let line = format!(
            "{:>4}:{:<3} {:<22} {}",
            position.line,
            position.column,
            token.kind_name(),
            shown
        );
        if writeln!(out, "{line}").is_err() {
            return ExitCode::SUCCESS; // the reader went away, as with `| head`
        }
    }
    let _ = write!(out, "{}", render_text(&file, &lexed.diagnostics));
    if lexed
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.is_error())
    {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn parse_command(args: &[String]) -> ExitCode {
    let json = args.iter().any(|arg| arg == "--json");
    // a library declaration file: functions have no bodies (`library/std/`)
    let declarations = args.iter().any(|arg| arg == "--declarations");
    let Some(path) = args.iter().find(|arg| !arg.starts_with("--")) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let file = match load(path) {
        Ok(file) => file,
        Err(code) => return code,
    };
    let parsed = if declarations {
        parse_declarations(&file.text)
    } else {
        parse(&file.text)
    };
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if json {
        // the tree first, then the diagnostics, each a complete JSON document on its own
        let _ = write!(out, "{}", module_to_json(&file, &parsed.module));
        if !parsed.diagnostics.is_empty() {
            let _ = write!(out, "{}", render_json(&file, &parsed.diagnostics));
        }
    } else {
        let _ = writeln!(out, "{:#?}", parsed.module);
        let _ = write!(out, "{}", render_text(&file, &parsed.diagnostics));
    }
    if parsed
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.is_error())
    {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn format_command(args: &[String]) -> ExitCode {
    let check_only = args.iter().any(|arg| arg == "--check");
    let files: Vec<&String> = args.iter().filter(|arg| !arg.starts_with("--")).collect();
    if files.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    let mut failed = false;
    for path in files {
        let file = match load(path) {
            Ok(file) => file,
            Err(code) => return code,
        };
        match format(&file) {
            Ok(formatted) => {
                if formatted == file.text {
                    continue;
                }
                if check_only {
                    println!("{path}: not in canonical layout");
                    failed = true;
                } else if let Err(error) = std::fs::write(path, formatted) {
                    eprintln!("renyi: cannot write {path}: {error}");
                    failed = true;
                } else {
                    println!("{path}: formatted");
                }
            }
            Err(diagnostics) => {
                print!("{}", render_text(&file, &diagnostics));
                failed = true;
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Check a file with its imports and compile it, or load a bytecode file
/// (`.ryc`, decision Z4); diagnostics go to stdout as `check` prints them,
/// and an error stops here.
fn compile(path: &str) -> Result<(renyi_vm::Program, Option<Image>), ExitCode> {
    compile_with_sources(path).map(|(program, _, image)| (program, image))
}

/// What the manifest's code hash is computed from: the source files of a
/// program compiled here, or the text of the bytecode file it was loaded
/// from.
enum Hashed {
    Sources(Vec<SourceFile>),
    File(String),
    /// The hash itself, as an image stores its bytecode file's (decision
    /// AT3).
    Given(String),
}

/// `compile`, with what the manifest's code hash is computed from.
fn compile_with_sources(
    path: &str,
) -> Result<(renyi_vm::Program, Hashed, Option<Image>), ExitCode> {
    if image::is_image(path) {
        return match load_image_file(path) {
            Ok((program, loaded)) => {
                let hash = loaded.code_hash.clone();
                Ok((program, Hashed::Given(hash), Some(loaded)))
            }
            Err(message) => {
                eprintln!("renyi: {message}");
                Err(ExitCode::FAILURE)
            }
        };
    }
    if file::is_bytecode(path) {
        return match load_bytecode(path) {
            Ok((program, text)) => Ok((program, Hashed::File(text), None)),
            Err(message) => {
                eprintln!("renyi: {message}");
                Err(ExitCode::FAILURE)
            }
        };
    }
    match compile_sources(path) {
        Ok(compiled) => {
            print!("{}", compiled.diagnostics);
            Ok((compiled.program, Hashed::Sources(compiled.sources), None))
        }
        Err(CompileError::Read(message)) => {
            eprintln!("renyi: {message}");
            Err(ExitCode::FAILURE)
        }
        Err(CompileError::Diagnostics(text)) => {
            print!("{text}");
            Err(ExitCode::FAILURE)
        }
    }
}

/// The modules of a program whose functions are bound outside Renyi, those
/// `bound` selects: each in backticks, in name order.
fn bound_modules(
    program: &renyi_vm::Program,
    bound: impl Fn(&renyi_vm::compile::FunctionMeta) -> bool,
) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for meta in &program.function_metas {
        if bound(meta) && !names.contains(&meta.module) {
            names.push(meta.module.clone());
        }
    }
    names.sort();
    names.into_iter().map(|name| format!("`{name}`")).collect()
}

/// Whether `main` grants the capability, with any scope.
fn grants(program: &renyi_vm::Program, kind: &str) -> bool {
    program.main.is_some_and(|id| {
        program.function_metas[id]
            .needs
            .iter()
            .any(|capability| capability.path == [kind])
    })
}

/// A program from an image file (decision AS1), with the image to load
/// its machine code from; the error names the path and what is wrong, or
/// why the image cannot run here with the fix (build again).
fn load_image_file(path: &str) -> Result<(renyi_vm::Program, Image), String> {
    let loaded = Image::open(path).map_err(|detail| format!("{path}: {detail}"))?;
    check_image(loaded, path)
}

/// An image checked against this machine and its program decoded, the
/// image named `path` in the messages.
fn check_image(loaded: Image, path: &str) -> Result<(renyi_vm::Program, Image), String> {
    let target = renyi_vm::native::Jit::host_target().ok_or_else(|| {
        format!("{path}: this machine generates no machine code; run the bytecode instead (`renyi compile`)")
    })?;
    if let Some(mismatch) = loaded.header.mismatch(&target) {
        return Err(format!("{path}: {mismatch}"));
    }
    let program =
        renyi_vm::binary::decode(&loaded.program).map_err(|detail| format!("{path}: {detail}"))?;
    Ok((program, loaded))
}

/// `renyi build [--exe] [--to <file>] [--opt speed|none] <file.ry>`:
/// check the program with its imports, compile every function to machine
/// code for this machine (at Cranelift's `speed` level unless asked
/// otherwise, decision AS3) and write the image (decision AS1), or with
/// `--exe` the self-contained executable that carries it (decision AS4).
fn build_command(args: &[String]) -> ExitCode {
    let mut to: Option<String> = None;
    let mut opt: Option<String> = None;
    let mut exe = false;
    let mut path: Option<&String> = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--exe" => exe = true,
            "--to" => match rest.next() {
                Some(value) => to = Some(value.clone()),
                None => {
                    eprintln!("renyi: `--to` needs a value");
                    return ExitCode::FAILURE;
                }
            },
            "--opt" => match rest.next() {
                Some(value) if value == "none" || value == "speed" => opt = Some(value.clone()),
                Some(value) => {
                    eprintln!("renyi: `--opt` takes `none` or `speed`, not `{value}`");
                    return ExitCode::FAILURE;
                }
                None => {
                    eprintln!("renyi: `--opt` needs a value");
                    return ExitCode::FAILURE;
                }
            },
            other if other.starts_with("--") => {
                eprintln!("renyi: unknown option `{other}`");
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            }
            _ => path = Some(arg),
        }
    }
    let Some(path) = path else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    if image::is_image(path) {
        eprintln!("renyi: {path} is an image already; `build` takes a .ry or a .ryc file");
        return ExitCode::FAILURE;
    }
    let (program, _, _) = match compile_with_sources(path) {
        Ok(compiled) => compiled,
        Err(code) => return code,
    };
    let built = match image::build(&program, opt.as_deref()) {
        Ok(built) => built,
        Err(message) => {
            eprintln!("renyi: {message}");
            return ExitCode::FAILURE;
        }
    };
    let stem = Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "program".to_string());
    let target = to.unwrap_or_else(|| {
        if exe {
            format!("{stem}{}", std::env::consts::EXE_SUFFIX)
        } else {
            format!("{stem}.{}", image::EXTENSION)
        }
    });
    let bytes = built.write();
    let compiled = built.codes.iter().flatten().count();
    if exe {
        return match exe::write(Path::new(&target), &bytes) {
            Ok(note) => {
                eprintln!(
                    "renyi: built {path} to {target}: a self-contained executable, {compiled} of {} code objects as machine code, the image {} bytes",
                    built.codes.len(),
                    bytes.len()
                );
                if let Some(note) = note {
                    eprintln!("renyi: {note}");
                }
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("renyi: {message}");
                ExitCode::FAILURE
            }
        };
    }
    match std::fs::write(&target, &bytes) {
        Ok(()) => {
            eprintln!(
                "renyi: built {path} to {target}: {compiled} of {} code objects as machine code, {} bytes",
                built.codes.len(),
                bytes.len()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("renyi: cannot write {target}: {error}");
            ExitCode::FAILURE
        }
    }
}

/// A program from a bytecode file, with the file's text; the error names
/// the path and the place in the file.
fn load_bytecode(path: &str) -> Result<(renyi_vm::Program, String), String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let program = file::load(&text).map_err(|detail| format!("{path}: {detail}"))?;
    Ok((program, text))
}

/// `renyi compile [--to <file.ryc>] <file.ry>`: check the program with its
/// imports and write it as a bytecode file (decision Z4).
fn compile_command(args: &[String]) -> ExitCode {
    let mut to: Option<String> = None;
    let mut path: Option<&String> = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--to" => match rest.next() {
                Some(value) => to = Some(value.clone()),
                None => {
                    eprintln!("renyi: `--to` needs a value");
                    return ExitCode::FAILURE;
                }
            },
            other if other.starts_with("--") => {
                eprintln!("renyi: unknown option `{other}`");
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            }
            _ => path = Some(arg),
        }
    }
    let Some(path) = path else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    if file::is_bytecode(path) {
        eprintln!("renyi: {path} is a bytecode file already; `compile` takes a .ry file");
        return ExitCode::FAILURE;
    }
    let compiled = match compile_sources(path) {
        Ok(compiled) => compiled,
        Err(CompileError::Read(message)) => {
            eprintln!("renyi: {message}");
            return ExitCode::FAILURE;
        }
        Err(CompileError::Diagnostics(text)) => {
            print!("{text}");
            return ExitCode::FAILURE;
        }
    };
    print!("{}", compiled.diagnostics);
    let target = to.unwrap_or_else(|| {
        let stem = Path::new(path)
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_else(|| "program".to_string());
        format!("{stem}.{}", file::EXTENSION)
    });
    match std::fs::write(&target, file::render(&compiled.program)) {
        Ok(()) => {
            eprintln!("renyi: compiled {path} to {target}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("renyi: cannot write {target}: {error}");
            ExitCode::FAILURE
        }
    }
}

/// A program checked and compiled, with its sources and the warnings the
/// checker reported, rendered as `check` prints them (empty when none).
pub(crate) struct Compiled {
    pub program: renyi_vm::Program,
    pub sources: Vec<SourceFile>,
    pub diagnostics: String,
}

pub(crate) enum CompileError {
    /// The file could not be read.
    Read(String),
    /// The program has errors, rendered as `check` prints them.
    Diagnostics(String),
}

/// Check a file with its imports and compile it, printing nothing: the
/// commands print what comes back, the MCP server answers with it.
pub(crate) fn compile_sources(path: &str) -> Result<Compiled, CompileError> {
    let file = read_source(path).map_err(CompileError::Read)?;
    compile_file(file)
}

/// `compile_sources` from a file already read, or a text in memory (the
/// sandbox of decision AP1): its imports are resolved from its name.
pub(crate) fn compile_file(file: SourceFile) -> Result<Compiled, CompileError> {
    let resolved = renyi_check::resolve(&file);
    let files = resolved.files;
    let checked = renyi_check::check_project_in(&library(), &files, &resolved.problems);
    let mut failed = false;
    let mut diagnostics = String::new();
    for module in &checked.modules {
        if module.diagnostics.is_empty() {
            continue;
        }
        failed |= module.diagnostics.iter().any(|d| d.is_error());
        diagnostics.push_str(&render_text(&files[module.file], &module.diagnostics));
    }
    if failed {
        return Err(CompileError::Diagnostics(diagnostics));
    }
    Ok(Compiled {
        program: renyi_vm::compile_project(&checked, &files),
        sources: files,
        diagnostics,
    })
}

pub(crate) fn toolchain() -> String {
    format!("renyi {}", env!("CARGO_PKG_VERSION"))
}

/// The content hash of the manifest: of `main` from the project map, which
/// covers every definition `main` reaches (design document 05, section 3),
/// or of the whole bytecode file the program was loaded from (decision Z4).
fn code_hash(program: &renyi_vm::Program, hashed: &Hashed) -> Option<String> {
    match hashed {
        Hashed::Sources(files) => main_hash(program, files),
        Hashed::File(text) => Some(renyi_vm::recording::sha256_of(text.as_bytes())),
        Hashed::Given(hash) => Some(hash.clone()),
    }
}

/// The dependencies of a program compiled from its sources: every package
/// among them, with the version and the hash the lockfile names (decision
/// AC1). A program loaded from a bytecode file has none to name: the
/// file's hash covers them.
fn dependencies_of(path: &str, hashed: &Hashed) -> Vec<Dependency> {
    let Hashed::Sources(files) = hashed else {
        return Vec::new();
    };
    let Some(lock) = renyi_package::Project::of(path).lock else {
        return Vec::new();
    };
    let mut dependencies: Vec<Dependency> = Vec::new();
    for file in files {
        let Some(package) = &file.package else {
            continue;
        };
        if dependencies.iter().any(|known| known.name == package.name) {
            continue;
        }
        if let Some(locked) = lock.get(&package.name) {
            dependencies.push(Dependency {
                name: package.name.clone(),
                version: locked.version.to_string(),
                hash: locked.hash.clone(),
            });
        }
    }
    dependencies.sort_by(|a, b| a.name.cmp(&b.name));
    dependencies
}

fn describe_dependencies(dependencies: &[Dependency]) -> String {
    if dependencies.is_empty() {
        return "none".to_string();
    }
    dependencies
        .iter()
        .map(|d| format!("{} {} {}", d.name, d.version, d.hash))
        .collect::<Vec<_>>()
        .join(", ")
}

fn main_hash(program: &renyi_vm::Program, files: &[SourceFile]) -> Option<String> {
    let main = program.main?;
    let module = &program.function_metas[main].module;
    let header = renyi_index::Header {
        project: String::new(),
        revision: String::new(),
        toolchain: toolchain(),
    };
    let index = renyi_index::index_files_in(&library(), files, header);
    index
        .definitions
        .iter()
        .find(|definition| definition.module == *module && definition.name == "main")
        .map(|definition| definition.id.clone())
}

/// The options of `run`, `record` and `test`, read up to the first
/// positional argument.
#[derive(Default)]
struct Flags {
    narrowing: renyi_vm::Narrowing,
    explain: bool,
    profile: bool,
    interpret: bool,
    replay: Option<String>,
    to: Option<String>,
    strict: bool,
    refresh: Option<String>,
    redact: Vec<String>,
    manifest: bool,
    /// `renyi serve --watch`.
    watch: bool,
    /// `--sandbox <grant.json>` (decision AP1).
    sandbox: Option<String>,
}

fn parse_flags(args: &[String]) -> Result<(Flags, &[String]), String> {
    let mut flags = Flags::default();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        if !arg.starts_with("--") {
            break;
        }
        let value = || {
            args.get(index + 1)
                .map(String::as_str)
                .ok_or_else(|| format!("`{arg}` needs a value"))
        };
        match arg.as_str() {
            "--explain" => {
                flags.explain = true;
                index += 1;
                continue;
            }
            "--profile" => {
                flags.profile = true;
                index += 1;
                continue;
            }
            "--interpret" => {
                flags.interpret = true;
                index += 1;
                continue;
            }
            "--strict" => {
                flags.strict = true;
                index += 1;
                continue;
            }
            "--manifest" => {
                flags.manifest = true;
                index += 1;
                continue;
            }
            "--watch" => {
                flags.watch = true;
                index += 1;
                continue;
            }
            "--replay" => flags.replay = Some(value()?.to_string()),
            "--to" => flags.to = Some(value()?.to_string()),
            "--refresh" => flags.refresh = Some(value()?.to_string()),
            "--sandbox" => flags.sandbox = Some(value()?.to_string()),
            "--redact" => flags.redact.push(value()?.to_string()),
            "--deny" => flags.narrowing.deny.push(parse_capability(value()?)?),
            "--allow-host" => flags
                .narrowing
                .allow
                .push(scoped(&["network", "http"], value()?)),
            "--allow-read" => flags
                .narrowing
                .allow
                .push(scoped(&["filesystem", "read"], value()?)),
            "--allow-write" => flags
                .narrowing
                .allow
                .push(scoped(&["filesystem", "write"], value()?)),
            "--at-most" => flags.narrowing.budgets.push(parse_budget(value()?)?),
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 2;
    }
    Ok((flags, &args[index..]))
}

pub(crate) fn scoped(path: &[&str], scope: &str) -> Capability {
    Capability {
        path: path.iter().map(|part| part.to_string()).collect(),
        scope: Some(scope.to_string()),
        budget: None,
        only_to: Vec::new(),
    }
}

/// `network.http=60/minute`, or with a scope, `network.http("host")=60/minute`.
pub(crate) fn parse_budget(text: &str) -> Result<Capability, String> {
    let (capability, budget) = text
        .rsplit_once('=')
        .ok_or_else(|| format!("`{text}`: a budget is <capability>=<count>/<unit>"))?;
    let (count, unit) = budget
        .split_once('/')
        .ok_or_else(|| format!("`{text}`: a budget is <capability>=<count>/<unit>"))?;
    if count.parse::<u64>().is_err() {
        return Err(format!(
            "`{text}`: the count `{count}` is not a whole number"
        ));
    }
    if Unit::parse(unit).is_none() {
        return Err(format!(
            "`{text}`: the unit `{unit}` is not second, minute, hour, day or run"
        ));
    }
    let mut capability = parse_capability(capability)?;
    if !matches!(
        capability.path.first().map(String::as_str),
        Some("network" | "process" | "filesystem")
    ) {
        return Err(format!(
            "`{}` takes no budget; budgets apply to network, process and filesystem",
            capability.spelling()
        ));
    }
    capability.budget = Some((count.to_string(), unit.to_string()));
    Ok(capability)
}

pub(crate) fn load_recording(path: &str) -> Result<renyi_vm::Recording, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    renyi_vm::Recording::parse(&text).map_err(|detail| format!("{path}: {detail}"))
}

/// `renyi run` and, with `record`, `renyi record`.
fn run_command(args: &[String], record: bool) -> ExitCode {
    let (mut flags, rest) = match parse_flags(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("renyi: {message}");
            return ExitCode::FAILURE;
        }
    };
    let memory = match sandbox::apply_flag(&mut flags) {
        Ok(memory) => memory,
        Err(message) => {
            eprintln!("renyi: {message}");
            return ExitCode::FAILURE;
        }
    };
    let Some(path) = rest.first() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    if flags.strict || flags.refresh.is_some() {
        eprintln!("renyi: `--strict` and `--refresh` are options of `renyi test`");
        return ExitCode::FAILURE;
    }
    if flags.watch {
        eprintln!("renyi: `--watch` is an option of `renyi serve`");
        return ExitCode::FAILURE;
    }
    if record && flags.replay.is_some() {
        eprintln!("renyi: `record` cannot `--replay`; a recording comes from a live run");
        return ExitCode::FAILURE;
    }
    if !record && !flags.redact.is_empty() {
        eprintln!("renyi: `--redact` is an option of `renyi record` and `renyi test --refresh`");
        return ExitCode::FAILURE;
    }
    let (program, sources, image) = match compile_with_sources(path) {
        Ok(compiled) => compiled,
        Err(code) => return code,
    };
    run_loaded(path, program, sources, image, flags, memory, rest, record)
}

/// `run` or `record` of a program loaded: the checks on what it may
/// reach, the manifest, the run, the recording written, the exit status.
/// `rest[0]` is the path (or the executable's name, decision AS4),
/// `rest[1..]` the program's arguments.
#[allow(clippy::too_many_arguments)]
fn run_loaded(
    path: &str,
    program: renyi_vm::Program,
    sources: Hashed,
    image: Option<Image>,
    flags: Flags,
    memory: Option<u64>,
    rest: &[String],
    record: bool,
) -> ExitCode {
    // native code and Python are visible (decisions AF1 and AJ2,
    // 07-system-design.md section 2.2)
    let native = bound_modules(&program, |meta| meta.foreign.is_some());
    if !native.is_empty() && grants(&program, "foreign") {
        eprintln!(
            "renyi: this program can call native code through {}",
            native.join(", ")
        );
    }
    let python = bound_modules(&program, |meta| meta.python.is_some());
    if !python.is_empty() && grants(&program, "python") {
        eprintln!(
            "renyi: this program can run Python through {}",
            python.join(", ")
        );
    }
    for denied in &flags.narrowing.deny {
        let functions = renyi_vm::denied_functions(&program, denied);
        if !functions.is_empty() {
            eprintln!(
                "renyi: `{}` is denied, but these functions need it: {}",
                denied.spelling(),
                functions.join(", ")
            );
            return ExitCode::FAILURE;
        }
    }
    if let Some(message) = sandbox::refusal(&program, &flags.narrowing) {
        eprintln!("renyi: {message}");
        return ExitCode::FAILURE;
    }
    let replay = match &flags.replay {
        Some(file) => match load_recording(file) {
            Ok(recording) => Some(recording),
            Err(message) => {
                eprintln!("renyi: {message}");
                return ExitCode::FAILURE;
            }
        },
        None => None,
    };
    let with_manifest = record || flags.manifest;
    let revision = with_manifest.then(|| renyi_index::git_revision(Path::new(path)));
    let manifest = if with_manifest {
        Manifest {
            toolchain: Some(toolchain()),
            source: Some(path.to_string()),
            code: code_hash(&program, &sources),
            dependencies: dependencies_of(path, &sources),
            extensions: registry().extras(),
            ..Manifest::default()
        }
    } else {
        Manifest::default()
    };
    let options = renyi_vm::Options {
        image,
        arguments: rest[1..].to_vec(),
        narrowing: flags.narrowing,
        record: with_manifest,
        replay,
        revision: revision.filter(|text| text != "unknown"),
        explain: flags.explain,
        profile: flags.profile,
        interpret: flags.interpret,
        redact: flags.redact,
        manifest,
        registry: registry().clone(),
        memory,
        ..renyi_vm::Options::default()
    };
    let run = renyi_vm::run_program(&program, options);
    if let (Some(recording), false) = (&run.recording, record) {
        eprint!("{}", recording.render_manifest());
    }
    if let (Some(recording), true) = (&run.recording, record) {
        let target = flags.to.clone().unwrap_or_else(|| {
            let stem = Path::new(path)
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_else(|| "program".to_string());
            format!("{stem}.recording.json")
        });
        match std::fs::write(&target, recording.render()) {
            Ok(()) => eprintln!(
                "renyi: recorded {} call{} to {target}",
                recording.calls.len(),
                if recording.calls.len() == 1 { "" } else { "s" }
            ),
            Err(error) => {
                eprintln!("renyi: cannot write {target}: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    if !run.unused.is_empty() {
        eprintln!(
            "renyi: {} recorded call{} not reached: {}",
            run.unused.len(),
            if run.unused.len() == 1 {
                " was"
            } else {
                "s were"
            },
            run.unused.join("; ")
        );
    }
    exit_of(path, run.outcome)
}

/// How a run of `main` ended, as the exit status of appendix B: 0 when it
/// finished, 1 when it failed, 2 on a crash, the code of
/// `environment.exit`; a run stopped for a new version is the business
/// of `renyi serve --watch`, which handles it before asking here.
pub(crate) fn exit_of(path: &str, outcome: renyi_vm::RunOutcome) -> ExitCode {
    match outcome {
        renyi_vm::RunOutcome::Finished => ExitCode::SUCCESS,
        renyi_vm::RunOutcome::Failed(error) => {
            eprintln!("{path}: main failed with {error}");
            ExitCode::FAILURE
        }
        renyi_vm::RunOutcome::Crashed { message, location } => {
            match location {
                Some(location) => eprintln!("{path}: crash: {message}\n  at {location}"),
                None => eprintln!("{path}: crash: {message}"),
            }
            ExitCode::from(2)
        }
        renyi_vm::RunOutcome::Exited(code) => ExitCode::from(code.clamp(0, 255) as u8),
        renyi_vm::RunOutcome::Reload => {
            eprintln!("{path}: the run stopped for a new version, which nothing watched for");
            ExitCode::FAILURE
        }
        renyi_vm::RunOutcome::OverMemory { limit, used } => {
            eprintln!(
                "{path}: the memory budget of {} was exceeded ({} held)",
                bytes_text(limit),
                bytes_text(used)
            );
            ExitCode::from(2)
        }
    }
}

fn test_command(args: &[String]) -> ExitCode {
    let (flags, files) = match parse_flags(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("renyi: {message}");
            return ExitCode::FAILURE;
        }
    };
    if files.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    let narrowing = &flags.narrowing;
    if flags.replay.is_some()
        || flags.to.is_some()
        || flags.manifest
        || flags.profile
        || flags.watch
        || flags.sandbox.is_some()
        || !narrowing.deny.is_empty()
        || !narrowing.allow.is_empty()
        || !narrowing.budgets.is_empty()
    {
        eprintln!(
            "renyi: `renyi test` takes only `--strict`, `--refresh <name>`, `--redact <name>`, `--explain` and `--interpret`"
        );
        return ExitCode::FAILURE;
    }
    let mut failed = false;
    for path in files {
        let (program, image) = match compile(path) {
            Ok(compiled) => compiled,
            Err(code) => return code,
        };
        let options = renyi_vm::Options {
            explain: flags.explain,
            interpret: flags.interpret,
            image,
            strict: flags.strict,
            refresh: flags.refresh.clone(),
            redact: flags.redact.clone(),
            registry: registry().clone(),
            ..renyi_vm::Options::default()
        };
        let report = renyi_vm::run_tests(&program, options);
        print!("{}", report.render());
        failed |= report.failed() > 0;
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// `renyi reproduce <recording> [<program>]`: the code must hash as the
/// manifest says, the toolchain should, and then the run is replayed and
/// compared (decision Q2).
fn reproduce_command(args: &[String]) -> ExitCode {
    let Some(file) = args.first() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let recording = match load_recording(file) {
        Ok(recording) => recording,
        Err(message) => {
            eprintln!("renyi: {message}");
            return ExitCode::FAILURE;
        }
    };
    let Some(code) = recording.manifest.code.clone() else {
        eprintln!("renyi: {file} has no manifest; record the run again with `renyi record`");
        return ExitCode::FAILURE;
    };
    let Some(path) = args
        .get(1)
        .cloned()
        .or_else(|| recording.manifest.source.clone())
    else {
        eprintln!("renyi: {file} does not name its program; give the path of the .ry file");
        return ExitCode::FAILURE;
    };
    let (program, sources, image) = match compile_with_sources(&path) {
        Ok(compiled) => compiled,
        Err(code) => return code,
    };
    let now = code_hash(&program, &sources).unwrap_or_default();
    if now != code {
        eprintln!(
            "renyi: the code differs from the manifest: `main` of {path} is {now}, the recording was made from {code}"
        );
        return ExitCode::FAILURE;
    }
    let dependencies = dependencies_of(&path, &sources);
    if dependencies != recording.manifest.dependencies {
        eprintln!(
            "renyi: the dependencies differ from the manifest: the recording names {}, this run has {}",
            describe_dependencies(&recording.manifest.dependencies),
            describe_dependencies(&dependencies)
        );
        return ExitCode::FAILURE;
    }
    if let Some(recorded) = &recording.manifest.toolchain {
        if *recorded != toolchain() {
            eprintln!(
                "renyi: warning: the recording was made with {recorded}, this is {}",
                toolchain()
            );
        }
    }
    let extensions = registry().extras();
    if recording.manifest.extensions != extensions {
        eprintln!(
            "renyi: warning: the recording was made with {}, this renyi has {}",
            describe_extensions(&recording.manifest.extensions),
            describe_extensions(&extensions)
        );
    }
    let options = renyi_vm::Options {
        image,
        registry: registry().clone(),
        ..renyi_vm::Options::default()
    };
    let reproduction = renyi_vm::reproduce(&program, recording, options);
    match &reproduction.run.outcome {
        renyi_vm::RunOutcome::Failed(error) => eprintln!("{path}: main failed with {error}"),
        renyi_vm::RunOutcome::Crashed { message, location } => match location {
            Some(location) => eprintln!("{path}: crash: {message}\n  at {location}"),
            None => eprintln!("{path}: crash: {message}"),
        },
        _ => {}
    }
    if reproduction.differences.is_empty() {
        eprintln!("renyi: reproduced {file}: the outcome and the output are the recorded ones");
        return ExitCode::SUCCESS;
    }
    for difference in &reproduction.differences {
        eprintln!("renyi: {difference}");
    }
    ExitCode::FAILURE
}

/// The extensions of a manifest for a message: `no extension`, or `the
/// extension x 1.0`, or `the extensions x 1.0, y 2.0`.
fn describe_extensions(extensions: &[String]) -> String {
    match extensions {
        [] => "no extension".to_string(),
        [one] => format!("the extension {one}"),
        many => format!("the extensions {}", many.join(", ")),
    }
}

/// `renyi tools [path]`: the tool manifest of decision D6 for a directory
/// or a file with its imports; a project with errors gets its diagnostics
/// instead, as `check` prints them.
fn tools_command(args: &[String]) -> ExitCode {
    if let Some(unknown) = args.iter().find(|arg| arg.starts_with("--")) {
        eprintln!("renyi: unknown option `{unknown}`");
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    let path = Path::new(args.first().map_or(".", String::as_str));
    let files = match renyi_index::load_project(path) {
        Ok(files) => files,
        Err(error) => {
            eprintln!("renyi: {error}");
            return ExitCode::FAILURE;
        }
    };
    let checked = renyi_check::check_project_in(&library(), &files, &[]);
    let mut failed = false;
    for module in &checked.modules {
        if module.diagnostics.iter().any(|d| d.is_error()) {
            failed = true;
            print!("{}", render_text(&files[module.file], &module.diagnostics));
        }
    }
    if failed {
        return ExitCode::FAILURE;
    }
    let tools = renyi_index::tools_of_in(&library(), &files);
    print!("{}", renyi_index::manifest_json(&tools).render());
    ExitCode::SUCCESS
}

/// The thresholds of `renyi index --budgets`: those `renyi.json` sets
/// when the project has them (decision AC1), else the defaults (R7).
fn budgets_of(path: &Path) -> renyi_index::Budgets {
    let defaults = renyi_index::Budgets::default();
    let shown = path.display().to_string();
    let directory = if path.is_dir() {
        shown
    } else {
        renyi_package::directory_of(&shown)
    };
    match renyi_package::Project::of_directory(&directory)
        .manifest
        .and_then(|manifest| manifest.budgets)
    {
        Some(budgets) => renyi_index::Budgets {
            public_per_module: budgets
                .public_per_module
                .unwrap_or(defaults.public_per_module),
            effect_paths_per_module: budgets
                .effect_paths_per_module
                .unwrap_or(defaults.effect_paths_per_module),
            fan_out_per_definition: budgets
                .fan_out_per_definition
                .unwrap_or(defaults.fan_out_per_definition),
        },
        None => defaults,
    }
}

fn index_command(args: &[String]) -> ExitCode {
    let mut json = false;
    let mut budgets = false;
    let mut base: Option<String> = None;
    let mut path: Option<&str> = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--json" => json = true,
            "--budgets" => budgets = true,
            "--diff" => match rest.next() {
                Some(value) => base = Some(value.clone()),
                None => {
                    eprintln!("renyi: --diff needs a saved map or a git revision");
                    return ExitCode::FAILURE;
                }
            },
            other if other.starts_with("--") => {
                eprintln!("renyi: unknown option {other}");
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            }
            other => path = Some(other),
        }
    }
    if budgets && base.is_some() {
        eprintln!("renyi: --budgets and --diff do not combine");
        return ExitCode::FAILURE;
    }
    let path = Path::new(path.unwrap_or("."));
    let files = match renyi_index::load_project(path) {
        Ok(files) => files,
        Err(error) => {
            eprintln!("renyi: {error}");
            return ExitCode::FAILURE;
        }
    };
    let header = renyi_index::Header {
        project: renyi_index::project_name(path),
        revision: renyi_index::git_revision(path),
        toolchain: toolchain(),
    };
    let index = renyi_index::index_files_in(&library(), &files, header);
    let rendered = if let Some(base) = base {
        let old = match maps::load_base(&base, path, &toolchain()) {
            Ok(old) => old,
            Err(error) => {
                eprintln!("renyi: {error}");
                return ExitCode::FAILURE;
            }
        };
        let diff = renyi_index::diff(&old, &index);
        if json {
            renyi_index::diff_json(&diff)
        } else {
            renyi_index::render_diff(&diff)
        }
    } else if budgets {
        // a report, not a gate: CI treats it as a warning (decision O3)
        let over = renyi_index::over_budget(&index, &budgets_of(path));
        if over.is_empty() {
            "nothing over budget\n".to_string()
        } else {
            format!("{}\n", over.join("\n"))
        }
    } else if json {
        renyi_index::to_json(&index)
    } else {
        renyi_index::to_text(&index)
    };
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{rendered}");
    ExitCode::SUCCESS
}
