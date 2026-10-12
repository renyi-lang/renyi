//! The template tier of decision AU18 against the interpreter: every
//! code object runs on template code from its first call and nothing is
//! promoted to the Cranelift tier (`RENYI_NATIVE_TIER=template`), so
//! that every sequence per op is exercised; the programs cover the ops,
//! the calls of every kind, the handlers, the crashes, the loops and the
//! updates in place, and each prints what the interpreter prints.

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

/// Run `main` on the template tier alone, or on the interpreter.
fn run(source: &str, interpret: bool) -> (RunOutcome, String, String) {
    std::env::set_var("RENYI_NATIVE_TIER", "template");
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
    (outcome, stdout.text(), stderr.text())
}

/// Both ways must agree; the template way's outcome and output are given
/// back for closer checks.
fn both_ways(source: &str) -> (RunOutcome, String, String) {
    let templates = run(source, false);
    let interpreted = run(source, true);
    assert_eq!(
        templates, interpreted,
        "template code against the interpreter"
    );
    templates
}

#[test]
fn loops_calls_and_updates_print_what_the_interpreter_prints() {
    let source = r#"module demo
  purpose: Loops over every source, calls of every kind, records updated in place, texts joined.

import std.console

type Point
  purpose: A record updated in place.
  has x_value: Integer
  has y_value: Integer
  has label: Text
end

type Shape is one of
  purpose: Variants with and without fields.
  Circle(radius: Integer)
  Dot
end

function walk(items: List of Integer, text: Text, limit: Integer) returns Text
  purpose: Sum the items while appending to the same list, count the glyphs, walk a range and a set.

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

function moved(point: Point, times: Integer) returns Point
  purpose: The point moved in place, a count of times.

  let mutable current be point
  for each step from 1 to times
    change current to current with x_value: current.x_value + step, y_value: current.y_value - 1
  end
  return current with label: current.label.to_upper()
end

function describe(shape: Shape) returns Text
  purpose: A variant matched.

  match shape
    when Circle(radius) then return "circle of {radius}"
    when Dot then return "dot"
  end
end

type DivisionError is one of
  purpose: Why a division failed.
  ByZero
end

function divide(numerator: Integer, denominator: Integer) returns Integer or fails with DivisionError
  purpose: A division that fails on zero.

  if denominator is 0 then fail with ByZero end
  return numerator.quotient(denominator)
end

public function main() needs console
  purpose: Print every result.

  console.print(walk(items: [1, 2, 3], text: "a b c", limit: 4))
  let start be Point(x_value: 1, y_value: 10, label: "start")
  let alias be start
  let after be moved(point: start, times: 3)
  console.print("{after.x_value} {after.y_value} {after.label} {alias.x_value} {alias.label}")
  console.print("{describe(Circle(radius: 5))} {describe(Dot)}")
  let answer be divide(numerator: 7, denominator: 2) otherwise -1
  let fallen be divide(numerator: 7, denominator: 0) otherwise -1
  console.print("{answer} {fallen}")
  let mutable total be 0
  let mutable turns be 0
  repeat until turns is 5
    change turns to turns + 1
    if turns is 3 then continue end
    change total to total + turns
  end
  console.print("{total} {turns}")
  let words be ["pear", "fig", "apple"]
  let lengths be for each word in words collect word.length()
  let ordered be for each word in words sorted by word collect word
  let lengths_shown be for each length in lengths collect length.to_text()
  let comma be ","
  let lengths_joined be lengths_shown.join(comma)
  let ordered_joined be ordered.join(comma)
  console.print("{lengths_joined} {ordered_joined}")
  let found be for each word in words where word.contains("p") count
  let shout be "abc".to_upper()
  let summed be for each word in words sum word.length()
  let measure be 3.5 + 1.25
  console.print("{found} {shout} {summed} {measure}")
end
"#;
    let (outcome, printed, _) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        printed,
        "6 6 3 10 6 x1x2y1y2\n7 7 START 1 start\ncircle of 5 dot\n3 -1\n12 5\n4,3,5 apple,fig,pear\n2 ABC 12 4.75\n"
    );
}

