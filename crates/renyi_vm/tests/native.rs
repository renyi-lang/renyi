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
use renyi_vm::bytecode::Op;
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
fn typed_entries_of_the_prelude_answer_or_decline_on_both_tiers() {
    // decision AU1: a call of a primitive with a typed entry borrows its
    // operands and takes a scalar answer in a register; the entry declines
    // to the native where the answer is not a plain scalar (a big
    // Integer), and the frame goes to the interpreter where the native's
    // answer does not fit the register either
    let source = r#"module demo
  purpose: The typed entries of the prelude, hot.

import std.console

function digits_in(text: Text) returns Integer
  purpose: How many of the text's characters are digits.

  let mutable found be 0
  for each glyph in text.characters()
    if "0123456789".contains(glyph) then change found to found + 1 end
  end
  return found
end

function sizes(items: List of Text, lookup: Map of Text to Integer) returns Text
  purpose: Lengths and lookups, hot.

  let mutable total be 0
  let mutable hits be 0
  for each round from 1 to 500
    for each item in items
      change total to total + item.length()
      if round is at least 1 and lookup.contains_key(item) then change hits to hits + 1 end
      if item.starts_with("b") and not items.contains("zzz") then change hits to hits + 1 end
    end
  end
  let head be items.at(0) otherwise "none"
  let missing be items.at(10) otherwise "none"
  let found be lookup.get("bb") otherwise 0 - 1
  let joined be items.join("+")
  let place be items.index_of("ccc") otherwise 0 - 1
  return "{total} {hits} {head} {missing} {found} {joined} {place}"
end

function extremes(value: Integer) returns Text
  purpose: `absolute` past the machine word declines to the native, whose answer the frame takes to the interpreter.

  let mutable low be value
  let mutable steps be 62
  repeat until steps is 0
    change low to low * 2
    change steps to steps - 1
  end
  change low to 0 - low - low
  return "{low.absolute()} {(0 - 7).absolute()} {low.at_least(3)} {4.at_most(low)} {[1, 2, 3].sum()}"
end

public function main() needs console
  purpose: Print the answers of both tiers.

  let sample be "item 1; item 22; item 333; "
  console.print("{digits_in(sample)}")
  console.print(sizes(items: ["a", "bb", "ccc"], lookup: {"bb": 2, "dddd": 4}))
  console.print(extremes(1))
  let two be 2.0.to_float()
  let root be two.square_root()
  console.print("{root * root} {(0.0 - 1.5).to_float().absolute()} {3.14159.to_float().rounded(2)}")
  let word be "héllo"
  let present be word.index_of("l") otherwise 0 - 1
  let absent be word.index_of("z") otherwise 0 - 1
  let padded be "  x "
  let abc be "abc"
  console.print("{present} {absent} {padded.trim()}|{abc.reversed()}")
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        printed,
        "6\n3000 1000 a none 2 a+bb+ccc 2\n9223372036854775808 7 3 -9223372036854775808 6\n2.0000000000000004 1.5 3.14\n2 -1 x|cba\n"
    );
}

#[test]
fn loops_over_lists_texts_sets_and_ranges_agree_on_both_tiers() {
    // decision AU1, stage ii: the generated code walks a list iterator in
    // place; a range held in a variable and the snapshots of a text and a
    // set go through the same slot kind, the first through the helper
    let source = r#"module demo
  purpose: Loops over every kind of source, with the list changed while it is walked.

import std.console

function walk(items: List of Integer, text: Text, limit: Integer) returns Text
  purpose: Sum the items while appending to the same list, count the glyphs, walk a range from a variable and a set.

  let mutable copy be items
  let mutable total be 0
  for each item in items
    change total to total + item
    change copy to copy.append(item * 10)
  end
  let mutable glyphs be 0
  for each glyph in text
    if glyph is not " " then change glyphs to glyphs + 1 end
  end
  let span be from 1 to limit
  let mutable walked be 0
  for each step in span
    change walked to walked + step
  end
  let mutable distinct be 0
  for each member in [3, 1, 3, 2].to_set()
    change distinct to distinct + member
  end
  let mutable nested be ""
  for each outer in ["x", "y"]
    for each inner in [1, 2]
      change nested to "{nested}{outer}{inner}"
    end
  end
  return "{total} {copy.length()} {glyphs} {walked} {distinct} {nested}"
end

public function main() needs console
  purpose: Print the walk.

  console.print(walk(items: [1, 2, 3], text: "a b c", limit: 4))
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "6 6 3 10 6 x1x2y1y2\n");
}

