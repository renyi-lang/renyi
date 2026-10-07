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
//! the source when the path ends in `.ryc`, decision Z4) and `mcp` (the
//! toolchain served to an agent host over standard input and output, in
//! `mcp.rs`).

mod maps;
mod mcp;

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use renyi_check::effects::Capability;
use renyi_syntax::diagnostics::{render_json, render_text};
use renyi_syntax::layout::check_layout;
use renyi_syntax::{format, lex, module_to_json, parse, parse_declarations, SourceFile, TokenKind};
use renyi_vm::grant::{parse_capability, Unit};
use renyi_vm::{file, Manifest};

/// The allocator of the whole binary (decision X6): the VM allocates a
/// record, a list, a text or a frame's locals at a time, and the system
/// allocator of Windows is slow at that; mimalloc is not.
#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
                                      also print the run manifest on stderr: toolchain, code hash, grant,
                                      arguments, environment variables read, outcome, output hash
  renyi record [--to <file.json>] [option...] <file.ry> [argument...]
                                      run `main` and write a recording of its effects, the manifest in its
                                      header (default: <name>.recording.json)
  renyi reproduce <file.json> [<file.ry>]
                                      replay a recording under its manifest and compare the outcome and the
                                      output byte for byte (exit 1 when they differ)
  renyi test [--strict] [--refresh <name> [--redact <name>]] [--explain] <file.ry>...
                                      run every `example:` line and `test` block (exit 1 when any fails)
  renyi compile [--to <file.ryc>] <file.ry>
                                      check the program and write its bytecode (default: <name>.ryc);
                                      run, record, test and reproduce load a .ryc in place of a .ry
  renyi mcp [path]                    serve the toolchain to an agent host over standard input and
                                      output (Model Context Protocol), for the directory given
  renyi version
options of run and record:
  --explain                           narrate the run on stderr: purposes, arguments, results, effects
  --profile                           count every operation, call and primitive call and sample where the
                                      time goes; the report on stderr when the run ends
  --replay <file.json>                run only: answer every effect from the recording; nothing is written or sent
  --deny <capability>                 refuse to start when any function needs the capability
  --allow-host <host>                 narrow network.http to one host
  --allow-read <path>                 narrow filesystem.read to a path prefix
  --allow-write <path>                narrow filesystem.write to a path prefix
  --at-most <capability>=<count>/<unit>
                                      add a budget; the unit is second, minute, hour, day or run
  --redact <name>                     record and test --refresh: replace the argument, header or
                                      environment variable of that name with a placeholder";

fn main() -> ExitCode {
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
        Some("reproduce") => reproduce_command(&args[1..]),
        Some("test") => test_command(&args[1..]),
        Some("compile") => compile_command(&args[1..]),
        Some("mcp") => mcp::serve(&args[1..]),
        Some("version") | Some("--version") => {
            println!("renyi {}", env!("CARGO_PKG_VERSION"));
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
    let parsed = parse(&file.text);
    let mut diagnostics = parsed.diagnostics;
    if !diagnostics.iter().any(|diagnostic| diagnostic.is_error()) {
        diagnostics.extend(renyi_check::check_file(file));
    }
    diagnostics.extend(check_layout(file));
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
fn compile(path: &str) -> Result<renyi_vm::Program, ExitCode> {
    compile_with_sources(path).map(|(program, _)| program)
}

/// What the manifest's code hash is computed from: the source files of a
/// program compiled here, or the text of the bytecode file it was loaded
/// from.
enum Hashed {
    Sources(Vec<SourceFile>),
    File(String),
}

/// `compile`, with what the manifest's code hash is computed from.
fn compile_with_sources(path: &str) -> Result<(renyi_vm::Program, Hashed), ExitCode> {
    if file::is_bytecode(path) {
        return match load_bytecode(path) {
            Ok((program, text)) => Ok((program, Hashed::File(text))),
            Err(message) => {
                eprintln!("renyi: {message}");
                Err(ExitCode::FAILURE)
            }
        };
    }
    match compile_sources(path) {
        Ok(compiled) => {
            print!("{}", compiled.diagnostics);
            Ok((compiled.program, Hashed::Sources(compiled.sources)))
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
    let resolved = renyi_check::resolve(&file);
    let files = resolved.files;
    let checked = renyi_check::check_project_with_problems(&files, &resolved.problems);
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
    }
}

fn main_hash(program: &renyi_vm::Program, files: &[SourceFile]) -> Option<String> {
    let main = program.main?;
    let module = &program.function_metas[main].module;
    let header = renyi_index::Header {
        project: String::new(),
        revision: String::new(),
        toolchain: toolchain(),
    };
    let index = renyi_index::index_files(files, header);
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
    replay: Option<String>,
    to: Option<String>,
    strict: bool,
    refresh: Option<String>,
    redact: Vec<String>,
    manifest: bool,
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
            "--replay" => flags.replay = Some(value()?.to_string()),
            "--to" => flags.to = Some(value()?.to_string()),
            "--refresh" => flags.refresh = Some(value()?.to_string()),
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
    let (flags, rest) = match parse_flags(args) {
        Ok(parsed) => parsed,
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
    if record && flags.replay.is_some() {
        eprintln!("renyi: `record` cannot `--replay`; a recording comes from a live run");
        return ExitCode::FAILURE;
    }
    if !record && !flags.redact.is_empty() {
        eprintln!("renyi: `--redact` is an option of `renyi record` and `renyi test --refresh`");
        return ExitCode::FAILURE;
    }
    let (program, sources) = match compile_with_sources(path) {
        Ok(compiled) => compiled,
        Err(code) => return code,
    };
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
            source: Some(path.clone()),
            code: code_hash(&program, &sources),
            ..Manifest::default()
        }
    } else {
        Manifest::default()
    };
    let options = renyi_vm::Options {
        arguments: rest[1..].to_vec(),
        narrowing: flags.narrowing,
        record: with_manifest,
        replay,
        revision: revision.filter(|text| text != "unknown"),
        explain: flags.explain,
        profile: flags.profile,
        redact: flags.redact,
        manifest,
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
    match run.outcome {
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
        || !narrowing.deny.is_empty()
        || !narrowing.allow.is_empty()
        || !narrowing.budgets.is_empty()
    {
        eprintln!(
            "renyi: `renyi test` takes only `--strict`, `--refresh <name>`, `--redact <name>` and `--explain`"
        );
        return ExitCode::FAILURE;
    }
    let mut failed = false;
    for path in files {
        let program = match compile(path) {
            Ok(program) => program,
            Err(code) => return code,
        };
        let options = renyi_vm::Options {
            explain: flags.explain,
            strict: flags.strict,
            refresh: flags.refresh.clone(),
            redact: flags.redact.clone(),
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
    let (program, sources) = match compile_with_sources(&path) {
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
    if let Some(recorded) = &recording.manifest.toolchain {
        if *recorded != toolchain() {
            eprintln!(
                "renyi: warning: the recording was made with {recorded}, this is {}",
                toolchain()
            );
        }
    }
    let reproduction = renyi_vm::reproduce(&program, recording, renyi_vm::Options::default());
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
    let checked = renyi_check::check_project(&files);
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
    let tools = renyi_index::tools_of(&files);
    print!("{}", renyi_index::manifest_json(&tools).render());
    ExitCode::SUCCESS
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
    let index = renyi_index::index_files(&files, header);
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
        let over = renyi_index::over_budget(&index, &renyi_index::Budgets::default());
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
