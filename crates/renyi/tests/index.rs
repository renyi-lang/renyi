//! `renyi index --diff`: the semantic diff against a saved map and against
//! a git revision (design document 05, section 6).

use std::path::PathBuf;
use std::process::Command;

fn renyi(arguments: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_renyi"))
        .args(arguments)
        .output()
        .expect("the renyi binary runs");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// A directory under Cargo's target directory for this test's files.
fn scratch(name: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&directory).expect("the scratch directory exists");
    directory
}

const BEFORE: &str = "module demo
  purpose: A caller and a helper.

public function caller(value: Integer) returns Integer
  purpose: One more than the helper.

  return helper(value) + 1
end

function helper(value: Integer) returns Integer
  return value + 1
end
";

#[test]
fn a_saved_map_is_the_base_of_a_diff() {
    let before = scratch("diff-before");
    let after = scratch("diff-after");
    std::fs::write(before.join("demo.ry"), BEFORE).expect("written");
    std::fs::write(
        after.join("demo.ry"),
        BEFORE.replace(
            "caller(value: Integer) returns Integer",
            "caller(value: Integer, extra: Integer) returns Integer",
        ),
    )
    .expect("written");
    let before_file = before.join("demo.ry").display().to_string();
    let after_file = after.join("demo.ry").display().to_string();
    let (ok, map, _) = renyi(&["index", "--json", &before_file]);
    assert!(ok);
    let map_file = before.join("map.json");
    std::fs::write(&map_file, map).expect("the map is saved");
    let map_file = map_file.display().to_string();

    let (ok, text, stderr) = renyi(&["index", "--diff", &map_file, &after_file]);
    assert!(ok, "{stderr}");
    assert!(text.contains(": 1 definition changed\n"), "{text}");
    assert!(
        text.contains("demo.caller (public function): signature `caller(value: Integer) returns Integer` -> `caller(value: Integer, extra: Integer) returns Integer`\n"),
        "{text}"
    );
    assert!(text.ends_with("version bump: major\n"), "{text}");

    let (ok, json, _) = renyi(&["index", "--diff", &map_file, "--json", &after_file]);
    assert!(ok);
    assert!(json.contains("\"bump\": \"major\""), "{json}");

    let (ok, same, _) = renyi(&["index", "--diff", &map_file, &before_file]);
    assert!(ok);
    assert!(same.contains(": no definition changed\n"), "{same}");
    assert!(same.ends_with("version bump: none\n"), "{same}");
}

#[test]
fn a_git_revision_is_the_base_of_a_diff() {
    let examples = format!("{}/../../examples", env!("CARGO_MANIFEST_DIR"));
    let (ok, text, stderr) = renyi(&["index", "--diff", "HEAD", &examples]);
    assert!(ok, "{stderr}");
    assert!(
        text.starts_with("compared examples at HEAD with examples at "),
        "{text}"
    );
    assert!(text.contains("version bump: "), "{text}");

    let hello = format!("{examples}/hello.ry");
    let (ok, text, stderr) = renyi(&["index", "--diff", "HEAD", &hello]);
    assert!(ok, "{stderr}");
    assert!(
        text.starts_with("compared hello at HEAD with hello at "),
        "{text}"
    );

    let (ok, _, stderr) = renyi(&["index", "--diff", "no-such-revision", &examples]);
    assert!(!ok);
    assert!(
        stderr.contains("`no-such-revision` is neither a map file nor a git revision"),
        "{stderr}"
    );
}
