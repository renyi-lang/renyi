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

#[test]
fn records_of_two_fields_returned_as_pairs_agree_when_promoted_from_template_code() {
    // decision AW9: a template-code caller reaches a promoted pair callee
    // through its trampoline, which answers the record; a frame promoted
    // at a loop header with a pair slot live has the slot's record taken
    // apart into its cells on the way in
    let source = r#"module demo
  purpose: Records of two fields returned as pairs: every shape of call, use and hand-back.

import std.console

type State
  purpose: A counter with a label and a history.
  has tally: Integer
  has label: Text
  has history: List of Integer
end

type Step of Value
  purpose: A value computed and the state after it.
  has value: Value
  has state: State
end

type Overflow
  purpose: The count past its limit.
  has reached: Integer
end

type Triple
  purpose: A record of three fields, which stays a record.
  has lowest: Integer
  has second: Integer
  has third: Text
end

function advance(state: State, amount: Integer) returns Step of Integer
  purpose: The count raised and recorded, with the new count.

  let tally be state.tally + amount
  return Step(value: tally, state: state with tally: tally, history: state.history.append(tally))
end

function examine(state: State, limit: Integer) returns Step of Boolean
  purpose: Whether the count is under the limit, the label marked when it is.

  if state.tally is less than limit then
    return Step(value: true, state: state with label: "{state.label}+")
  end
  return Step(value: false, state: state)
end

function relay(state: State, limit: Integer) returns Step of Boolean
  purpose: The check passed on as it comes.

  return examine(state: state, limit: limit)
end

function pick(state: State, limit: Integer) returns Step of Boolean
  purpose: The check kept in a local and returned whole, or remade.

  let done be examine(state: state, limit: limit)
  if done.value then return done end
  return Step(value: false, state: done.state with label: "over")
end

function bounded(state: State, limit: Integer) returns Step of Integer or fails with Overflow
  purpose: The count advanced by one, or a failure past the limit.

  if state.tally is at least limit then fail with Overflow(reached: state.tally) end
  return advance(state: state, amount: 1)
end

function climb(state: State, limit: Integer) returns Step of Integer or fails with Overflow
  purpose: Two bounded steps, the first kept, the second returned as it comes.

  let lower be bounded(state: state, limit: limit) otherwise fail
  return bounded(state: lower.state, limit: limit) otherwise fail
end

function settle(state: State, limit: Integer) returns Step of Integer
  purpose: A bounded step, or the state as it stands.

  let stepped be bounded(state: state, limit: limit) otherwise Step(value: 99, state: state)
  return stepped
end

function summed(state: State, turns: Integer) returns Step of Integer
  purpose: A loop that doubles past the machine word, with a step held across it.

  let before be advance(state: state, amount: 100)
  let mutable total be 0
  let mutable doubled be 1
  for each turn from 1 to turns
    change doubled to doubled * 2
    change total to total + before.value + turn
  end
  return Step(value: total, state: before.state with label: "{doubled}")
end

function triple(amount: Integer) returns Triple
  purpose: A record of three fields.

  return Triple(lowest: amount, second: amount * 2, third: "t")
end

public function main() needs console
  purpose: Runs every form and prints what each leaves.

  let mutable state be State(tally: 0, label: "", history: [0])
  let mutable total be 0
  for each amount from 1 to 5
    let step be advance(state: state, amount: amount)
    let value be step.value
    change state to step.state
    change total to total + value
  end
  console.print("advanced: {total} {state.tally}")
  let checked be examine(state: state, limit: 100)
  let flag be checked.value
  change state to checked.state
  console.print("checked: {flag} {state.label}")
  change state to relay(state: state, limit: 10).state
  console.print("relayed: {state.label}")
  let picked be pick(state: state, limit: 100)
  let over be pick(state: state, limit: 1)
  console.print("picked: {picked.value} {picked.state.label} {over.value} {over.state.label}")
  let kept be advance(state: state, amount: 7)
  let mutable steps be [kept]
  change steps to steps.append(kept)
  let head be steps.at(0) otherwise kept
  console.print("kept: {kept.value} {head.state.tally} {kept.state.tally}")
  let climbed be climb(state: state, limit: 100) otherwise Step(value: 0, state: state)
  console.print("climbed: {climbed.value} {climbed.state.tally}")
  let blocked be climb(state: state, limit: 1) otherwise Step(value: 0, state: state)
  console.print("blocked: {blocked.value} {blocked.state.tally}")
  let settled be settle(state: state, limit: 1)
  console.print("settled: {settled.value} {settled.state.tally}")
  let summed_step be summed(state: state, turns: 70)
  console.print("doubled: {summed_step.state.label} {summed_step.value} {summed_step.state.tally}")
  let three be triple(3)
  console.print("triple: {three.lowest} {three.second} {three.third}")
end
"#;
    let native = run(source, false);
    let interpreted = run(source, true);
    assert_eq!(native, interpreted, "promoted code against the interpreter");
    assert_eq!(native.0, RunOutcome::Finished);
    assert_eq!(
        native.1,
        "advanced: 35 15\nchecked: true +\nrelayed: +\npicked: true ++ false over\nkept: 22 22 22\nclimbed: 17 17\nblocked: 0 15\nsettled: 99 15\ndoubled: 1180591620717411303424 10535 115\ntriple: 3 6 t\n"
    );
}
