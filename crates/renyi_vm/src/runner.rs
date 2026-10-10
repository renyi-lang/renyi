//! Running a program's `main`, and its `example:` lines and `test` blocks,
//! each under its declared grant; recorded, replayed or narrated as the
//! options say.

use std::cell::RefCell;
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use renyi_check::effects::Capability;
use renyi_check::ModuleId;

use crate::bytecode::CodeKind;
use crate::compile::{Expected, Program};
use crate::recording::Recording;
use crate::value::Value;
use crate::vm::{Interrupt, Options, Vm};
use sha2::{Digest, Sha256};

/// How `main` ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunOutcome {
    Finished,
    /// `main` failed with this error, rendered.
    Failed(String),
    Crashed {
        message: String,
        location: Option<String>,
    },
    Exited(i32),
    /// The watch of `renyi serve --watch` found a new version (decision
    /// AO1): `main` is to be run again on it.
    Reload,
    /// The memory budget was exceeded (decision AP1): the limit, and the
    /// most bytes the run held above the level at its start.
    OverMemory {
        limit: u64,
        used: u64,
    },
}

/// A run of `main` and what the primitive boundary produced.
pub struct Run {
    pub outcome: RunOutcome,
    /// The recording, when the options asked for one.
    pub recording: Option<Recording>,
    /// Under `--replay`, the recorded calls the run never reached.
    pub unused: Vec<String>,
    /// The listening socket `server.serve` left for the next version
    /// (decision AO1).
    pub listener: Option<TcpListener>,
    /// How many code objects the run compiled to machine code (none
    /// loaded from an image): what tells `renyi run` that the program is
    /// worth an image in the cache (decision AU10).
    pub compiled: usize,
}

pub fn run_main(program: &Program, options: Options) -> RunOutcome {
    run_program(program, options).outcome
}

/// Run `main` under the grant it declares, narrowed by the options.
pub fn run_program(program: &Program, options: Options) -> Run {
    run_measured(program, options).0
}

/// The standard output's hash and length, kept while a run writes it.
type Measured = Rc<RefCell<(Sha256, u64)>>;

/// A writer that passes everything through and hashes it.
struct Hashing {
    inner: Box<dyn Write>,
    digest: Measured,
}

