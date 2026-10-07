//! `std.process` (decision AE1): a program runs to its end and reports its
//! completion; a status other than 0 is the failure `Exited` of `execute`
//! and a completion of `attempt`; the options set the directory, the
//! environment, the input and the limit; the scope, the budget and a
//! guard hold at the boundary; a recording replays without starting the
//! program. Every child is the `renyi` binary itself running a small
//! program, so the tests need nothing else installed.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BINARY: &str = env!("CARGO_BIN_EXE_renyi");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf()
}

/// A fresh scratch directory under the workspace's `target`.
fn scratch(name: &str) -> PathBuf {
    let directory = root().join("target/process").join(name);
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("a clean scratch directory");
    }
    std::fs::create_dir_all(&directory).expect("the scratch directory");
    directory
}

/// A path as a Renyi program can spell it: forward slashes.
fn spelled(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn renyi(directory: &Path, args: &[&str]) -> Output {
    Command::new(BINARY)
        .args(args)
        .current_dir(directory)
        .output()
        .expect("renyi runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n")
}

/// A program written into the scratch directory; its path as spelled.
fn write(directory: &Path, name: &str, text: &str) -> String {
    let path = directory.join(name);
    std::fs::write(&path, text).expect("a program written");
    spelled(&path)
}

fn succeeded(output: &Output) -> String {
    assert!(
        output.status.success(),
        "renyi failed:\n{}{}",
        stdout(output),
        stderr(output)
    );
    stdout(output)
}

const VERSION: &str = "module version
  purpose: Run the toolchain named on the command line and print what it reports.

import std.console
import std.environment
import std.process exposing ProcessError

public function main() or fails with ProcessError needs console, environment, process
  purpose: Print the status, the output and the sizes of what `renyi version` wrote.

  let binary be environment.arguments().first() otherwise crash with \"the binary\"
  let finished be process.execute(program: binary, arguments: [\"version\"]) otherwise fail
  let output be finished.output.trim()
  let errors be finished.errors.length()
  let bytes be finished.bytes.length()
  console.print(\"status {finished.status}\")
  console.print(\"output {output}\")
  console.print(\"errors {errors}\")
  console.print(\"bytes {bytes}\")
end
";

#[test]
fn a_program_runs_to_its_end_and_reports_its_completion() {
    let directory = scratch("version");
    let program = write(&directory, "version.ry", VERSION);
    let version = succeeded(&renyi(&directory, &["version"]));
    let output = succeeded(&renyi(&directory, &["run", &program, BINARY]));
    assert_eq!(
        output,
        format!(
            "status 0\noutput {}\nerrors 0\nbytes {}\n",
            version.trim(),
            version.len()
        )
    );
}

const STATUS: &str = "module status
  purpose: Run the toolchain with an unknown command, strictly and leniently.

import std.console
import std.environment
import std.process

public function main() needs console, environment, process
  purpose: Print what `execute` and `attempt` report of a program that exits with 1.

  let binary be environment.arguments().first() otherwise crash with \"the binary\"
  match process.execute(program: binary, arguments: [\"nonsense\"])
    when success(finished) then console.print(\"ran with {finished.status}\")
    when failure(error) then console.print(error.to_text().take(16))
  end
  let finished be process.attempt(
    program: binary,
    arguments: [\"nonsense\"]
  ) otherwise crash with \"attempt\"
  let usage be finished.errors.starts_with(\"usage:\") or finished.output.starts_with(\"usage:\")
  console.print(\"attempt {finished.status}\")
  console.print(\"usage {usage}\")
end
";

#[test]
fn a_status_other_than_zero_is_a_failure_of_execute_and_a_completion_of_attempt() {
    let directory = scratch("status");
    let program = write(&directory, "status.ry", STATUS);
    let output = succeeded(&renyi(&directory, &["run", &program, BINARY]));
    assert_eq!(output, "Exited(program: \nattempt 1\nusage true\n");
}

const PLACE: &str = "module place
  purpose: Print the working directory.

import std.console
import std.environment

public function main() needs console, environment
  purpose: Print it.

  console.print(environment.current_directory())
end
";

const VARIABLE: &str = "module variable
  purpose: Print a variable of the environment.

import std.console
import std.environment

public function main() needs console, environment
  purpose: Print it, or `unset`.

  let value be environment.get(\"RENYI_TEST_VARIABLE\") otherwise \"unset\"
  console.print(value)
end
";

const ECHO: &str = "module echo
  purpose: Print the line read from the standard input.

import std.console

public function main() needs console
  purpose: Read one line and print it back.

  let line be console.read_line() otherwise \"nothing\"
  console.print(\"heard {line}\")
end
";

const SLEEPER: &str = "module sleeper
  purpose: Sleep for ten seconds.

import std.time

public function main() needs time
  purpose: Sleep.

  time.sleep(time.seconds(10))
end
";

const OPTIONS: &str = "module options
  purpose: Run four small programs through the toolchain with options.

import std.console
import std.environment
import std.filesystem exposing Path
import std.process exposing ProcessError
import std.time

public function main() or fails with ProcessError needs console, environment, process
  purpose: Print what each child reports: its directory, a variable, its input, a timeout.

  let binary be environment.arguments().first() otherwise crash with \"the binary\"
  let directory be environment.arguments().at(1) otherwise crash with \"the directory\"
  let inside be Path(directory).join(\"inside\")
  let placed be process.execute_with(
    program: binary,
    arguments: [\"run\", \"{directory}/place.ry\"],
    options: (process.defaults() with directory: inside)
  ) otherwise fail
  let last be Path(placed.output.trim()).name()
  console.print(\"where {last}\")
  let varied be process.execute_with(
    program: binary,
    arguments: [\"run\", \"{directory}/variable.ry\"],
    options: (process.defaults() with environment: {\"RENYI_TEST_VARIABLE\": \"set by the parent\"})
  ) otherwise fail
  console.print(\"variable {varied.output.trim()}\")
  let heard be process.execute_with(
    program: binary,
    arguments: [\"run\", \"{directory}/echo.ry\"],
    options: (process.defaults() with input: \"hello from the parent\\n\")
  ) otherwise fail
  console.print(\"echo {heard.output.trim()}\")
  match process.execute_with(
    program: binary,
    arguments: [\"run\", \"{directory}/sleeper.ry\"],
    options: (process.defaults() with limit: time.milliseconds(300))
  )
    when success(finished) then console.print(\"slept {finished.status}\")
    when failure(error) then console.print(error.to_text().take(17))
  end
end
";

#[test]
fn the_options_set_the_directory_the_environment_the_input_and_the_limit() {
    let directory = scratch("options");
    std::fs::create_dir_all(directory.join("inside")).expect("the inner directory");
    write(&directory, "place.ry", PLACE);
    write(&directory, "variable.ry", VARIABLE);
    write(&directory, "echo.ry", ECHO);
    write(&directory, "sleeper.ry", SLEEPER);
    let program = write(&directory, "options.ry", OPTIONS);
    let started = std::time::Instant::now();
    let output = succeeded(&renyi(
        &directory,
        &["run", &program, BINARY, &spelled(&directory)],
    ));
    assert_eq!(
        output,
        "where inside\nvariable set by the parent\necho heard hello from the parent\nTimeout(program: \n"
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(8),
        "the sleeper was not killed at its limit"
    );
}

const LIMITS: &str = "module limits
  purpose: A program outside the scope, a missing one, and a call past the budget.

import std.console
import std.environment
import std.process

public function main() needs console, environment, process(\"renyi-nothing\") at most 1 per run
  purpose: Print the three failures.

  let binary be environment.arguments().first() otherwise crash with \"the binary\"
  match process.attempt(program: binary, arguments: [\"version\"])
    when success(finished) then console.print(\"ran with {finished.status}\")
    when failure(error) then console.print(error.to_text().take(27))
  end
  match process.attempt(program: \"renyi-nothing\", arguments: [])
    when success(finished) then console.print(\"ran with {finished.status}\")
    when failure(error) then console.print(error.to_text())
  end
  match process.attempt(program: \"renyi-nothing\", arguments: [])
    when success(finished) then console.print(\"ran with {finished.status}\")
    when failure(error) then console.print(error.to_text())
  end
end
";

#[test]
fn the_scope_and_the_budget_hold_at_the_boundary() {
    let directory = scratch("limits");
    let program = write(&directory, "limits.ry", LIMITS);
    let output = succeeded(&renyi(&directory, &["run", &program, BINARY]));
    assert_eq!(
        output,
        "ProgramNotAllowed(program: \nNotFound(program: \"renyi-nothing\")\nOverBudget(program: \"renyi-nothing\")\n"
    );
}

fn guard_program(directory: &str) -> String {
    format!(
        "module guard
  purpose: A value read under a guard may not become a program's argument.

import std.console
import std.environment
import std.filesystem exposing Path
import std.process exposing ProcessError

public function main() or fails with ProcessError needs console, environment, filesystem.read(\"{directory}/data\") only to console, process
  purpose: Read the secret and pass it to the toolchain.

  let binary be environment.arguments().first() otherwise crash with \"the binary\"
  let secret be filesystem.read_text(Path(\"{directory}/data/secret.txt\")) otherwise crash with \"the secret\"
  let finished be process.execute(program: binary, arguments: [secret]) otherwise fail
  console.print(finished.output)
end
"
    )
}

#[test]
fn a_guarded_value_does_not_reach_a_program() {
    let directory = scratch("guard");
    std::fs::create_dir_all(directory.join("data")).expect("the data directory");
    std::fs::write(directory.join("data/secret.txt"), "version").expect("the secret");
    let program = write(&directory, "guard.ry", &guard_program(&spelled(&directory)));
    let output = renyi(&directory, &["run", &program, BINARY]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    let report = stderr(&output);
    assert!(
        report.contains("Guarded(origin: \"filesystem.read("),
        "{report}"
    );
    assert!(report.contains("sink: \"process("), "{report}");
}

const TOUCH: &str = "module touch
  purpose: Leave a marker file named on the command line.

import std.console
import std.environment
import std.filesystem exposing Path, FileError

public function main() or fails with FileError needs console, environment, filesystem.write
  purpose: Write the marker and say so.

  let marker be environment.arguments().first() otherwise crash with \"the marker\"
  filesystem.write_text(path: Path(marker), content: \"ran\") otherwise fail
  console.print(\"touched\")
end
";

const RECORDED: &str = "module recorded
  purpose: Run the program that leaves a marker and print what it says.

import std.console
import std.environment
import std.process exposing ProcessError

public function main() or fails with ProcessError needs console, environment, process
  purpose: Print what the child printed.

  let binary be environment.arguments().first() otherwise crash with \"the binary\"
  let directory be environment.arguments().at(1) otherwise crash with \"the directory\"
  let finished be process.execute(
    program: binary,
    arguments: [\"run\", \"{directory}/touch.ry\", \"{directory}/marker.txt\"]
  ) otherwise fail
  console.print(finished.output.trim())
end
";

#[test]
fn a_recording_replays_without_starting_the_program() {
    let directory = scratch("recorded");
    write(&directory, "touch.ry", TOUCH);
    let program = write(&directory, "recorded.ry", RECORDED);
    let marker = directory.join("marker.txt");
    let recording = spelled(&directory.join("run.json"));
    let spelled_directory = spelled(&directory);
    let output = renyi(
        &directory,
        &[
            "record",
            "--to",
            &recording,
            &program,
            BINARY,
            &spelled_directory,
        ],
    );
    let recorded = succeeded(&output);
    assert_eq!(recorded, "touched\n", "stderr:\n{}", stderr(&output));
    assert_eq!(std::fs::read_to_string(&marker).expect("the marker"), "ran");
    std::fs::remove_file(&marker).expect("the marker removed");
    let replayed = succeeded(&renyi(
        &directory,
        &[
            "run",
            "--replay",
            &recording,
            &program,
            BINARY,
            &spelled_directory,
        ],
    ));
    // a replay answers every effect from the recording and writes nothing,
    // the console included; `reproduce` is the command that shows the output
    assert_eq!(replayed, "");
    assert!(!marker.exists(), "the replay started the program");
    let reproduced = renyi(&directory, &["reproduce", &recording, &program]);
    assert_eq!(
        succeeded(&reproduced),
        "touched\n",
        "stderr:\n{}",
        stderr(&reproduced)
    );
    assert!(!marker.exists(), "the reproduction started the program");
}
