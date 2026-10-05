//! Every program of the example corpus must lex without a diagnostic and pass
//! the layout checks. This is the first conformance test; the parser, checker
//! and VM extend it at later milestones.

use std::path::PathBuf;

use renyi_syntax::layout::check_layout;
use renyi_syntax::{lex, SourceFile, TokenKind};

fn corpus() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("examples directory")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("ry") | Some("renyi")
            )
        })
        .collect();
    files.sort();
    assert!(
        files.len() >= 30,
        "expected the thirty-program corpus, found {}",
        files.len()
    );
    files
}

#[test]
fn every_corpus_program_lexes_cleanly() {
    for path in corpus() {
        let text = std::fs::read_to_string(&path).expect("read");
        let file = SourceFile::new(path.display().to_string(), text);
        let lexed = lex(&file.text);
        assert!(
            lexed.diagnostics.is_empty(),
            "{}: {}",
            path.display(),
            renyi_syntax::diagnostics::render_text(&file, &lexed.diagnostics)
        );
        let layout = check_layout(&file);
        assert!(
            layout.is_empty(),
            "{}: {}",
            path.display(),
            renyi_syntax::diagnostics::render_text(&file, &layout)
        );
        assert_eq!(lexed.tokens.last().map(|t| &t.kind), Some(&TokenKind::Eof));
        assert!(!lexed.tokens.iter().any(|t| t.kind == TokenKind::Error));
    }
}

#[test]
fn every_corpus_program_starts_with_a_module_header() {
    for path in corpus() {
        let text = std::fs::read_to_string(&path).expect("read");
        let lexed = lex(&text);
        assert!(
            lexed.tokens[0].is_word(renyi_syntax::Word::Module),
            "{}",
            path.display()
        );
        assert_eq!(
            lexed.tokens[1].kind,
            TokenKind::Identifier,
            "{}",
            path.display()
        );
    }
}

#[test]
fn every_corpus_program_parses_cleanly() {
    let mut report = String::new();
    for path in corpus() {
        let text = std::fs::read_to_string(&path).expect("read");
        let file = SourceFile::new(path.display().to_string(), text);
        let parsed = renyi_syntax::parse(&file.text);
        if !parsed.diagnostics.is_empty() {
            report.push_str(&renyi_syntax::diagnostics::render_text(
                &file,
                &parsed.diagnostics,
            ));
            continue;
        }
        assert!(!parsed.module.name.is_empty(), "{}", path.display());
        assert!(!parsed.module.items.is_empty(), "{}", path.display());
        // every public item of the corpus carries a purpose clause
        for item in &parsed.module.items {
            let (public, purpose, what) = match item {
                renyi_syntax::ast::Item::Function(f) => {
                    (f.public, f.docs.purpose.is_some(), f.name.text.clone())
                }
                renyi_syntax::ast::Item::Type(t) => {
                    (t.public, t.docs.purpose.is_some(), t.name.text.clone())
                }
                renyi_syntax::ast::Item::Ability(a) => {
                    (a.public, a.docs.purpose.is_some(), a.name.text.clone())
                }
                renyi_syntax::ast::Item::Constant(c) => {
                    (c.public, c.docs.purpose.is_some(), c.name.text.clone())
                }
                _ => continue,
            };
            assert!(
                !public || purpose,
                "{}: public {what} has no purpose clause",
                path.display()
            );
        }
    }
    assert!(
        report.is_empty(),
        "programs with parse diagnostics:\n{report}"
    );
}
