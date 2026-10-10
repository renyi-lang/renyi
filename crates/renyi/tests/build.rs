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
        report.contains("code objects loaded from the image (the section mapped from the file); 0 code objects compiled"),
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

#[test]
fn a_self_contained_executable_runs_the_program_with_its_command_line() {
    let directory = scratch("exe");
    let file = path(&directory.join(format!("greet{}", std::env::consts::EXE_SUFFIX)));
    let built = renyi(&["build", "--exe", "--to", &file, "examples/hello.ry"]);
    assert!(built.status.success(), "{}", text(&built.stderr));
    let message = text(&built.stderr);
    assert!(
        message.starts_with(&format!(
            "renyi: built examples/hello.ry to {file}: a self-contained executable, "
        )),
        "{message}"
    );
    // the whole command line is the program's arguments: no subcommand
    // of renyi is read from it
    let run = Command::new(&file)
        .current_dir(&directory)
        .args(["build"])
        .output()
        .expect("the executable runs");
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "Hello, build!\n");
    // the image inside is what runs: nothing is compiled
    let report = Command::new(&file)
        .current_dir(&directory)
        .env("RENYI_NATIVE_REPORT", "1")
        .args(["Renyi"])
        .output()
        .expect("the executable runs");
    assert_eq!(text(&report.stdout), "Hello, Renyi!\n");
    let report = text(&report.stderr);
    assert!(
        report.contains("code objects loaded from the image (the section mapped from the file); 0 code objects compiled"),
        "{report}"
    );
    // the executable is this binary with the image and a trailer after it
    let bytes = std::fs::read(&file).expect("the executable");
    let own = std::fs::read(env!("CARGO_BIN_EXE_renyi")).expect("this binary");
    assert!(bytes.starts_with(&own));
    assert!(bytes.ends_with(b"RENYIEXE"));
    // the default name is the program's stem, in the current directory
    let built = renyi_in(
        &directory,
        &["build", "--exe", &path(&root().join("examples/hello.ry"))],
    );
    assert!(built.status.success(), "{}", text(&built.stderr));
    assert!(directory
        .join(format!("hello{}", std::env::consts::EXE_SUFFIX))
        .exists());
}

#[test]
fn the_image_carries_the_program_in_binary_with_the_bytecode_hash() {
    let directory = scratch("binary");
    // the compiler's own program, every kind of op in it, round-trips
    // through the encoding and renders as its bytecode file does
    let checker = path(&directory.join("checker.ryc"));
    let compiled = renyi(&["compile", "--to", &checker, "compiler/checker.ry"]);
    assert!(compiled.status.success(), "{}", text(&compiled.stderr));
    let rendered = std::fs::read_to_string(&checker).expect("the bytecode file");
    let program = renyi_vm::file::load(&rendered).expect("the program");
    let encoded = renyi_vm::binary::encode(&program);
    assert!(
        encoded.len() * 4 < rendered.len(),
        "{} bytes encoded against {} of JSON",
        encoded.len(),
        rendered.len()
    );
    let decoded = renyi_vm::binary::decode(&encoded).expect("the program decodes");
    assert_eq!(renyi_vm::file::render(&decoded), rendered);
    // an image holds the encoding and the bytecode file's hash, so that a
    // recording made from either reproduces against the other
    let bytecode = path(&directory.join("hello.ryc"));
    let image_file = path(&directory.join("hello.ryi"));
    let compiled = renyi(&["compile", "--to", &bytecode, "examples/hello.ry"]);
    assert!(compiled.status.success(), "{}", text(&compiled.stderr));
    let built = renyi(&["build", "--to", &image_file, "examples/hello.ry"]);
    assert!(built.status.success(), "{}", text(&built.stderr));
    let image =
        renyi_vm::native::image::Image::read(&std::fs::read(&image_file).expect("the image"))
            .expect("an image");
    let rendered = std::fs::read_to_string(&bytecode).expect("the bytecode file");
    assert_eq!(
        image.code_hash,
        renyi_vm::recording::sha256_of(rendered.as_bytes())
    );
    let program = renyi_vm::binary::decode(&image.program).expect("the image's program");
    assert_eq!(renyi_vm::file::render(&program), rendered);
    // the code section lies at the alignment that maps from the file,
    // and the file reads back as it was written
    assert_eq!(
        image.section_offset % renyi_vm::native::image::SECTION_ALIGN,
        0
    );
    let bytes = std::fs::read(&image_file).expect("the image");
    assert_eq!(image.write(), bytes);
    let recording = path(&directory.join("hello.recording.json"));
    let recorded = renyi(&["record", "--to", &recording, &bytecode, "Renyi"]);
    assert!(recorded.status.success(), "{}", text(&recorded.stderr));
    let reproduced = renyi(&["reproduce", &recording, &image_file]);
    assert!(reproduced.status.success(), "{}", text(&reproduced.stderr));
    let recorded = renyi(&["record", "--to", &recording, &image_file, "Renyi"]);
    assert!(recorded.status.success(), "{}", text(&recorded.stderr));
    let reproduced = renyi(&["reproduce", &recording, &bytecode]);
    assert!(reproduced.status.success(), "{}", text(&reproduced.stderr));
}