#[test]
fn a_record_held_once_is_updated_in_place_and_its_aliases_keep_their_value() {
    // decision AU11: `change x to x with f: x.f.append(item)` takes the
    // field out of the record in the slot and puts the record back
    // updated, `change x to f(x, ...)` moves `x` into the call; a record
    // another binding holds is copied, an update that may leave the loop
    // is the copy, and both tiers agree with each other on every case
    let source = r#"module demo
  purpose: Records updated in place when held once, with the aliasing cases.

import std.console

type Bag
  purpose: A record with a list and a counter.
  has items: List of Integer
  has total: Integer
  has name: Text
end

function add(bag: Bag, item: Integer) returns Bag
  purpose: The bag with the item appended and counted.

  return bag with items: bag.items.append(item), total: bag.total + 1
end

function add_taken(bag: Bag, item: Integer) returns Bag
  purpose: The bag with the item appended, the field taken out first.

  return bag with items: bag.items.append(item)
end

function shown(items: List of Integer) returns Text
  purpose: The items as a text.

  let mutable out be ""
  for each item in items
    change out to "{out}{item},"
  end
  return out
end

function risky(item: Integer) returns Integer or fails with Text
  purpose: Fails on odd items.

  if item remainder 2 is 1 then fail with "odd" end
  return item * 10
end

public function main() needs console
  purpose: Runs every form and prints what each leaves.

  let mutable bag be Bag(items: [], total: 0, name: "a")
  for each item from 1 to 5
    change bag to bag with items: bag.items.append(item)
  end
  console.print("takes: {shown(bag.items)} {bag.total}")
  for each item from 1 to 5
    change bag to add(bag: bag, item: item)
  end
  console.print("argument form: {shown(bag.items)} {bag.total}")
  for each item from 1 to 3
    change bag to add_taken(bag: bag, item: item)
  end
  console.print("taken in callee: {shown(bag.items)}")
  let kept be bag
  change bag to bag with items: bag.items.append(99), total: bag.total + 1
  console.print("aliased: {kept.items.length()} {kept.total} / {bag.items.length()} {bag.total}")
  change bag to bag with total: bag.total + 1, items: bag.items.append(bag.total)
  console.print("move plan: {bag.items.last() otherwise 0} {bag.total}")
  let mutable log be Bag(items: [], total: 0, name: "log")
  for each item from 1 to 6
    change log to log with items: (log.items.append(risky(item) otherwise continue))
  end
  console.print("escape: {shown(log.items)} {log.total}")
  change log to log with items: log.items.append(1), items: log.items.append(2)
  console.print("duplicate: {shown(log.items)}")
  let other be log
  change log to log with name: "renamed"
  console.print("shared base: {other.name} {log.name}")
  for each item from 1 to 3
    change log to log with items: log.items.append(item * 100), name: "{log.name}!"
  end
  console.print("two takes: {shown(log.items)} {log.name}")
end
"#;
    // the forms are emitted as the decision says: two takes, eight
    // updates in the slot, one copy, two moves into a call; and since
    // decision AU55 the last read of a local moves it too (`other.name`
    // takes the field, seven more loads move)
    let program = compile(source);
    let count = |wanted: fn(&renyi_vm::bytecode::Op) -> bool| {
        program
            .codes
            .iter()
            .flat_map(|code| code.ops.iter())
            .filter(|op| wanted(op))
            .count()
    };
    assert_eq!(count(|op| matches!(op, Op::TakeField { .. })), 3);
    assert_eq!(count(|op| matches!(op, Op::WithSlot { .. })), 8);
    assert_eq!(count(|op| matches!(op, Op::With(_))), 1);
    assert_eq!(count(|op| matches!(op, Op::LoadMove(_))), 9);
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        printed,
        "takes: 1,2,3,4,5, 0\n\
         argument form: 1,2,3,4,5,1,2,3,4,5, 5\n\
         taken in callee: 1,2,3,4,5,1,2,3,4,5,1,2,3,\n\
         aliased: 13 5 / 14 6\n\
         move plan: 6 7\n\
         escape: 20,40,60, 0\n\
         duplicate: 20,40,60,1,\n\
         shared base: log renamed\n\
         two takes: 20,40,60,1,100,200,300, renamed!!!\n"
    );
}

#[test]
fn comparisons_on_borrowed_operands_agree_with_the_general_path() {
    // decision AU13: a comparison on boxed operands is answered where they
    // lie, the operands a `Load`, a `Const` or `Nothing` pushed borrowed;
    // texts, small Integers in `maybe` slots, Booleans, `Nothing` beside a
    // value and fieldless variants take the fast path, records, variants
    // with fields, Decimals and a declared `equals` the general path, and
    // both tiers print the same tallies
    let source = r#"module demo
  purpose: Comparisons of every shape on borrowed operands (decision AU13), against the general path.

import std.console

type Color is one of
  purpose: Fieldless variants.
  Red
  Green
  Blue
end

type Shape is one of
  purpose: Variants with fields.
  Circle(radius: Integer)
  Dot
end

type Word
  purpose: A record with its own equality.
  has text: Text
end

ability Equal for Word
  function equals(self, other: Word) returns Boolean
    return self.text.to_lower() is other.text.to_lower()
  end
end

type Point
  purpose: A record with derived equality.
  has x_value: Integer
  has y_value: Integer
  can Equal
end

function pick(flag: Boolean) returns maybe Integer
  purpose: An Integer in a maybe, or nothing.

  if flag then return 7 end
  return nothing
end

function label(flag: Boolean) returns maybe Text
  purpose: A text in a maybe, or nothing.

  if flag then return "seven" end
  return nothing
end

function tally(words: List of Text, colors: List of Color, limit: Integer) returns Text
  purpose: Every comparison shape in loops, so that the code is compiled.

  let mutable hits be 0
  let mutable misses be 0
  let seven be pick(true)
  let none be pick(false)
  let name be label(true)
  let unnamed be label(false)
  for each word in words
    if word is "seven" then change hits to hits + 1 end
    if word is not "seven" then change misses to misses + 1 end
    if word is less than "m" then change hits to hits + 10 end
    if word is at least "seven" then change hits to hits + 100 end
    if name is word then change hits to hits + 1000 end
    if unnamed is word then change hits to hits + 10000 end
    if word is unnamed then change hits to hits + 10000 end
  end
  for each color in colors
    if color is Red then change hits to hits + 1 end
    if color is not Green then change misses to misses + 1 end
    if Blue is color then change hits to hits + 2 end
  end
  for each step from 1 to limit
    if seven is step then change hits to hits + 1 end
    if none is step then change hits to hits + 1000 end
    if step is none then change hits to hits + 1000 end
    if seven is not none then change hits to hits + 1 end
    if none is nothing then change hits to hits + 3 end
    if seven is nothing then change hits to hits + 5000 end
    if nothing is not seven then change hits to hits + 7 end
  end
  return "{hits} {misses}"
end

function records(limit: Integer) returns Text
  purpose: Comparisons that need the general path: records, variants with fields, decimals, a declared equals.

  let mutable hits be 0
  let shout be Word(text: "HELLO")
  let plain be Word(text: "hello")
  let origin be Point(x_value: 0, y_value: 0)
  let same be Point(x_value: 0, y_value: 0)
  let disc be Circle(radius: 2)
  let other be Circle(radius: 3)
  let price be 19.99
  let cost be 19.990
  for each step from 1 to limit
    if step is 2 then change hits to hits + 1000000 end
    if shout is plain then change hits to hits + 1 end
    if shout is not plain then change hits to hits + 1000 end
    if origin is same then change hits to hits + 10 end
    if disc is other then change hits to hits + 1000 end
    if disc is not Dot then change hits to hits + 100 end
    if price is cost then change hits to hits + 1000 end
    if price is at most cost then change hits to hits + 10000 end
    if origin is not nothing then change hits to hits + 100000 end
  end
  return "{hits}"
end

public function main() needs console
  purpose: Print the tallies.

  console.print(tally(words: ["seven", "eight", "a", "zebra"], colors: [Red, Green, Blue, Red], limit: 9))
  console.print(records(3))
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "1325 6\n1333333\n");
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

