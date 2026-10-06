//! `renyi tools`: the manifest of the corpus's one tool, and the
//! diagnostics of a project that does not check.

use std::path::PathBuf;
use std::process::Command;

fn renyi(arguments: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_renyi"))
        .args(arguments)
        .output()
        .expect("the renyi binary runs");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn example(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
        .display()
        .to_string()
}

#[test]
fn the_corpus_exposes_one_tool() {
    let (ok, manifest, stderr) = renyi(&["tools", &example("currency_tool.ry")]);
    assert!(ok, "{stderr}");
    assert!(
        manifest.starts_with("[\n  {\n    \"name\": \"currency_tool.convert\",\n    \"description\": \"Convert an amount from one currency to another at today's rate, rounded to cents.\","),
        "{manifest}"
    );
    assert!(
        manifest.contains(
            "\"permissions\": [\n      \"network.http(\\\"api.frankfurter.dev\\\")\"\n    ]"
        ),
        "{manifest}"
    );
    assert!(manifest.contains("\"fails\": [\n      \"RateUnavailable\",\n      \"HttpError\",\n      \"JsonError\"\n    ]"), "{manifest}");
    assert!(manifest.contains("\"amount\": {\n          \"type\": \"number\",\n          \"format\": \"decimal\"\n        }"), "{manifest}");
    assert!(manifest.contains("\"function\": \"convert\""), "{manifest}");
    // a program without tools: an empty manifest
    let (ok, manifest, _) = renyi(&["tools", &example("hello.ry")]);
    assert!(ok);
    assert_eq!(manifest, "[]\n");
}

#[test]
fn a_project_with_errors_gets_its_diagnostics() {
    let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("tools-broken");
    std::fs::create_dir_all(&directory).expect("the scratch directory exists");
    let file = directory.join("broken.ry");
    std::fs::write(
        &file,
        "module broken\n  purpose: Does not check.\n\npublic function go() returns Integer\n  purpose: Wrong.\n  expose as tool\n\n  return \"text\"\nend\n",
    )
    .expect("written");
    let (ok, stdout, _) = renyi(&["tools", &file.display().to_string()]);
    assert!(!ok);
    assert!(stdout.contains("[type-mismatch]"), "{stdout}");
    let (ok, _, stderr) = renyi(&["tools", "--json", &file.display().to_string()]);
    assert!(!ok);
    assert!(stderr.contains("unknown option"), "{stderr}");
}
