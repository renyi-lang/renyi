//! `renyi serve [--watch] [option...] <file.ry> [argument...]` (decisions
//! Q4 and AO1): `renyi run` for a service. With `--watch`, the project's
//! files are looked at every half second between requests (the resident
//! world of decision AN1 stats them); when one changed, the program is
//! compiled again and, when it checks clean, `main` is run again on the
//! new version between two requests, the listening socket handed over so
//! that no request is lost; a version with errors is reported and the
//! last good one keeps serving. The reload's message names what the
//! semantic diff found: the definitions that changed, a signature among
//! them.

use std::cell::RefCell;
use std::path::Path;
use std::process::ExitCode;
use std::rc::Rc;

use renyi_index::{diff, git_revision, project_name, Change, Diff, Header, Index};
use renyi_package::Project;
use renyi_syntax::SourceFile;
use renyi_vm::{file, Narrowing, Options, RunOutcome};
use renyi_workspace::Workspace;

use crate::{
    bound_modules, compile_sources, exit_of, grants, library, parse_flags, registry, run_command,
    toolchain, CompileError, Compiled, USAGE,
};

pub fn serve_command(args: &[String]) -> ExitCode {
    let (flags, positional) = match parse_flags(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("renyi: {message}");
            return ExitCode::FAILURE;
        }
    };
    if !flags.watch {
        return run_command(args, false);
    }
    if flags.replay.is_some()
        || flags.to.is_some()
        || flags.refresh.is_some()
        || !flags.redact.is_empty()
        || flags.manifest
        || flags.strict
        || flags.profile
    {
        eprintln!(
            "renyi: `serve --watch` takes only `--deny`, `--allow-host`, `--allow-read`, `--allow-write`, `--at-most`, `--explain` and `--interpret`"
        );
        return ExitCode::FAILURE;
    }
    let Some(path) = positional.first() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    if file::is_bytecode(path) {
        eprintln!("renyi: `serve --watch` reloads from the sources; give the .ry file");
        return ExitCode::FAILURE;
    }

    // the first version must be clean, as under `run`
    let mut compiled = match compile_sources(path) {
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
    let root = project_root(path);
    let mut workspace = Workspace::new(&root, &library());
    if let Err(message) = workspace.refresh() {
        eprintln!("renyi: {message}");
        return ExitCode::FAILURE;
    }
    let index = workspace
        .index(|| header_of(&root))
        .cloned()
        .expect("a refreshed workspace has a map");
    eprintln!(
        "renyi serve: watching {} for changes; the program reloads between requests",
        if root == "." {
            "the current directory"
        } else {
            &root
        }
    );
    let watch = Rc::new(RefCell::new(Watch {
        workspace,
        index,
        path: path.clone(),
        root,
        narrowing: flags.narrowing.clone(),
        serving: compiled.sources.clone(),
        pending: None,
    }));

    let mut listener = None;
    loop {
        if let Some(message) = denied(&compiled, &flags.narrowing) {
            eprintln!("renyi: {message}");
            return ExitCode::FAILURE;
        }
        notices(&compiled);
        let ready: Box<dyn FnMut() -> bool> = {
            let watch = watch.clone();
            Box::new(move || watch.borrow_mut().poll())
        };
        let options = Options {
            arguments: positional[1..].to_vec(),
            narrowing: flags.narrowing.clone(),
            explain: flags.explain,
            interpret: flags.interpret,
            registry: registry().clone(),
            watch: Some(ready),
            listener: listener.take(),
            ..Options::default()
        };
        let run = renyi_vm::run_program(&compiled.program, options);
        match run.outcome {
            RunOutcome::Reload => {
                let next = watch.borrow_mut().pending.take();
                match next {
                    Some(next) => {
                        compiled = next;
                        listener = run.listener;
                    }
                    None => {
                        eprintln!(
                            "renyi serve: the server stopped for a new version, but none was ready"
                        );
                        return ExitCode::FAILURE;
                    }
                }
            }
            other => return exit_of(path, other),
        }
    }
}

/// The directory the watch looks at: the project of the file (the nearest
/// `renyi.json` above it, else the file's directory; the working
/// directory for a bare name).
fn project_root(path: &str) -> String {
    let root = Project::of(path).root;
    if root.is_empty() {
        ".".to_string()
    } else {
        root
    }
}

fn header_of(root: &str) -> Header {
    Header {
        project: project_name(Path::new(root)),
        revision: git_revision(Path::new(root)),
        toolchain: toolchain(),
    }
}

