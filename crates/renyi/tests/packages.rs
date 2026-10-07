//! The package commands (decision AC1): `renyi publish` renders the
//! fixture's package file and never overwrites a version, `add` chooses,
//! fetches and verifies a package and names its effects, the run manifest
//! names the dependencies and `reproduce` compares them, `audit` reports
//! the coverage, `fetch` refuses a file that differs from its hash,
//! `update` refuses widened effects without `--accept-effects` and with
//! it until `main` declares them, a URL registry is fetched into the
//! store, and the map labels a dependency's definitions and reads the
//! budgets of the manifest.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FIXTURE: &str = "tests/conformance/packages";

const PROGRAM_OUTPUT: &str = "Hello, world!\nHello, Renyi!\n";

/// `greeting` 1.1.0: `dated` is new and needs `time`.
const GREETING_1_1_0: &str = "module greeting
  purpose: A greeting package: a greeting as text, and announced on the console.

import std.console
import std.time
import words

public function hello(name: Text) returns Text
  purpose: The greeting for a name.

  return words.exclaim(\"Hello, {name}\")
end

public function announce(name: Text) needs console
  purpose: Print the greeting for a name.

  console.print(hello(name))
end

public function dated(name: Text) returns Text needs time
  purpose: The greeting for a name with today's date.

  let day be time.today().to_text()
  return \"{hello(name)} on {day}\"
end
";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf()
}

fn fixture() -> PathBuf {
    root().join(FIXTURE)
}

/// A fresh scratch directory under the workspace's `target`.
fn scratch(name: &str) -> PathBuf {
    let directory = root().join("target/packages").join(name);
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("a clean scratch directory");
    }
    std::fs::create_dir_all(&directory).expect("the scratch directory");
    directory
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the target directory");
    for entry in std::fs::read_dir(from).expect("the source directory") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("a copy");
        }
    }
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the parent directory");
    }
    std::fs::write(path, text).expect("the file is written");
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        .replace("\r\n", "\n")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn renyi_in(directory: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_renyi"));
    command.current_dir(directory).args(args);
    for variable in [
        "HTTP_PROXY",
        "http_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        command.env_remove(variable);
    }
    command.output().expect("the renyi binary runs")
}

fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        text(&output.stdout),
        text(&output.stderr)
    );
    text(&output.stdout)
}

fn refused(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "succeeded:\n{}",
        text(&output.stdout)
    );
    text(&output.stderr)
}

fn manifest(name: &str, version: &str, purpose: &str, registry: &str) -> String {
    format!(
        "{{\n  \"name\": \"{name}\",\n  \"version\": \"{version}\",\n  \"purpose\": \"{purpose}\",\n  \"dependencies\": {{}},\n  \"registry\": \"{registry}\"\n}}\n"
    )
}

/// The sources of `greeting` 1.0.0 from the fixture's registry, as a
/// project to publish.
fn greeting_source(directory: &Path, version: &str) -> PathBuf {
    let source = directory.join("greeting");
    for name in ["greeting.ry", "words.ry"] {
        std::fs::copy(
            fixture().join("registry/greeting/1.0.0").join(name),
            source_file(&source, name),
        )
        .expect("a copy");
    }
    write(
        &source.join("renyi.json"),
        &manifest(
            "greeting",
            version,
            "A greeting package: a greeting as text, and announced on the console.",
            "../registry",
        ),
    );
    source
}

fn source_file(source: &Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(source).expect("the source directory");
    source.join(name)
}

/// An application depending on nothing yet: the fixture's `report.ry` and
/// a manifest naming the registry.
fn application(directory: &Path, registry: &str) -> PathBuf {
    let app = directory.join("app");
    std::fs::create_dir_all(&app).expect("the application directory");
    std::fs::copy(fixture().join("project/report.ry"), app.join("report.ry")).expect("a copy");
    write(
        &app.join("renyi.json"),
        &manifest(
            "report",
            "0.1.0",
            "The programs that use the greeting package.",
            registry,
        ),
    );
    app
}

/// The fixture's project files: a manifest requiring `greeting` 1.0.0 and
/// the lockfile naming it.
fn locked_application(directory: &Path) -> PathBuf {
    let app = application(directory, "../registry");
    for name in ["renyi.json", "renyi.lock.json"] {
        std::fs::copy(fixture().join("project").join(name), app.join(name)).expect("a copy");
    }
    app
}