impl Write for Hashing {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(buffer)?;
        let mut state = self.digest.borrow_mut();
        state.0.update(&buffer[..written]);
        state.1 += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn digest_text(digest: &Measured) -> (String, u64) {
    let state = digest.borrow();
    (format!("sha256:{:x}", state.0.clone().finalize()), state.1)
}

/// An outcome as the manifest spells it.
pub fn describe_outcome(outcome: &RunOutcome) -> String {
    match outcome {
        RunOutcome::Finished => "finished".to_string(),
        RunOutcome::Failed(error) => format!("failed with {error}"),
        RunOutcome::Crashed { message, .. } => format!("crashed: {message}"),
        RunOutcome::Exited(code) => format!("exited with {code}"),
        RunOutcome::Reload => "stopped for a new version".to_string(),
        RunOutcome::OverMemory { limit, used } => {
            format!("exceeded the memory budget of {limit} bytes ({used} bytes held)")
        }
    }
}

/// `main`, with the standard output hashed when the run is recorded or
/// reproduced, so that the manifest can name the output.
fn run_measured(program: &Program, mut options: Options) -> (Run, Option<(String, u64)>) {
    // the run part of decision AU22: from the program loaded (compiled
    // from its sources or read from a file) to the end of `main`
    let started = std::time::Instant::now();
    let digest = (options.record || options.replay_output).then(|| {
        let digest: Measured = Rc::new(RefCell::new((Sha256::new(), 0)));
        let inner = std::mem::replace(&mut options.stdout, Box::new(std::io::sink()));
        options.stdout = Box::new(Hashing {
            inner,
            digest: digest.clone(),
        });
        digest
    });
    // the last reads of the slots as moves (decision AU17), on a copy
    let program = &crate::liveness::prepared(program);
    let mut vm = Vm::new(program, options);
    let stopped = |message: String| {
        (
            Run {
                outcome: RunOutcome::Crashed {
                    message,
                    location: None,
                },
                recording: None,
                unused: Vec::new(),
                listener: None,
                compiled: 0,
            },
            None,
        )
    };
    let Some(main) = program.main else {
        return stopped("the program has no `main` function".to_string());
    };
    let meta = &program.function_metas[main];
    let declared = meta.needs.clone();
    if let Err(message) = vm.begin_run(&declared, &meta.module) {
        return stopped(message);
    }
    let outcome = match vm.run_main() {
        Ok(None) => RunOutcome::Finished,
        Ok(Some(error)) => match vm.text_for_console(&error, "the failure of `main`") {
            Ok(text) => RunOutcome::Failed(text),
            Err(interrupt) => crashed(interrupt),
        },
        Err(interrupt) => crashed(interrupt),
    };
    vm.report_profile();
    if std::env::var_os("RENYI_NATIVE_REPORT").is_some() {
        let elapsed = started.elapsed();
        if let Some(jit) = &vm.native {
            let _ = writeln!(vm.stderr, "{}", jit.report(&vm.hotness));
        }
        let _ = writeln!(
            vm.stderr,
            "run: {:.1} ms from the program loaded to the end of `main`",
            elapsed.as_secs_f64() * 1000.0
        );
    }
    let unused = vm.end_replay();
    let output = digest.as_ref().map(digest_text);
    let mut recording = vm.take_recording();
    if let (Some(recording), Some(output)) = (&mut recording, &output) {
        recording.finish(describe_outcome(&outcome), output.clone());
    }
    let listener = vm.listener.take();
    let compiled = vm.native.as_ref().map_or(0, |jit| jit.compiled);
    (
        Run {
            outcome,
            recording,
            unused,
            listener,
            compiled,
        },
        output,
    )
}

/// What `renyi reproduce` found.
pub struct Reproduction {
    pub run: Run,
    /// Where the run departed from its manifest, in words; empty when the
    /// run reproduced.
    pub differences: Vec<String>,
}

/// Replay a recording under its own arguments with the console output
/// written, and compare the outcome and the output with the manifest
/// (decision Q2). The caller checks the code hash and the toolchain.
pub fn reproduce(program: &Program, recording: Recording, mut options: Options) -> Reproduction {
    let manifest = recording.manifest.clone();
    options.arguments = manifest.arguments.clone();
    options.replay = Some(recording);
    options.replay_output = true;
    options.record = false;
    let (run, output) = run_measured(program, options);
    let mut differences = Vec::new();
    let outcome = describe_outcome(&run.outcome);
    match &manifest.outcome {
        Some(expected) if *expected != outcome => differences.push(format!(
            "the outcome differs: the recording says {expected}, this run {outcome}"
        )),
        None => differences.push("the recording has no outcome to compare with".to_string()),
        _ => {}
    }
    match (&manifest.output, &output) {
        (Some((hash, bytes)), Some((now_hash, now_bytes)))
            if hash != now_hash || bytes != now_bytes =>
        {
            differences.push(format!(
                "the output differs: the recording says {hash} ({bytes} bytes), this run {now_hash} ({now_bytes} bytes)"
            ))
        }
        (None, _) => differences.push("the recording has no output to compare with".to_string()),
        _ => {}
    }
    if !run.unused.is_empty() {
        differences.push(format!(
            "{} recorded call{} never reached: {}",
            run.unused.len(),
            if run.unused.len() == 1 {
                " was"
            } else {
                "s were"
            },
            run.unused.join("; ")
        ));
    }
    Reproduction { run, differences }
}

/// The functions of the program, not the library, that need a capability
/// the command line denies: `--deny` refuses to start when there are any.
pub fn denied_functions(program: &Program, denied: &Capability) -> Vec<String> {
    program
        .function_metas
        .iter()
        .filter(|meta| !meta.is_library && meta.needs.iter().any(|need| denied.covers(need, true)))
        .map(|meta| format!("{}.{}", meta.module, meta.name))
        .collect()
}

fn crashed(interrupt: Interrupt) -> RunOutcome {
    match interrupt {
        Interrupt::Crash { message, location } => RunOutcome::Crashed { message, location },
        Interrupt::Exit(code) => RunOutcome::Exited(code),
        Interrupt::Reload => RunOutcome::Reload,
        Interrupt::OverMemory { limit, used } => RunOutcome::OverMemory { limit, used },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestOutcome {
    Passed,
    Failed(String),
    Skipped(String),
    /// `--refresh`: the test passed live and its fixture was written here.
    Recorded(String),
}

#[derive(Clone, Debug)]
pub struct TestResult {
    /// `example of f: f(1) is 2` or `test "name"`.
    pub name: String,
    /// `file:line`.
    pub location: String,
    pub outcome: TestOutcome,
}

#[derive(Clone, Debug, Default)]
pub struct TestReport {
    pub results: Vec<TestResult>,
    /// How many code objects the tests compiled to machine code, as
    /// `Run::compiled` (decision AU10).
    pub compiled: usize,
}

impl TestReport {
    pub fn passed(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.outcome == TestOutcome::Passed)
            .count()
    }

    pub fn failed(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, TestOutcome::Failed(_)))
            .count()
    }

