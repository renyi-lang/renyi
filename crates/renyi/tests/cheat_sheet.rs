//! The crate's copy of the cheat sheet, which the MCP server embeds and
//! serves as the `cheat_sheet` tool, equals `docs/cheatsheet.md` byte for
//! byte (decision AI5): the copy is under the crate's directory so that
//! the crate builds from its crates.io tarball.

use std::path::Path;

#[test]
fn the_copy_of_the_cheat_sheet_equals_the_document() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let copy = std::fs::read(crate_root.join("cheatsheet.md")).expect("crates/renyi/cheatsheet.md");
    let document =
        std::fs::read(crate_root.join("../../docs/cheatsheet.md")).expect("docs/cheatsheet.md");
    assert!(
        copy == document,
        "crates/renyi/cheatsheet.md differs from docs/cheatsheet.md: copy the file again"
    );
}
