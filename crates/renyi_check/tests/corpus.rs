//! Every program of the example corpus checks without an error: the first
//! acceptance test of the type and effect checker.

use std::path::PathBuf;

use renyi_syntax::SourceFile;

fn corpus() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("examples directory")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("ry"))
        .collect();
    files.sort();
    assert!(files.len() >= 30);
    files
}

#[test]
fn every_corpus_program_checks_cleanly() {
    let mut report = String::new();
    for path in corpus() {
        let text = std::fs::read_to_string(&path).expect("read");
        let file = SourceFile::new(path.display().to_string(), text);
        let diagnostics = renyi_check::check_file(&file);
        if !diagnostics.is_empty() {
            report.push_str(&renyi_syntax::diagnostics::render_text(&file, &diagnostics));
        }
    }
    assert!(report.is_empty(), "\n{report}");
}
