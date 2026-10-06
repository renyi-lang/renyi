//! The primitive boundary: a run is recorded and replayed, budgets and
//! scopes are enforced, a `replays` test is answered from its fixture, and
//! `--explain` narrates a run.

use std::cell::RefCell;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;

use renyi_syntax::SourceFile;
use renyi_vm::grant::parse_capability;
use renyi_vm::natives::json::Json;
use renyi_vm::recording::{Call, Outcome};
use renyi_vm::{
    compile_project, denied_functions, run_program, run_tests, Narrowing, Options, Program,
    Recording, Run, RunOutcome, TestOutcome,
};

/// A scratch directory inside the workspace's `target`, forward slashes
/// so that it can be spelled in Renyi text.
fn scratch(name: &str) -> String {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf();
    let dir = workspace.join("target/vm-tests").join(name);
    std::fs::create_dir_all(dir.join("data")).expect("scratch directory");
    std::fs::write(dir.join("data/greeting.txt"), "hello").expect("data file");
    std::fs::write(dir.join("other.txt"), "secret").expect("other file");
    dir.display().to_string().replace('\\', "/")
}

fn compile(name: &str, source: &str) -> Program {
    let file = SourceFile::new(name, source);
    let files = vec![file];
    let checked = renyi_check::check_project(&files);
    let errors: Vec<_> = checked.modules[0]
        .diagnostics
        .iter()
        .filter(|d| d.is_error())
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    compile_project(&checked, &files)
}

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Capture {
    fn text(&self) -> String {
        String::from_utf8(self.0.borrow().clone()).expect("utf-8")
    }
}

struct Streams {
    stdout: Capture,
    stderr: Capture,
}

fn options(streams: &Streams) -> Options {
    Options {
        stdout: Box::new(streams.stdout.clone()),
        stderr: Box::new(streams.stderr.clone()),
        stdin: Box::new(std::io::Cursor::new(Vec::new())),
        ..Options::default()
    }
}

fn run(program: &Program, options: Options) -> Run {
    run_program(program, options)
}

fn effects_program(dir: &str) -> String {
    format!(
        r#"module demo
  purpose: Use several effects.

import std.console
import std.filesystem exposing Path, FileError
import std.random
import std.time

public function main() or fails with FileError needs console, filesystem.read("{dir}"), random, time
  purpose: Print a random number, today's date and a file.

  let number be random.integer(lowest: 1, highest: 1000000)
  let today be time.today()
  let text be filesystem.read_text(Path("{dir}/data/greeting.txt")) otherwise fail
  console.print("{{number}} {{today}} {{text}}")
end
"#
    )
}

#[test]
fn a_run_is_recorded_and_replayed() {
    let dir = scratch("record");
    let program = compile("demo.ry", &effects_program(&dir));

    // record
    let streams = Streams {
        stdout: Capture::default(),
        stderr: Capture::default(),
    };
    let recorded = run(
        &program,
        Options {
            record: true,
            revision: Some("abc1234".to_string()),
            ..options(&streams)
        },
    );
    assert_eq!(recorded.outcome, RunOutcome::Finished);
    let recording = recorded.recording.expect("a recording");
    let primitives: Vec<&str> = recording
        .calls
        .iter()
        .map(|call| call.primitive.as_str())
        .collect();
    assert_eq!(
        primitives,
        [
            "std.random.integer",
            "std.time.today",
            "std.filesystem.read_text",
            "std.console.print"
        ]
    );
    assert_eq!(recording.program, "demo");
    assert_eq!(recording.revision.as_deref(), Some("abc1234"));
    assert_eq!(recording.grant[0], "console");
    assert_eq!(recording.grant[1], format!("filesystem.read(\"{dir}\")"));
    assert_eq!(
        recording.calls[2].capability,
        format!("filesystem.read(\"{dir}/data/greeting.txt\")")
    );
    assert_eq!(
        recording.calls[2].outcome,
        Outcome::Success(Json::Text("hello".to_string()))
    );
    let printed = streams.stdout.text();
    assert!(printed.ends_with(" hello\n"), "{printed}");
    // the recording round-trips through its text
    let text = recording.render();
    assert_eq!(Recording::parse(&text).unwrap(), recording);

    // replay: the same run, nothing written
    let streams = Streams {
        stdout: Capture::default(),
        stderr: Capture::default(),
    };
    let replayed = run(
        &program,
        Options {
            replay: Some(recording.clone()),
            ..options(&streams)
        },
    );
    assert_eq!(replayed.outcome, RunOutcome::Finished);
    assert!(replayed.unused.is_empty());
    assert_eq!(streams.stdout.text(), "");

    // a replay stops at the first call that differs, naming both
    let mut edited = recording.clone();
    edited.calls[3].arguments[0].1 = Json::Text("something else".to_string());
    let stopped = run(
        &program,
        Options {
            replay: Some(edited),
            ..options(&streams)
        },
    );
    match stopped.outcome {
        RunOutcome::Crashed { message, location } => {
            assert!(
                message.starts_with("the recording has no call std.console.print(text: \""),
                "{message}"
            );
            assert!(
                message.ends_with(
                    "; the nearest recorded call is #4 std.console.print(text: \"something else\")"
                ),
                "{message}"
            );
            assert_eq!(location.as_deref(), Some("demo.ry:15"));
        }
        other => panic!("{other:?}"),
    }

    // a recording whose calls the grant does not cover is refused
    let refused = run(
        &program,
        Options {
            replay: Some(recording.clone()),
            narrowing: Narrowing {
                deny: vec![parse_capability("random").unwrap()],
                ..Narrowing::default()
            },
            ..options(&streams)
        },
    );
    assert_eq!(
        refused.outcome,
        RunOutcome::Crashed {
            message: format!(
                "the recording's call #1 uses random, which the grant console, filesystem.read(\"{dir}\"), time does not cover"
            ),
            location: None
        }
    );
    assert_eq!(
        denied_functions(&program, &parse_capability("random").unwrap()),
        ["demo.main"]
    );
    assert!(denied_functions(&program, &parse_capability("network").unwrap()).is_empty());
}

