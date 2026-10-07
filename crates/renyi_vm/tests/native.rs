//! The machine code of decision AG1 against the interpreter, with every
//! code object compiled before its first call and every loop entered at
//! its first turn (`RENYI_NATIVE_HOT=0`, so that nothing stays cold):
//! where a typed assumption of the generated
//! code fails at run time (an Integer that leaves the machine word, a
//! guarded Integer, a call past the depth of native frames), the frame
//! is handed to the interpreter and the program prints what the
//! interpreter prints; failures, crashes and deadlines inside generated
//! code come out the same way.

use std::cell::RefCell;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;

use renyi_syntax::SourceFile;
use renyi_vm::{compile_project, run_program, Options, Program, RunOutcome};

fn compile(source: &str) -> Program {
    let file = SourceFile::new("demo.ry", source);
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

/// Run `main` on machine code for everything, or on the interpreter.
fn run(source: &str, interpret: bool) -> (RunOutcome, String) {
    // every code object is compiled before its first call and every loop
    // entered at its first turn; the variable is read when the VM is made
    // (`Jit::new`)
    std::env::set_var("RENYI_NATIVE_HOT", "0");
    let program = compile(source);
    let stdout = Capture::default();
    let outcome = run_program(
        &program,
        Options {
            stdout: Box::new(stdout.clone()),
            stderr: Box::new(Capture::default()),
            stdin: Box::new(std::io::Cursor::new(Vec::new())),
            interpret,
            ..Options::default()
        },
    )
    .outcome;
    (outcome, stdout.text())
}

/// Both ways must agree; the native way's outcome and output are given
/// back for closer checks.
fn both_ways(source: &str) -> (RunOutcome, String) {
    let native = run(source, false);
    let interpreted = run(source, true);
    assert_eq!(native, interpreted, "machine code against the interpreter");
    native
}

/// A scratch directory inside the workspace's `target`, forward slashes
/// so that it can be spelled in Renyi text.
fn scratch(name: &str) -> String {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf();
    let dir = workspace.join("target/vm-tests/native").join(name);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    dir.display().to_string().replace('\\', "/")
}

#[test]
fn an_overflow_hands_the_op_to_the_interpreter_which_has_the_big_integers() {
    let source = r#"module demo
  purpose: Doubling past the machine word carries on in the interpreter.

import std.console

function doubled(value: Integer, times: Integer) returns Integer
  purpose: The value doubled this many times.

  let mutable result be value
  let mutable remaining be times
  repeat until remaining is 0
    change result to result * 2
    change remaining to remaining - 1
  end
  return result
end

public function main() needs console
  purpose: Print two to the sixty-third and to the hundredth.

  console.print("{doubled(value: 1, times: 63)}")
  console.print("{doubled(value: 1, times: 100)}")
  console.print("{doubled(value: 3, times: 64) remainder 1000}")
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        printed,
        "9223372036854775808\n1267650600228229401496703205376\n848\n"
    );
}

#[test]
fn a_guarded_integer_parameter_hands_the_frame_back_at_its_entry() {
    let dir = scratch("guarded");
    std::fs::write(format!("{dir}/count.txt"), "12345").expect("the data file");
    let source = format!(
        r#"module demo
  purpose: A guarded Integer does not fit the typed parameter: the interpreter takes the call.

import std.console
import std.filesystem exposing FileError, Path

function doubled(value: Integer) returns Integer
  purpose: Twice the value.

  return value * 2
end

public function main() or fails with FileError needs console, filesystem.read("{dir}") only to network.http("x")
  purpose: Print whether twice the length of the guarded text is ten.

  let text be filesystem.read_text(Path("{dir}/count.txt")) otherwise fail
  let length be text.length()
  if doubled(length) is 10 then console.print("ten") end
  if doubled(doubled(length)) is 20 then console.print("twenty") end
end
"#
    );
    let (outcome, printed) = both_ways(&source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "ten\ntwenty\n");
}

#[test]
fn recursion_past_the_depth_of_native_frames_carries_on_in_the_interpreter() {
    let source = r#"module demo
  purpose: A thousand nested calls, more than the machine stack takes natively.

import std.console

function depth(remaining: Integer) returns Integer
  purpose: Count down through nested calls.

  if remaining is 0 then return 0 end
  return depth(remaining - 1) + 1
end

public function main() needs console
  purpose: Print the depth reached.

  console.print("{depth(1000)}")
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "1000\n");
}

#[test]
fn failures_in_generated_code_land_on_their_handlers_or_leave_the_frame() {
    let source = r#"module demo
  purpose: A failure inside a handled region takes the fallback; one outside leaves the function.

import std.console

type Odd
  purpose: The number was odd.
  has value: Integer
end

function halved(value: Integer) returns Integer or fails with Odd
  purpose: Half of an even number.

  if value remainder 2 is 1 then fail with Odd(value: value) end
  return value.quotient(2)
end

function total(limit: Integer) returns Integer
  purpose: The halves of the numbers up to the limit, odd ones counting as zero.

  let mutable running be 0
  for each number from 1 to limit
    change running to running + (halved(number) otherwise 0)
  end
  return running
end

public function main() or fails with Odd needs console
  purpose: Print the total, then let an odd number's failure out.

  console.print("{total(10)}")
  let last be halved(7) otherwise fail
  console.print("unreachable {last}")
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(printed, "15\n");
    assert_eq!(outcome, RunOutcome::Failed("Odd(value: 7)".to_string()));
}

#[test]
fn a_crash_in_generated_code_names_its_line() {
    let source = r#"module demo
  purpose: A remainder by zero crashes where it happens.

import std.console

function remainder_of(value: Integer, divisor: Integer) returns Integer
  purpose: The remainder, crashing when the divisor is zero.

  return value remainder divisor
end

public function main() needs console
  purpose: Print one remainder, then crash on the next.

  console.print("{remainder_of(value: 7, divisor: 3)}")
  console.print("{remainder_of(value: 7, divisor: 0)}")
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(printed, "1\n");
    match outcome {
        RunOutcome::Crashed { message, location } => {
            assert_eq!(message, "division by zero");
            assert_eq!(location.as_deref(), Some("demo.ry:9"));
        }
        other => panic!("expected a crash, got {other:?}"),
    }
}

#[test]
fn floats_and_booleans_in_registers_print_as_the_interpreter_prints_them() {
    let source = r#"module demo
  purpose: Float arithmetic and comparisons in registers.

import std.console

function area(width: Float, height: Float) returns Float
  purpose: The product.

  return width * height
end

function between(value: Float, low: Float, high: Float) returns Boolean
  purpose: Whether the value lies between the bounds.

  return value is at least low and value is at most high
end

public function main() needs console
  purpose: Print a few values.

  let size be area(width: 1.5, height: 2.0)
  let inside be between(value: size, low: 1.0, high: 3.0)
  let outside be between(value: size, low: 4.0, high: 5.0)
  console.print("{size} {size / 4.0} {size - 0.25} {inside} {not outside}")
  let mutable tally be 0
  for each index from 10 to 1 by -3
    change tally to tally + index
  end
  console.print("{tally}")
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "3.0 0.75 2.75 true true\n22\n");
}
