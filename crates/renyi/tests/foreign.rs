//! The foreign function interface (decision AF1), driven through the
//! binary: a project whose manifest names foreign modules calls the C
//! library; a recording replays without the library; `--deny foreign`
//! refuses the call; a symbol or a library that is missing is a crash;
//! `renyi bind` writes a module from a header; `renyi publish` refuses a
//! project with native code.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn renyi(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_renyi"))
        .args(args)
        .current_dir(directory)
        .env("RENYI_FOREIGN_TEST", "present")
        .env_remove("RENYI_FOREIGN_UNSET")
        .output()
        .expect("the renyi binary runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

/// A fresh directory under `target/foreign/`, the scratch space of these
/// tests.
fn scratch(name: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/foreign")
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

const LIBC: &str = r#"["ucrtbase", "libc.so.6", "libSystem.B.dylib"]"#;
const LIBM: &str = r#"["ucrtbase", "libm.so.6", "libSystem.B.dylib"]"#;

fn manifest(libc: &str, libm: &str) -> String {
    format!(
        r#"{{
  "name": "ffi_test",
  "version": "0.1.0",
  "foreign": {{
    "libc": {{
      "library": {libc}
    }},
    "libm": {{
      "library": {libm},
      "symbols": {{
        "square_root": "sqrt"
      }}
    }}
  }}
}}
"#
    )
}

const LIBC_RY: &str = "module libc
  purpose: The C library, as the test binds it.

import std.foreign exposing Int32, Size

public function strlen(text: Text) returns Size needs foreign
  purpose: `size_t strlen(const char *s)`.

public function abs(value: Int32) returns Int32 needs foreign
  purpose: `int abs(int value)`.

public function atoi(text: Text) returns Int32 needs foreign
  purpose: `int atoi(const char *s)`.

public function getenv(name: Text) returns maybe Text needs foreign
  purpose: `char *getenv(const char *name)`.
";

const LIBM_RY: &str = "module libm
  purpose: The C mathematics library, as the test binds it: `square_root` is `sqrt`.

public function square_root(value: Float) returns Float needs foreign
  purpose: `double sqrt(double value)`.
";

const CALLS_RY: &str = "module calls
  purpose: Call the C library through the foreign function interface.

import std.console
import libc
import libm

