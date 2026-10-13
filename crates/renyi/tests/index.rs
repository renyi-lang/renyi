//! `renyi index --diff`: the semantic diff against a saved map and against
//! a git revision (design document 05, section 6); `renyi index --chunks`:
//! the chunks for retrieval by embedding as JSON Lines (section 8).

use std::path::PathBuf;
use std::process::Command;

use renyi_vm::natives::json::{read_json, Json};

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

fn get<'a>(json: &'a Json, path: &[&str]) -> &'a Json {
    let mut current = json;
    for key in path {
        current = match current {
            Json::Object(fields) => {
                &fields
                    .iter()
                    .find(|(name, _)| name == key)
                    .unwrap_or_else(|| panic!("no field `{key}` in {current:?}"))
                    .1
            }
            other => panic!("`{key}`: {other:?} has no fields"),
        };
    }
    current
}

fn text_of(json: &Json) -> &str {
    match json {
        Json::Text(text) => text,
        other => panic!("not text: {other:?}"),
    }
}

fn line_json(line: &str) -> Json {
    read_json(line).unwrap_or_else(|(detail, _)| panic!("not JSON: {detail}: {line}"))
}

#[test]
fn the_chunks_are_json_lines() {
    let directory = scratch("chunks");
    std::fs::write(directory.join("demo.ry"), BEFORE).expect("written");
    let file = directory.join("demo.ry").display().to_string();
    let (ok, output, stderr) = renyi(&["index", "--chunks", &file]);
    assert!(ok, "{stderr}");
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(
        lines.len(),
        4,
        "a header line, the module, two definitions:\n{output}"
    );
    let header = line_json(lines[0]);
    assert_eq!(text_of(get(&header, &["project"])), "demo");
    assert_eq!(get(&header, &["chunks"]), &Json::Number("3".to_string()));
    assert_eq!(get(&header, &["modules"]), &Json::Number("1".to_string()));
    assert_eq!(
        get(&header, &["definitions"]),
        &Json::Number("2".to_string())
    );
    assert_eq!(get(&header, &["sections"]), &Json::Number("0".to_string()));
    let module = line_json(lines[1]);
    assert_eq!(text_of(get(&module, &["metadata", "kind"])), "module");
    assert!(
        text_of(get(&module, &["text"]))
            .starts_with("module demo\npurpose: A caller and a helper.\n"),
        "{}",
        text_of(get(&module, &["text"]))
    );
    let caller = line_json(lines[2]);
    assert!(text_of(get(&caller, &["id"])).starts_with("sha256:"));
    assert_eq!(text_of(get(&caller, &["metadata", "name"])), "demo.caller");
    assert_eq!(text_of(get(&caller, &["metadata", "kind"])), "function");
    let text = text_of(get(&caller, &["text"]));
    assert!(
        text.starts_with("function demo.caller\ncaller(value: Integer) returns Integer\npurpose: One more than the helper.\nmodule demo: A caller and a helper.\n\npublic function caller"),
        "{text}"
    );
    assert!(
        text.ends_with("  return helper(value) + 1\nend\n"),
        "{text}"
    );
    assert_eq!(
        get(&caller, &["metadata", "calls"]),
        &Json::Array(vec![Json::Text("demo.helper".to_string())])
    );

    let (ok, _, stderr) = renyi(&["index", "--chunks", "--json", &file]);
    assert!(!ok);
    assert!(stderr.contains("--chunks"), "{stderr}");
}

#[test]
fn the_language_is_chunked_from_the_binary() {
    let (ok, output, stderr) = renyi(&["index", "--chunks", "--language"]);
    assert!(ok, "{stderr}");
    let header = line_json(output.lines().next().unwrap_or(""));
    assert_eq!(text_of(get(&header, &["project"])), "renyi");
    assert_eq!(
        text_of(get(&header, &["revision"])),
        env!("CARGO_PKG_VERSION")
    );
    assert!(
        output.contains("\"name\": \"cheatsheet: Effects\""),
        "a section of the cheat sheet"
    );
    assert!(
        output.contains("\"name\": \"reference: 11. Effects and capabilities / Meaning\""),
        "a subsection of the reference"
    );
    assert!(
        output.contains("\"name\": \"std.http.get\", \"kind\": \"function\""),
        "a declaration of the library"
    );
    let (ok, _, stderr) = renyi(&["index", "--language"]);
    assert!(!ok);
    assert!(stderr.contains("--language needs --chunks"), "{stderr}");
    let (ok, _, stderr) = renyi(&["index", "--chunks", "--language", "."]);
    assert!(!ok);
    assert!(stderr.contains("takes no path"), "{stderr}");
}
