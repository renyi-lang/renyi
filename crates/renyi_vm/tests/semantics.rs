//! Behaviour the design promises that the gap audit (`docs/GAPS.md`,
//! section 1) found missing: an expired deadline is the function's
//! failure, an update keeps a record's refinements, a scoped grant covers
//! the target of a copy, SQLite refuses a path outside the scope with its
//! own error, a declared `equals` decides `is`, a Float never overflows
//! silently, and the text and list methods answer their edge cases; and
//! a profiled run reports its counts (decision X4).

use std::cell::RefCell;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;

use renyi_syntax::SourceFile;
use renyi_vm::bytecode::Op;
use renyi_vm::grant::parse_capability;
use renyi_vm::{compile_project, run_program, Narrowing, Options, Program, RunOutcome};

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
    std::fs::create_dir_all(dir.join("out")).expect("scratch directory");
    std::fs::write(dir.join("data/greeting.txt"), "hello").expect("data file");
    for stale in ["out/copy.txt", "elsewhere.txt"] {
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

/// Run `main` with the options, capturing the standard output.
fn run(source: &str, options: Options) -> (RunOutcome, String) {
    let program = compile("demo.ry", source);
    let stdout = Capture::default();
    let outcome = run_program(
        &program,
        Options {
            stdout: Box::new(stdout.clone()),
            stderr: Box::new(Capture::default()),
            stdin: Box::new(std::io::Cursor::new(Vec::new())),
            ..options
        },
    )
    .outcome;
    (outcome, stdout.text())
}

fn crash_message(outcome: &RunOutcome) -> &str {
    match outcome {
        RunOutcome::Crashed { message, .. } => message,
        other => panic!("expected a crash, got {other:?}"),
    }
}

#[test]
fn an_expired_deadline_is_the_functions_failure() {
    let source = r#"module demo
  purpose: An expired deadline fails the function instead of crashing.

import std.console
import std.time

function slow() returns Integer or fails with TimedOut needs time
  purpose: Sleep past a short deadline.

  run concurrently within time.milliseconds(20)
    time.sleep(time.milliseconds(60))
  end
  return 1
end

public function main() or fails with TimedOut needs console, time
  purpose: Handle one expiry, then let the next one out.

  let outcome be slow() otherwise 0
  console.print("{outcome}")
  run concurrently within time.milliseconds(20)
    time.sleep(time.milliseconds(60))
  end
  console.print("unreachable")
end
"#;
    let (outcome, printed) = run(source, Options::default());
    assert_eq!(printed, "0\n");
    assert_eq!(
        outcome,
        RunOutcome::Failed("TimedOut(after: 20ms)".to_string())
    );
}

#[test]
fn an_update_keeps_the_refinements() {
    let source = r#"module demo
  purpose: An update checks the refinement again.

import std.console

type Person
  has name: Text
  has age: Integer where age is at least 0
end

public function main() or fails with ConstraintViolation needs console
  purpose: Age a person by a literal, then by a computed amount.

  let ann be Person(name: "Ann", age: 30)
  let fine be ann with age: 31
  console.print("{fine.age}")
  let years be 0 - 5
  let older be ann with age: years otherwise fail
  console.print(older.name)
end
"#;
    let (outcome, printed) = run(source, Options::default());
    assert_eq!(printed, "31\n");
    match outcome {
        RunOutcome::Failed(error) => assert!(
            error.starts_with("ConstraintViolation(type_name: \"Person\""),
            "{error}"
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_copy_needs_its_target_in_the_write_scope() {
    let dir = scratch("semantics-copy");
    let source = format!(
        r#"module demo
  purpose: A copy's target must lie in the write scope.

import std.console
import std.filesystem exposing Path, FileError

function copy_to(target: Path) returns Text or fails with FileError needs filesystem("{dir}")
  purpose: Copy the greeting to the target.

  filesystem.copy(source: Path("{dir}/data/greeting.txt"), target: target) otherwise fail
  return "copied"
end

public function main() needs console, filesystem("{dir}")
  purpose: Copy inside the write scope, then outside it.

  match copy_to(Path("{dir}/out/copy.txt"))
    when success(text) then console.print(text)
    when failure(error) then console.print(error.to_text())
  end
  match copy_to(Path("{dir}/elsewhere.txt"))
    when success(text) then console.print(text)
    when failure(error) then console.print(error.to_text())
  end
end
"#
    );
    let mut allow = parse_capability("filesystem.write").unwrap();
    allow.scope = Some(format!("{dir}/out"));
    let (outcome, printed) = run(
        &source,
        Options {
            narrowing: Narrowing {
                allow: vec![allow],
                ..Narrowing::default()
            },
            ..Options::default()
        },
    );
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        printed,
        format!("copied\nPermissionDenied(path: \"{dir}/elsewhere.txt\")\n")
    );
    assert_eq!(
        std::fs::read_to_string(format!("{dir}/out/copy.txt")).expect("the copy"),
        "hello"
    );
    assert!(!PathBuf::from(format!("{dir}/elsewhere.txt")).exists());
}

#[test]
fn sqlite_refuses_a_path_outside_the_scope_with_its_own_error() {
    let dir = scratch("semantics-sqlite");
    let source = format!(
        r#"module demo
  purpose: Opening a database outside the scope fails with the module's error.

import std.console
import std.filesystem exposing Path
import std.sqlite

public function main() needs console, filesystem("{dir}/db")
  purpose: Open a database outside the allowed directory.

  match sqlite.open(Path("{dir}/other.sqlite"))
    when success(connection) then connection.close()
    when failure(error) then console.print(error.to_text())
  end
end
"#
    );
    let (outcome, printed) = run(&source, Options::default());
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        printed,
        format!("PermissionDenied(path: \"{dir}/other.sqlite\")\n")
    );
}

#[test]
fn a_declared_equals_decides_is() {
    let source = r#"module demo
  purpose: A declared `equals` decides `is`; collections keep the derived form.

import std.console

type Word
  has text: Text
end

ability Equal for Word
  function equals(self, other: Word) returns Boolean
    return self.text.to_lower() is other.text.to_lower()
  end
end

public function main() needs console
  purpose: Compare two spellings of one word.

  let shout be Word(text: "HELLO")
  let plain be Word(text: "hello")
  console.print("{shout is plain}")
  console.print("{shout is not plain}")
  console.print("{[plain].contains(shout)}")
end
"#;
    let (outcome, printed) = run(source, Options::default());
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "true\nfalse\nfalse\n");
}

#[test]
fn a_float_past_its_range_is_a_crash() {
    let source = r#"module demo
  purpose: A Float past its range is a crash, never infinity.

import std.console

public function main() needs console
  purpose: Multiply until the Float overflows.

  let mutable value be 1.0.to_float()
  let ten be 10.0.to_float()
  let mutable rounds be 0
  repeat until rounds is 400
    change value to value * ten
    change rounds to rounds + 1
  end
  console.print("{value}")
end
"#;
    let (outcome, printed) = run(source, Options::default());
    assert_eq!(printed, "");
    assert_eq!(crash_message(&outcome), "the Float result is out of range");
}

#[test]
fn text_and_list_methods_answer_their_edge_cases() {
    let source = r#"module demo
  purpose: Edge cases of the prelude's text and list methods.

import std.console
import std.time

public function main() needs console
  purpose: Print the answers.

  let huge be 100000000 * 100000000 * 100000000
  let items be [1, 2, 3]
  let far be items.at(huge) otherwise 0
  console.print("{far}")
  console.print("{items.take(huge).length()}")
  console.print("{items.drop(huge).length()}")
  console.print("{items.slice(start: 1, stop: huge).length()}")
  console.print("{items.slice(start: 2, stop: 1).length()}")
  let first_two be items.slice(start: 0 - 1, stop: 2)
  console.print("{first_two.length()}")
  console.print("{first_two.last() otherwise 0}")
  let pieces be "abc".split("")
  console.print("{pieces.length()}")
  console.print("abc".replace(old: "", new: "x"))
  match time.parse_instant("2024-05-01T1:02:03Z")
    when success(instant) then console.print("{instant}")
    when failure(error) then console.print(error.to_text())
  end
end
"#;
    let (outcome, printed) = run(source, Options::default());
    assert_eq!(outcome, RunOutcome::Finished);
    let lines: Vec<&str> = printed.lines().collect();
    assert_eq!(&lines[..9], ["0", "3", "0", "2", "0", "2", "2", "3", "abc"]);
    assert!(lines[9].starts_with("InvalidDate("), "{}", lines[9]);
}

#[test]
fn a_public_constant_of_another_module_is_read_through_its_namespace() {
    let util = "module util\n  purpose: Limits.\n\npublic let limit: Integer be 3\n  purpose: The limit.\n";
    let main = "module demo\n  purpose: Read a constant of another module.\n\nimport std.console\nimport util\n\npublic function main() needs console\n  purpose: Print the limit.\n\n  console.print(\"{util.limit}\")\nend\n";
    let files = vec![
        SourceFile::new("demo.ry", main),
        SourceFile::new("util.ry", util),
    ];
    let checked = renyi_check::check_project(&files);
    for module in &checked.modules {
        let errors: Vec<_> = module.diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "{errors:?}");
    }
    let program = compile_project(&checked, &files);
    let stdout = Capture::default();
    let outcome = run_program(
        &program,
        Options {
            stdout: Box::new(stdout.clone()),
            stderr: Box::new(Capture::default()),
            stdin: Box::new(std::io::Cursor::new(Vec::new())),
            ..Options::default()
        },
    )
    .outcome;
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(stdout.text(), "3\n");
}