#[test]
fn failures_land_on_their_handlers_or_leave_the_frame() {
    let source = r#"module demo
  purpose: Failures inside template code: handled where a handler is open, passed on where none is.

import std.console

type OddStep is one of
  purpose: The failure of an odd step.
  Odd(step: Integer)
end

function risky(step: Integer) returns Integer or fails with OddStep
  purpose: Fail on an odd step.

  if step remainder 2 is 1 then fail with Odd(step: step) end
  return step * 2
end

function tally(limit: Integer) returns Text
  purpose: Every step handled in place, the failures counted.

  let mutable total be 0
  let mutable failures be 0
  for each step from 1 to limit
    let value be risky(step) otherwise 0
    if value is 0 then change failures to failures + 1 end
    change total to total + value
  end
  return "{total} {failures}"
end

function pass_on(step: Integer) returns Integer or fails with OddStep
  purpose: No handler here: the failure leaves the frame.

  let doubled be risky(step) otherwise fail
  return doubled + 1
end

function describe(step: Integer) returns Text
  purpose: The failure of a step, matched by its variant.

  let doubled be pass_on(step) otherwise return "odd {step}"
  return "even {doubled}"
end

public function main() needs console
  purpose: Print the tallies and the passed-on failures.

  console.print(tally(6))
  let passed be pass_on(4) otherwise -1
  let failed be pass_on(5) otherwise -1
  console.print("{passed} {failed}")
  console.print("{describe(3)} {describe(4)}")
end
"#;
    let (outcome, printed, _) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "24 3\n9 -1\nodd 3 even 9\n");
}

#[test]
fn a_crash_in_template_code_names_its_line() {
    let source = r#"module demo
  purpose: A crash deep in template code, located by its line.

import std.console

function depth(level: Integer) returns Integer
  purpose: Divide by zero at the bottom.

  if level is 0 then
    let zero be 0
    return 10.quotient(zero)
  end
  return depth(level - 1) + 1
end

public function main() needs console
  purpose: Crash.

  console.print("start")
  console.print("{depth(3)}")
end
"#;
    let (outcome, printed, _) = both_ways(source);
    assert_eq!(printed, "start\n");
    match outcome {
        RunOutcome::Crashed { message, location } => {
            assert_eq!(message, "division by zero");
            assert_eq!(location.as_deref(), Some("demo.ry:11"));
        }
        other => panic!("expected a crash, got {other:?}"),
    }
}

#[test]
fn recursion_past_the_depth_of_native_frames_carries_on() {
    let source = r#"module demo
  purpose: A recursion deeper than the machine stack allows for native frames.

import std.console

function count_down(level: Integer) returns Integer
  purpose: Recurse a thousand levels.

  if level is 0 then return 0 end
  return count_down(level - 1) + 1
end

public function main() needs console
  purpose: Print the depth.

  console.print("{count_down(1000)}")
end
"#;
    let (outcome, printed, _) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "1000\n");
}

/// A variant without fields built over and over (decision AU24): the
/// first construction builds it through the helper, the others copy it
/// from its slot in place with one more reference, on the template tier; the values
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
    let (outcome, output, _) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(output, "3000 1000 3000 Red Green 8001 666\n");
}

/// Texts compared on the template tier (decision AU26): two texts held in the
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
    let (outcome, output, _) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(output, "2000 10800 5400 20100 6\n");
}

/// Records and variants in one counted block each (decision AU27), on
/// the template tier: built from the top of the stack, kept in a list, read in
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
    let (outcome, output, _) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(output, "2000 3003 2000 33000 6000\n");
}

/// The retain and the release in place in template code (decision AU29):
/// records, long texts and lists counted in their blocks and freed when
/// the last reference goes, big Integers through the path out of line,
/// small values with no count; every count is the interpreter's.
#[test]
fn counts_raised_and_lowered_in_place_free_the_last_and_spare_big_integers() {
    let source = r#"module demo
  purpose: Values counted in place by template code: big Integers, long texts, records.

import std.console

type Box
  purpose: A box around a long label and a big weight.
  has label: Text
  has weight: Integer
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
  purpose: Make, keep and drop counted values over and over.

  let mutable kept: List of Box be []
  let mutable total be 0
  let mutable big be heavy(1)
  for each index from 1 to 3000
    let item be box_of(index)
    change big to item.weight
    if index remainder 3 is 0 then
      change kept to kept.append(item)
    end
    change total to total + item.label.length()
  end
  let mutable residues be 0
  for each item in kept
    change residues to residues + (item.weight remainder 7)
  end
  console.print("{kept.length()} {total} {big remainder 1000003} {residues}")
end
"#;
    let (outcome, output, _) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(output, "1000 112893 900027 3003\n");
}

/// `List.at` in place (decision AU34) and the in-place entries of
/// `Text.contains` and `List.contains` (decision AW11, in place of
/// AU34's lean helpers) in template code: items of every kind read
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
    let (outcome, output, _) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        output,
        "6617 3416 12 none,gone,one,a label past sixteen bytes 1,a text longer than sixteen bytes,short 2,three,x,none,gone,none,gone\n"
    );
}

