//! The C API (decision AW46, stage 40a) through its `extern "C"`
//! functions: a module loaded from a text with a grant, its description,
//! a record, a variant and a list across the JSON codec both ways, each
//! status reached once, an argument of the wrong shape refused with the
//! parameter named, a guard applied, two handles alive at once with their
//! texts, a load that fails, and the header held equal to the exports.

use std::collections::BTreeSet;
use std::path::PathBuf;

use renyi_c::*;

const SHAPES: &str = r#"module shapes
  purpose: A module for the test of the C API.

import std.console

type Point
  purpose: A point on the grid.
  has across: Integer
  has down: Integer
end

type Figure is one of
  purpose: A figure to describe.
  Dot(place: Point)
  Line(start: Point, finish: Point)
  Empty
end

type Oops
  purpose: A failure for the test.
  has value: Integer
end

public function shift(point: Point, step: Integer) returns Point
  purpose: Move the point along both axes.

  return Point(across: point.across + step, down: point.down + step)
end

public function describe(figure: Figure) returns Text
  purpose: Name the figure.

  match figure
    when Dot then return "a dot"
    when Line then return "a line"
    when Empty then return "nothing"
  end
end

public function tally(items: List of Integer) returns Integer
  purpose: Add the items.

  let mutable total be 0
  for each item in items
    change total to total + item
  end
  return total
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

  crash with "boom"
end

public function grow(size: Integer) returns Integer
  purpose: Build a list of size items and return its length.

  let items be for each index from 1 to size collect index
  return items.length()
end
"#;

fn load(grant: &str) -> *mut Handle {
    let name = "shapes.ry";
    unsafe {
        renyi_load_source(
            name.as_ptr(),
            name.len(),
            SHAPES.as_ptr(),
            SHAPES.len(),
            grant.as_ptr(),
            grant.len(),
        )
    }
}

fn text(handle: *const Handle) -> String {
    unsafe {
        let bytes = std::slice::from_raw_parts(renyi_text(handle), renyi_text_len(handle));
        String::from_utf8(bytes.to_vec()).expect("UTF-8")
    }
}

fn last_error() -> String {
    unsafe {
        let bytes = std::slice::from_raw_parts(renyi_last_error(), renyi_last_error_len());
        String::from_utf8(bytes.to_vec()).expect("UTF-8")
    }
}

fn call(handle: *mut Handle, name: &str, arguments: &str) -> (i32, String) {
    let status = unsafe {
        renyi_call(
            handle,
            name.as_ptr(),
            name.len(),
            arguments.as_ptr(),
            arguments.len(),
        )
    };
    (status, text(handle))
}

fn call_guarded(handle: *mut Handle, name: &str, arguments: &str, guards: &str) -> (i32, String) {
    let status = unsafe {
        renyi_call_guarded(
            handle,
            name.as_ptr(),
            name.len(),
            arguments.as_ptr(),
            arguments.len(),
            guards.as_ptr(),
            guards.len(),
        )
    };
    (status, text(handle))
}