#[test]
fn moved_loads_are_borrowed_by_typed_calls_and_comparisons() {
    // decision AU17: the VM moves the last read of a slot before the
    // program runs, and the generated code borrows a moved load for a
    // typed call or a comparison as it borrows a plain one, the slot
    // keeping its value; an alias of the moved value is unaffected, and
    // both tiers print the same counts
    let source = r#"module demo
  purpose: Loop variables read for the last time by a comparison and by a typed call (decision AU17).

import std.console

function digits_in(text: Text, marker: Text) returns Text
  purpose: Count the digits and the markers among the glyphs, and keep the last glyph through an alias.

  let mutable found be 0
  let mutable marked be 0
  let mutable last be ""
  for each glyph in text.characters()
    let alias be glyph
    let twin be glyph
    if twin is marker then change marked to marked + 1 end
    if "0123456789".contains(glyph) then change found to found + 1 end
    change last to alias
  end
  return "{found} {marked} {last}"
end

public function main() needs console
  purpose: Print the counts.

  console.print(digits_in(text: "a1b22c333-", marker: "-"))
end
"#;
    let prepared = renyi_vm::liveness::prepared(&compile(source));
    let code = prepared
        .codes
        .iter()
        .find(|code| code.name.ends_with("digits_in"))
        .expect("the function's code");
    let moved_into_call = code
        .ops
        .windows(2)
        .any(|pair| matches!(pair, [Op::LoadMove(_), Op::Call { .. }]));
    let moved_into_comparison = code.ops.windows(3).any(|three| {
        matches!(
            three,
            [
                Op::LoadMove(_),
                Op::Load(_),
                Op::Binary(renyi_syntax::ast::BinaryOp::Is)
            ]
        )
    });
    assert!(moved_into_call, "the glyph is moved into `contains`");
    assert!(moved_into_comparison, "the twin is moved into `is`");
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "6 1 -\n");
}

#[test]
fn a_hand_back_inside_a_handled_region_keeps_the_region() {
    // decision AU18: the generated code keeps the handled regions static
    // and the interpreter on `Vm::handlers`, so a frame handed back inside
    // one (here at an overflow of the machine word) gets the open regions
    // pushed by the hand-back; the call that fails after it lands on the
    // fallback as on the interpreter, where it crashed as unhandled before
    let source = r#"module demo
  purpose: An overflow hands the frame back inside a handled region, then a failure in it.

import std.console

type Problem is one of
  purpose: The failure of a step.
  Bad
end

function risky(amount: Integer) returns Integer or fails with Problem
  purpose: Fail on a positive amount.

  if amount is greater than 0 then fail with Bad end
  return amount
end

function guarded(big: Integer) returns Integer
  purpose: The product overflows the machine word inside the region, then the call fails.

  let value be risky(big * big) otherwise 7
  return value
end

public function main() needs console
  purpose: Print the fallback.

  console.print("{guarded(4000000000)}")
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "7\n");
}

/// A variant without fields built over and over (decision AU24): the
/// first construction builds it through the helper, the others copy it
/// from its slot in place with one more reference, on the Cranelift tier; the values
/// kept in a list, compared and matched print what the interpreter prints.
#[test]
fn variants_without_fields_are_copied_from_their_slot_and_counted() {
    let source = r#"module demo
  purpose: Variants without fields built over and over, kept, compared and matched.

import std.console

type Light is one of
  purpose: A colour of the light.
  Red
  Green
  Yellow
  can ToText
end

type Shape is one of
  purpose: A shape with or without a side.
  Dot
  Square(side: Integer)
end

function next(light: Light) returns Light
  purpose: The colour after this one.

  match light
    when Red then return Green
    when Green then return Yellow
    when Yellow then return Red
  end
end

function shape_of(index: Integer) returns Shape
  purpose: A dot for every third index, a square otherwise.

  if index remainder 3 is 0 then
    return Dot
  end
  return Square(side: index remainder 5)
end

function area(shape: Shape) returns Integer
  purpose: The area of the shape, nothing for a dot.

  match shape
    when Dot then return 0
    when Square(side) then return side * side
  end
end

public function main() needs console
  purpose: Cycle the lights and keep them, count the reds, sum the areas.

  let mutable light be Red
  let mutable kept: List of Light be []
  let mutable reds be 0
  let mutable steps be 0
  for each index from 1 to 3000
    change steps to index
    change light to next(light)
    change kept to kept.append(light)
    if light is Red then
      change reds to reds + 1
    end
  end
  let mutable total be 0
  let mutable dots be 0
  for each index from 1 to 2000
    let shape be shape_of(index)
    change total to total + area(shape)
    if shape is Dot then
      change dots to dots + 1
    end
  end
  console.print("{steps} {reds} {kept.length()} {light} {kept.first() otherwise Red} {total} {dots}")
end
"#;
    let (outcome, output) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(output, "3000 1000 3000 Red Green 8001 666\n");
}

