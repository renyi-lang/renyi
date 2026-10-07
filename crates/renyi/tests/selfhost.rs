//! The judges of the self-hosted front end. Decision W3: the parser written
//! in Renyi (`compiler/parse.ry`, run on the VM) must print, for every
//! program, the document `renyi parse --json` prints, byte for byte (the
//! tree, then the diagnostics when there are any, decision AD1), and exit
//! as it exits; the inputs are the corpus, the conformance programs, the
//! compiler's own sources and, with `--declarations`, the library
//! declarations. Decision W7: the checker written in Renyi
//! (`compiler/checker.ry`) must print, for every program, what `renyi
//! check --json` prints, with and without `--strict`, and exit as it
//! exits; the library declarations are not programs, so they are left
//! out. Decision Z3: the compiler written in Renyi (`compiler/compile.ry`)
//! must write, for every program `renyi compile` accepts, the bytecode
//! file it writes, byte for byte, and must refuse every program it
//! refuses, printing the diagnostics it prints.
//!
//! Each judge runs its driver from the bytecode file `renyi compile`
//! writes of it when the test starts (`target/selfhost/front/<driver>.ryc`,
//! loaded in place of the source by decision Z4), so that a run loads the
//! front end instead of checking its sources first; and the Renyi
//! compiler, run from that file, must write that file of itself.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf()
}

/// Where the programs the judges run on are: the corpus, the conformance
/// programs, the package fixture (decision AC1), the foreign fixtures (decision
/// AF1) and the compiler itself.
const PROGRAM_DIRECTORIES: [&str; 10] = [
    "examples",
    "tests/conformance/programs",
    "tests/conformance/packages/project",
    "tests/conformance/packages/stale",
    "tests/conformance/packages/unlocked",
    "tests/conformance/packages/broken",
    "tests/conformance/packages/registry/greeting/1.0.0",
    "tests/conformance/foreign",
    "tests/conformance/foreign_bad",
    "compiler",
];

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

/// Where a bytecode file is written under the workspace's `target/`: the
/// side is `rust` or `self` for the two compilers' files of a program,
/// `front` for the drivers of the judges.
fn bytecode_target(side: &str, name: &str) -> PathBuf {
    let directory = root().join("target").join("selfhost").join(side);
    std::fs::create_dir_all(&directory).expect("the target directory");
    directory.join(format!("{}.ryc", name.replace(['/', '.'], "_")))
}

