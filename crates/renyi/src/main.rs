//! The `renyi` command: `check` (lexer, parser, type and effect checker and
//! layout diagnostics, text or JSON), `format` (canonical layout, in place or
//! `--check`), `tokens` (a token dump), `parse` (a syntax tree dump, as Rust
//! debug output or as JSON for tools), `index` (the project map, text or
//! JSON), `run` (check, then execute `main` on the VM) and `test` (every
//! `example:` line and `test` block).

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use renyi_syntax::diagnostics::{render_json, render_text};
use renyi_syntax::layout::check_layout;
use renyi_syntax::{format, lex, module_to_json, parse, SourceFile, TokenKind};

const USAGE: &str = "usage:
  renyi check [--json] <file.ry>...   report diagnostics (exit 1 when any error)
  renyi format [--check] <file.ry>... rewrite files in canonical layout (--check: report only)
  renyi tokens <file.ry>              dump the token stream
  renyi parse [--json] <file.ry>      dump the syntax tree (--json: for tools)
  renyi index [--json] [path]         the project map of a directory or a file with its imports
  renyi index --budgets [path]        every value of the map over its budget (exit 0 either way)
  renyi run <file.ry> [argument...]   check the program, then run its `main` (exit 1 when it fails, 2 on a crash)
  renyi test <file.ry>...             run every `example:` line and `test` block (exit 1 when any fails)
  renyi version";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") => check(&args[1..]),
        Some("format") => format_command(&args[1..]),
        Some("tokens") => tokens(&args[1..]),
        Some("parse") => parse_command(&args[1..]),
        Some("index") => index_command(&args[1..]),
        Some("run") => run_command(&args[1..]),
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

fn run_command(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let program = match compile(path) {
        Ok(program) => program,
        Err(code) => return code,
    };
    let options = renyi_vm::Options {
        arguments: args[1..].to_vec(),
        ..renyi_vm::Options::default()
    };
    match renyi_vm::run_main(&program, options) {
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
    let files: Vec<&String> = args.iter().filter(|arg| !arg.starts_with("--")).collect();
    if files.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    let mut failed = false;
    for path in files {
        let program = match compile(path) {
            Ok(program) => program,
            Err(code) => return code,
        };
        let report = renyi_vm::run_tests(&program, renyi_vm::Options::default());
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