    pub fn skipped(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, TestOutcome::Skipped(_)))
            .count()
    }

    pub fn recorded(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, TestOutcome::Recorded(_)))
            .count()
    }

    /// One line per item and a summary line.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for result in &self.results {
            match &result.outcome {
                TestOutcome::Passed => out.push_str(&format!("ok    {}\n", result.name)),
                TestOutcome::Failed(reason) => out.push_str(&format!(
                    "FAIL  {} ({})\n      {reason}\n",
                    result.name, result.location
                )),
                TestOutcome::Skipped(reason) => {
                    out.push_str(&format!("skip  {} ({reason})\n", result.name))
                }
                TestOutcome::Recorded(path) => {
                    out.push_str(&format!("rec   {} -> {path}\n", result.name))
                }
            }
        }
        let mut summary = format!("{} passed, {} failed", self.passed(), self.failed());
        if self.skipped() > 0 {
            summary.push_str(&format!(", {} skipped", self.skipped()));
        }
        if self.recorded() > 0 {
            summary.push_str(&format!(", {} recorded", self.recorded()));
        }
        out.push_str(&summary);
        out.push('\n');
        out
    }
}

/// Run every `example:` line and `test` block of the program's own modules
/// (not the library's), in source order, each under its own grant.
pub fn run_tests(program: &Program, mut options: Options) -> TestReport {
    let strict = options.strict;
    let refresh = options.refresh.take();
    options.replay = None;
    options.record = false;
    // the last reads of the slots as moves (decision AU17), on a copy
    let program = &crate::liveness::prepared(program);
    let mut vm = Vm::new(program, options);
    let mut items: Vec<(usize, usize, Item)> = Vec::new();
    for (index, example) in program.examples.iter().enumerate() {
        items.push((example.module, example.span.start, Item::Example(index)));
    }
    for (index, test) in program.tests.iter().enumerate() {
        items.push((test.module, test.span.start, Item::Test(index)));
    }
    items.sort_by_key(|(module, start, _)| (*module, *start));
    let mut report = TestReport::default();
    for (_, _, item) in items {
        report.results.push(match item {
            Item::Example(index) => run_example(&mut vm, index),
            Item::Test(index) => run_test(&mut vm, index, strict, refresh.as_deref()),
        });
    }
    vm.report_profile();
    report.compiled = vm.native.as_ref().map_or(0, |jit| jit.compiled);
    report
}

enum Item {
    Example(usize),
    Test(usize),
}

fn run_example(vm: &mut Vm, index: usize) -> TestResult {
    let program = vm.program;
    let example = &program.examples[index];
    let function = &program.function_metas[example.function].name;
    let name = format!("example of {function}: {}", example.text);
    let location = program.location(example.module, example.span);
    // an example is pure: it runs under no grant at all
    vm.record = false;
    let outcome = match vm.begin_run(&[], &program.module_names[example.module]) {
        Ok(()) => match example_outcome(vm, index) {
            Ok(outcome) => outcome,
            Err(interrupt) => TestOutcome::Failed(describe_interrupt(interrupt)),
        },
        Err(message) => TestOutcome::Failed(message),
    };
    TestResult {
        name,
        location,
        outcome,
    }
}

fn example_outcome(vm: &mut Vm, index: usize) -> Result<TestOutcome, Interrupt> {
    let example = &vm.program.examples[index];
    debug_assert_eq!(vm.program.code(example.call).kind, CodeKind::Example);
    let actual = vm.call_code(example.call, Vec::new())?;
    Ok(match &example.expected {
        Expected::Is(code) => {
            let expected = vm.call_code(*code, Vec::new())?;
            match &actual {
                Value::Failure(error) => {
                    let shown = vm.to_text(error)?;
                    let wanted = vm.render(&expected, true)?;
                    TestOutcome::Failed(format!("failed with {shown}, expected {wanted}"))
                }
                _ if actual == expected => TestOutcome::Passed,
                _ => {
                    let got = vm.render(&actual, true)?;
                    let wanted = vm.render(&expected, true)?;
                    TestOutcome::Failed(format!("got {got}, expected {wanted}"))
                }
            }
        }
        Expected::FailsWith(code) => match &actual {
            Value::Failure(error) => {
                let error = (**error).clone();
                match vm.call_code(*code, vec![error.clone()])? {
                    Value::Boolean(true) => TestOutcome::Passed,
                    _ => {
                        let shown = vm.render(&error, true)?;
                        TestOutcome::Failed(format!(
                            "failed with {shown}, which does not match the pattern"
                        ))
                    }
                }
            }
            other => {
                let got = vm.render(other, true)?;
                TestOutcome::Failed(format!("returned {got}, expected a failure"))
            }
        },
    })
}

