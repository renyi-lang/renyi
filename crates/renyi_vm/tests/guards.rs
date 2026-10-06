//! Provenance guards (decision P3, `06-runtime-guarantees.md` section 3):
//! what enters through a capability with `only to` may leave only through
//! its sinks. The origins follow the data through interpolation, records,
//! lists and loops; a decision taken on guarded data is not tracked; a
//! fallible primitive fails with `Guarded`, one that cannot fail crashes; a
//! replay and a test's own grant enforce the same guards; a server does not
//! send a guarded response.

use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use renyi_syntax::SourceFile;
use renyi_vm::{run_program, run_tests, Options, Program, Run, RunOutcome, TestOutcome};

/// A scratch directory inside the workspace's `target`, forward slashes
/// so that it can be spelled in Renyi text: a `data` directory with the
/// secret, an `out` directory the guards allow, and a file beside them.
fn scratch(name: &str) -> String {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf();
    let dir = workspace.join("target/vm-tests").join(name);
    std::fs::create_dir_all(dir.join("data")).expect("scratch directory");
    std::fs::create_dir_all(dir.join("out")).expect("scratch directory");
    std::fs::write(dir.join("data/greeting.txt"), "hello").expect("data file");
    std::fs::write(dir.join("other.txt"), "untouched").expect("other file");
    for stale in ["out/copy.txt", "out/leak.txt"] {
        let _ = std::fs::remove_file(dir.join(stale));
    }
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
    compile_project_from(&checked, &files)
}

