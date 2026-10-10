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
    // updates in the slot, one copy, two moves into a call
    let program = compile(source);
    let count = |wanted: fn(&renyi_vm::bytecode::Op) -> bool| {
        program
            .codes
            .iter()
            .flat_map(|code| code.ops.iter())
            .filter(|op| wanted(op))
            .count()
    };
    assert_eq!(count(|op| matches!(op, Op::TakeField { .. })), 2);
    assert_eq!(count(|op| matches!(op, Op::WithSlot { .. })), 8);
    assert_eq!(count(|op| matches!(op, Op::With(_))), 1);
    assert_eq!(count(|op| matches!(op, Op::LoadMove(_))), 2);
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
