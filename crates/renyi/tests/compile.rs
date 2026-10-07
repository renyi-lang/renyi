//! `renyi compile` and the bytecode file on the command line (decision
//! Z4): a program compiled to a `.ryc` file runs, records and reproduces
//! from it, the manifest's code hash is the file's, and a file that does
//! not fit is refused naming the place.

use std::path::PathBuf;
use std::process::{Command, Output};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf()
}

/// A scratch directory under the workspace's `target`.
fn scratch(name: &str) -> PathBuf {
    let directory = root().join("target/compile-tests").join(name);
    std::fs::create_dir_all(&directory).expect("the scratch directory");
    directory
}

fn renyi(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_renyi"))
        .current_dir(root())
        .args(args)
        .output()
        .expect("the renyi binary runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn path(path: &std::path::Path) -> String {
    path.to_str().expect("a path").to_string()
}

#[test]
fn a_program_runs_records_and_reproduces_from_its_bytecode_file() {
    let directory = scratch("hello");
    let file = path(&directory.join("hello.ryc"));
    let compiled = renyi(&["compile", "--to", &file, "examples/hello.ry"]);
    assert!(compiled.status.success(), "{}", text(&compiled.stderr));
    assert_eq!(
        text(&compiled.stderr),
        format!("renyi: compiled examples/hello.ry to {file}\n")
    );
    let written = std::fs::read_to_string(&file).expect("the file");
    assert!(
        written.starts_with("{\n  \"format\": 1,\n  \"modules\": [\n"),
        "{}",
        &written[..60]
    );
    let run = renyi(&["run", &file, "Renyi"]);
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "Hello, Renyi!\n");
    assert_eq!(
        text(&run.stdout),
        text(&renyi(&["run", "examples/hello.ry", "Renyi"]).stdout)
    );
    // the manifest's code hash is the file's (decision Z4)
    let manifest = renyi(&["run", "--manifest", &file, "Renyi"]);
    let hash = renyi_vm::recording::sha256_of(written.as_bytes());
    assert!(
        text(&manifest.stderr).contains(&format!("\"code\": \"{hash}\"")),
        "{}",
        text(&manifest.stderr)
    );
    // a recording made from the file reproduces from it
    let recording = path(&directory.join("hello.recording.json"));
    let recorded = renyi(&["record", "--to", &recording, &file, "Renyi"]);
    assert!(recorded.status.success(), "{}", text(&recorded.stderr));
    let reproduced = renyi(&["reproduce", &recording]);
    assert!(reproduced.status.success(), "{}", text(&reproduced.stderr));
    assert!(
        text(&reproduced.stderr).contains("the outcome and the output are the recorded ones"),
        "{}",
        text(&reproduced.stderr)
    );
    // the default target is the stem in the working directory
    let default = root().join("hello.ryc");
    let _ = std::fs::remove_file(&default);
    let compiled = renyi(&["compile", "examples/hello.ry"]);
    assert!(compiled.status.success(), "{}", text(&compiled.stderr));
    assert!(default.exists());
    std::fs::remove_file(&default).expect("the default target");
}

#[test]
fn tests_run_from_a_bytecode_file_and_find_their_fixtures() {
    let directory = scratch("tests");
    let file = path(&directory.join("weather.ryc"));
    let compiled = renyi(&["compile", "--to", &file, "examples/weather.ry"]);
    assert!(compiled.status.success(), "{}", text(&compiled.stderr));
    let from_file = renyi(&["test", &file]);
    let from_source = renyi(&["test", "examples/weather.ry"]);
    assert!(from_file.status.success(), "{}", text(&from_file.stdout));
    assert_eq!(text(&from_file.stdout), text(&from_source.stdout));
}

#[test]
fn a_file_that_does_not_fit_is_refused() {
    let directory = scratch("bad");
    let wrong_format = path(&directory.join("format.ryc"));
    std::fs::write(&wrong_format, "{\n  \"format\": 2\n}\n").expect("the file");
    let run = renyi(&["run", &wrong_format]);
    assert_eq!(run.status.code(), Some(1));
    assert!(
        text(&run.stderr).contains("is format 2; this VM reads format 1"),
        "{}",
        text(&run.stderr)
    );
    let not_json = path(&directory.join("broken.ryc"));
    std::fs::write(&not_json, "{\n  \"format\": 1,\n").expect("the file");
    let run = renyi(&["run", &not_json]);
    assert_eq!(run.status.code(), Some(1));
    assert!(
        text(&run.stderr).starts_with(&format!("renyi: {not_json}: line ")),
        "{}",
        text(&run.stderr)
    );
    // `compile` takes a source file, and a program with errors compiles to
    // nothing
    let compiled = renyi(&["compile", &wrong_format]);
    assert_eq!(compiled.status.code(), Some(1));
    assert!(
        text(&compiled.stderr).contains("is a bytecode file already"),
        "{}",
        text(&compiled.stderr)
    );
    let target = path(&directory.join("range_loop.ryc"));
    let failed = renyi(&[
        "compile",
        "--to",
        &target,
        "tests/conformance/programs/range_loop.ry",
    ]);
    assert_eq!(failed.status.code(), Some(1));
    assert!(
        text(&failed.stdout).contains("[range-loop]"),
        "{}",
        text(&failed.stdout)
    );
    assert!(!directory.join("range_loop.ryc").exists());
}
