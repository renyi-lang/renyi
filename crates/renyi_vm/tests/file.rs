//! The bytecode file (decisions Z1 to Z4): a program rendered to a file and
//! loaded back renders the same and behaves the same, a crash after wide
//! characters is located on its line because every span counts characters,
//! and a file that does not fit is refused naming the place.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use renyi_syntax::SourceFile;
use renyi_vm::file::{is_bytecode, load, render, FORMAT};
use renyi_vm::{compile_project, run_program, run_tests, Options, Program, RunOutcome};

fn compile(name: &str, source: &str) -> Program {
    let file = SourceFile::new(name, source);
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

/// Run `main`, capturing the standard output.
fn run(program: &Program) -> (RunOutcome, String) {
    let stdout = Capture::default();
    let outcome = run_program(
        program,
        Options {
            stdout: Box::new(stdout.clone()),
            stderr: Box::new(Capture::default()),
            stdin: Box::new(std::io::Cursor::new(Vec::new())),
            ..Options::default()
        },
    )
    .outcome;
    (outcome, stdout.text())
}

/// Every kind of code object (functions, examples with `is` and `fails
/// with`, a constant, a test, refinements of a variant and of a subtype)
/// and every kind of constant (Boolean, Integer, Decimal, Float, Text, a
/// function value), with a loop that breaks, a grouped query and a sort.
const PROGRAM: &str = r#"module demo
  purpose: Every kind of code object and constant the bytecode file holds.

import std.console

public type Shape is one of
  purpose: A plane figure.
  Circle(radius: Decimal where radius is greater than 0)
  Square(side: Decimal)
  can Equal
  can ToText
end

public type Interval
  purpose: A closed interval of whole numbers.
  has low: Integer
  has high: Integer
  can Equal
  can Compare by low
  can ToText
end

public type Percent is Integer where value is at most 100
  purpose: A whole percentage.

let banner: Text be "measures"

public function area(shape: Shape) returns Decimal
  purpose: The area of a shape, with pi as 3.
  example: area(Square(side: 2.0)) is 4.00
  example: area(Circle(radius: 1.0)) is 3.00

  match shape
    when Circle(radius) then return 3.0 * radius * radius
    when Square(side) then return side * side
  end
end

public function percent(value: Integer) returns Percent or fails with ConstraintViolation
  purpose: A percentage from a whole number.
  example: percent(50) is 50
  example: percent(200) fails with ConstraintViolation

  return Percent(value) otherwise fail
end

public function widen(interval: Interval, amount: Integer) returns Interval
  purpose: The interval with its high end moved up.
  example: widen(interval: Interval(low: 1, high: 2), amount: 3) is Interval(low: 1, high: 5)

  return interval with high: interval.high + amount
end

public function total(values: List of Integer) returns Integer
  purpose: The values grouped by parity, each group summed, then added.
  example: total([1, 2, 3, 4]) is 10

  let sums be for each value in values group by value remainder 2 sum value
  let mutable result be 0
  for each parity, part in sums sorted by parity
    change result to result + part
  end
  return result
end

public function first_past(values: List of Integer, limit: Integer) returns maybe Integer
  purpose: The first value past the limit, found by a loop that breaks.
  example: first_past(values: [1, 5, 9], limit: 4) is 5
  example: first_past(values: [1, 2], limit: 4) is nothing

  let mutable found: maybe Integer be nothing
  for each value in values sorted by value
    if value is greater than limit then
      change found to value
      break
    end
  end
  return found
end

public function apply(measure: function(Shape) returns Decimal, shape: Shape) returns Decimal
  purpose: Apply a function value.

  return measure(shape)
end

public function main() needs console
  purpose: Print the measures.

  let ratio: Float be 1.5
  let mutable steps be 0
  repeat until steps is 3
    change steps to steps + 1
  end
  console.print("{banner}: {area(Square(side: 3.0))} {apply(measure: area, shape: Circle(radius: 2.0))}")
  console.print("{total([1, 2, 3, 4])} {first_past(values: [3, 8, 1], limit: 2) otherwise 0} {steps} {ratio * 2.0}")
  console.print("{percent(300) otherwise 100} {widen(interval: Interval(low: 0, high: 1), amount: 4).high}")
end

test "the parts agree"
  check total([1, 2, 3]) is 6
  check area(Square(side: 1.0)) is 1.00
end
"#;

#[test]
fn a_program_survives_the_round_trip() {
    let program = compile("demo.ry", PROGRAM);
    let text = render(&program);
    assert!(
        text.starts_with(&format!("{{\n  \"format\": {FORMAT},\n  \"modules\": [\n")),
        "{}",
        &text[..60]
    );
    for kind in [
        "BooleanConstant",
        "IntegerConstant",
        "DecimalConstant",
        "FloatConstant",
        "TextConstant",
        "FunctionConstant",
        "FunctionCode",
        "TestCode",
        "ConstantCode",
        "ExampleCode",
        "RefinementCode",
        "IsExpected",
        "FailsWithExpected",
        "OpGroupFold",
        "SumFold",
        "OpSortByKey",
        "OpMarkStack",
        "OpUnwindStack",
        "OpIterNext",
        "OpConstructVariant",
        "OpCallValue",
        "Remainder",
        "RecordShape",
        "SumShape",
        "SubtypeShape",
    ] {
        assert!(text.contains(&format!("\"kind\": \"{kind}\"")), "no {kind}");
    }
    assert!(text.contains("\"digits\": \"1.5\""), "the Float's digits");
    assert!(text.contains("\"digits\": \"2.0\""), "the Decimal's digits");
    assert!(text.contains("\"path\": \"demo.ry\""), "the source's path");
    let loaded = load(&text).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(render(&loaded), text);
    assert_eq!(loaded.codes.len(), program.codes.len());
    assert_eq!(loaded.main, program.main);
    assert_eq!(loaded.sources, program.sources);
    let (outcome, printed) = run(&program);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "measures: 9.00 12.000\n10 3 3 3.0\n100 5\n");
    let (outcome_from_file, printed_from_file) = run(&loaded);
    assert_eq!(outcome_from_file, RunOutcome::Finished);
    assert_eq!(printed_from_file, printed);
    let report = run_tests(&program, Options::default());
    assert_eq!(report.failed(), 0, "{}", report.render());
    let report_from_file = run_tests(&loaded, Options::default());
    assert_eq!(report_from_file.render(), report.render());
}

