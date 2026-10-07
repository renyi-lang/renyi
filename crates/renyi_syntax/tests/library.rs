//! The standard library declaration files under `library/std/` parse in
//! declaration mode, and they declare exactly the functions the library
//! sketch (`docs/design/04-stdlib-sketch.md`) lists.

use std::collections::BTreeSet;
use std::path::PathBuf;

use renyi_syntax::ast::Item;
use renyi_syntax::{parse_declarations, SourceFile};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn library_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(root().join("library/std"))
        .expect("library/std")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("ry"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 12, "twelve library modules");
    files
}

#[test]
fn every_library_file_parses_as_declarations() {
    for path in library_files() {
        let text = std::fs::read_to_string(&path).expect("read");
        let file = SourceFile::new(path.display().to_string(), text);
        let parsed = parse_declarations(&file.text);
        assert!(
            parsed.diagnostics.is_empty(),
            "{}",
            renyi_syntax::diagnostics::render_text(&file, &parsed.diagnostics)
        );
        assert!(parsed.module.docs.purpose.is_some(), "{}", path.display());
        for item in &parsed.module.items {
            match item {
                Item::Function(function) => {
                    assert!(
                        function.body.is_none(),
                        "{}: {} has a body",
                        path.display(),
                        function.name.text
                    );
                    assert!(
                        function.public,
                        "{}: {} is not public",
                        path.display(),
                        function.name.text
                    );
                }
                Item::Type(def) => assert!(
                    def.public && def.docs.purpose.is_some(),
                    "{}: {}",
                    path.display(),
                    def.name.text
                ),
                Item::Ability(ability) => assert!(ability.public, "{}", path.display()),
                Item::Implementation(implementation) => {
                    for function in &implementation.functions {
                        assert!(
                            function.body.is_none(),
                            "{}: {} has a body",
                            path.display(),
                            function.name.text
                        );
                    }
                }
                other => panic!("{}: unexpected item {other:?}", path.display()),
            }
        }
    }
}

/// `function name(` lines inside the sketch's code blocks.
fn sketch_functions() -> BTreeSet<String> {
    let text =
        std::fs::read_to_string(root().join("docs/design/04-stdlib-sketch.md")).expect("sketch");
    let mut names = BTreeSet::new();
    let mut in_block = false;
    for line in text.lines() {
        if line.starts_with("```") {
            in_block = !in_block;
            continue;
        }
        if !in_block {
            continue;
        }
        let trimmed = line.trim_start().trim_start_matches("public ");
        if let Some(rest) = trimmed.strip_prefix("function ") {
            if let Some(end) = rest.find('(') {
                names.insert(rest[..end].to_string());
            }
        }
    }
    names
}

#[test]
fn the_library_declares_what_the_sketch_lists() {
    let mut declared = BTreeSet::new();
    for path in library_files() {
        let text = std::fs::read_to_string(&path).expect("read");
        let parsed = parse_declarations(&text);
        // ability methods (equals, compare, hash, to_text) are listed in a
        // table of the sketch, not in a code block, so only free functions and
        // methods are compared
        for item in &parsed.module.items {
            match item {
                Item::Function(function) => {
                    declared.insert(function.name.text.clone());
                }
                Item::Implementation(implementation) => {
                    for function in &implementation.functions {
                        declared.insert(function.name.text.clone());
                    }
                }
                _ => {}
            }
        }
    }
    let sketched = sketch_functions();
    let missing: Vec<_> = sketched.difference(&declared).collect();
    let extra: Vec<_> = declared.difference(&sketched).collect();
    assert!(
        missing.is_empty(),
        "in the sketch but not declared: {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "declared but not in the sketch: {extra:?}"
    );
}
