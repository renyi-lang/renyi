//! The Python bridge (decisions AJ2, AJ3 and AL1 to AL4), driven through
//! the binary: a project whose manifest binds a Python module calls it
//! through the worker; a recording replays and reproduces without Python;
//! `--deny python` refuses the program; an interpreter that is not there,
//! an exception, a result of the wrong type and a worker that ends are
//! failures of the bridge's type; `renyi publish` refuses a project with
//! Python modules. The tests need a Python 3 on the PATH, as the
//! conformance suite does.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn renyi(directory: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_renyi"));
    command
        .args(args)
        .current_dir(directory)
        .env_remove("RENYI_PYTHON");
    for (name, value) in env {
        command.env(name, value);
    }
    command.output().expect("the renyi binary runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

/// A fresh directory under `target/python/`, the scratch space of these
/// tests.
fn scratch(name: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/python")
        .join(name);
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("a clean scratch directory");
    }
    std::fs::create_dir_all(&directory).expect("the scratch directory");
    directory
}

fn write(directory: &Path, name: &str, content: &str) {
    std::fs::write(directory.join(name), content).expect("the file is written");
}

/// The manifest: the module `helpers` bound to `helpers.py`, `mean`
/// renamed to `average`, and the interpreter when one is named.
fn manifest(interpreter: Option<&str>) -> String {
    let interpreter = interpreter
        .map(|name| format!("    \"interpreter\": \"{name}\",\n"))
        .unwrap_or_default();
    format!(
        "{{\n  \"name\": \"bridge_test\",\n  \"version\": \"0.1.0\",\n  \"python\": {{\n{interpreter}    \"modules\": {{\n      \"helpers\": {{\n        \"symbols\": {{\n          \"mean\": \"average\"\n        }}\n      }}\n    }}\n  }}\n}}\n"
    )
}

const HELPERS_PY: &str = "import sys


def average(values):
    return sum(values) / len(values)