/// `--deny`: the functions that need a denied capability, as `renyi run`
/// refuses them.
fn denied(compiled: &Compiled, narrowing: &Narrowing) -> Option<String> {
    for denied in &narrowing.deny {
        let functions = renyi_vm::denied_functions(&compiled.program, denied);
        if !functions.is_empty() {
            return Some(format!(
                "`{}` is denied, but these functions need it: {}",
                denied.spelling(),
                functions.join(", ")
            ));
        }
    }
    None
}

/// What `renyi run` says before a program that can call native code or
/// Python starts (decisions AF1 and AJ2).
fn notices(compiled: &Compiled) {
    let native = bound_modules(&compiled.program, |meta| meta.foreign.is_some());
    if !native.is_empty() && grants(&compiled.program, "foreign") {
        eprintln!(
            "renyi: this program can call native code through {}",
            native.join(", ")
        );
    }
    let python = bound_modules(&compiled.program, |meta| meta.python.is_some());
    if !python.is_empty() && grants(&compiled.program, "python") {
        eprintln!(
            "renyi: this program can run Python through {}",
            python.join(", ")
        );
    }
}

/// What the server asks between requests: whether a new version is
/// ready.
struct Watch {
    workspace: Workspace,
    /// The map of the version serving, for the reload's message.
    index: Index,
    path: String,
    root: String,
    narrowing: Narrowing,
    /// The sources of the version serving: a change elsewhere in the
    /// directory is no new version.
    serving: Vec<SourceFile>,
    /// The version compiled and ready, taken by the driver after the
    /// server stopped for it.
    pending: Option<Compiled>,
}

impl Watch {
    /// The files stat-ed; when any changed, the program compiled again:
    /// ready when its sources changed, it checks clean and nothing it
    /// needs is denied, with the semantic diff against the serving
    /// version on the standard error; an error reported and the serving
    /// version kept otherwise.
    fn poll(&mut self) -> bool {
        let report = match self.workspace.refresh() {
            Ok(report) => report,
            Err(message) => {
                eprintln!("renyi serve: {message}");
                return false;
            }
        };
        if !report.declared {
            return false;
        }
        let compiled = match compile_sources(&self.path) {
            Ok(compiled) => compiled,
            Err(CompileError::Diagnostics(text)) => {
                eprintln!(
                    "renyi serve: the new version has errors; still serving the last good one:\n{text}"
                );
                return false;
            }
            Err(CompileError::Read(message)) => {
                eprintln!("renyi serve: {message}; still serving the last good one");
                return false;
            }
        };
        if same_sources(&self.serving, &compiled.sources) {
            return false;
        }
        if let Some(message) = denied(&compiled, &self.narrowing) {
            eprintln!("renyi serve: {message}; still serving the last good one");
            return false;
        }
        let index = self
            .workspace
            .index(|| header_of(&self.root))
            .cloned()
            .expect("a refreshed workspace has a map");
        eprintln!(
            "renyi serve: reloading {}: {}",
            self.path,
            summary(&diff(&self.index, &index))
        );
        eprint!("{}", compiled.diagnostics);
        self.index = index;
        self.serving = compiled.sources.clone();
        self.pending = Some(compiled);
        true
    }
}

/// Whether the program's files are, name for name, the texts that were
/// compiled before.
fn same_sources(old: &[SourceFile], new: &[SourceFile]) -> bool {
    old.len() == new.len()
        && old
            .iter()
            .zip(new)
            .all(|(old, new)| old.name == new.name && old.text == new.text)
}

/// The reload's message: how many definitions changed and how; a
/// signature change is named, not refused (decision AO1).
fn summary(diff: &Diff) -> String {
    if diff.entries.is_empty() {
        return "no definition changed".to_string();
    }
    let entries: Vec<String> = diff
        .entries
        .iter()
        .map(|entry| {
            let changes: Vec<String> = entry
                .changes
                .iter()
                .map(|change| match change {
                    Change::Added => "added".to_string(),
                    Change::Removed => "removed".to_string(),
                    Change::Renamed { from, .. } => format!("renamed from {from}"),
                    Change::Signature { old, new } => format!("signature `{old}` -> `{new}`"),
                    Change::Visibility { public: true } => "made public".to_string(),
                    Change::Visibility { public: false } => "made private".to_string(),
                    Change::Effects { .. } => "effects changed".to_string(),
                    Change::Failures { .. } => "failures changed".to_string(),
                    Change::Body => "body changed".to_string(),
                })
                .collect();
            format!("{} ({})", entry.name, changes.join(", "))
        })
        .collect();
    format!(
        "{} definition{} changed: {}",
        diff.entries.len(),
        if diff.entries.len() == 1 { "" } else { "s" },
        entries.join("; ")
    )
}