/// Texts compared on the Cranelift tier (decision AU26): two texts held in the
/// value itself compared in place, a long one or a mix through the
/// general path, an ordering always through it, and the characters of a
/// line against a constant; every count is the interpreter's.
#[test]
fn texts_held_in_the_value_compare_in_place_and_others_as_before() {
    let source = r#"module demo
  purpose: Texts compared, short and long, equal and not, and characters against a constant.

import std.console

function same(left: Text, right: Text) returns Boolean
  purpose: Whether the two texts are the same.

  return left is right
end

function differ(left: Text, right: Text) returns Boolean
  purpose: Whether the two texts differ.

  return left is not right
end

public function main() needs console
  purpose: Count the matches over short and long texts, then the spaces of a line.

  let words be ["a", "ab", "abc", "fifteen bytes!!", "sixteen bytes!!!", "a much longer text than that", "é", "ab"]
  let mutable equal be 0
  let mutable unequal be 0
  let mutable before be 0
  let mutable rounds be 0
  for each round from 1 to 200
    change rounds to rounds + round
    for each left in words
      for each right in words
        if same(left: left, right: right) then
          change equal to equal + 1
        end
        if differ(left: left, right: right) then
          change unequal to unequal + 1
        end
        if left is less than right then
          change before to before + 1
        end
      end
    end
  end
  let mutable spaces be 0
  for each glyph in "a b  c   d".characters()
    if glyph is " " then
      change spaces to spaces + 1
    end
  end
  console.print("{equal} {unequal} {before} {rounds} {spaces}")
end
"#;
    let (outcome, output) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(output, "2000 10800 5400 20100 6\n");
}

/// Records and variants in one counted block each (decision AU27), on
/// the Cranelift tier: built from the top of the stack, kept in a list, read in
/// place, copied before an update while shared, compared and matched;
/// every count is the interpreter's.
#[test]
fn records_in_one_block_are_built_read_copied_and_compared() {
    let source = r#"module demo
  purpose: Records built over and over in one block each, kept, copied, updated and compared.

import std.console

type Point
  purpose: A point with a label.
  has east: Integer
  has north: Integer
  has label: Text
end

type Segment
  purpose: Two points.
  has start: Point
  has finish: Point
end

type Mark is one of
  purpose: A mark on a point or none.
  Blank
  Pin(spot: Point, note: Text)
end

function moved(point: Point, step: Integer) returns Point
  purpose: The point moved east.

  return point with east: point.east + step
end

function mark_of(index: Integer, point: Point) returns Mark
  purpose: A pin for every other index.

  if index remainder 2 is 0 then
    return Pin(spot: point, note: "a note longer than sixteen bytes")
  end
  return Blank
end

public function main() needs console
  purpose: Build segments and marks, keep them, move copies, count what matches.

  let mutable segments: List of Segment be []
  let mutable pins be 0
  let mutable same be 0
  let mutable total be 0
  for each index from 1 to 2000
    let start be Point(east: index, north: index remainder 7, label: "p")
    let finish be moved(point: start, step: 3)
    let segment be Segment(start: start, finish: finish)
    change segments to segments.append(segment)
    match mark_of(index: index, point: finish)
      when Blank then change total to total + 1
      when Pin(spot, note) then
        change pins to pins + spot.north
        change total to total + note.length()
    end
    let shifted be segment.start with east: segment.start.east + 3
    if shifted is segment.finish then
      change same to same + 1
    end
  end
  let mutable east be 0
  for each segment in segments
    change east to east + segment.finish.east - segment.start.east
  end
  console.print("{segments.length()} {pins} {same} {total} {east}")
end
"#;
    let (outcome, output) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(output, "2000 3003 2000 33000 6000\n");
}

/// The retain and the release in place in the code Cranelift makes
/// (decision AU32): records, long texts and lists counted in their blocks
/// and freed when the last reference goes, small values with no count,
/// and big Integers through the cold blocks; every count is the
/// interpreter's. The weight is a `maybe Integer`, boxed, so that its
/// copies are counted in this tier's code, which keeps an `Integer` in a
/// register and hands a big one to the interpreter (with the big
/// Integer's count not raised the run crashes, as it was checked to).
#[test]
fn counts_raised_and_lowered_in_place_free_the_last_and_spare_big_integers() {
    let source = r#"module demo
  purpose: Values counted in place by generated code: big Integers, long texts, records.

import std.console

type Box
  purpose: A box around a long label and a weight that is big when present.
  has label: Text
  has weight: maybe Integer
end

function heavy(seed: Integer) returns Integer
  purpose: An Integer past the machine word.

  return seed * 10000000000 * 10000000000
end

function box_of(index: Integer) returns Box
  purpose: A box whose label is long and whose weight is big.

  return Box(label: "a label longer than sixteen bytes {index}", weight: heavy(index))
end

public function main() needs console
  purpose: Make, keep, copy and drop counted values over and over.

  let mutable kept: List of Box be []
  let mutable weights: List of maybe Integer be []
  let mutable total be 0
  let mutable big: maybe Integer be nothing
  for each index from 1 to 3000
    let item be box_of(index)
    change big to item.weight
    let again be big
    if index remainder 3 is 0 then
      change kept to kept.append(item)
      change weights to weights.append(again)
    end
    change total to total + item.label.length()
  end
  let mutable residues be 0
  for each item in kept
    change residues to residues + ((item.weight otherwise 0) remainder 7)
  end
  let mutable sevens be 0
  for each weight in weights
    change sevens to sevens + ((weight otherwise 0) remainder 7)
  end
  let last be big otherwise 0
  console.print("{kept.length()} {total} {last remainder 1000003} {residues} {sevens}")
end
"#;
    let (outcome, output) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(output, "1000 112893 900027 3003 3003\n");
}

