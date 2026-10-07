//! The judges of the self-hosted front end. Decision W3: the parser written
//! in Renyi (`compiler/parse.ry`, run on the VM) must print, for every
//! program the Rust parser accepts, the document `renyi parse --json`
//! prints, byte for byte, and must reject every program the Rust parser
//! rejects; the inputs are the corpus, the conformance programs, the
//! compiler's own sources and, with `--declarations`, the library
//! declarations. Decision W7: the checker written in Renyi
//! (`compiler/checker.ry`) must print, for every program the Rust parser
//! accepts, what `renyi check --json` prints, with and without `--strict`,
//! and exit as it exits; the library declarations are not programs, so
//! they are left out. Decision Z3: the compiler written in Renyi
//! (`compiler/compile.ry`) must write, for every program `renyi compile`
//! accepts, the bytecode file it writes, byte for byte, and must refuse
//! every program it refuses.

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
fn programs_in(directory: &str) -> Vec<PathBuf> {
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

/// What the Renyi checker got wrong on one program, in one mode; `None`
/// when it agreed with the Rust checker. A program the Rust parser
/// rejects only has to fail.
fn judge_checker(program: &Path, parses: bool, strict: bool) -> Option<String> {
    let mut rust_args = vec!["check", "--json"];
    let mut self_args = vec!["run", "compiler/checker.ry", "--json"];
    if strict {
        rust_args.push("--strict");
        self_args.push("--strict");
    }
    let own = renyi(&self_args, program);
    let name = program
        .strip_prefix(root())
        .unwrap_or(program)
        .display()
        .to_string();
    let mode = if strict { " (--strict)" } else { "" };
    if !parses {
        if own.status.success() {
            return Some(format!(
                "{name}{mode}: the Rust parser rejects it, the Renyi checker accepts it"
            ));
        }
        return None;
    }
    let rust = renyi(&rust_args, program);
    if rust.status.code() != own.status.code() {
        return Some(format!(
            "{name}{mode}: `renyi check` exits with {:?}, the Renyi checker with {:?}:\n  {}",
            rust.status.code(),
            own.status.code(),
            String::from_utf8_lossy(&own.stderr).trim()
        ));
    }
    if rust.stdout != own.stdout {
        return Some(format!(
            "{name}{mode}: the diagnostics differ at {}",
            first_difference(&rust.stdout, &own.stdout)
        ));
    }
    None
}

/// Where one of the two compilers writes the file of a program, under the
/// workspace's `target/`.
fn bytecode_target(side: &str, name: &str) -> PathBuf {
    let directory = root().join("target").join("selfhost").join(side);
    std::fs::create_dir_all(&directory).expect("the target directory");
    directory.join(format!("{}.ryc", name.replace(['/', '.'], "_")))
}

/// What the Renyi compiler got wrong on one program, named relative to
/// the workspace root with `/`; `None` when it wrote what `renyi compile`
/// writes, or refused what it refuses.
fn judge_emitter(name: &str) -> Option<String> {
    let rust_target = bytecode_target("rust", name);
    let own_target = bytecode_target("self", name);
    let _ = std::fs::remove_file(&own_target);
    let rust_to = rust_target.to_string_lossy().to_string();
    let own_to = own_target.to_string_lossy().to_string();
    let rust = renyi(&["compile", "--to", &rust_to], Path::new(name));
    let own = renyi(
        &["run", "compiler/compile.ry", "--to", &own_to],
        Path::new(name),
    );
    if rust.status.success() {
        if !own.status.success() {
            return Some(format!(
                "{name}: `renyi compile` accepts it, the Renyi compiler fails:\n  {}",
                String::from_utf8_lossy(&own.stderr).trim()
            ));
        }
        let expected = std::fs::read(&rust_target).expect("the file `renyi compile` wrote");
        let got = std::fs::read(&own_target).unwrap_or_default();
        if expected != got {
            return Some(format!(
                "{name}: the two files differ at {}",
                first_difference(&expected, &got)
            ));
        }
    } else if own.status.success() {
        return Some(format!(
            "{name}: `renyi compile` rejects it, the Renyi compiler accepts it"
        ));
    }
    None
}

/// Judge every case on a few threads: each run of a Renyi program checks
/// and loads the whole front end first.
fn judge_all<Case: Sync>(
    cases: Vec<Case>,
    judge: impl Fn(&Case) -> Option<String> + Sync,
) -> Vec<String> {
    let workers = std::thread::available_parallelism().map_or(4, |count| count.get().min(8));
    let chunk = cases.len().div_ceil(workers).max(1);
    let judge = &judge;
    std::thread::scope(|scope| {
        let handles: Vec<_> = cases
            .chunks(chunk)
            .map(|chunk| {
                scope.spawn(move || chunk.iter().filter_map(judge).collect::<Vec<String>>())
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
        cases.extend(programs_in(directory).into_iter().map(|path| (path, false)));
    }
    cases.extend(
        programs_in("library/std")
            .into_iter()
            .map(|path| (path, true)),
    );
    assert!(cases.len() >= 70, "{} programs", cases.len());
    let problems = judge_all(cases, |(program, declarations)| {
        judge(program, *declarations)
    });
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn the_renyi_checker_prints_what_the_rust_checker_prints() {
    let mut programs: Vec<PathBuf> = Vec::new();
    for directory in ["examples", "tests/conformance/programs", "compiler"] {
        programs.extend(programs_in(directory));
    }
    assert!(programs.len() >= 70, "{} programs", programs.len());
    let problems = judge_all(programs, |program| {
        let parses = renyi(&["parse"], program).status.success();
        judge_checker(program, parses, false).or_else(|| judge_checker(program, parses, true))
    });
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn the_renyi_compiler_writes_what_renyi_compile_writes() {
    let mut programs: Vec<String> = Vec::new();
    for directory in ["examples", "tests/conformance/programs", "compiler"] {
        programs.extend(programs_in(directory).into_iter().map(|path| {
            let file = path.file_name().expect("a file name").to_string_lossy();
            format!("{directory}/{file}")
        }));
    }
    assert!(programs.len() >= 70, "{} programs", programs.len());
    let problems = judge_all(programs, |program| judge_emitter(program));
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn the_front_end_is_in_canonical_layout() {
    let output = Command::new(env!("CARGO_BIN_EXE_renyi"))
        .current_dir(root())
        .args(["format", "--check"])
        .args(programs_in("compiler"))
        .output()
        .expect("the renyi binary runs");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