/// Integers the checker typed compared in place on the template tier
/// (decision AU45): every op, Integers from a list, a field and a maybe,
/// against each other and against literal bounds, big Integers through
/// the general comparison.
#[test]
fn boxed_integers_compare_in_place_and_big_ones_take_the_general_path() {
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
    let (outcome, output, _) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(output, "5412023\n");
}

#[test]
fn module_constants_are_copied_in_place_once_built() {
    // decision AU53: a module constant is built by the VM at its first
    // read and copied from the JIT's table in place after, with one more
    // reference, whatever its shape (a text, a small Integer, a list, a
    // set, a big Integer, a maybe holding nothing); the template tier
    // prints what the interpreter prints
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
    let (outcome, output, _) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(output, "5661951\n");
}

/// A scratch directory inside the workspace's `target`, forward slashes
/// so that it can be spelled in Renyi text.
fn scratch(name: &str) -> String {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf();
    let dir = workspace.join("target/vm-tests/template").join(name);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    dir.display().to_string().replace('\\', "/")
}

#[test]
fn variant_tests_in_place_on_fieldless_and_fielded_variants_and_guarded_values() {
    // a `match` arm tests a variant's tag in place in template code
    // (decision AU49): fieldless variants, variants with fields, and a
    // guarded value, which takes the helper out of line
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
    let (outcome, printed, _) = both_ways(&source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "23700 20300\n");
}

#[test]
fn a_direct_call_passes_the_grant_through_the_template_tier() {
    // decision AW6: the frame's grant travels in the Cranelift body's signature and
    // a direct call writes the callee's record from it (the helper only
    // for a callee that narrows it); the stack's length is written by the
    // callee's prologue, where the room is checked, and the frames' room
    // for the records this body's direct calls push is checked there
    // once; every shape (a narrowing callee from `main` and from another
    // narrowing one, a callee that inherits the grant and calls on,
    // recursion past the frames' capacity and the machine stack's depth
    // limit, typed parameters before a boxed one at the deepest operand
    // of the caller); the
    // template tier pushes its own records and enters a Cranelift body
    // through its trampoline, which reads the grant from the record
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
    let (outcome, printed, _) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        printed,
        "say 1000\nsay 1000\nsay 2000\nsay 2000\nsay 3000\nsay 3000\n14582220\n"
    );
}

#[test]
fn records_are_updated_by_static_index_through_the_template_tier() {
    // decision AW8: the template tier calls the helper that takes the type
    // and the indices from the site, and the general one for a refined
    // type and for more than seven updates
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
    let (outcome, output, _) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        output,
        "shared base: 1 1 6 o\nin place: 500510 1020 own\nrelabelled: renamed 2040 / own 1020\nwide: 300 45152 third\nrefined: 9 -7 2\nnine: 1276 5 9\nfresh: 376250\n"
    );
}