#[test]
fn publish_writes_the_fixtures_package_file_and_never_overwrites_a_version() {
    let directory = scratch("publish");
    let source = greeting_source(&directory, "1.0.0");
    let published = ok(&renyi_in(&source, &["publish"]));
    assert_eq!(
        published,
        "published `greeting` 1.0.0 to ../registry: 2 files, 3 public functions\n"
    );
    let registry = directory.join("registry");
    for name in ["package.json", "greeting.ry", "words.ry"] {
        assert_eq!(
            read(&registry.join("greeting/1.0.0").join(name)),
            read(&fixture().join("registry/greeting/1.0.0").join(name)),
            "{name}"
        );
    }
    assert_eq!(
        read(&registry.join("greeting/versions.json")),
        read(&fixture().join("registry/greeting/versions.json"))
    );
    // a published version is never overwritten
    assert_eq!(
        refused(&renyi_in(&source, &["publish"])),
        "renyi: `greeting` 1.0.0 is published already; a published version is never overwritten: raise `version` in renyi.json\n"
    );
    // the project must check clean
    write(
        &source.join("broken.ry"),
        "module broken\n  purpose: A function that does not check.\n\nfunction wrong() returns Text\n  purpose: Return an integer as text.\n\n  return 1\nend\n",
    );
    write(
        &source.join("renyi.json"),
        &manifest(
            "greeting",
            "1.0.1",
            "A greeting package: a greeting as text, and announced on the console.",
            "../registry",
        ),
    );
    let unclean = refused(&renyi_in(&source, &["publish"]));
    assert!(
        unclean
            .starts_with("renyi: the project must check clean before it is published:\nbroken.ry:"),
        "{unclean}"
    );
    std::fs::remove_file(source.join("broken.ry")).expect("removed");
    // a public function added needs a new minor version
    write(&source.join("greeting.ry"), GREETING_1_1_0);
    assert_eq!(
        refused(&renyi_in(&source, &["publish"])),
        "renyi: version 1.0.1 is not enough: the changes since 1.0.0 need a new minor version, 1.1.0 at least (decision G1)\n"
    );
    write(
        &source.join("renyi.json"),
        &manifest(
            "greeting",
            "1.1.0",
            "A greeting package: a greeting as text, and announced on the console.",
            "../registry",
        ),
    );
    assert_eq!(
        ok(&renyi_in(&source, &["publish"])),
        "published `greeting` 1.1.0 to ../registry: 2 files, 4 public functions\n"
    );
    assert_eq!(
        read(&registry.join("greeting/versions.json")),
        "{\n  \"versions\": [\n    \"1.0.0\",\n    \"1.1.0\"\n  ]\n}\n"
    );
    let package = read(&registry.join("greeting/1.1.0/package.json"));
    assert!(
        package.contains(
            "\"function\": \"greeting.dated\",\n      \"needs\": [\n        \"time\"\n      ],"
        ),
        "{package}"
    );
}

#[test]
fn add_fetches_a_package_names_its_effects_and_the_run_manifest_names_it() {
    let directory = scratch("add");
    copy_tree(&fixture().join("registry"), &directory.join("registry"));
    let app = application(&directory, "../registry");
    assert_eq!(
        ok(&renyi_in(&app, &["add", "greeting"])),
        "added `greeting` 1.0.0 (renyi.json requires 1.0.0: the same major, at least that version)\n  greeting.announce needs console\n  greeting.hello needs nothing\n  words.exclaim needs nothing\n"
    );
    assert_eq!(
        read(&app.join("renyi.lock.json")),
        read(&fixture().join("project/renyi.lock.json"))
    );
    let written = read(&app.join("renyi.json"));
    assert!(
        written.contains("\"dependencies\": {\n    \"greeting\": \"1.0.0\"\n  }"),
        "{written}"
    );
    assert_eq!(ok(&renyi_in(&app, &["run", "report.ry"])), PROGRAM_OUTPUT);
    // the run manifest names the dependency with the lockfile's hash
    ok(&renyi_in(
        &app,
        &["record", "--to", "run.json", "report.ry"],
    ));
    let recording = read(&app.join("run.json"));
    assert!(
        recording.contains("\"dependencies\": {\n    \"greeting\": {\n      \"version\": \"1.0.0\",\n      \"hash\": \"sha256:5a0f7d1d6b3b82d2ac4413b1c37d8098ec0aaf9c425ecd562b76d25dec23d4b6\"\n    }\n  },\n"),
        "{recording}"
    );
    let reproduced = renyi_in(&app, &["reproduce", "run.json"]);
    assert!(reproduced.status.success(), "{}", text(&reproduced.stderr));
    // a recording made with another version of the package is refused
    write(
        &app.join("other.json"),
        &recording.replace("sha256:5a0f7d1d", "sha256:00000000"),
    );
    let differs = refused(&renyi_in(&app, &["reproduce", "other.json"]));
    assert!(
        differs.starts_with("renyi: the dependencies differ from the manifest: the recording names greeting 1.0.0 sha256:00000000"),
        "{differs}"
    );
    // `add` refuses what is not a package
    assert_eq!(
        refused(&renyi_in(&app, &["add", "std"])),
        "renyi: `std` is the standard library, not a package\n"
    );
    assert_eq!(
        refused(&renyi_in(&app, &["add", "greeting", "2.0.0"])),
        "renyi: no version of `greeting` satisfies 2.0.0 (the same major, at least that version): the registry has 1.0.0\n"
    );
}

