//! The references the generated code takes and gives back, against the
//! allocator (decision AW25): a program run twice on a tier holds no more
//! bytes after its second run than after its first, so that a value
//! pushed without a reference of its own and released as owned, or one
//! pushed owned and released nowhere, shows up as bytes never freed (or
//! as a double free). Each turn of a program allocates a fresh text of
//! 100 KB, so that one reference lost per turn costs 2 MB over the
//! twenty turns of a run. One run at a time: the tier is chosen through
//! the environment and the allocator counts for the whole process.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;

use renyi_syntax::SourceFile;
use renyi_vm::{compile_project, run_program, Options, Program, RunOutcome};

/// The bytes allocated and not freed since the process started.
static LIVE: AtomicI64 = AtomicI64::new(0);

/// The process's allocator, counting.
struct Counting;

// SAFETY: every method defers to `System` and only counts the sizes.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = System.alloc(layout);
        if !pointer.is_null() {
            LIVE.fetch_add(layout.size() as i64, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = System.alloc_zeroed(layout);
        if !pointer.is_null() {
            LIVE.fetch_add(layout.size() as i64, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        System.dealloc(pointer, layout);
        LIVE.fetch_sub(layout.size() as i64, Ordering::Relaxed);
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let moved = System.realloc(pointer, layout, new_size);
        if !moved.is_null() {
            LIVE.fetch_add(new_size as i64 - layout.size() as i64, Ordering::Relaxed);
        }
        moved
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// The bytes a run may leave behind: what the second run of a program
/// allocates for good (a lazily built table, a cache filled on a path the
/// first run did not take), well under the 2 MB one lost reference per
/// turn costs.
const SLACK: i64 = 64 * 1024;

fn compile(source: &str) -> Program {
    let file = SourceFile::new("probe.ry", source);
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

#[derive(Clone, Copy, Debug)]
enum Tier {
    /// Every code object compiled by the Cranelift tier before its first
    /// call, on this thread.
    Cranelift,
    /// Everything on the template tier, promoted nowhere.
    Template,
    Interpreter,
}

const TIERS: [Tier; 3] = [Tier::Cranelift, Tier::Template, Tier::Interpreter];

/// The environment the VM reads its tier from when it is made.
fn select(tier: Tier) {
    std::env::set_var("RENYI_NATIVE_HOT", "0");
    std::env::set_var("RENYI_NATIVE_SYNC", "1");
    match tier {
        Tier::Template => std::env::set_var("RENYI_NATIVE_TIER", "template"),
        _ => std::env::remove_var("RENYI_NATIVE_TIER"),
    }
}

/// Run `main` with the arguments on the tier.
fn run(program: &Program, tier: Tier, arguments: &[&str]) -> (RunOutcome, String) {
    let stdout = Capture::default();
    let outcome = run_program(
        program,
        Options {
            arguments: arguments.iter().map(|text| text.to_string()).collect(),
            stdout: Box::new(stdout.clone()),
            stderr: Box::new(Capture::default()),
            stdin: Box::new(std::io::Cursor::new(Vec::new())),
            interpret: matches!(tier, Tier::Interpreter),
            ..Options::default()
        },
    )
    .outcome;
    (outcome, stdout.text())
}

/// The program `which` run twice on the tier, twenty turns each: both
/// print `expected` (or crash so when `expected` is `None`), and the
/// second run leaves at most `SLACK` bytes behind.
fn holds_no_more_after(program: &Program, tier: Tier, which: &str, expected: Option<&str>) {
    let _one = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    select(tier);
    let arguments = ["20", which];
    let check = |outcome: RunOutcome, printed: String| match expected {
        Some(text) => {
            assert_eq!(outcome, RunOutcome::Finished, "{which} on {tier:?}");
            assert_eq!(printed, text, "{which} on {tier:?}");
        }
        None => {
            assert!(
                matches!(&outcome, RunOutcome::Crashed { message, .. } if message == "the constant cannot be built"),
                "{which} on {tier:?}: {outcome:?}"
            );
            assert_eq!(printed, "", "{which} on {tier:?}");
        }
    };
    let (outcome, printed) = run(program, tier, &arguments);
    check(outcome, printed);
    let before = LIVE.load(Ordering::Relaxed);
    let (outcome, printed) = run(program, tier, &arguments);
    let after = LIVE.load(Ordering::Relaxed);
    check(outcome, printed);
    let held = after - before;
    assert!(
        held <= SLACK,
        "{which} on {tier:?}: {held} bytes held after the second run"
    );
}

/// The programs: a loop of twenty turns, a fresh text of 100 KB and more
/// each turn, over the shapes whose references the generated code takes
/// in place.
const PROGRAMS: &str = r#"module probe
  purpose: The programs of the leak test, each a loop over fresh large values.

import std.console
import std.environment

type User
  purpose: A named account.
  has name: Text
  has age: Integer
end

type Admin is User
  purpose: A user passed where a user is, which takes the slow path of a field site.

let word: Text be "a constant text longer than sixteen bytes"
  purpose: A module constant compared and passed.

let unbuilt: Text be absent() otherwise crash with "the constant cannot be built"
  purpose: A constant whose construction crashes, pushed for a comparison that never runs.

function absent() returns maybe Text
  purpose: Nothing, for the constant that cannot be built.

  return nothing
end

function pick(flag: Boolean, left: Text, right: Text) returns Boolean
  purpose: Whether the chosen text is the word.

  return (if flag then left otherwise right end) is "word"
end

function fallback(name: maybe Text, other: Text) returns Boolean
  purpose: Whether the name or its default is the word.

  return (name otherwise other) is "word"
end

function named(user: User, wanted: Text) returns Boolean
  purpose: Whether the user has the wanted name, the user read again after.

  let found be user.name is wanted
  return found and user.age is at least 0
end

function length_of(text: Text) returns Integer
  purpose: The length, the text passed borrowed by a direct call.

  return text.length()
end

function is_word(text: Text) returns Boolean
  purpose: Whether the text is the constant.

  return text is word
end

function against_unbuilt(text: Text) returns Boolean
  purpose: Whether the text is the constant that cannot be built: the comparison never runs.

  return text is unbuilt
end

public function main() needs console, environment
  purpose: Run the chosen program this many turns, a fresh large value each turn.

  let arguments be environment.arguments()
  let turns be (arguments.at(0) otherwise "3").to_integer() otherwise crash with "a count"
  let which be arguments.at(1) otherwise "joined"
  let mutable hits be 0
  let mutable last be ""
  for each turn from 1 to turns
    let big be "x".repeat(100000 + turn)
    if which is "joined" then
      if pick(flag: true, left: big, right: "other") then
        change hits to hits + 1
      end
      if fallback(name: big, other: "word") then
        change hits to hits + 1
      end
    otherwise if which is "fields" then
      let user be User(name: big, age: turn)
      if named(user: user, wanted: big) then
        change hits to hits + 1
      end
      let admin be Admin(User(name: big, age: turn))
      if named(user: admin, wanted: "nobody") then
        change hits to hits + 1
      end
      change hits to hits + length_of(user.name)
    otherwise if which is "constants" then
      if is_word(big) then
        change hits to hits + 1
      end
      change last to big
    otherwise if which is "unbuilt" then
      if against_unbuilt(big) then
        change hits to hits + 1
      end
    end
  end
  console.print("hits {hits} {last.length()}")
end
"#;

#[test]
fn a_value_joined_from_two_branches_is_released_by_the_comparison_that_takes_it() {
    // the value of an `if` expression and of an `otherwise`, compared at
    // once: the walk of `borrowed_operands` once crossed the join, so that
    // the push of the branch before the comparison was marked borrowed
    // and the other branch's, moved out of its slot, was never released
    let program = compile(PROGRAMS);
    for tier in TIERS {
        holds_no_more_after(&program, tier, "joined", Some("hits 0 0\n"));
    }
}

#[test]
fn a_field_pushed_borrowed_is_released_by_nobody_and_its_helper_path_by_its_consumer() {
    // `user.name is wanted` with the user read again after: the field
    // read in place for the comparison, through the cache for a `User`
    // and through the helper for an `Admin` (another type at the same
    // site), and a field passed to a direct call that accepts it borrowed
    let program = compile(PROGRAMS);
    for tier in TIERS {
        holds_no_more_after(&program, tier, "fields", Some("hits 2000230 0\n"));
    }
}

#[test]
fn a_constant_pushed_borrowed_and_a_store_over_a_value_held() {
    // `text is word` with the module constant copied from the VM's table
    // for the comparison, and `change last to big` over the value of the
    // turn before, which the store must release
    let program = compile(PROGRAMS);
    for tier in TIERS {
        holds_no_more_after(&program, tier, "constants", Some("hits 0 100020\n"));
    }
}

#[test]
fn a_constant_that_cannot_be_built_leaves_the_operand_below_it_whole() {
    // `text is unbuilt`: the text pushed borrowed for the comparison, the
    // constant's helper crashing with it on the stack, which whoever
    // releases the frame counts as owned
    let program = compile(PROGRAMS);
    for tier in TIERS {
        holds_no_more_after(&program, tier, "unbuilt", None);
    }
}