/// `List.at` in place (decision AU34) and the in-place entries of
/// `Text.contains` and `List.contains` (decision AW11, in place of
/// AU34's lean helpers) in the code Cranelift makes: items of every kind read
/// at every position and past both ends, from lists that slots hold and
/// from lists made for the call, released after their item is copied; an
/// index past the machine word taking the general path (in a function of
/// its own, whose frame the interpreter takes at the overflow, so that
/// `main` stays in generated code); answers that the interpreter gives.
#[test]
fn list_items_read_in_place_and_contains_through_lean_helpers() {
    let source = r#"module demo
  purpose: List.at in place and the lean contains, on every kind of item and index.

import std.console

type Point
  purpose: A record to keep in a list.
  has label: Text
  has weight: Integer
end

function fresh(seed: Integer) returns List of Text
  purpose: A list no slot holds, made anew at every call.

  return ["a label past sixteen bytes {seed}", "short {seed}", "x"]
end

function past_the_word(numbers: List of Integer) returns Integer
  purpose: The item at an index past the machine word, which is never there.

  let huge be 10000000000 * 10000000000
  return numbers.at(huge) otherwise 5
end

public function main() needs console
  purpose: Read items in place, past both ends, from lists held and not.

  let numbers be [10, 20, 30, 40]
  let texts be ["one", "a text longer than sixteen bytes", "three"]
  let points be [
    Point(label: "origin", weight: 0),
    Point(label: "a point label past sixteen", weight: 7)
  ]
  let nested be [[1, 2], [3]]
  let empty: List of Integer be []
  let mutable total be 0
  let mutable found be 0
  let mutable kept: List of Text be []
  for each index from 0 to 5
    let position be index - 1
    change total to total + (numbers.at(position) otherwise 0)
    let text be texts.at(position) otherwise "none"
    change kept to kept.append(text)
    let point be points.at(position) otherwise Point(label: "none", weight: 100)
    change total to total + point.weight + point.label.length()
    change total to total + (empty.at(position) otherwise 1000)
    let inner be nested.at(position) otherwise [7]
    change total to total + (inner.at(0) otherwise 0)
    let made be fresh(index).at(position) otherwise "gone"
    change kept to kept.append(made)
    change total to total + past_the_word(numbers)
    if texts.contains("three") then change found to found + 1 end
    if made.contains("label") then change found to found + 10 end
    if numbers.contains(position * 10) then change found to found + 100 end
    if fresh(index).contains(made) then change found to found + 1000 end
  end
  let comma be ","
  console.print("{total} {found} {kept.length()} {kept.join(comma)}")
end
"#;
    let (outcome, output) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        output,
        "6617 3416 12 none,gone,one,a label past sixteen bytes 1,a text longer than sixteen bytes,short 2,three,x,none,gone,none,gone\n"
    );
}

#[test]
fn boxed_integers_compare_in_place_and_big_ones_take_the_helper() {
    // decision AU45: an Integer the checker typed, read from a list, a
    // field or a maybe, is compared in place with another such or with a
    // small Integer in a register (a literal bound) when both are small,
    // with every comparison op; a big Integer on either side takes the
    // helper; both tiers print what the interpreter prints
    let source = r#"module demo
  purpose: Integers read from a list, a field and a maybe, compared with every op (decision AU45), with a big one among them.

import std.console

type Reading
  purpose: A record holding an Integer.
  has value: Integer
end

function pick(flag: Boolean) returns maybe Integer
  purpose: An Integer in a maybe, or nothing.

  if flag then return 7 end
  return nothing
end

function tally(numbers: List of Integer, limit: Integer) returns Text
  purpose: Every comparison of Integers read from a list, against each other and a literal, in loops.

  let mutable hits be 0
  let seven be pick(true)
  let none be pick(false)
  let reading be Reading(value: 7)
  for each left in numbers
    for each right in numbers
      if left is right then change hits to hits + 1 end
      if left is not right then change hits to hits + 10 end
      if left is less than right then change hits to hits + 100 end
      if left is at most right then change hits to hits + 1000 end
      if left is greater than right then change hits to hits + 10000 end
      if left is at least right then change hits to hits + 100000 end
    end
    if reading.value is left then change hits to hits + 1000000 end
    if reading.value is less than left then change hits to hits + 1000000 end
    if seven is left then change hits to hits + 3 end
    if none is not left then change hits to hits + 7 end
    if left is at least 3 then change hits to hits + 20 end
    if 7 is less than left then change hits to hits + 200 end
    if left is not 7 then change hits to hits + 2000 end
    if left is at most 0 - 5 then change hits to hits + 20000 end
  end
  for each step from 1 to limit
    if (numbers.at(0) otherwise 0) is at most step then change hits to hits + 1 end
  end
  return "{hits}"
end

public function main() needs console
  purpose: Print the tally over small Integers and two big ones.

  let huge be 9223372036854775807 + 1
  let negative be 0 - huge
  console.print(tally(numbers: [3, 7, 7, huge, negative, 0 - 5], limit: 9))
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "5412023\n");
}

