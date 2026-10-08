//! The resident world (decision AN1) gives the map and the diagnostics a
//! fresh build gives, on the corpus, the compiler and the starter pack
//! and after every kind of change to a project: a body edited, a
//! signature edited, a file added, removed, broken and mended, an
//! overlay set and cleared; it parses and checks only what changed; and
//! as written it gives what `renyi check` gives.

use std::path::{Path, PathBuf};

use renyi_check::{check_project_in, Library};
use renyi_index::{canonical_files, index_files_in, load_project, Header, Index};
use renyi_syntax::Diagnostic;
use renyi_workspace::{Refresh, Workspace};

fn header() -> Header {
    Header {
        project: "test".to_string(),
        revision: "none".to_string(),
        toolchain: "renyi test".to_string(),
    }
}

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// What a fresh build gives: the map and every module's diagnostics.
fn fresh(directory: &Path) -> (Index, Vec<Vec<Diagnostic>>) {
    let library = Library::standard();
    let files = load_project(directory).expect("the project loads");
    let index = index_files_in(&library, &files, header());
    let (canonical, _) = canonical_files(&files);
    let checked = check_project_in(&library, &canonical, &[]);
    (
        index,
        checked
            .modules
            .into_iter()
            .map(|module| module.diagnostics)
            .collect(),
    )
}

fn assert_as_fresh(workspace: &mut Workspace, directory: &Path) {
    let (index, diagnostics) = fresh(directory);
    assert_eq!(workspace.index(header), Some(&index));
    let checked = workspace.checked().expect("a refreshed workspace");
    assert_eq!(checked.modules.len(), diagnostics.len());
    for (module, expected) in checked.modules.iter().zip(&diagnostics) {
        assert_eq!(module.diagnostics, *expected, "module {}", module.file);
    }
}

/// What a refresh did, the reads left out: a file written just before a
/// refresh is read again on the next one whatever its stamp says.
fn work(report: &Refresh) -> (usize, bool, usize, usize) {
    (
        report.parsed,
        report.declared,
        report.checked,
        report.reused,
    )
}

fn scratch(name: &str) -> PathBuf {
    let directory = repository().join("target/workspace").join(name);
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("a clean scratch directory");
    }
    std::fs::create_dir_all(&directory).expect("the scratch directory");
    directory
}

fn write(directory: &Path, name: &str, content: &str) {
    std::fs::write(directory.join(name), content).expect("the file is written");
}

fn has_error(workspace: &Workspace, file: &str) -> bool {
    workspace
        .diagnostics(file)
        .is_some_and(|diagnostics| diagnostics.iter().any(Diagnostic::is_error))
}

#[test]
fn the_corpus_the_compiler_and_the_starter_pack_index_as_a_fresh_build_does() {
    for directory in ["examples", "compiler", "starter/workflows"] {
        let directory = repository().join(directory);
        let mut workspace = Workspace::new(&directory, &Library::standard());
        let report = workspace.refresh().expect("the first refresh");
        assert!(report.declared && report.parsed > 0 && report.reused == 0);
        assert_as_fresh(&mut workspace, &directory);
        // nothing changed: nothing is parsed, declared or checked
        let report = workspace.refresh().expect("the second refresh");
        assert_eq!(work(&report), (0, false, 0, 0));
        assert_as_fresh(&mut workspace, &directory);
    }
}

#[test]
fn as_written_the_diagnostics_are_those_of_renyi_check() {
    for directory in ["examples", "compiler"] {
        let directory = repository().join(directory);
        let library = Library::standard();
        let files = load_project(&directory).expect("the project loads");
        let checked = check_project_in(&library, &files, &[]);
        let mut workspace = Workspace::new(&directory, &library).as_written();
        workspace.refresh().expect("the first refresh");
        let names: Vec<&str> = workspace.files().collect();
        assert_eq!(
            names.len(),
            files.len(),
            "{directory:?} has no dependencies"
        );
        for (name, module) in names.iter().zip(&checked.modules) {
            assert_eq!(
                workspace.diagnostics(name),
                Some(module.diagnostics.as_slice())
            );
            assert_eq!(
                workspace.file(name).map(|file| file.text.as_str()),
                Some(files[module.file].text.as_str()),
                "the text as written"
            );
            assert_eq!(workspace.module_of(name), module.id);
        }
    }
}

const SHAPES: &str = "module shapes
  purpose: Shapes for the workspace test.

public type Square
  purpose: A square by its side.
  has side: Integer
end

public function area(square: Square) returns Integer
  purpose: The area of the square.

  return square.side * square.side
end

public function perimeter(square: Square) returns Integer
  purpose: The perimeter of the square.

  return square.side * 4
end
";

const REPORT: &str = "module report
  purpose: Print the measures of a square.

