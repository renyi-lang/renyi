//! The embedding API (decisions Q3 and AP1): a module loaded with a
//! grant answers calls to its public functions and nothing else; a call
//! that needs more than the grant is refused before it runs; budgets
//! count across the calls of one sandbox; a guarded value cannot leave
//! through another sink; a memory budget stops a call that grows past
//! it; and `renyi run --sandbox` on the command line.

use std::path::PathBuf;
use std::process::Command;

use renyi::{CallError, Grant, Sandbox, Value};

/// The counting allocator the memory budget needs (decision AP1).
#[global_allocator]
static ALLOCATOR: renyi::Allocator = renyi::Allocator;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf()
}

/// A scratch directory under the workspace's `target`, as a path with
/// forward slashes.
fn scratch(name: &str) -> String {
    let directory = root().join("target/sandbox").join(name);
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("a clean scratch directory");
    }
    std::fs::create_dir_all(&directory).expect("the scratch directory");
    directory.to_str().expect("a path").replace('\\', "/")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

const CALC: &str = "module calc
  purpose: A module for the test of the sandbox.

import std.console
import std.filesystem exposing FileError, Path

type Oops
  purpose: A failure for the test.
  has value: Integer
end

public function add(left: Integer, right: Integer) returns Integer
  purpose: Add two numbers.

  return left + right
end

function hidden() returns Integer
  purpose: Not for the host.

  return 42
end

public function secret_number() returns Integer
  purpose: The number the hidden function knows.

  return hidden()
end

public function shout(text: Text) returns Text needs console
  purpose: Print the text and return it in capitals.

  console.print(text)
  return text.to_upper()
end

public function risky(number: Integer) returns Integer or fails with Oops
  purpose: Fail below zero.

  if number is at least 0 then return number end
  fail with Oops(value: number)
end

public function boom() returns Integer
  purpose: Crash on purpose.

  crash with \"boom\"
end

public function grow(size: Integer) returns Integer
  purpose: Build a list of size items and return its length.

  let items be
    for each index from 1 to size
    collect index
  return items.length()
end

public function peek(path: Text) returns Text or fails with FileError needs filesystem.read
  purpose: Read the file at the path.

  let target be Path(path)
  return filesystem.read_text(target) otherwise fail
end

public function stash(secret: Text, path: Text) or fails with FileError needs filesystem.write
  purpose: Write the secret to the path.

  let target be Path(path)
  filesystem.write_text(path: target, content: secret) otherwise fail
end
";

fn sandbox(grant: &str) -> Sandbox {
    Sandbox::load_source("calc.ry", CALC, Grant::parse(grant).expect("a grant"))
        .expect("the module loads")
}