#[test]
fn variant_tests_in_place_on_fieldless_and_fielded_variants_and_guarded_values() {
    // a `match` arm tests a variant's tag in place (decision AU49):
    // fieldless variants, variants with fields, and a guarded value, which
    // takes the helper; both tiers print what the interpreter prints
    let dir = scratch("variant-tests");
    std::fs::create_dir_all(format!("{dir}/data")).expect("the data directory");
    std::fs::create_dir_all(format!("{dir}/out")).expect("the out directory");
    std::fs::write(
        format!("{dir}/data/value.json"),
        r#"{"kind": "box", "items": [1, 2]}"#,
    )
    .expect("the data");
    let source = format!(
        r#"module demo
  purpose: Variants tested in loops, fieldless and with fields, and a guarded JSON value through the helper (decision AU49).

import std.console
import std.filesystem exposing Path, FileError
import std.json exposing JsonValue, JsonError

type Shape is one of
  purpose: A few shapes.
  Dot
  Line(length: Integer)
  Box(width: Integer, height: Integer)
  Blank
end

function weight(shape: Shape) returns Integer
  purpose: A number per kind of shape.

  match shape
    when Dot then return 1
    when Line(length) then return length
    when Box(width, height) then return width * height
    when Blank then return 0
  end
end

function kind_number(value: JsonValue) returns Integer
  purpose: A number per kind of JSON value: a decision on the data, nothing of it.

  match value
    when JsonObject then return 1
    when JsonArray then return 2
    when JsonText then return 3
    when JsonNumber then return 4
    when JsonBoolean then return 5
    when JsonNull then return 6
  end
end

function weigh(shapes: List of Shape, turns: Integer) returns Integer
  purpose: The shapes weighed this many times, the turn added each time.

  let mutable total be 0
  for each turn from 1 to turns
    for each shape in shapes
      change total to total + weight(shape)
    end
    change total to total + turn
  end
  return total
end

function classify(document: JsonValue, turns: Integer) returns Integer
  purpose: The document classified this many times, the turn added each time.

  let mutable kinds be 0
  for each turn from 1 to turns
    change kinds to kinds + kind_number(document) + turn
  end
  return kinds
end

public function main() or fails with FileError or JsonError needs console, filesystem.read("{dir}/data") only to filesystem.write("{dir}/out"), filesystem.write("{dir}/out")
  purpose: Weigh the shapes and classify the guarded document, many times each, and print both counts.

  let shapes be [Dot, Line(length: 3), Box(width: 2, height: 5), Blank, Line(length: 4)]
  let text be filesystem.read_text(Path("{dir}/data/value.json")) otherwise fail
  let document: JsonValue be json.parse(text) otherwise fail
  console.print("{{weigh(shapes: shapes, turns: 200)}} {{classify(document: document, turns: 200)}}")
end
"#
    );
    let (outcome, printed) = both_ways(&source);
    assert_eq!(outcome, RunOutcome::Finished);
    // 200 turns over the five shapes (18 a turn) plus the turns, and 200
    // classifications of an object (1 each) plus the turns
    assert_eq!(printed, "23700 20300\n");
}

#[test]
fn module_constants_are_copied_in_place_once_built() {
    // decision AU53: a module constant is built by the VM at its first
    // read and copied from the JIT's table in place after, with one more
    // reference, whatever its shape (a text, a small Integer, a list, a
    // set, a big Integer, a maybe holding nothing); compiled before the
    // first read, the code takes the helper once and the copy after; the
    // Cranelift tier prints what the interpreter prints
    let source = r#"module demo
  purpose: Module constants of every shape read in loops (AU53): built once, then copied in place.

import std.console

let digits: Text be "0123456789"

let limit: Integer be 40

let primes: List of Integer be [2, 3, 5, 7, 11]

let names: Set of Text be ["ab", "cd"].to_set()

let huge: Integer be 9223372036854775807 + 1

let absent: maybe Integer be nothing

function tally(turns: Integer) returns Integer
  purpose: Every constant read once per turn of a loop, the hits weighted by the constant.

  let mutable hits be 0
  for each step from 0 to turns
    let digit be step remainder 10
    if digits.contains("{digit}") then change hits to hits + 1 end
    if step is less than limit then change hits to hits + 10 end
    if primes.contains(step) then change hits to hits + 100 end
    if names.contains("ab") then change hits to hits + 1000 end
    if huge is greater than step then change hits to hits + 10000 end
    match absent
      when some(value) then change hits to hits + value
      otherwise change hits to hits + 100000
    end
  end
  return hits
end

public function main() needs console
  purpose: Print the tally over fifty turns.

  console.print("{tally(50)}")
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "5661951\n");
}