#[test]
fn budgets_and_scopes_are_enforced() {
    let dir = scratch("budget");
    let source = format!(
        r#"module demo
  purpose: Read past a budget and outside a scope.

import std.console
import std.filesystem exposing Path, FileError

function read(path: Path) needs console, filesystem.read("{dir}/data")
  purpose: Print the file or the error.

  match filesystem.read_text(path)
    when success(text) then console.print(text)
    when failure(error) then console.print(error.to_text())
  end
end

public function main() needs console, filesystem.read("{dir}/data") at most 2 per run
  purpose: Read three times inside the scope, once outside.

  read(Path("{dir}/data/greeting.txt"))
  read(Path("{dir}/data/../data/greeting.txt"))
  read(Path("{dir}/data/greeting.txt"))
  read(Path("{dir}/other.txt"))
end
"#
    );
    let program = compile("demo.ry", &source);
    let streams = Streams {
        stdout: Capture::default(),
        stderr: Capture::default(),
    };
    let outcome = run(&program, options(&streams)).outcome;
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        streams.stdout.text(),
        format!(
            "hello\nhello\nOverBudget(path: \"{dir}/data/greeting.txt\")\nPermissionDenied(path: \"{dir}/other.txt\")\n"
        )
    );

    // the command line can only tighten: one read per run
    let streams = Streams {
        stdout: Capture::default(),
        stderr: Capture::default(),
    };
    let outcome = run(
        &program,
        Options {
            narrowing: Narrowing {
                budgets: vec![parse_capability("filesystem at most 1 per run").unwrap()],
                ..Narrowing::default()
            },
            ..options(&streams)
        },
    )
    .outcome;
    assert_eq!(outcome, RunOutcome::Finished);
    assert!(streams.stdout.text().starts_with("hello\nOverBudget("));

    // `--allow-read` narrows the scope further
    let streams = Streams {
        stdout: Capture::default(),
        stderr: Capture::default(),
    };
    let mut allow = parse_capability("filesystem.read").unwrap();
    allow.scope = Some(format!("{dir}/data/elsewhere"));
    let outcome = run(
        &program,
        Options {
            narrowing: Narrowing {
                allow: vec![allow],
                ..Narrowing::default()
            },
            ..options(&streams)
        },
    )
    .outcome;
    assert_eq!(outcome, RunOutcome::Finished);
    assert!(streams.stdout.text().starts_with("PermissionDenied("));
}

