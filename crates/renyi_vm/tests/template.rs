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

/// `List.at` in place and the lean helpers of `Text.contains` and
/// `List.contains` in template code (decision AU34): items of every kind read
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