function main() needs console, foreign
  purpose: Print what C computes.

  let length be libc.strlen(\"hello\")
  console.print(\"{length}\")
  let magnitude be libc.abs(-42)
  console.print(\"{magnitude}\")
  let parsed be libc.atoi(\"123abc\")
  console.print(\"{parsed}\")
  let root be libm.square_root(2.25)
  console.print(\"{root}\")
  match libc.getenv(\"RENYI_FOREIGN_TEST\")
    when some(found) then console.print(\"variable {found}\")
    when nothing then console.print(\"no variable\")
  end
  match libc.getenv(\"RENYI_FOREIGN_UNSET\")
    when some(other) then console.print(\"variable {other}\")
    when nothing then console.print(\"no variable\")
  end
end
";

const EXPECTED: &str = "5\n42\n123\n1.5\nvariable present\nno variable\n";

/// The project of the tests: the manifest with two foreign modules, their
/// declaration files and the program.
fn project(name: &str) -> PathBuf {
    let directory = scratch(name);
    write(&directory, "renyi.json", &manifest(LIBC, LIBM));
    write(&directory, "libc.ry", LIBC_RY);
    write(&directory, "libm.ry", LIBM_RY);
    write(&directory, "calls.ry", CALLS_RY);
    directory
}

#[test]
fn a_program_calls_the_c_library_through_its_foreign_modules() {
    let directory = project("calls");
    let output = renyi(&directory, &["run", "calls.ry"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), EXPECTED);
    assert!(
        text(&output.stderr).contains("this program can call native code through `libc`, `libm`"),
        "{}",
        text(&output.stderr)
    );
    // the bytecode file carries the bindings
    let output = renyi(&directory, &["compile", "calls.ry", "--to", "calls.ryc"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let bytecode = std::fs::read_to_string(directory.join("calls.ryc")).expect("the bytecode file");
    assert!(
        bytecode.contains("\"symbol\": \"sqrt\""),
        "the renamed symbol is in the file"
    );
    let output = renyi(&directory, &["run", "calls.ryc"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), EXPECTED);
}

#[test]
fn a_recording_replays_and_reproduces_without_the_library() {
    let directory = project("replay");
    let output = renyi(&directory, &["record", "--to", "calls.rec", "calls.ry"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), EXPECTED);
    let recording = std::fs::read_to_string(directory.join("calls.rec")).expect("the recording");
    assert!(
        recording.contains("libc.strlen"),
        "the foreign call is recorded"
    );
    // a library that cannot load proves that the replay calls nothing
    let bogus = r#"["renyi-no-such-library"]"#;
    write(&directory, "renyi.json", &manifest(bogus, bogus));
    let output = renyi(&directory, &["run", "--replay", "calls.rec", "calls.ry"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "", "a replay prints nothing");
    let output = renyi(&directory, &["reproduce", "calls.rec", "calls.ry"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), EXPECTED);
}

#[test]
fn denying_foreign_refuses_the_program() {
    let directory = project("deny");
    let output = renyi(&directory, &["run", "--deny", "foreign", "calls.ry"]);
    assert!(!output.status.success(), "the run is refused");
    assert!(
        text(&output.stderr).contains("foreign"),
        "{}",
        text(&output.stderr)
    );
}

#[test]
fn a_missing_symbol_is_a_crash_at_the_call() {
    let directory = project("symbol");
    write(
        &directory,
        "missing.ry",
        "module missing
  purpose: A symbol no C library has.

public function renyi_no_such_symbol() needs foreign
  purpose: `void renyi_no_such_symbol(void)`.
",
    );
    let text_manifest = manifest(LIBC, LIBM).replace(
        "    \"libm\": {",
        "    \"missing\": {\n      \"library\": [\"ucrtbase\", \"libc.so.6\", \"libSystem.B.dylib\"]\n    },\n    \"libm\": {",
    );
    write(&directory, "renyi.json", &text_manifest);
    write(
        &directory,
        "calls_missing.ry",
        "module calls_missing
  purpose: Call a symbol that is not there.

import std.console
import missing

function main() needs console, foreign
  purpose: The call before the crash is printed; the crash is reported.

  console.print(\"before\")
  missing.renyi_no_such_symbol()
  console.print(\"after\")
end
",
    );
    let output = renyi(&directory, &["run", "calls_missing.ry"]);
    assert!(!output.status.success(), "the run crashes");
    assert_eq!(text(&output.stdout), "before\n");
    assert!(
        text(&output.stderr).contains("the symbol `renyi_no_such_symbol` is not in the library"),
        "{}",
        text(&output.stderr)
    );
}

#[test]
fn a_library_that_cannot_load_is_a_crash_at_the_call() {
    let directory = project("library");
    let bogus = r#"["renyi-no-such-library", "renyi-no-such-library-either"]"#;
    write(&directory, "renyi.json", &manifest(bogus, LIBM));
    let output = renyi(&directory, &["run", "calls.ry"]);
    assert!(!output.status.success(), "the run crashes");
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("cannot load a library for the foreign module `libc`"),
        "{stderr}"
    );
    assert!(stderr.contains("`renyi-no-such-library`"), "{stderr}");
    assert!(
        stderr.contains("`renyi-no-such-library-either`"),
        "{stderr}"
    );
}

#[test]
fn bind_writes_a_module_from_a_header_and_the_manifest_entry() {
    let directory = scratch("bind");
    write(
        &directory,
        "header.h",
        "#include <stddef.h>
/* the length of a string */
size_t strlen(const char *s);
int abs(int value); // absolute
char *getenv(const char *name);
int printf(const char *format, ...);
double sqrt(double x);
",
    );
    let libraries = "ucrtbase,libc.so.6,libSystem.B.dylib";
    let output = renyi(
        &directory,
        &[
            "bind",
            "header.h",
            "--module",
            "cstring",
            "--library",
            libraries,
        ],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stderr = text(&output.stderr);
    assert!(stderr.contains("no renyi.json"), "{stderr}");
    assert!(
        stderr.contains(
            "\"cstring\": {\"library\": [\"ucrtbase\", \"libc.so.6\", \"libSystem.B.dylib\"]}"
        ),
        "{stderr}"
    );
    let written = std::fs::read_to_string(directory.join("cstring.ry")).expect("the module");
    assert_eq!(
        written,
        "module cstring
  purpose: Bindings generated by `renyi bind` from header.h.

import std.foreign exposing Int32, Size

# skipped: int printf(const char *format, ...) (variadic)

public function strlen(argument_1: Text) returns Size needs foreign
  purpose: `size_t strlen(const char *s)`.

public function abs(value: Int32) returns Int32 needs foreign
  purpose: `int abs(int value)`.

public function getenv(name: Text) returns maybe Text needs foreign
  purpose: `char *getenv(const char *name)`.

public function sqrt(argument_1: Float) returns Float needs foreign
  purpose: `double sqrt(double x)`.
"
    );
    // with a manifest, the entry is written into it, and the module checks
    write(
        &directory,
        "renyi.json",
        "{\n  \"name\": \"bound\",\n  \"version\": \"0.1.0\"\n}\n",
    );
    let output = renyi(
        &directory,
        &[
            "bind",
            "header.h",
            "--module",
            "cstring",
            "--library",
            libraries,
        ],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));
    let manifest = std::fs::read_to_string(directory.join("renyi.json")).expect("the manifest");
    assert_eq!(
        manifest,
        "{
  \"name\": \"bound\",
  \"version\": \"0.1.0\",
  \"dependencies\": {},
  \"foreign\": {
    \"cstring\": {
      \"library\": [
        \"ucrtbase\",
        \"libc.so.6\",
        \"libSystem.B.dylib\"
      ]
    }
  }
}
"
    );
    let output = renyi(&directory, &["check", "cstring.ry"]);
    assert!(
        output.status.success(),
        "{}{}",
        text(&output.stdout),
        text(&output.stderr)
    );
    assert_eq!(text(&output.stdout), "");
}

#[test]
fn publish_refuses_a_project_with_foreign_modules() {
    let directory = project("publish");
    std::fs::create_dir_all(directory.join("registry")).expect("the registry directory");
    let output = renyi(&directory, &["publish", "--to", "registry"]);
    assert!(!output.status.success(), "publishing is refused");
    let stderr = text(&output.stderr);
    assert!(stderr.contains("foreign"), "{stderr}");
    assert!(stderr.contains("`libc`"), "{stderr}");
}
