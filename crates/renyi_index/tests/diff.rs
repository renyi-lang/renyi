//! The semantic diff (design document 05, section 6; decision O4): origin
//! and reach of a change, renames, and the version bump of G1.

use renyi_index::{diff, index_files, Bump, Change, Entry, Header, Index};
use renyi_syntax::SourceFile;

fn header(revision: &str) -> Header {
    Header {
        project: "demo".to_string(),
        revision: revision.to_string(),
        toolchain: "renyi test".to_string(),
    }
}

fn program(revision: &str, source: &str) -> Index {
    index_files(&[SourceFile::new("demo.ry", source)], header(revision))
}

const BASE: &str = "module demo
  purpose: A caller and a helper.

public function caller(value: Integer) returns Integer
  purpose: One more than the helper.

  return helper(value) + 1
end

function helper(value: Integer) returns Integer
  return value + 1
end

test \"caller adds two\"
  check caller(1) is 3
end
";

fn only(diff: &renyi_index::Diff) -> &Entry {
    assert_eq!(diff.entries.len(), 1, "{:?}", diff.entries);
    &diff.entries[0]
}

#[test]
fn a_changed_body_is_reported_once_and_reaches_its_callers() {
    let old = program("one", BASE);
    let new = program("two", &BASE.replace("return value + 1", "return value + 2"));
    let result = diff(&old, &new);
    let entry = only(&result);
    assert_eq!(entry.name, "demo.helper");
    assert_eq!(entry.changes, vec![Change::Body]);
    assert_eq!(
        entry.reaches,
        vec!["demo.caller", "demo.test:caller adds two"]
    );
    assert!(!entry.public);
    assert_eq!(result.bump, Bump::None);
    let text = renyi_index::render_diff(&result);
    assert!(text.starts_with("compared demo at one with demo at two: 1 definition changed\n"));
    assert!(text.contains(
        "demo.helper (function): body changed; reaches demo.caller, demo.test:caller adds two\n"
    ));
    assert!(text.ends_with("version bump: none\n"));
}

#[test]
fn a_changed_public_signature_forces_a_major_bump() {
    let old = program("one", BASE);
    let changed = BASE
        .replace(
            "caller(value: Integer) returns Integer",
            "caller(value: Integer, extra: Integer) returns Integer",
        )
        .replace("caller(1)", "caller(1, extra: 0)");
    let new = program("two", &changed);
    let result = diff(&old, &new);
    let caller = result
        .entries
        .iter()
        .find(|entry| entry.name == "demo.caller")
        .expect("the caller changed");
    assert_eq!(
        caller.changes,
        vec![Change::Signature {
            old: "caller(value: Integer) returns Integer".to_string(),
            new: "caller(value: Integer, extra: Integer) returns Integer".to_string(),
        }]
    );
    assert_eq!(caller.reaches, vec!["demo.test:caller adds two"]);
    assert_eq!(result.bump, Bump::Major);
    let json = renyi_index::diff_json(&result);
    assert!(json.contains("\"bump\": \"major\""), "{json}");
    assert!(json.contains("\"change\": \"signature\""), "{json}");
}

#[test]
fn a_rename_keeps_identity_and_does_not_touch_the_callers() {
    let old = program("one", BASE);
    // the head and the call; not the word in the caller's purpose line
    let new = program("two", &BASE.replace("helper(", "assistant("));
    let result = diff(&old, &new);
    let entry = only(&result);
    assert_eq!(entry.name, "demo.assistant");
    assert_eq!(
        entry.reaches,
        vec!["demo.caller", "demo.test:caller adds two"]
    );
    assert_eq!(
        entry.changes,
        vec![Change::Renamed {
            from: "demo.helper".to_string(),
            was_public: false,
        }]
    );
    assert_eq!(result.bump, Bump::None);
}

#[test]
fn added_and_removed_definitions_set_the_bump() {
    let old = program("one", BASE);
    let added = format!(
        "{BASE}
public function other(value: Integer) returns Integer
  purpose: The value itself.

  return value
end
"
    );
    let result = diff(&old, &program("two", &added));
    let entry = only(&result);
    assert_eq!(
        (entry.name.as_str(), &entry.changes),
        ("demo.other", &vec![Change::Added])
    );
    assert_eq!(result.bump, Bump::Minor);

    let without_test = BASE
        .split("\ntest ")
        .next()
        .expect("the program before the test");
    let result = diff(&old, &program("two", without_test));
    let entry = only(&result);
    assert_eq!(entry.name, "demo.test:caller adds two");
    assert_eq!(entry.changes, vec![Change::Removed]);
    assert_eq!(result.bump, Bump::None, "a test is not public API");

    let backwards = diff(&program("two", without_test), &old);
    assert_eq!(only(&backwards).changes, vec![Change::Added]);
    assert_eq!(backwards.bump, Bump::None);
}

#[test]
fn an_effect_gained_below_widens_what_the_callers_reach() {
    let old = program("one", BASE);
    let widened = BASE
        .replace(
            "public function caller(value: Integer) returns Integer\n",
            "public function caller(value: Integer) returns Integer needs console\n",
        )
        .replace(
            "function helper(value: Integer) returns Integer\n  return value + 1\n",
            "function helper(value: Integer) returns Integer needs console\n  console.print(\"helping\")\n  return value + 1\n",
        )
        .replace(
            "  purpose: A caller and a helper.\n",
            "  purpose: A caller and a helper.\n\nimport std.console\n",
        );
    let new = program("two", &widened);
    let result = diff(&old, &new);
    let names: Vec<&str> = result
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["demo.caller", "demo.helper", "demo.test:caller adds two"]
    );
    let caller = &result.entries[0];
    assert!(
        caller.changes.contains(&Change::Effects {
            widened: vec!["console".to_string()],
            narrowed: Vec::new(),
        }),
        "{:?}",
        caller.changes
    );
    let test = &result.entries[2];
    assert_eq!(
        test.changes,
        vec![Change::Effects {
            widened: vec!["console".to_string()],
            narrowed: Vec::new(),
        }],
        "the test's own text did not change; it reaches the effect"
    );
    assert_eq!(
        result.bump,
        Bump::Major,
        "`needs` is part of the public signature"
    );
}
