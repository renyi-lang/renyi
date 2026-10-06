//! The `renyi` command: `check` (lexer, parser, type and effect checker and
//! layout diagnostics, text or JSON), `format` (canonical layout, in place or
//! `--check`), `tokens` (a token dump), `parse` (a syntax tree dump, as Rust
//! debug output or as JSON for tools), `index` (the project map, text or
//! JSON), `run` (check, then execute `main` on the VM, optionally replaying
//! a recording or narrating the run), `record` (run and write a recording of
//! every effect) and `test` (every `example:` line and `test` block, with
//! `replays` tests answered from their recordings).

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use renyi_check::effects::Capability;
use renyi_syntax::diagnostics::{render_json, render_text};
use renyi_syntax::layout::check_layout;
use renyi_syntax::{format, lex, module_to_json, parse, SourceFile, TokenKind};
use renyi_vm::grant::{parse_capability, Unit};

const USAGE: &str = "usage:
  renyi check [--json] <file.ry>...   report diagnostics (exit 1 when any error)
  renyi format [--check] <file.ry>... rewrite files in canonical layout (--check: report only)
  renyi tokens <file.ry>              dump the token stream
  renyi parse [--json] <file.ry>      dump the syntax tree (--json: for tools)
  renyi index [--json] [path]         the project map of a directory or a file with its imports
  renyi index --budgets [path]        every value of the map over its budget (exit 0 either way)
  renyi run [option...] <file.ry> [argument...]
                                      check the program, then run its `main` (exit 1 when it fails, 2 on a crash)
  renyi record [--to <file.json>] [option...] <file.ry> [argument...]
                                      run `main` and write a recording of its effects (default: <name>.recording.json)
  renyi test [--strict] [--refresh <name>] [--explain] <file.ry>...
                                      run every `example:` line and `test` block (exit 1 when any fails)
  renyi version
options of run and record:
  --explain                           narrate the run on stderr: purposes, arguments, results, effects
  --replay <file.json>                run only: answer every effect from the recording; nothing is written or sent
  --deny <capability>                 refuse to start when any function needs the capability
  --allow-host <host>                 narrow network.http to one host
  --allow-read <path>                 narrow filesystem.read to a path prefix
  --allow-write <path>                narrow filesystem.write to a path prefix
  --at-most <capability>=<count>/<unit>
                                      add a budget; the unit is second, minute, hour, day or run";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") => check(&args[1..]),
        Some("format") => format_command(&args[1..]),
        Some("tokens") => tokens(&args[1..]),
        Some("parse") => parse_command(&args[1..]),
        Some("index") => index_command(&args[1..]),
        Some("run") => run_command(&args[1..], false),
        Some("record") => run_command(&args[1..], true),
        Some("test") => test_command(&args[1..]),
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

fn load(path: &str) -> Result<SourceFile, ExitCode> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(SourceFile::new(path, text)),
        Err(error) => {
            eprintln!("renyi: cannot read {path}: {error}");
            Err(ExitCode::FAILURE)
        }
    }
}

fn check(args: &[String]) -> ExitCode {
    let json = args.iter().any(|arg| arg == "--json");
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
        let parsed = parse(&file.text);
        let mut diagnostics = parsed.diagnostics;
        if !diagnostics.iter().any(|diagnostic| diagnostic.is_error()) {
            diagnostics.extend(renyi_check::check_file(&file));
        }
        diagnostics.extend(check_layout(&file));
        diagnostics.sort_by_key(|diagnostic| diagnostic.span.start);
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
    let Some(path) = args.iter().find(|arg| !arg.starts_with("--")) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let file = match load(path) {
        Ok(file) => file,
        Err(code) => return code,
    };
    let parsed = parse(&file.text);
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

/// Check a file with its imports and compile it; diagnostics go to stdout as
/// `check` prints them, and an error stops here.
fn compile(path: &str) -> Result<renyi_vm::Program, ExitCode> {
    let file = load(path)?;
    let mut files = vec![file.clone()];
    files.extend(renyi_check::imported_files(&file));
    let checked = renyi_check::check_project(&files);
    let mut failed = false;
    for module in &checked.modules {
        if module.diagnostics.is_empty() {
            continue;
        }
        failed |= module.diagnostics.iter().any(|d| d.is_error());
        print!("{}", render_text(&files[module.file], &module.diagnostics));
    }
    if failed {
        return Err(ExitCode::FAILURE);
    }
    Ok(renyi_vm::compile_project(&checked, &files))
}

/// The options of `run`, `record` and `test`, read up to the first
/// positional argument.
#[derive(Default)]
struct Flags {
    narrowing: renyi_vm::Narrowing,
    explain: bool,
    replay: Option<String>,
    to: Option<String>,
    strict: bool,
    refresh: Option<String>,
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
            "--strict" => {
                flags.strict = true;
                index += 1;
                continue;
            }
            "--replay" => flags.replay = Some(value()?.to_string()),
            "--to" => flags.to = Some(value()?.to_string()),
            "--refresh" => flags.refresh = Some(value()?.to_string()),
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

fn scoped(path: &[&str], scope: &str) -> Capability {
    Capability {
        path: path.iter().map(|part| part.to_string()).collect(),
        scope: Some(scope.to_string()),
        budget: None,
        only_to: Vec::new(),
    }
}

/// `network.http=60/minute`, or with a scope, `network.http("host")=60/minute`.
fn parse_budget(text: &str) -> Result<Capability, String> {
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

fn load_recording(path: &str) -> Result<renyi_vm::Recording, String> {
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
    let program = match compile(path) {
        Ok(program) => program,
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
    let revision = record.then(|| renyi_index::git_revision(Path::new(path)));
    let options = renyi_vm::Options {
        arguments: rest[1..].to_vec(),
        narrowing: flags.narrowing,
        record,
        replay,
        revision: revision.filter(|text| text != "unknown"),
        explain: flags.explain,
        ..renyi_vm::Options::default()
    };
    let run = renyi_vm::run_program(&program, options);
    if let Some(recording) = &run.recording {
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
        || !narrowing.deny.is_empty()
        || !narrowing.allow.is_empty()
        || !narrowing.budgets.is_empty()
    {
        eprintln!("renyi: `renyi test` takes only `--strict`, `--refresh <name>` and `--explain`");
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

fn index_command(args: &[String]) -> ExitCode {
    let json = args.iter().any(|arg| arg == "--json");
    let path = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .map(String::as_str)
        .unwrap_or(".");
    let path = Path::new(path);
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
        toolchain: format!("renyi {}", env!("CARGO_PKG_VERSION")),
    };
    let index = renyi_index::index_files(&files, header);
    let rendered = if args.iter().any(|arg| arg == "--budgets") {
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