fn compile_project_from(checked: &renyi_check::CheckedProject, files: &[SourceFile]) -> Program {
    renyi_vm::compile_project(checked, files)
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

fn options(stdout: &Capture) -> Options {
    Options {
        stdout: Box::new(stdout.clone()),
        stderr: Box::new(Capture::default()),
        stdin: Box::new(std::io::Cursor::new(Vec::new())),
        ..Options::default()
    }
}

fn run(program: &Program, stdout: &Capture, options: Options) -> Run {
    run_program(
        program,
        Options {
            stdout: Box::new(stdout.clone()),
            stderr: Box::new(Capture::default()),
            stdin: Box::new(std::io::Cursor::new(Vec::new())),
            ..options
        },
    )
}

fn crash_message(outcome: &RunOutcome) -> &str {
    match outcome {
        RunOutcome::Crashed { message, .. } => message,
        other => panic!("expected a crash, got {other:?}"),
    }
}

/// Reads the secret, copies it where the guard allows, then prints a
/// number computed from it.
fn copy_program(dir: &str) -> String {
    format!(
        r#"module guarded
  purpose: Copy a secret to the allowed place, then try to print its length.

import std.console
import std.filesystem exposing Path, FileError

public function main() or fails with FileError needs console, filesystem.read("{dir}/data") only to filesystem.write("{dir}/out"), filesystem.write("{dir}/out")
  purpose: Read the greeting, copy it where the guard allows, then print its length.

  let secret be filesystem.read_text(Path("{dir}/data/greeting.txt")) otherwise fail
  filesystem.write_text(path: Path("{dir}/out/copy.txt"), content: "copied: {{secret}}") otherwise fail
  console.print("copied")
  console.print("length {{secret.length()}}")
end
"#
    )
}

#[test]
fn a_guarded_value_reaches_its_sink_and_nothing_else() {
    let dir = scratch("guards-copy");
    let program = compile("guarded.ry", &copy_program(&dir));
    let stdout = Capture::default();
    let outcome = run(&program, &stdout, Options::default()).outcome;
    // the copy went through the sink
    assert_eq!(
        std::fs::read_to_string(format!("{dir}/out/copy.txt")).expect("the copy"),
        "copied: hello"
    );
    assert_eq!(stdout.text(), "copied\n");
    // the length is computed from the secret: it may not reach the console,
    // and `console.print` cannot fail, so the program crashes
    assert_eq!(
        crash_message(&outcome),
        format!(
            "`std.console.print` would send a value from filesystem.read(\"{dir}/data\") to console; the guard allows only filesystem.write(\"{dir}/out\")"
        )
    );
}

#[test]
fn a_fallible_primitive_fails_with_guarded() {
    let dir = scratch("guards-fail");
    let source = format!(
        r#"module leak
  purpose: Try to write the secret outside the sink.

import std.filesystem exposing Path, FileError

public function main() or fails with FileError needs filesystem.read("{dir}/data") only to filesystem.write("{dir}/out"), filesystem.write("{dir}")
  purpose: Read the greeting and write it beside the allowed directory.

  let secret be filesystem.read_text(Path("{dir}/data/greeting.txt")) otherwise fail
  filesystem.write_text(path: Path("{dir}/other.txt"), content: secret) otherwise fail
end
"#
    );
    let program = compile("leak.ry", &source);
    let stdout = Capture::default();
    let outcome = run(&program, &stdout, Options::default()).outcome;
    assert_eq!(
        outcome,
        RunOutcome::Failed(format!(
            "Guarded(origin: \"filesystem.read(\\\"{dir}/data\\\")\", sink: \"filesystem.write(\\\"{dir}/other.txt\\\")\")"
        ))
    );
    assert_eq!(
        std::fs::read_to_string(format!("{dir}/other.txt")).expect("the other file"),
        "untouched"
    );
}

#[test]
fn origins_follow_the_data_and_decisions_do_not() {
    let dir = scratch("guards-flow");
    let source = format!(
        r#"module flows
  purpose: Guarded data through a decision, a record, a list and a loop.

import std.console
import std.filesystem exposing Path, FileError

type Note
  has body: Text
  has times: Integer
end

public function main() or fails with FileError needs console, filesystem.read("{dir}/data") only to filesystem.write("{dir}/out"), filesystem.write("{dir}/out")
  purpose: Decide on the secret, then print what was computed from it.

  let secret be filesystem.read_text(Path("{dir}/data/greeting.txt")) otherwise fail
  if secret.contains("hello") then
    console.print("it greets")
  end
  console.print("{{secret.length() is greater than 3}}")
  let notes be [Note(body: secret, times: 1), Note(body: "plain", times: 2)]
  for each note in notes
    console.print("{{note.times}}")
  end
end
"#
    );
    let program = compile("flows.ry", &source);
    let stdout = Capture::default();
    let outcome = run(&program, &stdout, Options::default()).outcome;
    // a Boolean decided on the secret carries nothing; a record built from
    // it, the list holding the record and every item of a loop over the
    // list carry its origin, so the count of the first note may not print
    assert_eq!(stdout.text(), "it greets\ntrue\n");
    assert!(
        crash_message(&outcome).starts_with("`std.console.print` would send a value from"),
        "{outcome:?}"
    );
}

#[test]
fn a_replay_enforces_the_guard_again() {
    let dir = scratch("guards-replay");
    let program = compile("guarded.ry", &copy_program(&dir));
    let stdout = Capture::default();
    let recorded = run(
        &program,
        &stdout,
        Options {
            record: true,
            ..options(&stdout)
        },
    );
    let message = crash_message(&recorded.outcome).to_string();
    let recording = recorded.recording.expect("a recording");
    assert!(
        recording.grant.contains(&format!(
            "filesystem.read(\"{dir}/data\") only to filesystem.write(\"{dir}/out\")"
        )),
        "{:?}",
        recording.grant
    );
    let primitives: Vec<&str> = recording
        .calls
        .iter()
        .map(|call| call.primitive.as_str())
        .collect();
    assert_eq!(
        primitives,
        [
            "std.filesystem.read_text",
            "std.filesystem.write_text",
            "std.console.print"
        ]
    );
    // the replay answers the read from the recording; the value it brings
    // in is guarded by the grant, not by anything the recording stores
    let replayed = run(
        &program,
        &Capture::default(),
        Options {
            replay: Some(recording),
            ..options(&stdout)
        },
    );
    assert_eq!(crash_message(&replayed.outcome), message);
}

#[test]
fn a_test_runs_under_its_own_guards() {
    let dir = scratch("guards-tests");
    let source = format!(
        r#"module checked
  purpose: Tests with guards of their own.

import std.console
import std.filesystem exposing Path

test "the secret may be printed" needs console, filesystem.read("{dir}/data") only to console
  let secret be filesystem.read_text(Path("{dir}/data/greeting.txt")) otherwise fail
  console.print(secret)
end

test "the secret may not be written" needs filesystem.read("{dir}/data") only to console, filesystem.write("{dir}/out")
  let secret be filesystem.read_text(Path("{dir}/data/greeting.txt")) otherwise fail
  filesystem.write_text(path: Path("{dir}/out/leak.txt"), content: secret) otherwise fail
end
"#
    );
    let program = compile("checked.ry", &source);
    let stdout = Capture::default();
    let report = run_tests(&program, options(&stdout));
    let outcomes: Vec<&TestOutcome> = report.results.iter().map(|r| &r.outcome).collect();
    assert_eq!(outcomes[0], &TestOutcome::Passed, "{}", report.render());
    assert_eq!(
        outcomes[1],
        &TestOutcome::Failed(format!(
            "Guarded(origin: \"filesystem.read(\\\"{dir}/data\\\")\", sink: \"filesystem.write(\\\"{dir}/out/leak.txt\\\")\")"
        )),
        "{}",
        report.render()
    );
    assert_eq!(stdout.text(), "hello\n");
    assert!(!PathBuf::from(format!("{dir}/out/leak.txt")).exists());
}

#[test]
fn a_server_does_not_send_a_guarded_response() {
    let dir = scratch("guards-server");
    let port = 21000 + (std::process::id() % 2000) as u16;
    let source = format!(
        r#"module leaky
  purpose: A server that tries to answer with the secret.

import std.console
import std.filesystem exposing Path
import std.server exposing Request, Response, Port, StartError

function handle(request: Request) returns Response needs filesystem.read("{dir}/data")
  purpose: Answer every request with the greeting file.

  let secret be filesystem.read_text(Path("{dir}/data/greeting.txt")) otherwise return server.bad_request("no greeting")
  return server.ok("{{request.path}}: {{secret}}")
end

public function main() or fails with StartError needs console, network.socket, filesystem.read("{dir}/data") only to console
  purpose: Serve one request, then stop.

  console.print("listening")
  server.serve(port: Port({port}), handler: handle) otherwise fail
  console.print("done")
end
"#
    );
    let server = std::thread::spawn(move || {
        let program = compile("leaky.ry", &source);
        let stdout = Capture::default();
        let outcome = run(
            &program,
            &stdout,
            Options {
                serve_limit: Some(1),
                ..Options::default()
            },
        )
        .outcome;
        (outcome, stdout.text())
    });
    let started = Instant::now();
    let mut stream = loop {
        if let Ok(stream) = TcpStream::connect(("127.0.0.1", port)) {
            break stream;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the server did not start"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    stream
        .write_all(b"GET /hello HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .expect("the request is sent");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("the response is read");
    assert!(response.starts_with("HTTP/1.1 500 "), "{response}");
    assert!(
        response.ends_with(&format!(
            "the response would send a value from filesystem.read(\"{dir}/data\") to network.socket; the guard allows only console"
        )),
        "{response}"
    );
    assert!(!response.contains("hello"), "{response}");
    let (outcome, printed) = server.join().expect("the server thread");
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "listening\ndone\n");
}