/// The binary with the image cache on (decision AU10; `.cargo/config.toml`
/// turns it off for the tests) in a directory of the test's own.
fn renyi_cached(cache: &std::path::Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_renyi"));
    command
        .current_dir(root())
        .env_remove("RENYI_NO_CACHE")
        .env("RENYI_CACHE_DIR", cache)
        .args(args);
    for (name, value) in env {
        command.env(name, value);
    }
    command.output().expect("the renyi binary runs")
}

/// The files of the cache's directory with the extension.
fn cached_files(cache: &std::path::Path, extension: &str) -> Vec<PathBuf> {
    std::fs::read_dir(cache)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|e| e == extension))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_run_that_compiled_machine_code_leaves_its_image_for_the_next() {
    let cache = scratch("cache");
    let _ = std::fs::remove_dir_all(&cache);
    // every code object compiled at its first call, so that the run
    // compiles machine code and leaves the image, built in the background
    let first = renyi_cached(
        &cache,
        &["run", "examples/hello.ry", "Renyi"],
        &[("RENYI_NATIVE_HOT", "0")],
    );
    assert!(first.status.success(), "{}", text(&first.stderr));
    assert_eq!(text(&first.stdout), "Hello, Renyi!\n");
    let mut waited = 0;
    while (cached_files(&cache, "ryi").is_empty() || !cached_files(&cache, "lock").is_empty())
        && waited < 600
    {
        std::thread::sleep(std::time::Duration::from_millis(500));
        waited += 1;
    }
    let images = cached_files(&cache, "ryi");
    assert_eq!(images.len(), 1, "the cache holds {images:?}");
    // the next run loads the code from the cache and compiles nothing
    let report = [("RENYI_NATIVE_REPORT", "1")];
    let second = renyi_cached(&cache, &["run", "examples/hello.ry", "Renyi"], &report);
    assert_eq!(text(&second.stdout), "Hello, Renyi!\n");
    assert!(
        text(&second.stderr).contains("code objects loaded from the image"),
        "{}",
        text(&second.stderr)
    );
    // `--no-cache` and `RENYI_NO_CACHE` leave it alone
    let unused = renyi_cached(
        &cache,
        &["run", "--no-cache", "examples/hello.ry", "Renyi"],
        &report,
    );
    assert_eq!(text(&unused.stdout), "Hello, Renyi!\n");
    assert!(!text(&unused.stderr).contains("loaded from the image"));
    let off = renyi_cached(
        &cache,
        &["run", "examples/hello.ry", "Renyi"],
        &[("RENYI_NATIVE_REPORT", "1"), ("RENYI_NO_CACHE", "1")],
    );
    assert!(!text(&off.stderr).contains("loaded from the image"));
    // an entry that holds another program's image is a miss, never that
    // program run
    let other = path(&cache.join("other.ryi.build"));
    let built = renyi(&["build", "--to", &other, "examples/active_users.ry"]);
    assert!(built.status.success(), "{}", text(&built.stderr));
    std::fs::copy(&other, &images[0]).expect("the entry overwritten");
    let mismatched = renyi_cached(&cache, &["run", "examples/hello.ry", "Renyi"], &report);
    assert_eq!(text(&mismatched.stdout), "Hello, Renyi!\n");
    assert!(!text(&mismatched.stderr).contains("loaded from the image"));
    // `build --cache` puts the image there at once, as the run would have
    let stored = renyi_cached(&cache, &["build", "--cache", "examples/hello.ry"], &[]);
    assert!(stored.status.success(), "{}", text(&stored.stderr));
    assert!(
        text(&stored.stderr).starts_with("renyi: cached the image of examples/hello.ry as "),
        "{}",
        text(&stored.stderr)
    );
    let tested = renyi_cached(&cache, &["test", "examples/hello.ry"], &[]);
    assert!(tested.status.success(), "{}", text(&tested.stderr));
    let again = renyi_cached(&cache, &["run", "examples/hello.ry", "Renyi"], &report);
    assert!(text(&again.stderr).contains("code objects loaded from the image"));
    // with the cache off, `build --cache` says so
    let refused = renyi_cached(
        &cache,
        &["build", "--cache", "examples/hello.ry"],
        &[("RENYI_NO_CACHE", "1")],
    );
    assert!(!refused.status.success());
    assert!(text(&refused.stderr).contains("the image cache is off"));
}

