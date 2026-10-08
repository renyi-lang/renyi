//! `renyi build` and the image on the command line (decision AS1): a
//! program built to a `.ryi` file runs, tests, records and reproduces
//! from it without compiling, the manifest's code hash is the bytecode's,
//! and an image that does not fit this machine or this `renyi` is refused
//! naming the fix.

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
    let directory = root().join("target/build-tests").join(name);
    std::fs::create_dir_all(&directory).expect("the scratch directory");
    directory
}

fn renyi_in(directory: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_renyi"))
        .current_dir(directory)
        .args(args)
        .output()
        .expect("the renyi binary runs")
}

fn renyi(args: &[&str]) -> Output {
    renyi_in(&root(), args)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn path(path: &std::path::Path) -> String {
    path.to_str().expect("a path").to_string()
}

#[test]
fn a_program_runs_records_and_reproduces_from_its_image() {
    let directory = scratch("hello");
    let file = path(&directory.join("hello.ryi"));
    let built = renyi(&["build", "--to", &file, "examples/hello.ry"]);
    assert!(built.status.success(), "{}", text(&built.stderr));
    let message = text(&built.stderr);
    assert!(
        message.starts_with(&format!("renyi: built examples/hello.ry to {file}: ")),
        "{message}"
    );
    assert!(
        message.contains("code objects as machine code"),
        "{message}"
    );
    let run = renyi(&["run", &file, "Renyi"]);
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "Hello, Renyi!\n");
    assert_eq!(
        text(&run.stdout),
        text(&renyi(&["run", "examples/hello.ry", "Renyi"]).stdout)
    );
    // nothing is compiled at run time: the image's code is loaded
    let report = Command::new(env!("CARGO_BIN_EXE_renyi"))
        .current_dir(root())
        .env("RENYI_NATIVE_REPORT", "1")
        .args(["run", &file, "Renyi"])
        .output()
        .expect("the renyi binary runs");
    let report = text(&report.stderr);
    assert!(
        report.contains("code objects loaded from the image; 0 code objects compiled"),
        "{report}"
    );
    // the manifest's code hash is the bytecode's, so a recording from the
    // image reproduces
    let recording = path(&directory.join("hello.recording.json"));
    let recorded = renyi(&["record", "--to", &recording, &file, "Renyi"]);
    assert!(recorded.status.success(), "{}", text(&recorded.stderr));
    let reproduced = renyi(&["reproduce", &recording, &file]);
    assert!(reproduced.status.success(), "{}", text(&reproduced.stderr));
    // the default name is beside the program's stem, in the current
    // directory, as `compile` does it
    let built = renyi_in(
        &directory,
        &["build", &path(&root().join("examples/hello.ry"))],
    );
    assert!(built.status.success(), "{}", text(&built.stderr));
    assert!(directory.join("hello.ryi").exists());
    // a bytecode file builds too
    let bytecode = path(&directory.join("hello.ryc"));
    let compiled = renyi(&["compile", "--to", &bytecode, "examples/hello.ry"]);
    assert!(compiled.status.success(), "{}", text(&compiled.stderr));
    let from_bytecode = path(&directory.join("from_bytecode.ryi"));
    let built = renyi(&["build", "--to", &from_bytecode, &bytecode]);
    assert!(built.status.success(), "{}", text(&built.stderr));
    let run = renyi(&["run", &from_bytecode, "Renyi"]);
    assert_eq!(text(&run.stdout), "Hello, Renyi!\n");
}

#[test]
fn tests_run_from_an_image_and_find_their_fixtures() {
    let directory = scratch("weather");
    let file = path(&directory.join("weather.ryi"));
    let built = renyi(&["build", "--to", &file, "examples/weather.ry"]);
    assert!(built.status.success(), "{}", text(&built.stderr));
    let from_image = renyi(&["test", &file]);
    let from_source = renyi(&["test", "examples/weather.ry"]);
    assert!(from_image.status.success(), "{}", text(&from_image.stdout));
    assert_eq!(text(&from_image.stdout), text(&from_source.stdout));
}

#[test]
fn an_image_that_does_not_fit_is_refused_naming_the_fix() {
    let directory = scratch("refused");
    let file = path(&directory.join("primes.ryi"));
    let built = renyi(&["build", "--to", &file, "bench/primes.ry"]);
    assert!(built.status.success(), "{}", text(&built.stderr));
    let bytes = std::fs::read(&file).expect("the image");
    // another target: the triple's first letter changed in the header
    let target = bytes
        .windows(8)
        .position(|window| window == b"-unknown" || window == b"-pc-wind" || window == b"-apple-d")
        .expect("the target triple in the header");
    let mut other = bytes.clone();
    let start = other[..target]
        .iter()
        .rposition(|byte| !byte.is_ascii_alphanumeric() && *byte != b'_')
        .map(|at| at + 1)
        .unwrap_or(0);
    other[start] = if other[start] == b'z' { b'y' } else { b'z' };
    let other_file = path(&directory.join("other.ryi"));
    std::fs::write(&other_file, &other).expect("the other image");
    let run = renyi(&["run", &other_file]);
    assert_eq!(run.status.code(), Some(1));
    let message = text(&run.stderr);
    assert!(
        message.contains("the image was built by renyi")
            && message.contains("run `renyi build` again on this machine"),
        "{message}"
    );
    // a truncated file
    let truncated = path(&directory.join("truncated.ryi"));
    std::fs::write(&truncated, &bytes[..bytes.len() / 2]).expect("the truncated image");
    let run = renyi(&["run", &truncated]);
    assert_eq!(run.status.code(), Some(1));
    assert!(
        text(&run.stderr).contains("truncated"),
        "{}",
        text(&run.stderr)
    );
    // not an image at all
    let not_an_image = path(&directory.join("text.ryi"));
    std::fs::write(&not_an_image, b"hello").expect("the text");
    let run = renyi(&["run", &not_an_image]);
    assert_eq!(run.status.code(), Some(1));
    assert!(
        text(&run.stderr).contains("not an image file"),
        "{}",
        text(&run.stderr)
    );
    // `build` takes a program, not an image; `--opt` takes two levels
    let again = renyi(&["build", &file]);
    assert_eq!(again.status.code(), Some(1));
    assert!(
        text(&again.stderr).contains("is an image already"),
        "{}",
        text(&again.stderr)
    );
    let bogus = renyi(&["build", "--opt", "fast", "bench/primes.ry"]);
    assert_eq!(bogus.status.code(), Some(1));
    assert!(
        text(&bogus.stderr).contains("`--opt` takes `none` or `speed`"),
        "{}",
        text(&bogus.stderr)
    );
    // the image runs the program as the source does, at both levels
    let run = renyi(&["run", &file]);
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert_eq!(
        text(&run.stdout),
        text(&renyi(&["run", "bench/primes.ry"]).stdout)
    );
    let fast = path(&directory.join("primes_speed.ryi"));
    let built = renyi(&["build", "--opt", "speed", "--to", &fast, "bench/primes.ry"]);
    assert!(built.status.success(), "{}", text(&built.stderr));
    let run = renyi(&["run", &fast]);
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert_eq!(
        text(&run.stdout),
        text(&renyi(&["run", "bench/primes.ry"]).stdout)
    );
}
