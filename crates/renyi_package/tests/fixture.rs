//! The package fixture of the conformance suite (decision AC1), held in
//! canonical form: the registry's `package.json` renders to itself, the
//! lockfile's hash is its hash, and the resolver reads the projects as the
//! suite expects.

use std::path::PathBuf;

use renyi_package::{hash_of, resolve, Lock, PackageFile};
use renyi_syntax::SourceFile;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(root().join(relative))
        .unwrap_or_else(|error| panic!("{relative}: {error}"))
}

#[test]
fn the_fixture_is_in_canonical_form() {
    let text = read("tests/conformance/packages/registry/greeting/1.0.0/package.json");
    let package = PackageFile::read(&text).expect("the package file reads");
    assert_eq!(
        package.render(),
        text,
        "package.json is not as `renyi publish` renders it"
    );
    let lock = Lock::read(&read("tests/conformance/packages/project/renyi.lock.json"))
        .expect("the lockfile reads");
    let locked = lock.get("greeting").expect("greeting is locked");
    assert_eq!(locked.hash, hash_of(text.as_bytes()));
}

#[test]
fn the_resolver_reads_the_projects_as_the_suite_expects() {
    // the resolver names files from the working directory, as the commands do
    std::env::set_current_dir(root()).expect("the repository root");
    let main = "tests/conformance/packages/project/report.ry";
    let resolved = resolve(&SourceFile::new(main, read(main)));
    let names: Vec<&str> = resolved.files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            main,
            "tests/conformance/packages/registry/greeting/1.0.0/greeting.ry",
            "tests/conformance/packages/registry/greeting/1.0.0/words.ry",
        ]
    );
    assert!(resolved.problems.is_empty(), "{:?}", resolved.problems);
    let package = resolved.files[1]
        .package
        .as_ref()
        .expect("greeting.ry is tagged with its package");
    assert_eq!(
        (package.name.as_str(), package.version.as_str()),
        ("greeting", "1.0.0")
    );
    assert_eq!(resolved.files[2].package, resolved.files[1].package);

    let stale = "tests/conformance/packages/stale/stale.ry";
    let resolved = resolve(&SourceFile::new(stale, read(stale)));
    let codes: Vec<String> = resolved
        .problems
        .iter()
        .map(|p| p.diagnostic.code.to_string())
        .collect();
    assert_eq!(codes, ["package-mismatch"]);
    assert_eq!(
        resolved.files.len(),
        3,
        "a package whose hash differs is reported and read all the same"
    );
}