/// The two files of a program with an own import, whose call to a
/// deprecated function warns, written under `directory`.
fn write_greeting_program(directory: &std::path::Path) {
    std::fs::create_dir_all(directory).expect("the program's directory");
    std::fs::write(
        directory.join("main.ry"),
        "module main
  purpose: Greet through an own module; the call to a deprecated function warns.

import std.console
import words

function main() needs console
  purpose: Print the greeting, then the loud one.

  console.print(words.greet(\"Renyi\"))
  console.print(words.shout(\"hi\"))
end
",
    )
    .expect("main.ry written");
    std::fs::write(directory.join("words.ry"), words_module("Hello")).expect("words.ry written");
}

fn words_module(greeting: &str) -> String {
    format!(
        "module words
  purpose: Greetings.

public function greet(name: Text) returns Text
  purpose: The greeting for a name.

  return \"{greeting}, {{name}}!\"
end

public function shout(text: Text) returns Text
  purpose: The text, loudly.
  deprecated: since 0.2, replaced by greet

  return \"{{text}}!\"
end
"
    )
}

#[test]
fn a_run_leaves_its_program_keyed_by_the_sources_and_the_next_runs_no_front_end() {
    let cache = scratch("sources");
    let _ = std::fs::remove_dir_all(&cache);
    let project = scratch("sources_project");
    let _ = std::fs::remove_dir_all(&project);
    let source = project.join("src");
    write_greeting_program(&source);
    let main = path(&source.join("main.ry"));
    let report = [("RENYI_CACHE_REPORT", "1")];
    let stored = "stored in the cache keyed by the sources";
    let loaded = "loaded from the cache keyed by the sources";
    // the first run compiles, prints the warning and leaves the program
    let first = renyi_cached(&cache, &["run", &main], &report);
    assert!(first.status.success(), "{}", text(&first.stderr));
    let output = text(&first.stdout);
    assert!(
        output.contains("warning [deprecated]: `shout` is deprecated")
            && output.ends_with("Hello, Renyi!\nhi!\n"),
        "{output}"
    );
    assert!(
        text(&first.stderr).contains(stored),
        "{}",
        text(&first.stderr)
    );
    assert_eq!(cached_files(&cache, "rys").len(), 1);
    // the next run loads it, the warning printed again, no front end run
    let second = renyi_cached(&cache, &["run", &main], &report);
    assert!(second.status.success(), "{}", text(&second.stderr));
    assert_eq!(text(&second.stdout), output);
    assert!(
        text(&second.stderr).contains(loaded),
        "{}",
        text(&second.stderr)
    );
    assert_eq!(cached_files(&cache, "rys").len(), 1);
    // `test` and the image built in the background find it too
    let tested = renyi_cached(&cache, &["test", &main], &report);
    assert!(tested.status.success(), "{}", text(&tested.stderr));
    assert!(
        text(&tested.stderr).contains(loaded),
        "{}",
        text(&tested.stderr)
    );
    let built = renyi_cached(&cache, &["build", "--cache", &main], &report);
    assert!(built.status.success(), "{}", text(&built.stderr));
    assert!(
        text(&built.stderr).contains(loaded),
        "{}",
        text(&built.stderr)
    );
    // an imported file changed is a miss, and the run shows the change
    std::fs::write(source.join("words.ry"), words_module("Welcome")).expect("words.ry changed");
    let changed = renyi_cached(&cache, &["run", &main], &report);
    assert!(changed.status.success(), "{}", text(&changed.stderr));
    assert!(
        text(&changed.stdout).ends_with("Welcome, Renyi!\nhi!\n"),
        "{}",
        text(&changed.stdout)
    );
    assert!(
        text(&changed.stderr).contains(stored),
        "{}",
        text(&changed.stderr)
    );
    let again = renyi_cached(&cache, &["run", &main], &report);
    assert!(
        text(&again.stderr).contains(loaded),
        "{}",
        text(&again.stderr)
    );
    // a manifest made in a directory above the program is a miss: the
    // project's root moves there, and the import is read from it
    std::fs::write(
        project.join("renyi.json"),
        "{\n  \"name\": \"main\",\n  \"version\": \"0.1.0\",\n  \"purpose\": \"A greeting.\"\n}\n",
    )
    .expect("the manifest written");
    std::fs::write(project.join("words.ry"), words_module("Welcome")).expect("words.ry moved");
    let with_manifest = renyi_cached(&cache, &["run", &main], &report);
    assert!(
        with_manifest.status.success(),
        "{}",
        text(&with_manifest.stderr)
    );
    assert_eq!(text(&with_manifest.stdout), text(&changed.stdout));
    assert!(
        text(&with_manifest.stderr).contains(stored),
        "{}",
        text(&with_manifest.stderr)
    );
    // `--no-cache` and `RENYI_NO_CACHE` leave the cache alone
    for entry in cached_files(&cache, "rys") {
        std::fs::remove_file(entry).expect("the entry removed");
    }
    let unused = renyi_cached(&cache, &["run", "--no-cache", &main], &report);
    assert!(unused.status.success(), "{}", text(&unused.stderr));
    assert_eq!(text(&unused.stdout), text(&changed.stdout));
    assert!(!text(&unused.stderr).contains("cache keyed by the sources"));
    let off = renyi_cached(
        &cache,
        &["run", &main],
        &[("RENYI_CACHE_REPORT", "1"), ("RENYI_NO_CACHE", "1")],
    );
    assert!(off.status.success(), "{}", text(&off.stderr));
    assert!(!text(&off.stderr).contains("cache keyed by the sources"));
    assert!(cached_files(&cache, "rys").is_empty());
}

#[test]
fn a_recording_from_the_cache_keyed_by_the_sources_carries_the_manifest_and_reproduces() {
    let cache = scratch("sources_record");
    let _ = std::fs::remove_dir_all(&cache);
    let report = [("RENYI_CACHE_REPORT", "1")];
    let stored = "stored in the cache keyed by the sources";
    let loaded = "loaded from the cache keyed by the sources";
    // a program with a dependency, whose manifest names the package
    let program = "tests/conformance/packages/project/report.ry";
    let plain = renyi_cached(&cache, &["run", program], &report);
    assert!(plain.status.success(), "{}", text(&plain.stderr));
    assert!(
        text(&plain.stderr).contains(stored),
        "{}",
        text(&plain.stderr)
    );
    // `record` after a plain run misses once, since the entry kept no
    // manifest, and then hits with the manifest kept
    let recording = path(&cache.join("report.recording.json"));
    let recorded = renyi_cached(&cache, &["record", "--to", &recording, program], &report);
    assert!(recorded.status.success(), "{}", text(&recorded.stderr));
    assert!(
        text(&recorded.stderr).contains(stored),
        "{}",
        text(&recorded.stderr)
    );
    let from_cache = path(&cache.join("report_cached.recording.json"));
    let recorded = renyi_cached(&cache, &["record", "--to", &from_cache, program], &report);
    assert!(recorded.status.success(), "{}", text(&recorded.stderr));
    assert!(
        text(&recorded.stderr).contains(loaded),
        "{}",
        text(&recorded.stderr)
    );
    assert_eq!(cached_files(&cache, "rys").len(), 1);
    // the manifest kept is the one a compile computes
    let fresh = path(&cache.join("report_fresh.recording.json"));
    let recorded = renyi_cached(
        &cache,
        &["record", "--no-cache", "--to", &fresh, program],
        &report,
    );
    assert!(recorded.status.success(), "{}", text(&recorded.stderr));
    let manifest = |file: &str| {
        let text = std::fs::read_to_string(file).expect("the recording");
        renyi_vm::Recording::parse(&text)
            .expect("a recording")
            .manifest
    };
    let kept = manifest(&from_cache);
    let computed = manifest(&fresh);
    assert!(kept.code.is_some());
    assert_eq!(kept.code, computed.code);
    assert_eq!(kept.dependencies, computed.dependencies);
    assert_eq!(kept.dependencies.len(), 1);
    assert_eq!(kept.dependencies[0].name, "greeting");
    // `reproduce` accepts the recording, through the cache too
    let reproduced = renyi_cached(&cache, &["reproduce", &from_cache], &report);
    assert!(reproduced.status.success(), "{}", text(&reproduced.stderr));
    // `--manifest` on a hit prints the same code hash
    let printed = renyi_cached(&cache, &["run", "--manifest", program], &report);
    assert!(printed.status.success(), "{}", text(&printed.stderr));
    let message = text(&printed.stderr);
    assert!(message.contains(loaded), "{message}");
    assert!(
        message.contains(kept.code.as_deref().unwrap_or("")),
        "{message}"
    );
}