#[test]
fn a_module_answers_its_public_functions_under_the_grant() {
    let mut calc = sandbox("console");
    assert_eq!(calc.module(), "calc");
    assert_eq!(calc.warnings(), "");
    let names: Vec<&str> = calc
        .functions()
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "add",
            "secret_number",
            "shout",
            "risky",
            "boom",
            "grow",
            "peek",
            "stash"
        ]
    );
    let shout = calc
        .functions()
        .iter()
        .find(|function| function.name == "shout")
        .expect("shout");
    assert_eq!(shout.parameters, 1);
    assert_eq!(shout.needs, ["console"]);
    assert_eq!(
        shout.purpose.as_deref(),
        Some("Print the text and return it in capitals.")
    );
    assert!(
        shout.signature.contains("needs console"),
        "{}",
        shout.signature
    );
    assert!(!shout.exposed_as_tool);

    assert_eq!(
        calc.call("add", vec![Value::integer(2), Value::integer(3)]),
        Ok(Value::integer(5))
    );
    assert_eq!(calc.call("secret_number", vec![]), Ok(Value::integer(42)));
    assert_eq!(
        calc.call("hidden", vec![]),
        Err(CallError::Refused(
            "`calc` has no public function `hidden`".to_string()
        ))
    );
    assert_eq!(
        calc.call("add", vec![Value::integer(1)]),
        Err(CallError::Refused(
            "`add` takes 2 arguments, not 1".to_string()
        ))
    );
    assert_eq!(
        calc.call("shout", vec![Value::text("hi")]),
        Ok(Value::text("HI"))
    );
    assert_eq!(
        calc.call("risky", vec![Value::integer(-1)]),
        Err(CallError::Failed("Oops(value: -1)".to_string()))
    );
    assert_eq!(
        calc.call("risky", vec![Value::integer(7)]),
        Ok(Value::integer(7))
    );
    match calc.call("boom", vec![]) {
        Err(CallError::Crashed { message, location }) => {
            assert_eq!(message, "boom");
            assert!(
                location
                    .as_deref()
                    .is_some_and(|at| at.starts_with("calc.ry:")),
                "{location:?}"
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_call_outside_the_grant_is_refused_before_it_runs() {
    let mut calc = sandbox("");
    assert_eq!(
        calc.call("shout", vec![Value::text("hi")]),
        Err(CallError::Refused(
            "`shout` needs `console`, which the grant does not cover".to_string()
        ))
    );
    assert_eq!(
        calc.call("add", vec![Value::integer(1), Value::integer(1)]),
        Ok(Value::integer(2))
    );
    // the directory given, not another one
    let dir = scratch("elsewhere");
    let mut calc = sandbox(&format!("filesystem.read(\"{dir}/inside\")"));
    std::fs::create_dir_all(format!("{dir}/inside")).expect("the inside directory");
    std::fs::write(format!("{dir}/inside/a.txt"), "in").expect("a.txt");
    std::fs::write(format!("{dir}/b.txt"), "out").expect("b.txt");
    assert_eq!(
        calc.call("peek", vec![Value::text(format!("{dir}/inside/a.txt"))]),
        Ok(Value::text("in"))
    );
    match calc.call("peek", vec![Value::text(format!("{dir}/b.txt"))]) {
        Err(CallError::Failed(error)) => {
            assert!(error.starts_with("PermissionDenied("), "{error}")
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn budgets_count_across_the_calls_of_one_sandbox() {
    let dir = scratch("budget");
    let path = format!("{dir}/note.txt");
    std::fs::write(&path, "hello").expect("note.txt");
    let mut calc = sandbox(&format!("filesystem.read(\"{dir}\") at most 2 per run"));
    assert_eq!(
        calc.call("peek", vec![Value::text(&path)]),
        Ok(Value::text("hello"))
    );
    assert_eq!(
        calc.call("peek", vec![Value::text(&path)]),
        Ok(Value::text("hello"))
    );
    match calc.call("peek", vec![Value::text(&path)]) {
        Err(CallError::Failed(error)) => assert!(error.starts_with("OverBudget("), "{error}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_guarded_value_leaves_only_through_the_sinks_of_its_guard() {
    let dir = scratch("guard");
    let mut calc = sandbox(&format!(
        "console, filesystem.read(\"{dir}\") only to console, filesystem.write(\"{dir}\")"
    ));
    let secret = calc
        .guarded(Value::text("key"), &format!("filesystem.read(\"{dir}\")"))
        .expect("a guarded value");
    assert_ne!(secret.origins(), 0);
    let shouted = calc
        .call("shout", vec![secret.clone()])
        .expect("console is a sink");
    assert_eq!(shouted.plain(), &Value::text("KEY"));
    let leak = format!("{dir}/leak.txt");
    match calc.call("stash", vec![secret, Value::text(&leak)]) {
        Err(CallError::Failed(error)) => assert!(error.starts_with("Guarded(origin:"), "{error}"),
        other => panic!("{other:?}"),
    }
    assert!(!std::path::Path::new(&leak).exists());
    assert_eq!(
        calc.call("stash", vec![Value::text("plain"), Value::text(&leak)]),
        Ok(Value::Nothing)
    );
    assert_eq!(std::fs::read_to_string(&leak).expect("leak.txt"), "plain");
    assert!(calc
        .guarded(Value::text("x"), "console")
        .unwrap_err()
        .contains("no `only to` guard on `console`"));
}

#[test]
fn a_memory_budget_stops_a_call_that_grows_past_it() {
    let mut grant = Grant::parse("").expect("an empty grant");
    grant.memory = Some(8 << 20);
    let mut calc = Sandbox::load_source("calc.ry", CALC, grant).expect("the module loads");
    assert_eq!(
        calc.call("grow", vec![Value::integer(1000)]),
        Ok(Value::integer(1000))
    );
    match calc.call("grow", vec![Value::integer(4_000_000)]) {
        Err(CallError::OverMemory { limit, used }) => {
            assert_eq!(limit, 8 << 20);
            assert!(used > limit, "{used}");
        }
        other => panic!("{other:?}"),
    }
    // the next call runs again, the budget counted afresh
    assert_eq!(
        calc.call("grow", vec![Value::integer(10)]),
        Ok(Value::integer(10))
    );
}

#[test]
fn run_takes_a_sandbox_grant_from_a_file() {
    let dir = scratch("cli");
    let wide = format!("{dir}/wide.json");
    let narrow = format!("{dir}/narrow.json");
    let broken = format!("{dir}/broken.json");
    std::fs::write(
        &wide,
        "{\"grant\": \"console, environment\", \"memory\": \"64 megabytes\"}\n",
    )
    .expect("wide.json");
    std::fs::write(&narrow, "{\"grant\": \"console\"}\n").expect("narrow.json");
    std::fs::write(&broken, "{\"grant\": \"teleport\"}\n").expect("broken.json");
    let run = |grant: &str| {
        Command::new(env!("CARGO_BIN_EXE_renyi"))
            .current_dir(root())
            .args(["run", "--sandbox", grant, "examples/hello.ry", "Renyi"])
            .output()
            .expect("the renyi binary runs")
    };
    let ok = run(&wide);
    assert!(ok.status.success(), "{}", text(&ok.stderr));
    assert_eq!(text(&ok.stdout), "Hello, Renyi!\n");
    let refused = run(&narrow);
    assert_eq!(refused.status.code(), Some(1));
    assert_eq!(
        text(&refused.stderr),
        "renyi: the sandbox does not grant `environment`, which `main` needs\n"
    );
    let bad = run(&broken);
    assert_eq!(bad.status.code(), Some(1));
    assert!(
        text(&bad.stderr).contains("`teleport`, which is not a capability"),
        "{}",
        text(&bad.stderr)
    );
    let test = Command::new(env!("CARGO_BIN_EXE_renyi"))
        .current_dir(root())
        .args(["test", "--sandbox", &wide, "examples/hello.ry"])
        .output()
        .expect("the renyi binary runs");
    assert_eq!(test.status.code(), Some(1));
    assert!(
        text(&test.stderr).contains("takes only"),
        "{}",
        text(&test.stderr)
    );
}

#[test]
fn frames_left_by_generated_code_release_what_they_hold_under_a_memory_budget() {
    // decision AW4: a frame's leave in generated code releases the slots
    // that may hold a counted block and skips the rest; a slot skipped
    // wrongly would leak its block once per call, which two hundred
    // thousand calls of a frame with many locals turn into tens of
    // megabytes, past the budget of sixteen; on the Cranelift tier, with
    // every code object compiled before its first call, and on the
    // template tier
    let dir = scratch("leaves");
    let program = format!("{dir}/leaves.ry");
    let grant = format!("{dir}/grant.json");
    std::fs::write(&program, LEAVES).expect("leaves.ry");
    std::fs::write(
        &grant,
        "{\"grant\": \"console\", \"memory\": \"16 megabytes\"}\n",
    )
    .expect("grant.json");
    for tier in [None, Some("template")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_renyi"));
        command
            .current_dir(root())
            .args(["run", "--sandbox", &grant, &program])
            .env("RENYI_NO_CACHE", "1")
            .env("RENYI_NATIVE_HOT", "0")
            .env("RENYI_NATIVE_SYNC", "1");
        if let Some(tier) = tier {
            command.env("RENYI_NATIVE_TIER", tier);
        }
        let run = command.output().expect("the renyi binary runs");
        assert!(run.status.success(), "{tier:?}: {}", text(&run.stderr));
        assert_eq!(text(&run.stdout), "11488897\n", "{tier:?}");
    }
}

/// The program of the leave's test (decision AW4): a frame with many
/// locals, most moved out before one of its three returns.
const LEAVES: &str = r#"module leaves
  purpose: Frames with many locals, most moved out before the return (AW4), left by every path.

import std.console

type Box
  purpose: A record with a label, items and a weight.
  has label: Text
  has items: List of Text
  has weight: Integer
end

type Problem
  purpose: A failure for the test.
  has reason: Text
end

function risky(value: Integer) returns Integer or fails with Problem
  purpose: Fail on multiples of five.

  if value remainder 5 is 0 then fail with Problem(reason: "five") end
  return value * 3
end

public function build(index: Integer) returns Text
  purpose: Counted values in a frame with many locals, returned by one of three paths.

  let label be "a label longer than sixteen bytes {index}"
  let items be [label, "second item of the list {index}", label]
  let box be Box(label: label, items: items, weight: index)
  let moved be box with weight: index * 2
  let head be items.at(0) otherwise "none"
  let kept be "{head} kept"
  let tally be risky(index) otherwise 0
  if index remainder 4 is 0 then
    return "{moved.label} early {tally}"
  end
  let mutable joined be ""
  for each item in moved.items
    change joined to "{joined}{item.length()},"
  end
  if index remainder 4 is 1 then
    return "{kept} {joined}"
  end
  let last be items.at(2) otherwise "none"
  return "{last} {joined} {tally} {moved.weight}"
end

public function main() needs console
  purpose: Call build many times and print a digest.

  let mutable total be 0
  for each index from 1 to 200000
    change total to total + build(index).length()
  end
  console.print("{total}")
end
"#;

#[test]
fn borrowed_parameters_leave_nothing_behind_under_a_memory_budget() {
    // decision AW5: a parameter passed borrowed is neither released by
    // the callee's leave nor leaked by its other ways out (a failure
    // caught by the caller, one propagated); forty thousand turns of
    // every path under a budget of sixteen megabytes, through the binary,
    // on the Cranelift tier with every code object compiled before its
    // first call, and on the template tier; and a crash inside a frame
    // holding a borrowed parameter comes out as the crash
    let dir = scratch("borrows");
    let program = format!("{dir}/borrows.ry");
    let crashes = format!("{dir}/crashes.ry");
    let grant = format!("{dir}/grant.json");
    std::fs::write(&program, BORROWS).expect("borrows.ry");
    std::fs::write(&crashes, CRASHES).expect("crashes.ry");
    std::fs::write(
        &grant,
        "{\"grant\": \"console\", \"memory\": \"16 megabytes\"}\n",
    )
    .expect("grant.json");
    for tier in [None, Some("template")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_renyi"));
        command
            .current_dir(root())
            .args(["run", "--sandbox", &grant, &program])
            .env("RENYI_NO_CACHE", "1")
            .env("RENYI_NATIVE_HOT", "0")
            .env("RENYI_NATIVE_SYNC", "1");
        if let Some(tier) = tier {
            command.env("RENYI_NATIVE_TIER", tier);
        }
        let run = command.output().expect("the renyi binary runs");
        assert!(run.status.success(), "{tier:?}: {}", text(&run.stderr));
        assert_eq!(text(&run.stdout), "4502367506\n", "{tier:?}");
        let run = Command::new(env!("CARGO_BIN_EXE_renyi"))
            .current_dir(root())
            .args(["run", "--sandbox", &grant, &crashes])
            .env("RENYI_NO_CACHE", "1")
            .env("RENYI_NATIVE_HOT", "0")
            .env("RENYI_NATIVE_SYNC", "1")
            .output()
            .expect("the renyi binary runs");
        assert_eq!(
            run.status.code(),
            Some(2),
            "{tier:?}: {}",
            text(&run.stderr)
        );
        assert!(
            text(&run.stderr).contains("boom a label longer than sixteen bytes 300"),
            "{tier:?}: {}",
            text(&run.stderr)
        );
    }
}

/// The program of the borrowed parameters' test (decision AW5): every
/// way out of a frame holding a parameter passed borrowed.
const BORROWS: &str = r#"module borrows
  purpose: Read-only parameters passed borrowed between compiled bodies (AW5), on every path.

import std.console

type Box
  purpose: A record with a label and a weight.
  has label: Text
  has weight: Integer
end

type Problem
  purpose: A failure for the test.
  has reason: Text
end

function label_length(box: Box) returns Integer
  purpose: A field of the record on its last read, which takes it: not read-only.

  return box.label.length()
end

function describe(box: Box, suffix: Text) returns Text
  purpose: A borrowed record read into a text beside a text moved into it.

  return "{box.label}{suffix}{box.weight}"
end

function same(text: Text) returns Text
  purpose: The parameter returned: moved on its last read, so passed owned.

  return text
end

function through(text: Text) returns Integer
  purpose: The parameter passed on, then into a record on its last read: passed owned.

  return same(text).length() + label_length(Box(label: text, weight: 1))
end

function heavier(box: Box) returns Box
  purpose: An updated copy of the record, which is not read-only; the caller's is unchanged.

  return box with weight: box.weight + 1
end

function risky(text: Text, value: Integer) returns Integer or fails with Problem
  purpose: Fail on multiples of five in a frame holding the borrowed text.

  if value remainder 5 is 0 then fail with Problem(reason: "a multiple of five") end
  return text.length() + value
end

function relayed(text: Text, value: Integer) returns Integer or fails with Problem
  purpose: A callee's failure propagated through a frame holding a borrowed parameter.

  let inner be risky(text: text, value: value) otherwise fail
  return inner + text.length()
end

function doubled(text: Text, times: Integer) returns Integer
  purpose: Doubled past the machine word: the frame is handed back after reading a text passed by its last use.

  let mutable result be text.length()
  let mutable remaining be times
  repeat until remaining is 0
    change result to result * 2
    change remaining to remaining - 1
  end
  return result
end

function digest(text: Text) returns Integer
  purpose: The doubling past the machine word, brought back small.

  return doubled(text: text, times: 60) remainder 1000
end

function tally(items: List of Text, box: Box) returns Integer
  purpose: A borrowed record and a list passed by its last use, read many times in a loop.

  let mutable total be 0
  for each item in items
    change total to total + item.length() + box.weight
  end
  return total + items.length()
end

public function main() needs console
  purpose: Every function called many times, four of the calls with a borrowed argument; a digest printed.

  let mutable total be 0
  for each index from 1 to 40000
    let text be "a label longer than sixteen bytes {index}"
    let box be Box(label: text, weight: index)
    let items be [text, "second {index}", text]
    change total to total + label_length(box)
    change total to total + describe(box: box, suffix: " and ").length()
    let temporary be Box(label: "temporary {index}", weight: 2)
    change total to total + describe(box: heavier(box), suffix: text).length()
    change total to total + describe(box: temporary, suffix: same(text)).length()
    change total to total + through(text)
    let safe be risky(text: text, value: index) otherwise 0
    let relay be relayed(text: text, value: index) otherwise 1
    change total to total + safe + relay
    change total to total + tally(items: items, box: box)
    change total to total + digest(text)
    change total to total + box.weight
  end
  console.print("{total}")
end
"#;

/// A crash inside a frame holding a borrowed parameter (decision AW5).
const CRASHES: &str = r#"module crashes
  purpose: A crash inside a frame holding a borrowed parameter (AW5).

import std.console

function shout(label: Text, text: Text, index: Integer) returns Integer
  purpose: Crash on the last index in a frame holding the borrowed label.

  if index is 300 then crash with "boom {text}" end
  return label.length() + text.length() + index
end

public function main() needs console
  purpose: Call shout until it crashes.

  let mutable total be 0
  for each index from 1 to 300
    let text be "a label longer than sixteen bytes {index}"
    change total to total + shout(label: text, text: text, index: index)
  end
  console.print("{total}")
end
"#;

#[test]
fn the_grant_travels_with_a_direct_call_under_a_memory_budget() {
    // decision AW6: the frame's grant travels in the body's signature and
    // a direct call writes the callee's record from it (the helper only
    // for a callee that narrows it); the stack's length is written by the
    // callee's prologue, where the room is checked, and the frames' room
    // for the records this body's direct calls push is checked there
    // once; every shape (a narrowing callee from `main` and from another
    // narrowing one, a callee that inherits the grant and calls on,
    // recursion past the frames' capacity and the machine stack's depth
    // limit, typed parameters before a boxed one at the deepest operand
    // of the caller), through the binary under a budget of sixteen megabytes,
    // the frames and the stack growing under the counting allocator, on
    // both tiers
    let dir = scratch("grants");
    let program = format!("{dir}/grants.ry");
    let grant = format!("{dir}/grant.json");
    std::fs::write(&program, GRANTS).expect("grants.ry");
    std::fs::write(
        &grant,
        "{\"grant\": \"console\", \"memory\": \"16 megabytes\"}\n",
    )
    .expect("grant.json");
    for tier in [None, Some("template")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_renyi"));
        command
            .current_dir(root())
            .args(["run", "--sandbox", &grant, &program])
            .env("RENYI_NO_CACHE", "1")
            .env("RENYI_NATIVE_HOT", "0")
            .env("RENYI_NATIVE_SYNC", "1");
        if let Some(tier) = tier {
            command.env("RENYI_NATIVE_TIER", tier);
        }
        let run = command.output().expect("the renyi binary runs");
        assert!(run.status.success(), "{tier:?}: {}", text(&run.stderr));
        assert_eq!(
            text(&run.stdout),
            "say 1000\nsay 1000\nsay 2000\nsay 2000\nsay 3000\nsay 3000\n14582220\n",
            "{tier:?}"
        );
    }
}

/// The program of the grant's test (decision AW6): every shape of direct
/// call.
const GRANTS: &str = r#"module grants
  purpose: The frame record of a direct call (AW6), every shape.

import std.console

type Box
  purpose: A record with a label and a weight.
  has label: Text
  has weight: Integer
end

function say(text: Text, index: Integer) returns Integer needs console
  purpose: A callee that narrows the grant: it prints on every thousandth call and reads the text.

  if index remainder 1000 is 0 then console.print("say {index}") end
  return text.length() + index
end

function relay(text: Text, index: Integer) returns Integer needs console
  purpose: A narrowing callee that calls another narrowing callee.

  return say(text: text, index: index) + 1
end

function plain(text: Text, index: Integer) returns Integer
  purpose: A callee that inherits the caller's grant and calls another that does.

  return shifted(left: index, right: 2, box: Box(label: text, weight: 3)) + 2
end

function deep(remaining: Integer, text: Text) returns Integer
  purpose: Recursion past the frames' capacity and past the machine stack's depth limit.

  if remaining is 0 then return text.length() end
  return deep(remaining: remaining - 1, text: text) + 1
end

function shifted(left: Integer, right: Integer, box: Box) returns Integer
  purpose: Typed parameters before a boxed one: the argument moves to the third slot.

  return left + right + box.weight
end

public function main() needs console
  purpose: Every shape of direct call many times; a digest printed.

  let mutable total be 0
  for each index from 1 to 3000
    let text be "a label longer than sixteen bytes {index}"
    let box be Box(label: text, weight: index)
    change total to total + say(text: text, index: index)
    change total to total + relay(text: text, index: index)
    change total to total + plain(text: text, index: index)
    change total to total + deep(remaining: index remainder 400, text: text)
    change total to total + "{text} {shifted(left: 1, right: index, box: box)}".length()
    let fresh be Box(label: text, weight: 3)
    change total to total + [index, shifted(left: index, right: 2, box: fresh), box.weight].length()
  end
  console.print("{total}")
end
"#;

/// A module that draws, for the test of the sandbox's screen (decision
/// AW31).
const FRAMES: &str = r#"module frames
  purpose: A module that draws, for the test of the sandbox's screen.

import std.screen exposing Colour, Shape

public function draw(turns: Integer) returns Integer needs screen
  purpose: Present a frame a turn until the turns are up or the host asks to stop; the turns drawn.

  let mutable turn be 0
  repeat until turn is at least turns
    let input be screen.present([
      Column(screen_x: turn, top: 0, bottom: 9, colour: Colour(red: 0, green: 0, blue: 255))
    ])
    if input.quit then break end
    change turn to turn + 1
  end
  return turn
end
"#;

#[test]
fn a_sandbox_hands_the_frames_to_its_screen_which_answers_at_once() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use renyi_vm::screen::{Answer, Input, Screen, Shape};

    /// Counts the frames; asks to stop at the third, or answers later.
    struct Host {
        frames: usize,
        later: bool,
    }

    impl Screen for Host {
        fn present(&mut self, frame: Vec<Shape>) -> Answer {
            self.frames += 1;
            assert_eq!(frame.len(), 1);
            if self.later {
                Answer::Later
            } else {
                Answer::Now(Input {
                    quit: self.frames == 3,
                    ..Input::default()
                })
            }
        }

        fn suspends(&self) -> bool {
            self.later
        }
    }

    let mut frames = Sandbox::load_source(
        "frames.ry",
        FRAMES,
        Grant::parse("screen").expect("a grant"),
    )
    .expect("the module loads");
    // without a host, the first frame crashes the call
    let error = frames.call("draw", vec![Value::integer(5)]).unwrap_err();
    assert!(
        matches!(&error, CallError::Crashed { message, .. } if message.starts_with("the screen is not available here")),
        "{error}"
    );
    let host = Rc::new(RefCell::new(Host {
        frames: 0,
        later: false,
    }));
    frames.set_screen(host.clone());
    assert_eq!(
        frames.call("draw", vec![Value::integer(5)]),
        Ok(Value::integer(2))
    );
    assert_eq!(host.borrow().frames, 3);
    // a sandbox call runs in one piece: a host that answers later ends it
    host.borrow_mut().later = true;
    let error = frames.call("draw", vec![Value::integer(5)]).unwrap_err();
    assert!(
        matches!(&error, CallError::Crashed { message, .. } if message.starts_with("suspended at `screen.present`")),
        "{error}"
    );
    assert_eq!(host.borrow().frames, 4);
}
