//! The direct calls of decision AU21 against the interpreter: a call from
//! template code enters its callee's machine code itself, a template or,
//! once the callee is promoted, the Cranelift tier's trampoline. The
//! hotness factor is the default and the compilations synchronous
//! (`RENYI_NATIVE_SYNC`), so that a short callee called often is promoted
//! while its caller, run once, stays on template code: the calls of the
//! caller then enter the trampoline directly, and the one that passes an
//! Integer the callee's register does not hold sees the frame handed to
//! the interpreter at the callee's start, which runs it to its end.

use std::cell::RefCell;
use std::io::Write;
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

fn run(source: &str, interpret: bool) -> (RunOutcome, String) {
    std::env::set_var("RENYI_NATIVE_SYNC", "1");
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

fn both_ways(source: &str) -> (RunOutcome, String) {
    let direct = run(source, false);
    let interpreted = run(source, true);
    assert_eq!(direct, interpreted, "direct calls against the interpreter");
    direct
}

#[test]
fn a_promoted_callee_handed_back_at_its_start_is_finished_by_the_interpreter() {
    let source = r#"module demo
  purpose: A short callee promoted while its caller stays on template code, then called with a big Integer.

import std.console

function doubled(amount: Integer) returns Integer
  purpose: Twice the amount.

  return amount * 2
end

public function main() needs console
  purpose: Call the callee often enough to promote it, then once with an Integer past the machine word.

  let mutable total be 0
  for each step from 1 to 20000
    change total to total + doubled(step)
  end
  let big be 1000000000000 * 1000000000000
  console.print("{total} {doubled(big)}")
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "400020000 2000000000000000000000000\n");
}

#[test]
fn a_callee_that_narrows_the_grant_gets_its_own() {
    let source = r#"module demo
  purpose: Direct calls of a function whose needs narrow the grant, and of one whose do not.

import std.console

function say(word: Text) needs console
  purpose: Print a word.

  console.print(word)
end

function shout(word: Text) returns Text
  purpose: The word in capitals.

  return word.to_upper()
end

public function main() needs console
  purpose: Print three words twice.

  for each word in ["one", "two", "three"]
    say(word)
    say(shout(word))
  end
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "one\nONE\ntwo\nTWO\nthree\nTHREE\n");
}