/// A test under its `needs`: live, or answered from its `replays`
/// recording, or, under `--refresh`, live and recorded to that file.
fn run_test(vm: &mut Vm, index: usize, strict: bool, refresh: Option<&str>) -> TestResult {
    let program = vm.program;
    let test = &program.tests[index];
    let name = format!("test {:?}", test.name);
    let location = program.location(test.module, test.span);
    let result = |outcome: TestOutcome| TestResult {
        name: name.clone(),
        location: location.clone(),
        outcome,
    };
    let refreshing = refresh == Some(test.name.as_str());
    let fixture = test
        .replays
        .as_ref()
        .map(|path| fixture_path(program, test.module, path));
    if refreshing && fixture.is_none() {
        return result(TestOutcome::Failed(
            "`--refresh` names this test, but it has no `replays` clause".to_string(),
        ));
    }
    vm.record = refreshing;
    if let Err(message) = vm.begin_run(&test.needs, &program.module_names[test.module]) {
        vm.record = false;
        return result(TestOutcome::Failed(message));
    }
    if let (Some(path), false) = (&fixture, refreshing) {
        let loaded = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read the recording {}: {error}", path.display()))
            .and_then(|text| {
                Recording::parse(&text).map_err(|detail| format!("{}: {detail}", path.display()))
            })
            .and_then(|recording| vm.replay_with(recording));
        if let Err(message) = loaded {
            return result(TestOutcome::Failed(message));
        }
    }
    let mut outcome = match vm.call_code(test.code, Vec::new()) {
        Ok(Value::Failure(error)) => match vm.text_for_console(&error, "the failure of a test") {
            Ok(text) => TestOutcome::Failed(text),
            Err(interrupt) => TestOutcome::Failed(describe_interrupt(interrupt)),
        },
        Ok(_) => TestOutcome::Passed,
        Err(interrupt) => TestOutcome::Failed(describe_interrupt(interrupt)),
    };
    let unused = vm.end_replay();
    if strict && outcome == TestOutcome::Passed && !unused.is_empty() {
        outcome = TestOutcome::Failed(format!(
            "{} recorded call{} never reached: {}",
            unused.len(),
            if unused.len() == 1 { " was" } else { "s were" },
            unused.join("; ")
        ));
    }
    if refreshing {
        vm.record = false;
        if let (Some(recording), Some(path)) = (vm.take_recording(), &fixture) {
            let written = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|_| std::fs::write(path, recording.render()));
            outcome = match (written, outcome) {
                (Err(error), _) => TestOutcome::Failed(format!(
                    "cannot write the recording {}: {error}",
                    path.display()
                )),
                (Ok(()), TestOutcome::Passed) => TestOutcome::Recorded(path.display().to_string()),
                (Ok(()), other) => other,
            };
        }
    }
    result(outcome)
}

/// Where a test's `replays` file lives: beside the module's source file
/// (recordings live with the tests), else relative to the working
/// directory.
fn fixture_path(program: &Program, module: ModuleId, path: &str) -> PathBuf {
    let relative = PathBuf::from(path);
    if relative.is_absolute() {
        return relative;
    }
    let base = program
        .source(module)
        .and_then(|source| Path::new(&source.name).parent())
        .filter(|parent| !parent.as_os_str().is_empty());
    match base {
        Some(base) => {
            let beside = base.join(&relative);
            if beside.exists() || !relative.exists() {
                beside
            } else {
                relative
            }
        }
        None => relative,
    }
}

fn describe_interrupt(interrupt: Interrupt) -> String {
    match interrupt {
        Interrupt::Crash {
            message,
            location: Some(location),
        } => format!("crashed: {message} (at {location})"),
        Interrupt::Crash { message, .. } => format!("crashed: {message}"),
        Interrupt::Exit(code) => format!("exited with code {code}"),
        Interrupt::Reload => "stopped for a new version".to_string(),
        Interrupt::OverMemory { limit, used } => {
            format!("exceeded the memory budget of {limit} bytes ({used} bytes held)")
        }
    }
}