#[test]
fn audit_reports_every_main_that_reaches_a_dependency() {
    let directory = scratch("audit");
    copy_tree(&fixture().join("registry"), &directory.join("registry"));
    let app = locked_application(&directory);
    let audit = renyi_in(&app, &["audit"]);
    assert!(audit.status.success(), "{}", text(&audit.stderr));
    assert_eq!(
        text(&audit.stdout),
        "`greeting` 1.0.0 needs `console`\n  covered by `main` of report.ry\n"
    );
    std::fs::copy(
        fixture().join("project/uncovered.ry"),
        app.join("uncovered.ry"),
    )
    .expect("a copy");
    let audit = renyi_in(&app, &["audit"]);
    assert!(!audit.status.success(), "{}", text(&audit.stdout));
    assert_eq!(
        text(&audit.stdout),
        "`greeting` 1.0.0 needs `console`\n  covered by `main` of report.ry\n  not covered by `main` of uncovered.ry: `console`\n"
    );
}

#[test]
fn fetch_verifies_every_file_against_the_package_file() {
    let directory = scratch("fetch");
    copy_tree(&fixture().join("registry"), &directory.join("registry"));
    let app = locked_application(&directory);
    assert_eq!(
        ok(&renyi_in(&app, &["fetch"])),
        "`greeting` 1.0.0 at ../registry/greeting/1.0.0\n"
    );
    // a file of the package changed under its hash
    let words = directory.join("registry/greeting/1.0.0/words.ry");
    let original = read(&words);
    write(&words, &format!("{original}\n"));
    assert_eq!(
        refused(&renyi_in(&app, &["fetch"])),
        "renyi: ../registry/greeting/1.0.0/words.ry is not the file `greeting` 1.0.0 names: its hash differs\n"
    );
    write(&words, &original);
    // the lockfile names a package the registry does not have
    std::fs::copy(
        fixture().join("stale/renyi.lock.json"),
        app.join("renyi.lock.json"),
    )
    .expect("a copy");
    assert_eq!(
        refused(&renyi_in(&app, &["fetch"])),
        "renyi: `greeting` 1.0.0 in the registry is not the package renyi.lock.json names: its hash differs; run `renyi update` to take it\n"
    );
}

#[test]
fn update_refuses_widened_effects_until_accepted_and_declared() {
    let directory = scratch("update");
    copy_tree(&fixture().join("registry"), &directory.join("registry"));
    let source = greeting_source(&directory, "1.1.0");
    write(&source.join("greeting.ry"), GREETING_1_1_0);
    ok(&renyi_in(&source, &["publish"]));
    let app = locked_application(&directory);
    let widened = "`greeting` 1.0.0 -> 1.1.0\n  needs `time`, which 1.0.0 did not\n";
    assert_eq!(
        refused(&renyi_in(&app, &["update"])),
        format!("renyi: {widened}the effects widen; run `renyi update --accept-effects` to take the new versions (nothing was written)\n")
    );
    assert_eq!(
        refused(&renyi_in(&app, &["update", "--accept-effects"])),
        format!("renyi: {widened}`main` of report.ry does not declare `time`, which `greeting` 1.1.0 needs; add it to its `needs` first (nothing was written)\n")
    );
    assert_eq!(
        read(&app.join("renyi.lock.json")),
        read(&fixture().join("project/renyi.lock.json"))
    );
    let report = read(&app.join("report.ry"));
    write(
        &app.join("report.ry"),
        &report.replace(
            "function main() needs console\n",
            "function main() needs console, time\n",
        ),
    );
    assert_eq!(
        ok(&renyi_in(&app, &["update", "--accept-effects"])),
        widened
    );
    let lock = read(&app.join("renyi.lock.json"));
    assert!(lock.contains("\"version\": \"1.1.0\""), "{lock}");
    assert_eq!(ok(&renyi_in(&app, &["run", "report.ry"])), PROGRAM_OUTPUT);
    assert_eq!(
        ok(&renyi_in(&app, &["update"])),
        "nothing to update: every dependency is at the highest version its requirement allows\n"
    );
}