#[test]
fn a_module_is_described_and_called_through_json() {
    let handle = load(r#"{"grant": "console", "memory": "8 megabytes"}"#);
    assert!(!handle.is_null(), "{}", last_error());
    unsafe { renyi_module(handle) };
    assert_eq!(text(handle), "shapes");
    unsafe { renyi_grant(handle) };
    assert_eq!(text(handle), "console");
    unsafe { renyi_warnings(handle) };
    assert_eq!(text(handle), "");
    unsafe { renyi_functions(handle) };
    let functions = text(handle);
    assert!(
        functions.starts_with(r#"[{"name":"shift","signature":"#),
        "{functions}"
    );
    assert!(functions.contains(r#""purpose":"Move the point along both axes.""#));
    assert!(functions.contains(r#""parameters":2"#));
    assert!(functions.contains(r#""needs":["console"]"#));
    assert!(functions.contains(r#""callable":true"#));
    assert!(!functions.contains(r#""callable":false"#));

    assert_eq!(
        call(handle, "shift", r#"[{"across": 1, "down": 2}, 3]"#),
        (OK, r#"{"across":4,"down":5}"#.to_string())
    );
    assert_eq!(
        call(
            handle,
            "describe",
            r#"[{"kind": "Dot", "place": {"across": 0, "down": 0}}]"#
        ),
        (OK, r#""a dot""#.to_string())
    );
    assert_eq!(
        call(
            handle,
            "describe",
            r#"[{"kind": "Line", "start": {"across": 0, "down": 0}, "finish": {"across": 1, "down": 1}}]"#
        ),
        (OK, r#""a line""#.to_string())
    );
    assert_eq!(
        call(handle, "describe", r#"[{"kind": "Empty"}]"#),
        (OK, r#""nothing""#.to_string())
    );
    assert_eq!(call(handle, "tally", "[[1, 2, 3]]"), (OK, "6".to_string()));
    assert_eq!(
        call(handle, "shout", r#"["hi"]"#),
        (OK, r#""HI""#.to_string())
    );

    let (status, failure) = call(handle, "risky", "[-1]");
    assert_eq!(status, FAILED);
    assert!(
        failure.contains("Oops") && failure.contains("-1"),
        "{failure}"
    );
    let (status, crash) = call(handle, "boom", "[]");
    assert_eq!(status, CRASHED);
    assert!(crash.contains("boom"), "{crash}");
    let (status, over) = call(handle, "grow", "[4000000]");
    assert_eq!(status, OVER_MEMORY, "{over}");
    assert_eq!(unsafe { renyi_memory_limit(handle) }, 8 << 20);
    assert!(unsafe { renyi_memory_used(handle) } > 8 << 20);
    assert!(over.contains("8 megabytes"), "{over}");
    assert_eq!(call(handle, "tally", "[[4]]"), (OK, "4".to_string()));
    assert_eq!(unsafe { renyi_memory_limit(handle) }, 0);
    unsafe { renyi_free_sandbox(handle) };
}

#[test]
fn a_call_is_refused_before_it_runs_with_the_reason() {
    let handle = load("");
    assert!(!handle.is_null(), "{}", last_error());
    let (status, reason) = call(handle, "shift", r#"["no", 3]"#);
    assert_eq!(status, REFUSED);
    assert!(
        reason.starts_with("argument 1 (`point`) does not decode as `Point`:"),
        "{reason}"
    );
    let (status, reason) = call(handle, "shift", r#"[{"across": 1, "down": "two"}, 3]"#);
    assert_eq!(status, REFUSED);
    assert!(reason.contains("down"), "{reason}");
    let (status, reason) = call(handle, "shift", r#"[{"across": 1, "down": 2}]"#);
    assert_eq!(
        (status, reason.as_str()),
        (REFUSED, "`shift` takes 2 arguments, not 1")
    );
    let (status, reason) = call(handle, "hidden", "[]");
    assert_eq!(
        (status, reason.as_str()),
        (REFUSED, "`shapes` has no public function `hidden`")
    );
    let (status, reason) = call(handle, "tally", "[[1, 2");
    assert_eq!(status, REFUSED);
    assert!(
        reason.starts_with("the arguments do not parse as JSON:"),
        "{reason}"
    );
    let (status, reason) = call(handle, "tally", r#"{"items": []}"#);
    assert_eq!(
        (status, reason.as_str()),
        (
            REFUSED,
            "the arguments are a JSON array, one item per parameter"
        )
    );
    let (status, reason) = call(handle, "shout", r#"["hi"]"#);
    assert_eq!(
        (status, reason.as_str()),
        (
            REFUSED,
            "`shout` needs `console`, which the grant does not cover"
        )
    );
    unsafe { renyi_free_sandbox(handle) };
}

#[test]
fn a_guard_names_a_capability_of_the_grant() {
    let handle = load(r#"{"grant": "console, filesystem.read(\"secrets\") only to console"}"#);
    assert!(!handle.is_null(), "{}", last_error());
    assert_eq!(
        call_guarded(
            handle,
            "shout",
            r#"["key"]"#,
            r#"["filesystem.read(\"secrets\")"]"#
        ),
        (OK, r#""KEY""#.to_string())
    );
    let (status, reason) = call_guarded(handle, "shout", r#"["key"]"#, r#"["network.http"]"#);
    assert_eq!(status, REFUSED);
    assert!(reason.contains("no `only to` guard"), "{reason}");
    let (status, reason) = call_guarded(handle, "shout", r#"["key"]"#, "[7]");
    assert_eq!(
        (status, reason.as_str()),
        (
            REFUSED,
            "a guard is a capability text or null, not a number"
        )
    );
    assert_eq!(
        call_guarded(handle, "shout", r#"["key"]"#, "[null]"),
        (OK, r#""KEY""#.to_string())
    );
    unsafe { renyi_free_sandbox(handle) };
}

#[test]
fn two_handles_keep_their_own_texts() {
    let first = load("");
    let second = load("");
    assert!(!first.is_null() && !second.is_null(), "{}", last_error());
    assert_eq!(call(first, "tally", "[[1]]"), (OK, "1".to_string()));
    let kept = unsafe { renyi_text(first) };
    assert_eq!(call(second, "tally", "[[2]]"), (OK, "2".to_string()));
    assert_eq!(unsafe { renyi_text(first) }, kept);
    assert_eq!(text(first), "1");
    unsafe {
        renyi_free_sandbox(first);
        renyi_free_sandbox(second);
        renyi_free_sandbox(std::ptr::null_mut());
    }
}

#[test]
fn a_load_that_fails_answers_null_with_the_reason() {
    assert!(load(r#"{"grant": "teleport"}"#).is_null());
    assert!(
        last_error().contains("not a capability"),
        "{}",
        last_error()
    );
    assert!(load(r#"{"grants": "console"}"#).is_null());
    assert!(
        last_error().contains("no field `grants`"),
        "{}",
        last_error()
    );
    let (name, bad) = ("bad.ry", "module bad\n  purpose: A module with an error.\n\npublic function f() returns Integer\n  purpose: Wrong.\n\n  return \"text\"\nend\n");
    let handle = unsafe {
        renyi_load_source(
            name.as_ptr(),
            name.len(),
            bad.as_ptr(),
            bad.len(),
            "".as_ptr(),
            0,
        )
    };
    assert!(handle.is_null());
    assert!(
        last_error().starts_with("the module has errors:"),
        "{}",
        last_error()
    );
    let missing = "nowhere/missing.ry";
    let handle = unsafe { renyi_load(missing.as_ptr(), missing.len(), "".as_ptr(), 0) };
    assert!(handle.is_null());
    assert!(last_error().contains("missing.ry"), "{}", last_error());
}

#[test]
fn the_version_and_the_api_version_are_told() {
    assert_eq!(renyi_api_version(), API_VERSION);
    let version = unsafe {
        String::from_utf8_lossy(std::slice::from_raw_parts(
            renyi_version(),
            renyi_version_len(),
        ))
        .into_owned()
    };
    assert_eq!(version, format!("renyi {}", env!("CARGO_PKG_VERSION")));
}

/// The cost of a call through the C API against `Sandbox::call` from
/// Rust (decision AW46's measure): `risky(5)` ten thousand times each
/// way, the difference being the JSON decode and encode per call.
/// `cargo test --release -p renyi_c -- --ignored --nocapture the_cost`.
#[test]
#[ignore]
fn the_cost_of_a_call_each_way() {
    use renyi::{Grant, Sandbox};
    use renyi_vm::Value;
    use std::time::Instant;
    const TURNS: u32 = 10_000;

    let started = Instant::now();
    let mut sandbox = Sandbox::load_source("shapes.ry", SHAPES, Grant::default()).expect("loads");
    let loaded = started.elapsed();
    let started = Instant::now();
    for _ in 0..TURNS {
        let value = sandbox
            .call("risky", vec![Value::integer(5)])
            .expect("answers");
        assert_eq!(value, Value::integer(5));
    }
    let rust = started.elapsed();

    let handle = load("");
    assert!(!handle.is_null(), "{}", last_error());
    let started = Instant::now();
    for _ in 0..TURNS {
        assert_eq!(call(handle, "risky", "[5]"), (OK, "5".to_string()));
    }
    let c_api = started.elapsed();
    unsafe { renyi_free_sandbox(handle) };
    println!(
        "load {:.1} ms; per call: Sandbox::call {:.1} us, renyi_call {:.1} us",
        loaded.as_secs_f64() * 1e3,
        rust.as_secs_f64() * 1e6 / f64::from(TURNS),
        c_api.as_secs_f64() * 1e6 / f64::from(TURNS)
    );
}

/// The functions the header declares, and the ones the library exports.
#[test]
fn the_header_declares_what_the_library_exports() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let header = std::fs::read_to_string(root.join("include/renyi.h")).expect("the header");
    let source = std::fs::read_to_string(root.join("src/lib.rs")).expect("the source");
    // every `renyi_<name>(` of the header: the functions, not the words of
    // its comments or the handle's type
    let mut declared = BTreeSet::new();
    for (start, _) in header.match_indices("renyi_") {
        let name: String = header[start..]
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();
        if header[start + name.len()..].trim_start().starts_with('(') {
            declared.insert(name);
        }
    }
    let exported: BTreeSet<String> = source
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("pub "))
        .filter_map(|line| {
            line.strip_prefix("unsafe extern \"C\" fn ")
                .or_else(|| line.strip_prefix("extern \"C\" fn "))
        })
        .map(|rest| rest.split('(').next().expect("a name").to_string())
        .collect();
    assert_eq!(declared, exported);
    for (status, name) in [
        (OK, "RENYI_OK 0"),
        (FAILED, "RENYI_FAILED 1"),
        (CRASHED, "RENYI_CRASHED 2"),
        (EXITED, "RENYI_EXITED 3"),
        (REFUSED, "RENYI_REFUSED 4"),
        (OVER_MEMORY, "RENYI_OVER_MEMORY 5"),
    ] {
        assert!(header.contains(name), "{name}");
        assert_eq!(
            status,
            name.rsplit(' ').next().unwrap().parse::<i32>().unwrap()
        );
    }
    assert!(header.contains(&format!("#define RENYI_API_VERSION {API_VERSION}")));
}