#[test]
fn an_image_builds_the_pure_module_constants_and_leaves_the_rest_to_the_run() {
    // decision AW2: when an image is built, every module constant whose
    // code reaches only declared functions, constant reads and natives
    // without `needs` is built on the interpreter, so that a small
    // Integer among them is a literal in the image's code; a constant
    // that reaches an ability call (a method on a type parameter; one on
    // a known type is a declared function) or a function value is left
    // to the run, and one whose build fails too; the run from the image
    // prints what the interpreter prints
    let source = r#"module demo
  purpose: Module constants an image builds before any run (AW2): the pure ones, the rest left to the run.

import std.console

type Square
  has side: Integer
  has name: Text
end

ability Measured
  function area(self) returns Integer
end

ability Measured for Square
  function area(self) returns Integer
    return self.side * self.side
  end
end

let base: Integer be 20

let doubled: Integer be twice(base)

let names: List of Text be ["ab", "cd"]

let zero: Integer be twice(0)

let broken: Integer be base.quotient(zero)

let measured: Integer be area_of(Square(side: 3, name: "s"))

let picked: Integer be apply(action: twice, value: 4)

function twice(value: Integer) returns Integer
  return value * 2
end

function area_of(shape: Shape) returns Integer for any Shape where Shape can Measured
  return shape.area()
end

function apply(action: function(Integer) returns Integer, value: Integer) returns Integer
  return action(value)
end

public function main() needs console
  purpose: Print every constant but the broken one, which is read past a branch not taken.

  if base is greater than 100 then console.print("{broken}") end
  console.print("{base} {doubled} {names.length()} {measured} {picked}")
end
"#;
    let program = compile(source);
    let registry = renyi_vm::Registry::standard();
    let calls = renyi_vm::vm::call_kinds(&program, &registry);
    let pure: Vec<&str> = renyi_vm::native::image::pure_constants(&program, &calls)
        .into_iter()
        .map(|index| program.constants[index].name.as_str())
        .collect();
    assert_eq!(pure, ["base", "doubled", "names", "zero", "broken"]);
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "20 40 2 9 8\n");
    // the image: `broken` fails to build and is left to the run, the
    // others are built, and `base`, `doubled` and `zero` are literals
    let image = renyi_vm::native::image::build(&program, None, &registry).expect("an image");
    let stdout = Capture::default();
    let outcome = run_program(
        &program,
        Options {
            stdout: Box::new(stdout.clone()),
            stderr: Box::new(Capture::default()),
            stdin: Box::new(std::io::Cursor::new(Vec::new())),
            image: Some(image),
            ..Options::default()
        },
    )
    .outcome;
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(stdout.text(), "20 40 2 9 8\n");
}

#[test]
fn typed_parameters_take_boxed_operands_the_checker_typed_from_the_stack() {
    // decision AW3: a call whose Integer, Boolean or Float parameter gets
    // an operand the generated code holds boxed (a variant's field bound
    // by a pattern, a maybe after its otherwise), noted by the checker as
    // the parameter's type, enters the callee's body directly, the
    // operand's tags tested at the site; a big Integer at the same site
    // takes the helper's path, and both tiers print what the interpreter
    // prints
    let source = r#"module demo
  purpose: Typed parameters with boxed operands (decision AW3), a big Integer among them.

import std.console

type Reading is one of
  purpose: An Integer, a Boolean or a Float behind a variant's field, or nothing.
  Count(value: Integer)
  Flag(value: Boolean)
  Measure(value: Float)
  Blank
end

function doubled(value: Integer) returns Integer
  purpose: Twice the value.

  return value + value
end

function negated(flag: Boolean) returns Boolean
  purpose: The flag turned.

  return not flag
end

function halved(value: Float) returns Float
  purpose: Half the value.

  return value / 2.0
end

function describe(reading: Reading, fallback: maybe Integer) returns Text
  purpose: Each kind of reading through its typed parameter, with the fallback after its otherwise.

  let code be fallback otherwise 3
  match reading
    when Count(value) then return "{doubled(value)} {doubled(code)}"
    when Flag(value) then return "{negated(value)} {doubled(code)}"
    when Measure(value) then return "{halved(value)} {doubled(code)}"
    when Blank then return "{doubled(code)}"
  end
end

public function main() needs console
  purpose: Print every reading with and without a fallback.

  let huge be 9223372036854775807 + 1
  let readings be [
    Count(value: 21), Flag(value: true), Measure(value: 5.0), Count(value: huge), Blank
  ]
  for each reading in readings
    console.print(describe(reading: reading, fallback: 4))
    console.print(describe(reading: reading, fallback: nothing))
  end
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        printed,
        "42 8\n42 6\nfalse 8\nfalse 6\n2.5 8\n2.5 6\n18446744073709551616 8\n18446744073709551616 6\n8\n6\n"
    );
}

#[test]
fn frames_leave_by_the_slots_that_may_hold_a_reference() {
    // decision AW4: a return releases only the slots that may hold a
    // counted block there (a boxed slot moved out by its last read or
    // updated in place holds `Nothing` and is skipped), on every path out
    // of a frame with many locals: an early return, one after a loop, one
    // after a failing call caught by `otherwise`; both tiers print what
    // the interpreter prints
    let source = r#"module demo
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
  for each index from 1 to 2000
    change total to total + build(index).length()
  end
  console.print("{total}")
end
"#;
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "106495\n");
}

#[test]
fn read_only_parameters_are_passed_borrowed_between_compiled_bodies() {
    // decision AW5: a direct call passes a parameter the callee accepts
    // borrowed (boxed, read-only, never moved into an owned sink) without
    // a reference of its own and the callee's leave skips it; four of
    // `main`'s eleven direct calls do (a record read by its fields, a
    // text read by its length in a frame that fails beside, the same
    // text passed on borrowed again under a failure propagated, a record
    // beside a list passed by its last use, read in a loop), the others
    // take the owned protocol by the rule (a parameter returned, passed
    // on by a last use, constructed into a record, updated by `with`,
    // read through a field on its last use; a temporary or a last use as
    // the argument; a hand-back past the machine word beside), and both
    // tiers print what the interpreter prints
    let source = r#"module demo
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
  for each index from 1 to 2000
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
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "12340316\n");
}

#[test]
fn a_direct_call_passes_the_grant_and_checks_its_room_once() {
    // decision AW6: the frame's grant travels in the body's signature and
    // a direct call writes the callee's record from it (the helper only
    // for a callee that narrows it); the stack's length is written by the
    // callee's prologue, where the room is checked, and the frames' room
    // for the records this body's direct calls push is checked there
    // once; every shape (a narrowing callee from `main` and from another
    // narrowing one, a callee that inherits the grant and calls on,
    // recursion past the frames' capacity and the machine stack's depth
    // limit, typed parameters before a boxed one at the deepest operand
    // of the caller), against the interpreter
    let source = r#"module demo
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
    let (outcome, printed) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        printed,
        "say 1000\nsay 1000\nsay 2000\nsay 2000\nsay 3000\nsay 3000\n14582220\n"
    );
}