#[test]
fn a_date_built_at_run_time_needs_a_day_its_month_has() {
    let source = r#"module demo
  purpose: A Date from run-time values fails for a day the month lacks.

import std.console
import std.time exposing Date

public function main() or fails with ConstraintViolation needs console
  purpose: Build February 30th from a variable.

  let day be 30
  let leap be Date(year: 2024, month: 2, day: day - 1) otherwise fail
  console.print("{leap.day}")
  let invalid be Date(year: 2024, month: 2, day: day) otherwise fail
  console.print("{invalid.day}")
end
"#;
    let (outcome, printed) = run(source, Options::default());
    assert_eq!(printed, "29\n");
    match outcome {
        RunOutcome::Failed(error) => assert!(
            error.starts_with("ConstraintViolation(type_name: \"Date\""),
            "{error}"
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_loop_and_a_query_walk_a_type_through_its_to_list() {
    let source = r#"module demo
  purpose: A loop and a query walk a type through its to_list.

import std.console

type Deck
  has cards: List of Text
end

ability Iterable of Text for Deck
  function to_list(self) returns List of Text
    return self.cards
  end
end

public function main() needs console
  purpose: Print the cards in order and their count.

  let deck be Deck(cards: ["queen", "ace", "king"])
  for each card in deck sorted by card
    console.print(card)
  end
  let total be for each card in deck where card is not "joker" count
  console.print("{total}")
end
"#;
    let (outcome, printed) = run(source, Options::default());
    assert!(matches!(outcome, RunOutcome::Finished), "{outcome:?}");
    assert_eq!(printed, "ace\nking\nqueen\n3\n");
}

#[test]
fn a_profiled_run_reports_its_counts() {
    let source = r#"module demo
  purpose: A profiled run reports what it ran.

import std.console

public function main() needs console
  purpose: Add the numbers up to a thousand.

  let mutable total be 0
  for each number from 1 to 1000
    change total to total + number
  end
  console.print("{total}")
end
"#;
    let program = compile("demo.ry", source);
    let stdout = Capture::default();
    let stderr = Capture::default();
    let outcome = run_program(
        &program,
        Options {
            stdout: Box::new(stdout.clone()),
            stderr: Box::new(stderr.clone()),
            stdin: Box::new(std::io::Cursor::new(Vec::new())),
            profile: true,
            ..Options::default()
        },
    )
    .outcome;
    assert!(matches!(outcome, RunOutcome::Finished), "{outcome:?}");
    assert_eq!(stdout.text(), "500500\n");
    let report = stderr.text();
    assert!(report.starts_with("profile: "), "{report}");
    for title in [
        "samples by function and operation",
        "samples by function",
        "samples by operation",
        "operations by kind",
        "operations by pair",
        "calls by function",
        "primitive calls",
    ] {
        assert!(report.contains(&format!("\n{title}")), "{report}");
    }
    assert!(report.contains("demo.main"), "{report}");
    assert!(report.contains("primitive std.console.print"), "{report}");
    // the loop's thousand steps are counted
    let row = report
        .lines()
        .find(|line| line.ends_with("  IterNext"))
        .expect("the IterNext row");
    let count: u64 = row
        .split_whitespace()
        .next()
        .expect("a count")
        .replace(',', "")
        .parse()
        .expect("a number");
    assert!(count > 1000, "{row}");
}

#[test]
fn break_and_continue_inside_an_expression_drop_its_operands() {
    // decision Y4: a statement loop marks the operand stack at its entry and
    // every `break` or `continue` unwinds to the mark, so an outcome nested
    // in an expression leaves nothing behind
    let source = r#"module demo
  purpose: Interrupt a loop from inside an expression.

import std.console

public function main() needs console
  purpose: Sum the items, skipping one and stopping at another.

  let mutable total be 0
  for each item in [1, 2, 3, 4, 5, 6]
    change total to total + (if item is 3 then continue otherwise item end)
    change total to total * (if item is 5 then break otherwise 1 end)
  end
  let mutable steps be 0
  repeat until steps is 10
    change steps to steps + (match steps when 7 then break otherwise 1 end)
  end
  console.print("{total} {steps}")
end
"#;
    let program = compile("demo.ry", source);
    let main = program
        .codes
        .iter()
        .find(|code| code.name == "main")
        .expect("main");
    let marks = main
        .ops
        .iter()
        .filter(|op| matches!(op, Op::MarkStack(_)))
        .count();
    let unwinds = main
        .ops
        .iter()
        .filter(|op| matches!(op, Op::UnwindStack(_)))
        .count();
    assert_eq!((marks, unwinds), (2, 3), "{:?}", main.ops);
    let (outcome, printed) = run(source, Options::default());
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "12 7\n");
}