import std.console
import shapes exposing Square

public function main() needs console
  purpose: Print the area and the perimeter.

  let square be Square(side: 3)
  console.print(\"{shapes.area(square)} {shapes.perimeter(square)}\")
end
";

const EXTRA: &str = "module extra
  purpose: An extra module.

public function twice(value: Integer) returns Integer
  purpose: Twice the value.

  return value * 2
end
";

#[test]
fn a_project_is_followed_through_every_kind_of_change() {
    let directory = scratch("changes");
    write(&directory, "shapes.ry", SHAPES);
    write(&directory, "report.ry", REPORT);
    let mut workspace = Workspace::new(&directory, &Library::standard());
    let report = workspace.refresh().expect("the first refresh");
    assert_eq!(report.read, 2);
    assert_eq!(work(&report), (2, true, 4, 0));
    assert_as_fresh(&mut workspace, &directory);
    assert!(!has_error(&workspace, "report.ry"));
    assert!(workspace
        .file_of(workspace.module_of("shapes.ry").expect("shapes declares"))
        .is_some_and(|file| file.name.ends_with("shapes.ry")));

    // a body edited: one item checked again, the other three kept
    write(
        &directory,
        "report.ry",
        &REPORT.replace("side: 3", "side: 12"),
    );
    let report = workspace.refresh().expect("after a body edit");
    assert_eq!(work(&report), (1, true, 1, 3));
    assert_as_fresh(&mut workspace, &directory);

    // a signature edited: every item checked again, the caller now wrong
    let wider = SHAPES.replace(
        "perimeter(square: Square) returns Integer",
        "perimeter(square: Square, times: Integer) returns Integer",
    );
    write(&directory, "shapes.ry", &wider);
    let report = workspace.refresh().expect("after a signature edit");
    assert_eq!(work(&report), (1, true, 4, 0));
    assert_as_fresh(&mut workspace, &directory);
    assert!(
        has_error(&workspace, "report.ry"),
        "the call no longer fits"
    );

    // the caller mended: a body edit again
    let mended = REPORT.replace("side: 3", "side: 12").replace(
        "shapes.perimeter(square)",
        "shapes.perimeter(square: square, times: 2)",
    );
    write(&directory, "report.ry", &mended);
    let report = workspace.refresh().expect("after the caller is mended");
    assert_eq!(work(&report), (1, true, 1, 3));
    assert_as_fresh(&mut workspace, &directory);
    assert!(!has_error(&workspace, "report.ry"));

    // a file added, then removed: the world declared again each time
    write(&directory, "extra.ry", EXTRA);
    let report = workspace.refresh().expect("after a file is added");
    assert_eq!(work(&report), (1, true, 5, 0));
    assert_as_fresh(&mut workspace, &directory);
    std::fs::remove_file(directory.join("extra.ry")).expect("the file is removed");
    let report = workspace.refresh().expect("after a file is removed");
    assert_eq!(work(&report), (0, true, 4, 0));
    assert_as_fresh(&mut workspace, &directory);

    // a file broken, then mended
    write(
        &directory,
        "shapes.ry",
        "module shapes\n  purpose: Broken.\n\npublic function area(\n",
    );
    let report = workspace.refresh().expect("after a file is broken");
    assert_eq!(work(&report), (1, true, 1, 0));
    assert_as_fresh(&mut workspace, &directory);
    assert!(has_error(&workspace, "shapes.ry"));
    assert_eq!(workspace.module_of("shapes.ry"), None);
    write(&directory, "shapes.ry", &wider);
    let report = workspace.refresh().expect("after the file is mended");
    assert_eq!(work(&report), (1, true, 4, 0));
    assert_as_fresh(&mut workspace, &directory);
    assert!(!has_error(&workspace, "report.ry"));

    // the same text written again: read, not parsed
    write(&directory, "shapes.ry", &wider);
    let report = workspace.refresh().expect("after a rewrite");
    assert!(report.read >= 1);
    assert_eq!(work(&report), (0, false, 0, 0));

    // an overlay stands in for the file, and the disk counts again once
    // it is cleared
    workspace.set_overlay(
        "report.ry",
        mended.replace("console.print(", "console.print(nothing_here, "),
    );
    let report = workspace.refresh().expect("with an overlay");
    assert_eq!(work(&report), (1, true, 1, 3));
    assert!(
        workspace
            .diagnostics("report.ry")
            .is_some_and(|diagnostics| diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("nothing_here"))),
        "the overlay's text is what is checked"
    );
    workspace.clear_overlay("report.ry");
    let report = workspace.refresh().expect("after the overlay is cleared");
    assert_eq!(work(&report), (1, true, 1, 3));
    assert_as_fresh(&mut workspace, &directory);
    assert!(!has_error(&workspace, "report.ry"));
}