def shout(text):
    print(\"noise from python\")
    return text.upper() + \"!\"


def tally(words):
    return {\"total\": len(words), \"longest\": max(words, key=len)}


def explode(message):
    raise KeyError(message)


def oddly():
    return [1, 2]


def quit(code):
    sys.exit(code)
";

const HELPERS_RY: &str = "module helpers
  purpose: The functions of `helpers.py`, declared for the bridge; `mean` is `average` in Python.

import std.python exposing PythonError

public type Tally
  purpose: What `tally` gives back: how many words and the longest.
  has total: Integer
  has longest: Text
  can FromJson
end

public function mean(values: List of Decimal)
  returns Decimal
  or fails with PythonError
  needs python(\"helpers\")
  purpose: The arithmetic mean.

public function shout(text: Text) returns Text or fails with PythonError needs python(\"helpers\")
  purpose: The text in capitals with a bang; prints a line on the way.

public function tally(words: List of Text)
  returns Tally
  or fails with PythonError
  needs python(\"helpers\")
  purpose: The words counted.

public function explode(message: Text) or fails with PythonError needs python(\"helpers\")
  purpose: Raises `KeyError`.

public function oddly() returns Integer or fails with PythonError needs python(\"helpers\")
  purpose: Gives back a list where an Integer is declared.

public function quit(code: Integer) or fails with PythonError needs python(\"helpers\")
  purpose: Ends the worker.
";

const CALLS_RY: &str = "module calls
  purpose: Call Python through the bridge.

import std.console
import std.python exposing PythonError
import helpers

function main() or fails with PythonError needs console, python(\"helpers\")
  purpose: Print what Python computes, and how an exception and a wrong result come back.

  let mean be helpers.mean([1.5, 2.5]) otherwise fail
  console.print(\"{mean}\")
  let shouted be helpers.shout(\"hello\") otherwise fail
  console.print(shouted)
  let tally be helpers.tally([\"a\", \"bbb\", \"cc\"]) otherwise fail
  console.print(\"{tally.total} {tally.longest}\")
  match helpers.explode(\"missing\")
    when failure(Raised(exception, message)) then console.print(\"{exception}: {message}\")
    otherwise console.print(\"no exception\")
  end
  match helpers.oddly()
    when failure(NotCarried(detail)) then console.print(\"not carried: {detail}\")
    otherwise console.print(\"carried\")
  end
end
";

const EXPECTED_HEAD: &str = "2.0\nHELLO!\n3 bbb\nKeyError: 'missing'\nnot carried: the result of `helpers.oddly` does not fit its declared type";

/// The project of the tests: the manifest, the Python module, its
/// declaration file and the program.
fn project(name: &str) -> PathBuf {
    let directory = scratch(name);
    write(&directory, "renyi.json", &manifest(None));
    write(&directory, "helpers.py", HELPERS_PY);
    write(&directory, "helpers.ry", HELPERS_RY);
    write(&directory, "calls.ry", CALLS_RY);
    directory
}

#[test]
fn a_program_calls_python_through_its_module() {
    let directory = project("calls");
    let output = renyi(&directory, &["run", "calls.ry"], &[]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(stdout.starts_with(EXPECTED_HEAD), "{stdout}");
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("this program can run Python through `helpers`"),
        "{stderr}"
    );
    // what Python prints goes to the standard error, not into the protocol
    assert!(stderr.contains("noise from python"), "{stderr}");
    // the bytecode file carries the bindings
    let output = renyi(
        &directory,
        &["compile", "calls.ry", "--to", "calls.ryc"],
        &[],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));
    let bytecode = std::fs::read_to_string(directory.join("calls.ryc")).expect("the bytecode file");
    assert!(
        bytecode.contains("\"symbol\": \"average\""),
        "the renamed symbol is in the file"
    );
    let output = renyi(&directory, &["run", "calls.ryc"], &[]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), stdout);
}

#[test]
fn a_recording_replays_and_reproduces_without_python() {
    let directory = project("replay");
    let output = renyi(
        &directory,
        &["record", "--to", "calls.rec", "calls.ry"],
        &[],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));
    let recorded = text(&output.stdout);
    assert!(recorded.starts_with(EXPECTED_HEAD), "{recorded}");
    let recording = std::fs::read_to_string(directory.join("calls.rec")).expect("the recording");
    assert!(
        recording.contains("helpers.mean"),
        "the Python call is recorded"
    );
    // an interpreter that cannot start proves that the replay calls nothing
    write(
        &directory,
        "renyi.json",
        &manifest(Some("renyi-no-such-python")),
    );
    let output = renyi(
        &directory,
        &["run", "--replay", "calls.rec", "calls.ry"],
        &[],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "", "a replay prints nothing");
    let output = renyi(&directory, &["reproduce", "calls.rec", "calls.ry"], &[]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), recorded);
}

#[test]
fn denying_python_refuses_the_program() {
    let directory = project("deny");
    let output = renyi(&directory, &["run", "--deny", "python", "calls.ry"], &[]);
    assert!(!output.status.success(), "the run is refused");
    assert!(
        text(&output.stderr).contains("python"),
        "{}",
        text(&output.stderr)
    );
}

#[test]
fn an_interpreter_that_does_not_answer_is_the_failure_unavailable() {
    // decision AL3: the manifest's interpreter, else the variable, else the PATH
    let directory = project("interpreter");
    write(
        &directory,
        "renyi.json",
        &manifest(Some("renyi-no-such-python")),
    );
    let output = renyi(&directory, &["run", "calls.ry"], &[]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "");
    let stderr = text(&output.stderr);
    assert!(stderr.contains("Unavailable"), "{stderr}");
    assert!(stderr.contains("`renyi-no-such-python`"), "{stderr}");
    write(&directory, "renyi.json", &manifest(None));
    let output = renyi(
        &directory,
        &["run", "calls.ry"],
        &[("RENYI_PYTHON", "renyi-no-such-python-either")],
    );
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stderr));
    let stderr = text(&output.stderr);
    assert!(stderr.contains("Unavailable"), "{stderr}");
    assert!(stderr.contains("`renyi-no-such-python-either`"), "{stderr}");
}

#[test]
fn a_worker_that_ends_is_reported_and_started_again() {
    let directory = project("ended");
    write(
        &directory,
        "quits.ry",
        "module quits
  purpose: End the worker from Python, then call again.

import std.console
import std.python exposing PythonError
import helpers

function main() or fails with PythonError needs console, python(\"helpers\")
  purpose: The call that ends the worker fails; the next one starts a worker again.

  match helpers.quit(3)
    when failure(Unavailable(detail)) then console.print(\"unavailable: {detail}\")
    otherwise console.print(\"still here\")
  end
  let shouted be helpers.shout(\"again\") otherwise fail
  console.print(shouted)
end
",
    );
    let output = renyi(&directory, &["run", "quits.ry"], &[]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        "unavailable: `helpers.quit`: the Python worker ended\nAGAIN!\n"
    );
}

#[test]
fn publish_refuses_a_project_with_python_modules() {
    let directory = project("publish");
    std::fs::create_dir_all(directory.join("registry")).expect("the registry directory");
    let output = renyi(&directory, &["publish", "--to", "registry"], &[]);
    assert!(!output.status.success(), "publishing is refused");
    let stderr = text(&output.stderr);
    assert!(stderr.contains("Python modules"), "{stderr}");
    assert!(stderr.contains("`helpers`"), "{stderr}");
}

/// A package for the binder (decisions AM1 and AM2): annotations the
/// mapping reaches and ones it does not, a default, a reserved word, a
/// keyword-only parameter, a private function.
const GEOMETRY_PY: &str = r#""""Shapes, for the binder."""

from typing import Sequence


def area(width: float, height: float = 1.0) -> float:
    """The area of a rectangle."""
    return width * height


def labels(names: Sequence[str], upper: bool) -> list[str]:
    return [name.upper() if upper else name for name in names]


def lookup(table: dict[str, int], key: str) -> int | None:
    return table.get(key)


def count(items: list) -> int:
    return len(items)


def weird(*, key: str) -> str:
    return key


def untyped(value, flag=False):
    return [value, flag]


def greet(x) -> None:
    print("hi", x)


def _private():
    return 1
"#;

const GEOMETRY_RY: &str = r#"module geometry
  purpose: Bindings generated by `renyi bind --python` from geometry.

import std.python exposing PythonError
import std.json exposing JsonValue

# skipped: weird(*, key: str) -> str (a keyword-only parameter without a default: `key`)

public function area(width: Float, height: Float)
  returns Float
  or fails with PythonError
  needs python("geometry")
  purpose: The area of a rectangle.

public function labels(names: List of Text, upper: Boolean)
  returns List of Text
  or fails with PythonError
  needs python("geometry")
  purpose: `labels(names: Sequence[str], upper: bool) -> list[str]`.

public function lookup(table: Map of Text to Integer, key: Text)
  returns maybe Integer
  or fails with PythonError
  needs python("geometry")
  purpose: `lookup(table: dict[str, int], key: str) -> int | None`.

public function py_count(items: List of JsonValue)
  returns Integer
  or fails with PythonError
  needs python("geometry")
  purpose: `count(items: list) -> int`.

# as JsonValue: `value` (no annotation), `flag` (no annotation), the result (no annotation)
public function untyped(value: JsonValue, flag: JsonValue)
  returns JsonValue
  or fails with PythonError
  needs python("geometry")
  purpose: `untyped(value, flag=False)`.

# as JsonValue: `x` (no annotation)
public function greet(argument_1: JsonValue) or fails with PythonError needs python("geometry")
  purpose: `greet(x) -> None`.
"#;

/// A package with `__all__`: a re-export is in, a class and a missing
/// name are not, a function it does not list is not.
const LISTED_PY: &str = r#""""A package that lists what it exports."""

from support import helper

__all__ = ["shown", "helper", "Thing", "MISSING"]


class Thing:
    pass


def shown(text: str) -> str:
    """The text shown."""
    return text


def hidden(text: str) -> str:
    return text
"#;

const SUPPORT_PY: &str = r#"def helper(value: int) -> int:
    """A helper another package re-exports."""
    return value * 2
"#;

const EXPORTS_RY: &str = r#"module exports
  purpose: Bindings generated by `renyi bind --python` from listed.

import std.python exposing PythonError

public function shown(text: Text) returns Text or fails with PythonError needs python("listed")
  purpose: The text shown.

public function helper(value: Integer)
  returns Integer
  or fails with PythonError
  needs python("listed")
  purpose: A helper another package re-exports.
"#;

const SHAPES_RY: &str = r#"module shapes
  purpose: Call the modules `renyi bind --python` wrote.

import std.console
import std.json exposing JsonValue
import std.python exposing PythonError
import geometry
import exports

function main() or fails with PythonError needs console, python("geometry"), python("listed")
  purpose: Print what the generated declarations carry.

  let area be geometry.area(width: 2.5, height: 4.0) otherwise fail
  console.print("{area}")
  let labels be geometry.labels(names: ["a", "b"], upper: true) otherwise fail
  console.print(labels.join(" "))
  let found be geometry.lookup(table: {"a": 1, "b": 2}, key: "b") otherwise fail
  let found_value be found otherwise 0
  console.print("{found_value}")
  let missing be geometry.lookup(table: {"a": 1}, key: "z") otherwise fail
  let missing_value be missing otherwise 0
  console.print("{missing_value}")
  let pair be geometry.untyped(value: JsonText(value: "x"), flag: JsonNull) otherwise fail
  console.print(json.render(pair))
  geometry.greet(JsonText(value: "there")) otherwise fail
  let helped be exports.helper(20) otherwise fail
  console.print("{helped}")
end
"#;

#[test]
fn bind_writes_a_module_from_a_python_package_and_the_manifest_entry() {
    let directory = scratch("bind");
    write(&directory, "geometry.py", GEOMETRY_PY);
    // without a manifest: the file, and the entry printed
    let output = renyi(&directory, &["bind", "--python", "geometry"], &[]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stderr = text(&output.stderr);
    assert!(stderr.contains("no renyi.json"), "{stderr}");
    assert!(
        stderr.contains(
            "\"python\": {\"modules\": {\"geometry\": {\"symbols\": {\"py_count\": \"count\"}}}}"
        ),
        "{stderr}"
    );
    let written = std::fs::read_to_string(directory.join("geometry.ry")).expect("the module");
    assert_eq!(written, GEOMETRY_RY);
    // with a manifest: the entries are written into it; `--module` names
    // the module otherwise than the package; `__all__` chooses
    write(
        &directory,
        "renyi.json",
        "{\n  \"name\": \"bound\",\n  \"version\": \"0.1.0\"\n}\n",
    );
    write(&directory, "support.py", SUPPORT_PY);
    write(&directory, "listed.py", LISTED_PY);
    let output = renyi(&directory, &["bind", "--python", "geometry"], &[]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let output = renyi(
        &directory,
        &["bind", "--python", "listed", "--module", "exports"],
        &[],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(
        text(&output.stderr).contains("the Python module `exports` of"),
        "{}",
        text(&output.stderr)
    );
    let manifest = std::fs::read_to_string(directory.join("renyi.json")).expect("the manifest");
    assert_eq!(
        manifest,
        "{
  \"name\": \"bound\",
  \"version\": \"0.1.0\",
  \"dependencies\": {},
  \"python\": {
    \"modules\": {
      \"exports\": {
        \"package\": \"listed\"
      },
      \"geometry\": {
        \"symbols\": {
          \"py_count\": \"count\"
        }
      }
    }
  }
}
"
    );
    let written = std::fs::read_to_string(directory.join("exports.ry")).expect("the module");
    assert_eq!(written, EXPORTS_RY);
    // the generated modules check clean and run through the bridge
    write(&directory, "shapes.ry", SHAPES_RY);
    let output = renyi(&directory, &["run", "shapes.ry"], &[]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "10.0\nA B\n2\n0\n[\"x\",null]\n40\n");
    let stderr = text(&output.stderr);
    assert!(stderr.contains("hi there"), "{stderr}");
    // what is refused: a package that does not import, a name that is
    // not an import name, a module name the package name cannot be
    let output = renyi(&directory, &["bind", "--python", "no_such_package"], &[]);
    assert!(!output.status.success());
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("cannot import `no_such_package`: ModuleNotFoundError"),
        "{stderr}"
    );
    let output = renyi(&directory, &["bind", "--python", "my-package"], &[]);
    assert!(!output.status.success());
    assert!(
        text(&output.stderr).contains("is not a Python import name"),
        "{}",
        text(&output.stderr)
    );
    let output = renyi(&directory, &["bind", "--python", "Geometry"], &[]);
    assert!(!output.status.success());
    assert!(
        text(&output.stderr).contains("is not a module name"),
        "{}",
        text(&output.stderr)
    );
    let output = renyi(
        &directory,
        &["bind", "header.h", "--python", "geometry"],
        &[],
    );
    assert!(!output.status.success());
    assert!(
        text(&output.stderr).contains("binds a package, not a header"),
        "{}",
        text(&output.stderr)
    );
}
