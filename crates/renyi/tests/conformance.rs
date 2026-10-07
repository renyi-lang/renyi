//! The conformance suite (`tests/conformance/manifest.json`, decision V8)
//! run against this binary, so that `cargo test` covers what the suite
//! promises; `tools/conformance.py` runs the same manifest against any
//! implementation. Every `run` case is run a second time from the bytecode
//! file `renyi compile` writes for its program (decision Z3).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

/// The binary's output on a case, with the program path given.
fn output_of(case: &[(String, Json)], program: &OsStr) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_renyi"));
    command
        .current_dir(root())
        .arg(text(field(case, "command")).expect("a command"))
        .args(texts(field(case, "options")))
        .arg(program)
        .args(texts(field(case, "arguments")));
    command.output().expect("the renyi binary runs")
}

/// What the binary got wrong on one case; empty when it passed.
fn problems_of(case: &[(String, Json)], index: usize) -> Vec<String> {
    let root = root();
    let program = text(field(case, "program")).expect("a program");
    let output = output_of(case, program.as_ref());
    let mut problems = problems_in(case, &output, &root);
    for problem in problems_from_bytecode(case, index) {
        problems.push(format!("from the bytecode file: {problem}"));
    }
    problems
}

/// A `run` case again from the bytecode file `renyi compile` writes for
/// its program (decision Z3): the file must print the same output and exit
/// the same way. A case about diagnostics is left out: they are reported
/// when the file is written, not when it runs.
fn problems_from_bytecode(case: &[(String, Json)], index: usize) -> Vec<String> {
    if text(field(case, "command")) != Some("run") || field(case, "diagnostics").is_some() {
        return Vec::new();
    }
    let root = root();
    let program = text(field(case, "program")).expect("a program");
    let directory = root.join("target/conformance");
    std::fs::create_dir_all(&directory).expect("the scratch directory");
    let file = directory.join(format!("case{index}.ryc"));
    let compiled = Command::new(env!("CARGO_BIN_EXE_renyi"))
        .current_dir(&root)
        .args(["compile", "--to"])
        .arg(&file)
        .arg(program)
        .output()
        .expect("the renyi binary runs");
    if !compiled.status.success() {
        return vec![format!(
            "`renyi compile` fails:\n{}{}",
            normalised(&compiled.stdout),
            normalised(&compiled.stderr)
        )];
    }
    let output = output_of(case, file.as_os_str());
    problems_in(case, &output, &root)
}

/// What is wrong with the binary's output on a case; empty when nothing.
fn problems_in(case: &[(String, Json)], output: &Output, root: &Path) -> Vec<String> {
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
    // every diagnostic suggests a fix on the line after it (decision D3)
    let all = format!("{stdout}{stderr}");
    let lines: Vec<&str> = all.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        let fixed = lines
            .get(index + 1)
            .is_some_and(|next| next.trim_start().starts_with("fix: "));
        if is_diagnostic(line) && !fixed {
            problems.push(format!("no fix after {line:?}"));
        }
    }
    problems
}

/// `file:line:column: severity [code]: message`
fn is_diagnostic(line: &str) -> bool {
    line.contains(": error [") || line.contains(": warning [")
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
    for (index, case) in cases.iter().enumerate() {
        let Json::Object(case) = case else {
            panic!("a case is not an object");
        };
        count += 1;
        let problems = problems_of(case, index);
        if !problems.is_empty() {
            let name = text(field(case, "name")).unwrap_or("?");
            report.push_str(&format!("{name}:\n  {}\n", problems.join("\n  ")));
        }
    }
    assert!(report.is_empty(), "{report}");
    assert!(count >= 19, "{count} cases");
}
