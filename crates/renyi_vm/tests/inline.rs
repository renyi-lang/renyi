//! Small callees expanded into their callers on the Cranelift tier
//! (decision AU50): the expanded code prints what the interpreter prints,
//! on the paths that leave an expanded callee by every door (a return, a
//! nothing, a failure caught by the caller, one caught by the callee's own
//! handler, a crash with the callee's line), and on the hand-backs to the interpreter from inside one (an
//! overflow in the callee's body, a big Integer stored into a callee's
//! parameter before its body began), nested one level.

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

/// What the JIT reported about a run: callees expanded, hand-backs taken.
struct Report {
    expansions: usize,
    deopts: usize,
}

fn report(stderr: &str) -> Report {
    let number = |line: &str| -> usize {
        line.trim_start_matches("native: ")
            .split(' ')
            .next()
            .and_then(|word| word.parse().ok())
            .unwrap_or(0)
    };
    let mut found = Report {
        expansions: 0,
        deopts: 0,
    };
    for line in stderr.lines() {
        if line.contains("callees expanded into") {
            found.expansions = number(line);
        } else if line.contains(" deopts;") {
            found.deopts = number(line);
        }
    }
    found
}

/// Run `main` on machine code for everything (every code object compiled
/// before its first call, the expansion on and every small callee expanded
/// whatever the caller's size, with the JIT's report on the standard
/// error), or on the interpreter.
fn run(source: &str, interpret: bool) -> (RunOutcome, String, Report) {
    std::env::set_var("RENYI_NATIVE_HOT", "0");
    std::env::set_var("RENYI_NATIVE_REPORT", "1");
    std::env::set_var("RENYI_NATIVE_INLINE", "1");
    std::env::set_var("RENYI_NATIVE_INLINE_BUDGET", "400");
    let program = compile(source);
    let stdout = Capture::default();
    let stderr = Capture::default();
    let outcome = run_program(
        &program,
        Options {
            stdout: Box::new(stdout.clone()),
            stderr: Box::new(stderr.clone()),
            stdin: Box::new(std::io::Cursor::new(Vec::new())),
            interpret,
            ..Options::default()
        },
    )
    .outcome;
    (outcome, stdout.text(), report(&stderr.text()))
}

/// Both ways must agree; the native way's outcome, output and report are
/// given back for closer checks.
fn both_ways(source: &str) -> (RunOutcome, String, Report) {
    let (outcome, printed, found) = run(source, false);
    let (interpreted, printed_interpreted, _) = run(source, true);
    assert_eq!(outcome, interpreted, "machine code against the interpreter");
    assert_eq!(
        printed, printed_interpreted,
        "machine code against the interpreter"
    );
    (outcome, printed, found)
}

#[test]
fn small_callees_are_expanded_and_leave_by_every_door() {
    // a helper returning a value, one returning nothing, one with an
    // `otherwise` of its own, one failing into the caller's `otherwise`,
    // one nested in another, all in loops; the output is the interpreter's
    let source = r#"module demo
  purpose: Small helpers called in loops, expanded into their callers (decision AU50).

import std.console

type Token
  purpose: A token with a position.
  has text: Text
  has position: Integer
end

type Missing
  purpose: Nothing was there.
  has index: Integer
end

function plus_one(value: Integer) returns Integer
  purpose: The value plus one.

  return value + 1
end

function token_at(tokens: List of Token, index: Integer) returns Token
  purpose: The token at the index, or the last one.

  match tokens.at(index)
    when some(token) then return token
    otherwise return tokens.last() otherwise crash with "no tokens"
  end
end

function text_at(tokens: List of Token, index: Integer) returns Text
  purpose: The text of the token at the index, through the other helper.

  return token_at(tokens: tokens, index: index).text
end

function pick(tokens: List of Token, index: Integer) returns Token or fails with Missing
  purpose: The token at the index, or a failure.

  return tokens.at(index) otherwise fail with Missing(index: index)
end

function note(value: Integer) returns maybe Integer
  purpose: Nothing for an odd value, the value for an even one.

  if value remainder 2 is 1 then return nothing end
  return value
end

function number_of(text: Text) returns Integer
  purpose: The text as a number, zero when it is not one: a handled region of the callee's own.

  return text.to_integer() otherwise 0
end

public function main() needs console
  purpose: Print the counts the helpers give.

  let tokens be [Token(text: "a", position: 1), Token(text: "bb", position: 2), Token(text: "ccc", position: 3)]
  let mutable total be 0
  let mutable letters be 0
  let mutable missing be 0
  let mutable evens be 0
  let mutable numbers be 0
  for each turn from 0 to 999
    change total to plus_one(total)
    change letters to letters + text_at(tokens: tokens, index: turn remainder 5).length()
    let found be pick(tokens: tokens, index: turn remainder 4) otherwise Token(text: "", position: 0)
    if found.position is 0 then change missing to missing + 1 end
    match note(turn)
      when some(value) then change evens to evens + value
      when nothing then change evens to evens + 0
    end
    change numbers to numbers + number_of(text_at(tokens: tokens, index: turn remainder 5)) + number_of("{turn}")
  end
  console.print("{total} {letters} {missing} {evens} {numbers}")
end
"#;
    let (outcome, printed, found) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    // 1000 turns; the texts a, bb, ccc, ccc, ccc by turn modulo 5 (200
    // each: 200 + 400 + 600 * 3 = 2400); every fourth turn misses; the
    // even turns sum to 249500; no token's text is a number, every turn
    // is one (0 + 1 + ... + 999 = 499500)
    assert_eq!(printed, "1000 2400 250 249500 499500\n");
    assert!(
        found.expansions >= 4,
        "{} callees expanded",
        found.expansions
    );
    assert_eq!(found.deopts, 0);
}

