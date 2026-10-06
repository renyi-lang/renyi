//! The conformance suite (`tests/conformance/manifest.json`, decision V8)
//! run against this binary, so that `cargo test` covers what the suite
//! promises; `tools/conformance.py` runs the same manifest against any
//! implementation.

use std::path::PathBuf;
use std::process::Command;

use renyi_vm::natives::json::{read_json, Json};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf()
}

fn field<'a>(object: &'a [(String, Json)], name: &str) -> Option<&'a Json> {
    object
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

fn text(json: Option<&Json>) -> Option<&str> {
    match json {
        Some(Json::Text(text)) => Some(text),
        _ => None,
    }
}

fn texts(json: Option<&Json>) -> Vec<String> {
    match json {
        Some(Json::Array(items)) => items
            .iter()
            .filter_map(|item| match item {
                Json::Text(text) => Some(text.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn number(json: Option<&Json>) -> Option<i32> {
    match json {
        Some(Json::Number(text)) => text.parse().ok(),
        _ => None,
    }
}

fn normalised(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

/// What the binary got wrong on one case; empty when it passed.
fn problems_of(case: &[(String, Json)]) -> Vec<String> {
    let root = root();
    let program = text(field(case, "program")).expect("a program");
    let mut command = Command::new(env!("CARGO_BIN_EXE_renyi"));
    command
        .current_dir(&root)
        .arg(text(field(case, "command")).expect("a command"))
        .args(texts(field(case, "options")))
        .arg(program)
        .args(texts(field(case, "arguments")));
    let output = command.output().expect("the renyi binary runs");
    let stdout = normalised(&output.stdout);
    let stderr = normalised(&output.stderr);
    let mut problems = Vec::new();
    let expected_code = number(field(case, "exit_code")).unwrap_or(0);
    if output.status.code() != Some(expected_code) {
        problems.push(format!(
            "exit code {:?}, expected {expected_code}",
            output.status.code()
        ));
    }
    if let Some(path) = text(field(case, "stdout")) {
        let expected = std::fs::read_to_string(root.join(path))
            .unwrap_or_else(|error| panic!("{path}: {error}"))
            .replace("\r\n", "\n");
        if stdout != expected {
            problems.push(format!(
                "standard output differs:\n--- got\n{stdout}--- expected\n{expected}"
            ));
        }
    }
    for code in texts(field(case, "diagnostics")) {
        if !stdout.contains(&format!("[{code}]")) && !stderr.contains(&format!("[{code}]")) {
            problems.push(format!(
                "no diagnostic [{code}] reported:\n{stdout}{stderr}"
            ));
        }
    }
    for wanted in texts(field(case, "stderr_contains")) {
        if !stderr.contains(&wanted) {
            problems.push(format!("standard error lacks {wanted:?}:\n{stderr}"));
        }
    }
    problems
}

#[test]
fn the_binary_passes_the_conformance_suite() {
    let manifest = std::fs::read_to_string(root().join("tests/conformance/manifest.json"))
        .expect("the manifest");
    let Json::Object(top) = read_json(&manifest).expect("manifest JSON") else {
        panic!("the manifest is not an object");
    };
    let Some(Json::Array(cases)) = field(&top, "cases") else {
        panic!("the manifest has no cases");
    };
    let mut report = String::new();
    let mut count = 0;
    for case in cases {
        let Json::Object(case) = case else {
            panic!("a case is not an object");
        };
        count += 1;
        let problems = problems_of(case);
        if !problems.is_empty() {
            let name = text(field(case, "name")).unwrap_or("?");
            report.push_str(&format!("{name}:\n  {}\n", problems.join("\n  ")));
        }
    }
    assert!(report.is_empty(), "{report}");
    assert!(count >= 19, "{count} cases");
}
