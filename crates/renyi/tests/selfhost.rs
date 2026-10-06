//! The judge of the self-hosted front end (decision W3): the parser written
//! in Renyi (`compiler/parse.ry`, run on the VM) must print, for every
//! program the Rust parser accepts, the document `renyi parse --json`
//! prints, byte for byte, and must reject every program the Rust parser
//! rejects. The inputs are the corpus, the conformance programs, the
//! compiler's own sources and, with `--declarations`, the library
//! declarations.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf()
}

/// The `.ry` files of a directory, in name order.
fn programs(directory: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(root().join(directory))
        .unwrap_or_else(|error| panic!("{directory}: {error}"))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ry"))
        .collect();
    files.sort();
    files
}

fn renyi(args: &[&str], program: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_renyi"))
        .current_dir(root())
        .args(args)
        .arg(program)
        .output()
        .expect("the renyi binary runs")
}

/// The first line on which two documents differ, for the report.
fn first_difference(expected: &[u8], got: &[u8]) -> String {
    let expected = String::from_utf8_lossy(expected);
    let got = String::from_utf8_lossy(got);
    for (index, (left, right)) in expected.lines().zip(got.lines()).enumerate() {
        if left != right {
            return format!("line {}:\n    rust: {left}\n    self: {right}", index + 1);
        }
    }
    format!(
        "one document is a prefix of the other ({} and {} lines)",
        expected.lines().count(),
        got.lines().count()
    )
}

/// What the Renyi parser got wrong on one program; `None` when it agreed
/// with the Rust parser.
fn judge(program: &Path, declarations: bool) -> Option<String> {
    let mut rust_args = vec!["parse", "--json"];
    let mut self_args = vec!["run", "compiler/parse.ry"];
    if declarations {
        rust_args.push("--declarations");
        self_args.push("--declarations");
    }
    let rust = renyi(&rust_args, program);
    let own = renyi(&self_args, program);
    let name = program
        .strip_prefix(root())
        .unwrap_or(program)
        .display()
        .to_string();
    if rust.status.success() {
        if !own.status.success() {
            return Some(format!(
                "{name}: the Rust parser accepts it, the Renyi parser fails:\n  {}",
                String::from_utf8_lossy(&own.stderr).trim()
            ));
        }
        if rust.stdout != own.stdout {
            return Some(format!(
                "{name}: the two trees differ at {}",
                first_difference(&rust.stdout, &own.stdout)
            ));
        }
    } else if own.status.success() {
        return Some(format!(
            "{name}: the Rust parser rejects it, the Renyi parser accepts it"
        ));
    }
    None
}

/// Judge every program on a few threads: each run of the Renyi parser
/// checks and loads the whole front end first.
fn judge_all(cases: Vec<(PathBuf, bool)>) -> Vec<String> {
    let workers = std::thread::available_parallelism().map_or(4, |count| count.get().min(8));
    let chunk = cases.len().div_ceil(workers).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = cases
            .chunks(chunk)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .filter_map(|(program, declarations)| judge(program, *declarations))
                        .collect::<Vec<String>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("a judge thread"))
            .collect()
    })
}

#[test]
fn the_renyi_parser_prints_what_the_rust_parser_prints() {
    let mut cases: Vec<(PathBuf, bool)> = Vec::new();
    for directory in ["examples", "tests/conformance/programs", "compiler"] {
        cases.extend(programs(directory).into_iter().map(|path| (path, false)));
    }
    cases.extend(programs("library/std").into_iter().map(|path| (path, true)));
    assert!(cases.len() >= 70, "{} programs", cases.len());
    let problems = judge_all(cases);
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn the_front_end_is_in_canonical_layout() {
    let output = Command::new(env!("CARGO_BIN_EXE_renyi"))
        .current_dir(root())
        .args(["format", "--check"])
        .args(programs("compiler"))
        .output()
        .expect("the renyi binary runs");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