#[test]
fn a_crash_inside_an_expanded_callee_names_the_callee_s_line() {
    let source = r#"module demo
  purpose: A remainder by zero inside a small helper crashes where it happens.

import std.console

function share(value: Integer, parts: Integer) returns Integer
  purpose: The value divided into parts, crashing on zero parts.

  return value remainder parts
end

public function main() needs console
  purpose: Print one share, then crash on the next.

  let mutable total be 0
  for each turn from 1 to 50
    change total to total + share(value: turn, parts: 7)
  end
  console.print("{total}")
  console.print("{share(value: 3, parts: 0)}")
end
"#;
    let (outcome, printed, found) = both_ways(source);
    assert_eq!(printed, "148\n");
    match outcome {
        RunOutcome::Crashed { message, location } => {
            assert_eq!(message, "division by zero");
            assert_eq!(location.as_deref(), Some("demo.ry:9"));
        }
        other => panic!("expected a crash, got {other:?}"),
    }
    assert!(
        found.expansions >= 1,
        "{} callees expanded",
        found.expansions
    );
}

#[test]
fn a_hand_back_inside_an_expanded_callee_rebuilds_the_frames() {
    // an overflow in the callee's body hands the frames back: the caller's
    // state and the callee's are rebuilt and the big Integer comes out
    // right, inside a loop with a handled region open in the caller; a
    // big Integer stored into an Integer parameter hands back before the
    // body began, and the call is made by the interpreter
    let source = r#"module demo
  purpose: Hand-backs from inside expanded callees: an overflow in the body, a big argument at the door.

import std.console

function doubled(value: Integer) returns Integer
  purpose: The value doubled.

  return value * 2
end

function nudge(value: Integer, offset: Integer) returns Integer
  purpose: The value moved by an offset, through the other helper.

  return doubled(value) + offset
end

function grow(seed: Integer, steps: Integer) returns Integer
  purpose: The seed doubled this many times, with a nudge each turn.

  let mutable value be seed
  let mutable turns be 0
  for each step from 1 to steps
    change value to nudge(value: value, offset: step)
    change turns to turns + value.to_text().length()
  end
  return value + turns
end

public function main() needs console
  purpose: Print a small result, a result past the machine word, and a result from a big argument.

  console.print("{grow(seed: 3, steps: 10)}")
  console.print("{grow(seed: 3, steps: 70)}")
  let big be 9223372036854775807 + 10
  let mutable total be 0
  for each turn from 1 to 20
    change total to total + doubled(big) - doubled(turn)
  end
  console.print("{total}")
end
"#;
    let (outcome, printed, found) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    let lines: Vec<&str> = printed.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "5136");
    assert!(lines[1].len() > 20, "a big result: {}", lines[1]);
    assert_eq!(lines[2], "368934881474191032260");
    assert!(
        found.expansions >= 2,
        "{} callees expanded",
        found.expansions
    );
    assert!(found.deopts >= 2, "{} hand-backs", found.deopts);
}
