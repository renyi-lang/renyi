//! The promotion of decision AU18 against the interpreter: code objects
//! start on template code at their first call and go on to the Cranelift
//! tier once hot, at a call or, through `rt_osr`, at a loop header of a
//! frame running on template code. The factor is 1 and the compilations
//! synchronous (`RENYI_NATIVE_HOT=1`, `RENYI_NATIVE_SYNC`), so that every
//! loop and every repeated call is promoted early and the same way every
//! run; each program prints what the interpreter prints.

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
    std::env::set_var("RENYI_NATIVE_HOT", "1");
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

#[test]
fn loops_and_calls_promoted_from_template_code_agree_with_the_interpreter() {
    let source = r#"module demo
  purpose: Loops and calls promoted from template code to Cranelift code.

import std.console

type Problem is one of
  purpose: The failure of a step.
  Bad
end

function risky(amount: Integer) returns Integer or fails with Problem
  purpose: Fail on a multiple of seven.

  if amount remainder 7 is 0 then fail with Bad end
  return amount
end

function step_sum(limit: Integer) returns Integer
  purpose: A loop with a handled region in its body, called often enough to be promoted.

  let mutable total be 0
  for each step from 1 to limit
    let value be risky(step) otherwise 0
    change total to total + value
  end
  return total
end

public function main() needs console
  purpose: Print the sums, from loops long enough to be promoted while they run.

  let mutable grand be 0
  let mutable turn be 0
  repeat until turn is 300
    change turn to turn + 1
    change grand to grand + step_sum(turn)
  end
  console.print("{grand}")
  let mutable digits be ""
  for each index from 1 to 2000
    let digit be index remainder 10
    change digits to "{digit}"
  end
  let mutable words: List of Text be []
  for each index from 1 to 500
    change words to words.append("w{index}")
  end
  console.print("{digits} {words.length()}")
end
"#;
    let promoted = run(source, false);
    let interpreted = run(source, true);
    assert_eq!(
        promoted, interpreted,
        "promoted code against the interpreter"
    );
    assert_eq!(promoted.0, RunOutcome::Finished);
}