/// The driver of a judge, `compiler/<driver>.ry`, compiled by `renyi
/// compile` to the file `renyi run` takes in place of the source; written
/// anew on every call, so that it is never older than the sources.
fn front_end(driver: &str) -> String {
    let target = bytecode_target("front", driver);
    let _ = std::fs::remove_file(&target);
    let to = target.to_string_lossy().to_string();
    let source = format!("compiler/{driver}.ry");
    let output = renyi(&["compile", "--to", &to], Path::new(&source));
    assert!(
        output.status.success(),
        "{source} does not compile:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    to
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

/// What the Renyi parser, run from `parser`, got wrong on one program;
/// `None` when it printed what the Rust parser printed, the diagnostics
/// of a rejected program included, and exited as it exited.
fn judge(program: &Path, declarations: bool, parser: &str) -> Option<String> {
    let mut rust_args = vec!["parse", "--json"];
    let mut self_args = vec!["run", parser];
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
    if rust.status.code() != own.status.code() {
        return Some(format!(
            "{name}: `renyi parse` exits with {:?}, the Renyi parser with {:?}:\n  {}",
            rust.status.code(),
            own.status.code(),
            String::from_utf8_lossy(&own.stderr).trim()
        ));
    }
    if rust.stdout != own.stdout {
        return Some(format!(
            "{name}: the two documents differ at {}",
            first_difference(&rust.stdout, &own.stdout)
        ));
    }
    None
}

/// What the Renyi checker, run from `checker`, got wrong on one program,
/// in one mode; `None` when it agreed with the Rust checker, on a program
/// with syntax errors too.
fn judge_checker(program: &Path, strict: bool, checker: &str) -> Option<String> {
    let mut rust_args = vec!["check", "--json"];
    let mut self_args = vec!["run", checker, "--json"];
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

/// What the Renyi compiler, run from `compiler`, got wrong on one program,
/// named relative to the workspace root with `/`; `None` when it wrote
/// what `renyi compile` writes, or refused what it refuses with the same
/// diagnostics on the standard output.
fn judge_emitter(name: &str, compiler: &str) -> Option<String> {
    let rust_target = bytecode_target("rust", name);
    let own_target = bytecode_target("self", name);
    let _ = std::fs::remove_file(&own_target);
    let rust_to = rust_target.to_string_lossy().to_string();
    let own_to = own_target.to_string_lossy().to_string();
    let rust = renyi(&["compile", "--to", &rust_to], Path::new(name));
    let own = renyi(&["run", compiler, "--to", &own_to], Path::new(name));
    if rust.status.code() != own.status.code() {
        return Some(format!(
            "{name}: `renyi compile` exits with {:?}, the Renyi compiler with {:?}:\n  {}",
            rust.status.code(),
            own.status.code(),
            String::from_utf8_lossy(&own.stderr).trim()
        ));
    }
    if rust.stdout != own.stdout {
        return Some(format!(
            "{name}: the diagnostics differ at {}",
            first_difference(&rust.stdout, &own.stdout)
        ));
    }
    if rust.status.success() {
        let expected = std::fs::read(&rust_target).expect("the file `renyi compile` wrote");
        let got = std::fs::read(&own_target).unwrap_or_default();
        if expected != got {
            return Some(format!(
                "{name}: the two files differ at {}",
                first_difference(&expected, &got)
            ));
        }
    }
    None
}

/// Judge every case on a few threads: each run of a Renyi program loads
/// the whole front end first.
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
    let parser = front_end("parse");
    let mut cases: Vec<(PathBuf, bool)> = Vec::new();
    for directory in PROGRAM_DIRECTORIES {
        cases.extend(programs_in(directory).into_iter().map(|path| (path, false)));
    }
    cases.extend(
        programs_in("library/std")
            .into_iter()
            .map(|path| (path, true)),
    );
    assert!(cases.len() >= 70, "{} programs", cases.len());
    let problems = judge_all(cases, |(program, declarations)| {
        judge(program, *declarations, &parser)
    });
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn the_renyi_checker_prints_what_the_rust_checker_prints() {
    let checker = front_end("checker");
    let mut programs: Vec<PathBuf> = Vec::new();
    for directory in PROGRAM_DIRECTORIES {
        programs.extend(programs_in(directory));
    }
    assert!(programs.len() >= 70, "{} programs", programs.len());
    let problems = judge_all(programs, |program| {
        judge_checker(program, false, &checker).or_else(|| judge_checker(program, true, &checker))
    });
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn the_renyi_compiler_writes_what_renyi_compile_writes() {
    let compiler = front_end("compile");
    let mut programs: Vec<String> = Vec::new();
    for directory in PROGRAM_DIRECTORIES {
        programs.extend(programs_in(directory).into_iter().map(|path| {
            let file = path.file_name().expect("a file name").to_string_lossy();
            format!("{directory}/{file}")
        }));
    }
    assert!(programs.len() >= 70, "{} programs", programs.len());
    let problems = judge_all(programs, |program| judge_emitter(program, &compiler));
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
    // The fixed point: the compiler, run from its bytecode, writes that
    // bytecode of itself.
    let written = std::fs::read(bytecode_target("self", "compiler/compile.ry"))
        .expect("the file the Renyi compiler wrote of itself");
    let ran_from = std::fs::read(&compiler).expect("the file the Renyi compiler ran from");
    assert!(
        written == ran_from,
        "the Renyi compiler writes a different file of itself than it ran from, at {}",
        first_difference(&ran_from, &written)
    );
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
