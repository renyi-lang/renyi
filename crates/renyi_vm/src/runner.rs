//! Running a program's `main`, and its `example:` lines and `test` blocks.

use crate::bytecode::CodeKind;
use crate::compile::{Expected, Program};
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

pub fn run_main(program: &Program, options: Options) -> RunOutcome {
    let mut vm = Vm::new(program, options);
    match vm.run_main() {
        Ok(None) => RunOutcome::Finished,
        Ok(Some(error)) => match vm.to_text(&error) {
            Ok(text) => RunOutcome::Failed(text),
            Err(interrupt) => crashed(interrupt),
        },
        Err(interrupt) => crashed(interrupt),
    }
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
            }
        }
        let skipped = self.skipped();
        out.push_str(&format!(
            "{} passed, {} failed{}\n",
            self.passed(),
            self.failed(),
            if skipped > 0 {
                format!(", {skipped} skipped")
            } else {
                String::new()
            }
        ));
        out
    }
}

/// Run every `example:` line and `test` block of the program's own modules
/// (not the library's), in source order.
pub fn run_tests(program: &Program, options: Options) -> TestReport {
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
            Item::Test(index) => run_test(&mut vm, index),
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
    let outcome = match example_outcome(vm, index) {
        Ok(outcome) => outcome,
        Err(interrupt) => TestOutcome::Failed(describe_interrupt(interrupt)),
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

fn run_test(vm: &mut Vm, index: usize) -> TestResult {
    let program = vm.program;
    let test = &program.tests[index];
    let name = format!("test {:?}", test.name);
    let location = program.location(test.module, test.span);
    if let Some(recording) = &test.replays {
        return TestResult {
            name,
            location,
            outcome: TestOutcome::Skipped(format!(
                "replays {recording:?}: recorded runs are not supported by this build"
            )),
        };
    }
    let outcome = match vm.call_code(test.code, Vec::new()) {
        Ok(Value::Failure(error)) => match vm.to_text(&error) {
            Ok(text) => TestOutcome::Failed(text),
            Err(interrupt) => TestOutcome::Failed(describe_interrupt(interrupt)),
        },
        Ok(_) => TestOutcome::Passed,
        Err(interrupt) => TestOutcome::Failed(describe_interrupt(interrupt)),
    };
    TestResult {
        name,
        location,
        outcome,
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