#[test]
fn a_replays_test_is_answered_from_its_fixture() {
    let dir = scratch("replays");
    let fixture = format!("{dir}/fixtures/greeting.json");
    let _ = std::fs::remove_file(&fixture);
    let source = format!(
        r#"module demo
  purpose: A test that replays a recording.

import std.console
import std.filesystem exposing Path, FileError

test "the greeting is read" needs console, filesystem.read replays "{fixture}"
  let text be filesystem.read_text(Path("{dir}/data/greeting.txt")) otherwise fail
  console.print(text)
  check text is "hello"
end

test "a live test" needs filesystem.read
  check filesystem.exists(Path("{dir}/data/greeting.txt"))
end
"#
    );
    let program = compile("demo.ry", &source);
    let streams = Streams {
        stdout: Capture::default(),
        stderr: Capture::default(),
    };

    // without the fixture the test fails and says so
    let report = run_tests(&program, options(&streams));
    match &report.results[0].outcome {
        TestOutcome::Failed(reason) => {
            assert!(reason.starts_with("cannot read the recording"), "{reason}")
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(report.results[1].outcome, TestOutcome::Passed);

    // `--refresh` runs it live and writes the fixture
    let report = run_tests(
        &program,
        Options {
            refresh: Some("the greeting is read".to_string()),
            ..options(&streams)
        },
    );
    assert_eq!(
        report.results[0].outcome,
        TestOutcome::Recorded(fixture.clone())
    );
    assert_eq!(streams.stdout.text(), "hello\n");
    assert!(report.render().contains("1 passed, 0 failed, 1 recorded"));
    let recording = Recording::parse(&std::fs::read_to_string(&fixture).unwrap()).unwrap();
    assert_eq!(recording.calls.len(), 2);
    assert_eq!(recording.grant, ["console", "filesystem.read"]);

    // now the test is answered from the fixture: offline, nothing printed
    std::fs::remove_file(format!("{dir}/data/greeting.txt")).unwrap();
    let streams = Streams {
        stdout: Capture::default(),
        stderr: Capture::default(),
    };
    let report = run_tests(&program, options(&streams));
    assert_eq!(report.results[0].outcome, TestOutcome::Passed);
    assert_eq!(streams.stdout.text(), "");
    match &report.results[1].outcome {
        TestOutcome::Failed(reason) => assert!(reason.starts_with("check failed"), "{reason}"),
        other => panic!("{other:?}"),
    }

    // `--strict` reports a recorded call the test never reaches
    let mut padded = recording.clone();
    padded.push(Call {
        sequence: 0,
        capability: "console".to_string(),
        primitive: "std.console.print".to_string(),
        arguments: vec![("text".to_string(), Json::Text("extra".to_string()))],
        outcome: Outcome::Success(Json::Null),
        duration_ms: None,
        at_ms: 9,
    });
    std::fs::write(&fixture, padded.render()).unwrap();
    let report = run_tests(
        &program,
        Options {
            strict: true,
            ..options(&streams)
        },
    );
    assert_eq!(
        report.results[0].outcome,
        TestOutcome::Failed(
            "1 recorded call was never reached: #3 std.console.print(text: \"extra\")".to_string()
        )
    );
    let report = run_tests(&program, options(&streams));
    assert_eq!(report.results[0].outcome, TestOutcome::Passed);

    // a recording the test's grant does not cover is refused
    let mut foreign = recording.clone();
    foreign.calls[0].capability = "network.http(\"evil.example\")".to_string();
    std::fs::write(&fixture, foreign.render()).unwrap();
    let report = run_tests(&program, options(&streams));
    assert_eq!(
        report.results[0].outcome,
        TestOutcome::Failed(
            "the recording's call #1 uses network.http(\"evil.example\"), which the grant console, filesystem.read does not cover".to_string()
        )
    );

    // `--refresh` of a test without `replays`
    let report = run_tests(
        &program,
        Options {
            refresh: Some("a live test".to_string()),
            ..options(&streams)
        },
    );
    assert_eq!(
        report.results[1].outcome,
        TestOutcome::Failed(
            "`--refresh` names this test, but it has no `replays` clause".to_string()
        )
    );
}

#[test]
fn a_run_is_narrated() {
    let dir = scratch("explain");
    let source = format!(
        r#"module demo
  purpose: A narrated run.

import std.console
import std.filesystem exposing Path, FileError

function size_of(path: Path) returns Integer needs filesystem.read
  purpose: The length of a file's text.

  let text be filesystem.read_text(path) otherwise return 0
  return text.length()
end

function twice(number: Integer) returns Integer
  return number * 2
end

public function main() needs console, filesystem.read
  purpose: Print the doubled size of the greeting.

  let size be size_of(Path("{dir}/data/greeting.txt"))
  console.print("{{twice(size)}}")
end
"#
    );
    let program = compile("demo.ry", &source);
    let streams = Streams {
        stdout: Capture::default(),
        stderr: Capture::default(),
    };
    let outcome = run(
        &program,
        Options {
            explain: true,
            ..options(&streams)
        },
    )
    .outcome;
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(streams.stdout.text(), "10\n");
    let expected = format!(
        "Print the doubled size of the greeting. (main)
  The length of a file's text. (demo.size_of, path: \"{dir}/data/greeting.txt\")
    filesystem.read read_text(path: \"{dir}/data/greeting.txt\") -> \"hello\"
    -> 5
  twice (demo.twice, number: 5)
    -> 10
  console \"10\"
"
    );
    // a live effect line ends in its duration when it took a millisecond
    let narrated: String = streams
        .stderr
        .text()
        .lines()
        .map(|line| match line.rsplit_once(", ") {
            Some((head, tail)) if tail.ends_with(" ms") => format!("{head}\n"),
            _ => format!("{line}\n"),
        })
        .collect();
    assert_eq!(narrated, expected);
}

#[test]
fn the_grant_narrows_along_the_call_chain() {
    let dir = scratch("stack");
    let source = format!(
        r#"module demo
  purpose: A helper declared for one directory cannot read outside it.

import std.console
import std.filesystem exposing Path, FileError

function show(path: Path) needs console, filesystem.read("{dir}/data")
  purpose: Print the file or the error.

  match filesystem.read_text(path)
    when success(text) then console.print(text)
    when failure(error) then console.print(error.to_text())
  end
end

function present(path: Path) returns Boolean needs filesystem.read("{dir}/data")
  purpose: Whether the file exists.

  return filesystem.exists(path)
end

public function main() needs console, filesystem.read("{dir}")
  purpose: Read through the helper, then directly, then crash in the helper.

  show(Path("{dir}/data/greeting.txt"))
  show(Path("{dir}/data/../other.txt"))
  match filesystem.read_text(Path("{dir}/other.txt"))
    when success(text) then console.print(text)
    when failure(error) then console.print(error.to_text())
  end
  let there be present(Path("{dir}/other.txt"))
  console.print("{{there}}")
end
"#
    );
    let program = compile("demo.ry", &source);
    let streams = Streams {
        stdout: Capture::default(),
        stderr: Capture::default(),
    };
    let outcome = run(&program, options(&streams)).outcome;
    // `main` may read `other.txt`; `show`, declared for `data`, may not
    assert_eq!(
        streams.stdout.text(),
        format!("hello\nPermissionDenied(path: \"{dir}/other.txt\")\nsecret\n")
    );
    // a primitive that cannot fail crashes, naming the narrowing function
    assert_eq!(
        outcome,
        RunOutcome::Crashed {
            message: format!(
                "`std.filesystem.exists` needs filesystem.read(\"{dir}/other.txt\"), which the grant console, filesystem.read(\"{dir}/data\") of `demo.present` does not allow"
            ),
            location: Some("demo.ry:19".to_string()),
        }
    );
}

#[test]
fn a_recording_is_redacted() {
    let dir = scratch("redact");
    let source = format!(
        r#"module demo
  purpose: Keep a secret out of the recording.

import std.console
import std.environment
import std.filesystem exposing Path, FileError

public function main() or fails with FileError
  needs console, environment("RENYI_DEMO_SECRET"), filesystem.write("{dir}")
  purpose: Write the secret to a file and say so.

  let secret be environment.get("RENYI_DEMO_SECRET") otherwise "none"
  filesystem.write_text(path: Path("{dir}/token.txt"), content: "token {{secret}}") otherwise fail
  console.print("written")
end
"#
    );
    let program = compile("demo.ry", &source);
    // test data: a placeholder standing in for a real secret
    std::env::set_var("RENYI_DEMO_SECRET", "dummy-value");
    let streams = Streams {
        stdout: Capture::default(),
        stderr: Capture::default(),
    };
    let recorded = run(
        &program,
        Options {
            record: true,
            redact: vec!["RENYI_DEMO_SECRET".to_string(), "content".to_string()],
            ..options(&streams)
        },
    );
    assert_eq!(recorded.outcome, RunOutcome::Finished);
    let recording = recorded.recording.expect("a recording");
    let text = recording.render();
    assert!(!text.contains("dummy-value"), "{text}");
    assert_eq!(
        recording.calls[0].outcome,
        Outcome::Success(Json::Text("<redacted>".to_string()))
    );
    assert_eq!(
        recording.calls[1].arguments[1],
        ("content".to_string(), Json::Text("<redacted>".to_string()))
    );
    assert_eq!(
        recording.calls[2].arguments[0],
        ("text".to_string(), Json::Text("written".to_string()))
    );
    assert_eq!(
        std::fs::read_to_string(format!("{dir}/token.txt")).unwrap(),
        "token dummy-value"
    );

    // the placeholder answers the replayed variable and matches the write
    std::env::remove_var("RENYI_DEMO_SECRET");
    let streams = Streams {
        stdout: Capture::default(),
        stderr: Capture::default(),
    };
    let replayed = run(
        &program,
        Options {
            replay: Some(recording),
            ..options(&streams)
        },
    );
    assert_eq!(replayed.outcome, RunOutcome::Finished);
    assert!(replayed.unused.is_empty());
}
