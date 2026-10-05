//! The VM against the corpus: every Predict program of the readability
//! manifest prints its reference output, every `example:` line and `test`
//! block of every example passes, and a program's failure and crash are
//! reported as such.

use std::cell::RefCell;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;

use renyi_syntax::SourceFile;
use renyi_vm::natives::json::{read_json, Json};
use renyi_vm::{compile_project, Options, Program, RunOutcome, TestOutcome};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus() -> Vec<PathBuf> {
    let dir = root().join("examples");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("examples directory")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("ry"))
        .collect();
    files.sort();
    assert!(files.len() >= 30);
    files
}

fn compile(path: &PathBuf) -> Program {
    let text = std::fs::read_to_string(path).expect("read");
    let file = SourceFile::new(path.display().to_string(), text);
    let mut files = vec![file.clone()];
    files.extend(renyi_check::imported_files(&file));
    let checked = renyi_check::check_project(&files);
    for module in &checked.modules {
        let errors: Vec<_> = module.diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "{}: {errors:?}", path.display());
    }
    compile_project(&checked, &files)
}

/// A writer the test can read back after the run.
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

fn run(program: &Program, arguments: &[&str]) -> (RunOutcome, String, String) {
    let stdout = Capture::default();
    let stderr = Capture::default();
    let options = Options {
        arguments: arguments.iter().map(|a| a.to_string()).collect(),
        stdout: Box::new(stdout.clone()),
        stderr: Box::new(stderr.clone()),
        stdin: Box::new(std::io::Cursor::new(Vec::new())),
        grant: Vec::new(),
    };
    let outcome = renyi_vm::run_main(program, options);
    (outcome, stdout.text(), stderr.text())
}

#[test]
fn every_predict_program_prints_its_reference_output() {
    let manifest =
        std::fs::read_to_string(root().join("tests/readability/manifest.json")).expect("manifest");
    let Json::Object(top) = read_json(&manifest).expect("manifest JSON") else {
        panic!("the manifest is not an object");
    };
    let Some((_, Json::Object(programs))) = top.iter().find(|(key, _)| key == "programs") else {
        panic!("the manifest has no programs");
    };
    let mut checked = 0;
    for (name, entry) in programs {
        let Json::Object(fields) = entry else {
            continue;
        };
        let Some((_, Json::Object(predict))) = fields.iter().find(|(key, _)| key == "predict")
        else {
            continue;
        };
        let arguments: Vec<String> = match predict.iter().find(|(key, _)| key == "arguments") {
            Some((_, Json::Array(items))) => items
                .iter()
                .map(|item| match item {
                    Json::Text(text) => text.clone(),
                    other => panic!("argument {other:?}"),
                })
                .collect(),
            _ => Vec::new(),
        };
        let program = compile(&root().join(format!("examples/{name}.ry")));
        let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
        let (outcome, stdout, stderr) = run(&program, &arguments);
        assert_eq!(outcome, RunOutcome::Finished, "{name}: {stderr}");
        let expected =
            std::fs::read_to_string(root().join(format!("tests/readability/reference/{name}.out")))
                .expect("reference output");
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected.replace("\r\n", "\n"),
            "{name}"
        );
        checked += 1;
    }
    assert_eq!(checked, 10);
}

#[test]
fn every_example_line_and_test_block_of_the_corpus_passes() {
    let mut total = 0;
    let mut report = String::new();
    for path in corpus() {
        let program = compile(&path);
        let results = renyi_vm::run_tests(&program, Options::default());
        for result in &results.results {
            total += 1;
            if let TestOutcome::Failed(reason) = &result.outcome {
                report.push_str(&format!(
                    "{} ({}): {reason}\n",
                    result.name, result.location
                ));
            }
        }
    }
    assert!(report.is_empty(), "\n{report}");
    assert!(total >= 80, "only {total} examples and tests ran");
}

#[test]
fn a_failing_main_and_a_crash_are_reported() {
    let source = "module demo
  purpose: Fail on purpose.

import std.console

public type Broken
  purpose: Why the program stops.
  has reason: Text
end

public function main() or fails with Broken needs console
  purpose: Fail after printing.

  console.print(\"before\")
  fail with Broken(reason: \"on purpose\")
end
";
    let file = SourceFile::new("demo.ry", source);
    let files = vec![file];
    let checked = renyi_check::check_project(&files);
    assert!(
        checked.modules[0].diagnostics.is_empty(),
        "{:?}",
        checked.modules[0].diagnostics
    );
    let program = compile_project(&checked, &files);
    let (outcome, stdout, _) = run(&program, &[]);
    assert_eq!(stdout, "before\n");
    assert_eq!(
        outcome,
        RunOutcome::Failed("Broken(reason: \"on purpose\")".to_string())
    );

    let source = "module demo
  purpose: Crash on purpose.

import std.console

public function main() needs console
  purpose: Crash after printing.

  console.print(\"before\")
  crash with \"stop here\"
end
";
    let file = SourceFile::new("demo.ry", source);
    let files = vec![file];
    let checked = renyi_check::check_project(&files);
    assert!(
        checked.modules[0].diagnostics.is_empty(),
        "{:?}",
        checked.modules[0].diagnostics
    );
    let program = compile_project(&checked, &files);
    let (outcome, stdout, _) = run(&program, &[]);
    assert_eq!(stdout, "before\n");
    assert_eq!(
        outcome,
        RunOutcome::Crashed {
            message: "stop here".to_string(),
            location: Some("demo.ry:10".to_string())
        }
    );
}