#[test]
fn variants_compare_in_place_by_tag_and_fielded_ones_through_the_helper() {
    // two values the checker typed as one variant type without its own
    // `equals` are compared in place by the type and the tag of their
    // blocks, the operands not borrowed released (decision AW13): different
    // tags and the same tag of a variant without fields answer there, the
    // same tag of a variant with fields takes the helper; a guarded one, a
    // `maybe`, a type with its own `equals` and lists take the helper too,
    // which answers `is` and `is not` as structural equality unless the
    // left operand's type declares its own: template code prints what the
    // interpreter prints
    let dir = scratch("unit-variants");
    std::fs::write(format!("{dir}/greeting.txt"), "hello").expect("the data file");
    let source = format!(
        r#"module demo
  purpose: Variants without fields compared in place, the rest through the helper (decision AW13).

import std.console
import std.filesystem exposing FileError, Path

type Kind is one of
  purpose: Kinds without fields.
  Word
  Number
  Symbol
end

type Mood is one of
  purpose: Kinds without fields whose equality is declared: any two are equal.
  Glad
  Sad
end

function weight(mood: Mood) returns Integer
  purpose: A number per mood.

  match mood
    when Glad then return 1
    when Sad then return 2
  end
end

ability Equal for Mood
  function equals(self, other: Mood) returns Boolean
    return weight(self) + weight(other) is at least 0
  end
end

type Shape is one of
  purpose: Variants with fields, compared structurally.
  Dot
  Line(length: Integer)
end

type Note
  purpose: A kind with a text.
  has kind: Kind
  has body: Text
end

function kind_of(text: Text) returns Kind
  purpose: A kind by the text.

  if text is "1" then return Number end
  if text is "+" then return Symbol end
  return Word
end

function tally_kinds(kinds: List of Kind, wanted: Kind) returns Integer
  purpose: How many kinds are the wanted one, in both orders and with both operators.

  let mutable hits be 0
  for each kind in kinds
    if kind is wanted then change hits to hits + 1 end
    if wanted is kind then change hits to hits + 10 end
    if kind is not wanted then change hits to hits + 100 end
    if kind is Word then change hits to hits + 1000 end
    if kind_of("1") is kind then change hits to hits + 10000 end
  end
  return hits
end

function shapes(moods: List of Mood) returns Integer
  purpose: Comparisons through the helper: variants with fields, lists and a declared equals.

  let mutable hits be 0
  let short be Line(length: 3)
  let long be Line(length: 4)
  for each mood in moods
    if short is Line(length: 3) then change hits to hits + 1 end
    if short is not long then change hits to hits + 10 end
    if Dot is short then change hits to hits + 100 end
    if Dot is Dot then change hits to hits + 100000 end
    if [1, 2] is [1, 2] then change hits to hits + 1000 end
    if mood is Glad then change hits to hits + 10000 end
  end
  return hits
end

function first_kind(kinds: List of Kind) returns maybe Kind
  purpose: The first kind, or nothing.

  return kinds.at(0)
end

public function main() or fails with FileError needs console, filesystem.read("{dir}") only to console
  purpose: Print the tallies, with a guarded note's kind compared through the helper.

  let secret be filesystem.read_text(Path("{dir}/greeting.txt")) otherwise fail
  let note be Note(kind: kind_of(secret), body: secret)
  let kinds be [Word, Number, Word, Symbol]
  let mutable kept be 0
  for each kind in kinds
    if note.kind is kind then change kept to kept + 1 end
    if note.kind is not kind_of("+") then change kept to kept + 10 end
  end
  let none be first_kind([])
  let found be first_kind(kinds)
  let mutable maybe_hits be 0
  if none is Word then change maybe_hits to maybe_hits + 1 end
  if found is Word then change maybe_hits to maybe_hits + 10 end
  if found is not none then change maybe_hits to maybe_hits + 100 end
  console.print("{{tally_kinds(kinds: kinds, wanted: Word)}} {{shapes([Glad, Sad, Sad])}} {{kept}} {{maybe_hits}}")
end
"#
    );
    let (outcome, printed, _) = both_ways(&source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "12222 333033 42 110\n");
}

#[test]
fn loops_in_two_slots_keep_their_list_and_position_across_a_hand_back() {
    // a loop's iterator lies in two slots of the frame (decision AW14):
    // the source listed in the first, the position in the one after it,
    // which the generated code keeps in a register and reads the list in
    // place; a hand-back in the body (a product that leaves the machine
    // word) carries the position to the interpreter, the list grown in
    // its own body keeps the loop's items, an inner loop uses its slots
    // again on every row, and a sorted loop and a query over two sources
    // allocate their slots as the compile judge holds: template code prints what
    // the interpreter prints
    let source = r#"module demo
  purpose: Loops whose iterator lies in two slots (decision AW14): a hand-back in the body, the list grown in the body, nested loops entered again.

import std.console

function total(items: List of Integer, factor: Integer) returns Integer
  purpose: A product that leaves the machine word partway, so that the loop goes on in the interpreter.

  let mutable product be 1
  let mutable seen be 0
  for each item in items
    change product to product * factor + item
    change seen to seen + 1
  end
  return seen + product remainder 1000
end

function grown(items: List of Integer) returns Integer
  purpose: The loop's list appended to in its own body keeps its items.

  let mutable list be items
  let mutable turns be 0
  for each item in list
    change list to list.append(item * 2)
    change turns to turns + 1
  end
  return turns * 1000 + list.length()
end

function nested(rows: List of List of Integer) returns Integer
  purpose: An inner loop entered once per row, its slot used again.

  let mutable total_value be 0
  for each row in rows
    for each cell in row
      change total_value to total_value + cell
    end
  end
  return total_value
end

function sorted_turns(items: List of Integer) returns Text
  purpose: A sorted loop, two iterators in turn, and a query over two sources.

  let mutable seen: List of Integer be []
  for each item in items sorted by item descending
    change seen to seen.append(item)
  end
  let pairs be for each left in items, right in [10, 20] where left is less than 3 collect left * right
  return "{seen.length()} {seen.first() otherwise 0} {pairs.length()} {pairs.last() otherwise 0}"
end

public function main() needs console
  purpose: Print the tallies.

  console.print("{total(items: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10], factor: 1000000007)} {grown([1, 2, 3])} {nested([[1, 2], [3], [], [4, 5, 6]])}")
  console.print(sorted_turns([3, 1, 2]))
end
"#;
    let (outcome, printed, _) = both_ways(source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "1000 3006 21\n3 3 4 40\n");
}