#[test]
fn a_url_registry_is_fetched_into_the_store() {
    let directory = scratch("url");
    copy_tree(&fixture().join("registry"), &directory.join("registry"));
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = listener.local_addr().expect("the address").port();
    let served = directory.join("registry");
    std::thread::spawn(move || serve(listener, &served));
    let app = application(&directory, &format!("http://127.0.0.1:{port}"));
    let added = ok(&renyi_in(&app, &["add", "greeting", "1.0.0"]));
    assert!(added.starts_with("added `greeting` 1.0.0"), "{added}");
    let store = app.join(".renyi/packages/greeting/1.0.0");
    for name in ["package.json", "greeting.ry", "words.ry"] {
        assert_eq!(
            read(&store.join(name)),
            read(&fixture().join("registry/greeting/1.0.0").join(name)),
            "{name}"
        );
    }
    assert_eq!(ok(&renyi_in(&app, &["run", "report.ry"])), PROGRAM_OUTPUT);
    assert_eq!(
        ok(&renyi_in(&app, &["fetch"])),
        "`greeting` 1.0.0 at .renyi/packages/greeting/1.0.0\n"
    );
    assert_eq!(
        refused(&renyi_in(&app, &["add", "absent"])),
        "renyi: the registry has no package `absent`\n"
    );
    assert_eq!(
        refused(&renyi_in(&app, &["publish"])),
        "renyi: the registry is a URL; publish into its local copy with `--to <directory>`\n"
    );
}

/// A registry over HTTP: every request answered with the file under the
/// directory, or 404.
fn serve(listener: TcpListener, directory: &Path) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            continue;
        };
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        loop {
            let Ok(count) = stream.read(&mut buffer) else {
                break;
            };
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let line = String::from_utf8_lossy(&request).to_string();
        let path = line
            .split_whitespace()
            .nth(1)
            .unwrap_or("/")
            .trim_start_matches('/')
            .to_string();
        let response = match std::fs::read(directory.join(&path)) {
            Ok(body) if !path.is_empty() => {
                let mut response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .into_bytes();
                response.extend(body);
                response
            }
            _ => {
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()
            }
        };
        let _ = stream.write_all(&response);
        let _ = stream.flush();
    }
}

#[test]
fn the_map_labels_a_dependencys_definitions_and_reads_the_budgets() {
    let project = fixture().join("project");
    let map = ok(&renyi_in(&project, &["index"]));
    assert!(map.contains("  package: greeting 1.0.0"), "{map}");
    assert!(map.contains("module greeting.words  "), "{map}");
    let json = ok(&renyi_in(&project, &["index", "--json"]));
    assert!(json.contains("\"package\": \"greeting 1.0.0\""), "{json}");
    assert!(json.contains("\"package\": null"), "{json}");
    // the budgets of the manifest
    let directory = scratch("budgets");
    copy_tree(&fixture().join("registry"), &directory.join("registry"));
    let app = locked_application(&directory);
    assert_eq!(
        ok(&renyi_in(&app, &["index", "--budgets"])),
        "nothing over budget\n"
    );
    let manifest = read(&app.join("renyi.json"));
    write(
        &app.join("renyi.json"),
        &manifest.replace(
            "\"registry\": \"../registry\"\n",
            "\"registry\": \"../registry\",\n  \"budgets\": {\n    \"effect_paths_per_module\": 0\n  }\n",
        ),
    );
    let over = ok(&renyi_in(&app, &["index", "--budgets"]));
    assert!(
        over.contains("module report: 1 transitive effect paths (budget 0): console\n"),
        "{over}"
    );
}