#[test]
fn a_crash_after_wide_characters_is_located_on_its_line() {
    // every span counts characters (decision Z4): the banner is 20
    // characters and 60 bytes, and the crash is on line 12 of 13
    let source = "module demo
  purpose: A crash after a line of wide characters is located on its line.

import std.console

let banner: Text be \"二十个汉字二十个汉字二十个汉字二十个汉字\"

public function main() needs console
  purpose: Print the banner, then crash.

  console.print(banner)
  crash with \"boom\"
end
";
    let program = compile("demo.ry", source);
    let (outcome, printed) = run(&program);
    assert_eq!(printed, "二十个汉字二十个汉字二十个汉字二十个汉字\n");
    assert_eq!(
        outcome,
        RunOutcome::Crashed {
            message: "boom".to_string(),
            location: Some("demo.ry:12".to_string()),
        }
    );
    let text = render(&program);
    let loaded = load(&text).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(run(&loaded).0, outcome);
    assert_eq!(loaded.sources, program.sources);
}

#[test]
fn a_file_that_does_not_fit_is_refused_naming_the_place() {
    assert!(is_bytecode("out/demo.ryc"));
    assert!(!is_bytecode("demo.ry"));
    assert!(!is_bytecode("demo.ryc.bak"));
    let text = render(&compile("demo.ry", PROGRAM));
    let error = |text: &str| load(text).err().expect("refused");
    assert_eq!(
        error("{\n  \"format\": 5\n}\n"),
        "the file is format 5; this VM reads format 4"
    );
    assert_eq!(
        error("{\"format\": \"1\"}"),
        "the file.format: expected a number, found a string"
    );
    assert_eq!(error("{}"), "the file: no `format`");
    assert_eq!(error("[]"), "the file: expected an object, found an array");
    assert!(error("{\n  \"format\": 4,\n").starts_with("line "));
    let out_of_range = text.replacen("\"main\": ", "\"main\": 99999", 1);
    assert!(
        error(&out_of_range).starts_with("the file is not consistent: main 99999"),
        "{}",
        error(&out_of_range)
    );
    let unknown = text.replacen("\"kind\": \"OpReturnNothing\"", "\"kind\": \"OpHalt\"", 1);
    let message = error(&unknown);
    assert!(
        message.starts_with("the file.codes[")
            && message.ends_with("]: unknown operation `OpHalt`"),
        "{message}"
    );
    let wrong_slot = text.replacen(
        "\"kind\": \"OpStore\",\n",
        "\"kind\": \"OpStore\",\n            \"slot\": 60000,\n",
        1,
    );
    let message = error(&wrong_slot);
    assert!(
        message.contains("is out of range") || message.contains("does not fit"),
        "{message}"
    );
}