#[test]
fn matches_by_a_table_enter_the_arm_by_the_tag_and_fall_through_otherwise() {
    // a `match` with at least three arms naming a variant goes through a
    // table by the subject's tag (decision AW15): the first arm that can
    // match the tag, entered past its own tag test; a guard or a nested
    // pattern that fails goes on down the arms, a binding arm is entered
    // at its start, a tag past the table and a guarded subject fall
    // through to the arms' own tests, an overflow inside an arm hands the
    // frame back, a `maybe` subject and a two-arm match have no table:
    // template code prints what the interpreter prints
    let dir = scratch("match-table");
    std::fs::write(format!("{dir}/greeting.txt"), "hello").expect("the data file");
    let source = format!(
        r#"module demo
  purpose: Matches by a table (decision AW15): arms with and without fields, a guard that fails to a later arm of the same tag, a nested literal, a binding arm, a tag past the table, an overflow inside an arm, a maybe subject, a guarded subject and a two-arm match without a table.

import std.console
import std.filesystem exposing FileError, Path

type Shape is one of
  purpose: Shapes, the last one past the table of `area`.
  Circle(radius: Integer)
  Square(side: Integer)
  Triangle(base: Integer, height: Integer)
  Dot
  Blank
end

function area(shape: Shape) returns Integer
  purpose: Six arms: a guarded circle falls to the next circle arm, a square with a nested literal to the next square arm, the field-less ones, the otherwise for the tag past the table.

  match shape
    when Circle(radius) where radius is greater than 100 then return radius * 3
    when Circle(radius) then return radius * radius * 3
    when Square(side: 0) then return 7
    when Square(side) then return side * side
    when Triangle(base, height) then return base * height
    when Dot then return 1
    otherwise return 5
  end
end

function wide(shape: Shape, factor: Integer) returns Integer
  purpose: A product that leaves the machine word inside an arm entered through the table, so that the arm goes on in the interpreter.

  match shape
    when Circle(radius) then return radius * factor * factor remainder 1000
    when Square(side) then return side * factor remainder 1000
    when Dot then return factor remainder 1000
    otherwise return 0
  end
end

function two(shape: Shape) returns Integer
  purpose: Two arms naming a variant: no table.

  match shape
    when Dot then return 10
    when Blank then return 20
    otherwise return 30
  end
end

function wrapped(shape: maybe Shape) returns Integer
  purpose: A maybe subject: nothing, then variants through some; no arm names a variant directly.

  match shape
    when nothing then return 100
    when some(Circle(radius)) then return radius
    when some(Dot) then return 7
    otherwise return 9
  end
end

function kind_number(shape: Shape) returns Integer
  purpose: Three arms naming a variant, then a binding arm that catches the rest, which the table enters at its start.

  match shape
    when Dot then return 1
    when Blank then return 2
    when Circle(radius) then return 3 + radius - radius
    when rest then return area(rest)
  end
end

function total_area(shapes: List of Shape) returns Integer
  purpose: The areas summed.

  let mutable total be 0
  for each shape in shapes
    change total to total + area(shape)
  end
  return total
end

public function main() or fails with FileError needs console, filesystem.read("{dir}") only to console
  purpose: Print the tallies, with a guarded shape matched through the table's fallthrough.

  let shapes be [Circle(radius: 2), Circle(radius: 200), Square(side: 0), Square(side: 3), Triangle(base: 4, height: 5), Dot, Blank]
  let secret be filesystem.read_text(Path("{dir}/greeting.txt")) otherwise fail
  let guarded_shape be Square(side: secret.length())
  let kinds be kind_number(Square(side: 3)) + kind_number(Triangle(base: 4, height: 5)) + kind_number(Dot) + kind_number(Blank) + kind_number(Circle(radius: 9))
  console.print("{{total_area(shapes)}} {{wide(shape: Circle(radius: 3), factor: 10000000019)}} {{wide(shape: Dot, factor: 12)}} {{two(Dot) + two(Blank) + two(Circle(radius: 1))}} {{wrapped(nothing) + wrapped(Circle(radius: 4)) + wrapped(Dot) + wrapped(Blank)}} {{area(guarded_shape)}} {{kinds}}")
end
"#
    );
    let (outcome, printed, _) = both_ways(&source);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "654 83 12 60 120 25 35\n");
}
