//! Running a program's `main`, and its `example:` lines and `test` blocks,
//! each under its declared grant; recorded, replayed or narrated as the
//! options say.

use std::path::{Path, PathBuf};

use renyi_check::effects::Capability;
use renyi_check::ModuleId;

use crate::bytecode::CodeKind;
use crate::compile::{Expected, Program};
use crate::recording::Recording;
use crate::value::Value;
use crate::vm::{Interrupt, Options, Vm};

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
}

/// A run of `main` and what the primitive boundary produced.
pub struct Run {
    pub outcome: RunOutcome,
    /// The recording, when the options asked for one.
    pub recording: Option<Recording>,
    /// Under `--replay`, the recorded calls the run never reached.
    pub unused: Vec<String>,
}

pub fn run_main(program: &Program, options: Options) -> RunOutcome {
    run_program(program, options).outcome
}

/// Run `main` under the grant it declares, narrowed by the options.
pub fn run_program(program: &Program, options: Options) -> Run {
    let mut vm = Vm::new(program, options);
    let stopped = |message: String| Run {
        outcome: RunOutcome::Crashed {
            message,
            location: None,
        },
        recording: None,
        unused: Vec::new(),
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
        Ok(Some(error)) => match vm.to_text(&error) {
            Ok(text) => RunOutcome::Failed(text),
            Err(interrupt) => crashed(interrupt),
        },
        Err(interrupt) => crashed(interrupt),
    };
    let unused = vm.end_replay();
    Run {
        outcome,
        recording: vm.take_recording(),
        unused,
    }
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
        Ok(Value::Failure(error)) => match vm.to_text(&error) {
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
        .sources
        .get(&module)
        .and_then(|file| Path::new(&file.name).parent())
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
    }
}