#[test]
fn records_are_updated_by_static_index_on_every_shape_of_with() {
    // decision AW8: a `with` whose base the checker typed goes through
    // the helper that takes the type and the field indices from the site
    // (a parameter's record shared with the caller, a record held once in
    // its slot, long field names, the base and the result of a loop), and
    // the general helper keeps a refined type and an update of more than
    // seven fields; every form prints what the interpreter prints
    let source = r#"module demo
  purpose: Records updated by static index on both tiers, with every case the site cannot take.

import std.console

type Point
  purpose: A point with a label.
  has east: Integer
  has north: Integer
  has label: Text
end

type Wide
  purpose: A record whose field names are longer than a small text holds.
  has first_component_value: Integer
  has second_component_value: Integer
  has third_component_value: Text
end

type Span
  purpose: A refined record: the update goes through the general helper.
  has low: Integer
  has high: Integer where high is at least 0
end

type Nine
  purpose: More fields than the packed indices hold.
  has a1: Integer
  has a2: Integer
  has a3: Integer
  has a4: Integer
  has a5: Integer
  has a6: Integer
  has a7: Integer
  has a8: Integer
  has a9: Integer
end

function moved(point: Point, step: Integer) returns Point
  purpose: The point moved east, the base a parameter.

  return point with east: point.east + step
end

function relabelled(point: Point, label: Text) returns Point
  purpose: The point relabelled in its slot.

  let mutable next be point
  change next to next with label: label, north: next.north * 2
  return next
end

function widened(wide: Wide, amount: Integer) returns Wide
  purpose: Two long-named fields updated, the third kept.

  return wide
    with second_component_value: wide.second_component_value + amount, first_component_value: amount
end

function spanned(span: Span, high: Integer) returns Span or fails with Text
  purpose: The refined record updated, which may fail.

  let next be span with high: high otherwise fail with "below zero"
  return next
end

function all_nine(nine: Nine, step: Integer) returns Nine
  purpose: Eight fields updated at once: more than the packed indices hold.

  return nine with a1: nine.a1 + step, a2: 2, a3: 3, a4: 4, a5: 5, a6: 6, a7: 7, a8: 8
end

public function main() needs console
  purpose: Runs every form and prints what each leaves.

  let origin be Point(east: 1, north: 2, label: "o")
  let kept be origin
  let far be moved(point: origin, step: 5)
  console.print("shared base: {origin.east} {kept.east} {far.east} {far.label}")
  let mutable own be Point(east: 10, north: 20, label: "own")
  for each step from 1 to 1000
    change own to own with east: own.east + step, north: own.north + 1
  end
  console.print("in place: {own.east} {own.north} {own.label}")
  let renamed be relabelled(point: own, label: "renamed")
  console.print("relabelled: {renamed.label} {renamed.north} / {own.label} {own.north}")
  let mutable wide be Wide(
    first_component_value: 1,
    second_component_value: 2,
    third_component_value: "third"
  )
  for each step from 1 to 300
    change wide to widened(wide: wide, amount: step)
  end
  console.print(
    "wide: {wide.first_component_value} {wide.second_component_value} {wide.third_component_value}"
  )
  let span be Span(low: 1, high: 2)
  let raised be spanned(span: span, high: 9) otherwise Span(low: 0, high: 0)
  let lowered be spanned(span: span, high: -1) otherwise Span(low: -7, high: 0)
  console.print("refined: {raised.high} {lowered.low} {span.high}")
  let mutable nine be Nine(a1: 1, a2: 2, a3: 3, a4: 4, a5: 5, a6: 6, a7: 7, a8: 8, a9: 9)
  for each step from 1 to 50
    change nine to all_nine(nine: nine, step: step)
  end
  console.print("nine: {nine.a1} {nine.a5} {nine.a9}")
  let mutable total be 0
  for each step from 1 to 500
    let fresh be Point(east: step, north: step, label: "f")
    let twice be fresh with north: fresh.north * 2, east: fresh.east + 1
    change total to total + twice.east + twice.north
  end
  console.print("fresh: {total}")
end
"#;
    let (outcome, output) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        output,
        "shared base: 1 1 6 o\nin place: 500510 1020 own\nrelabelled: renamed 2040 / own 1020\nwide: 300 45152 third\nrefined: 9 -7 2\nnine: 1276 5 9\nfresh: 376250\n"
    );
}

#[test]
fn records_of_two_fields_are_returned_as_pairs_on_every_shape_of_call_and_use() {
    // decision AW9: a function whose declared result is a record of two
    // fields leaves its two fields in two cells of the caller's stack (a
    // construct returned at once, a call passed on, a pair slot returned
    // whole), and the caller takes the pair as it lies (stored and taken
    // apart, read as a field at once, kept whole in a list, held across a
    // loop whose Integer leaves the machine word and hands the frame back,
    // its two fields rebuilt into the record, kept past `otherwise fail`
    // and returned past one, taken with a fallback value, on the success
    // path and the failure path); a record of three fields stays a
    // record; every form prints what the interpreter prints
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
    let (outcome, output) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        output,
        "advanced: 35 15\nchecked: true +\nrelayed: +\npicked: true ++ false over\nkept: 22 22 22\nclimbed: 17 17\nblocked: 0 15\nsettled: 99 15\ndoubled: 1180591620717411303424 10535 115\ntriple: 3 6 t\n"
    );
}
