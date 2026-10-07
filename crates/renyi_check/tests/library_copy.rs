//! The crate's copy of the standard library declarations, under its own
//! `library/std/`, equals the canonical `library/std/` at the repository
//! root file for file (decision AI5): the copy is what `include_str!`
//! embeds, so that the crate builds from its crates.io tarball, which
//! carries only the files under the crate's directory.

use std::collections::BTreeMap;
use std::path::Path;

fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("ry"))
        .map(|path| {
            let name = path
                .file_name()
                .expect("name")
                .to_string_lossy()
                .into_owned();
            (name, std::fs::read(&path).expect("read"))
        })
        .collect()
}

#[test]
fn the_copy_of_the_library_equals_the_canonical_files() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let copy = files(&crate_root.join("library/std"));
    let canonical = files(&crate_root.join("../../library/std"));
    assert!(!canonical.is_empty());
    let copied: Vec<&String> = copy.keys().collect();
    let canonical_names: Vec<&String> = canonical.keys().collect();
    assert_eq!(copied, canonical_names, "the same files, by name");
    for (name, bytes) in &canonical {
        assert!(
            copy[name] == *bytes,
            "crates/renyi_check/library/std/{name} differs from library/std/{name}: copy the file again"
        );
    }
    // every embedded module is the canonical text
    assert_eq!(renyi_check::LIBRARY.len(), canonical.len());
    for (module, text) in renyi_check::LIBRARY {
        let name = format!("{}.ry", module.trim_start_matches("std."));
        assert!(
            text.as_bytes() == canonical[&name].as_slice(),
            "{module} is embedded from a file that differs from library/std/{name}"
        );
    }
}
