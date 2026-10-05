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

fn strip_spans(debug: &str) -> String {
    // spans differ between a source and its formatted text; everything else must match
    let mut out = String::new();
    let mut rest = debug;
    while let Some(index) = rest.find("Span {") {
        out.push_str(&rest[..index]);
        let after = &rest[index..];
        let close = after.find('}').expect("span closes");
        out.push('_');
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

#[test]
fn formatting_the_corpus_is_idempotent_and_preserves_the_tree() {
    let mut report = String::new();
    for path in corpus() {
        let text = std::fs::read_to_string(&path).expect("read");
        let file = SourceFile::new(path.display().to_string(), text.clone());
        let once = match renyi_syntax::format(&file) {
            Ok(formatted) => formatted,
            Err(diagnostics) => {
                report.push_str(&renyi_syntax::diagnostics::render_text(&file, &diagnostics));
                continue;
            }
        };
        let twice = renyi_syntax::format(&SourceFile::new("formatted.ry", once.clone()))
            .expect("formatted text parses");
        if once != twice {
            report.push_str(&format!(
                "{}: formatting is not idempotent\n",
                path.display()
            ));
        }
        let before = strip_spans(&format!("{:?}", renyi_syntax::parse(&text).module));
        let after = strip_spans(&format!("{:?}", renyi_syntax::parse(&once).module));
        if before != after {
            report.push_str(&format!(
                "{}: formatting changed the syntax tree\n",
                path.display()
            ));
        }
        for (number, line) in once.lines().enumerate() {
            if line.chars().count() > 100 && !line.contains('"') {
                report.push_str(&format!(
                    "{}:{}: formatted line is wider than 100 columns\n",
                    path.display(),
                    number + 1
                ));
            }
        }
    }
    assert!(report.is_empty(), "{report}");
}

#[test]
fn the_corpus_is_in_canonical_form() {
    let mut report = String::new();
    for path in corpus() {
        let text = std::fs::read_to_string(&path).expect("read");
        let file = SourceFile::new(path.display().to_string(), text.clone());
        if let Ok(formatted) = renyi_syntax::format(&file) {
            if formatted != text {
                report.push_str(&format!(
                    "{} differs from its formatted form (run `renyi format`)\n",
                    path.display()
                ));
            }
        }
    }
    assert!(report.is_empty(), "{report}");
}

/// Braces and brackets balance outside string literals: a cheap guard that
/// the JSON encoder closes every object and array it opens.
fn json_is_balanced(text: &str) -> bool {
    let mut depth: i64 = 0;
    let mut in_string = false;
    let mut escaped = false;
    for character in text.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '{' | '[' => depth += 1,
            '}' | ']' => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            return false;
        }
    }
    depth == 0 && !in_string
}

#[test]
fn every_corpus_program_encodes_as_json() {
    for path in corpus() {
        let text = std::fs::read_to_string(&path).expect("read");
        let file = SourceFile::new(path.display().to_string(), text);
        let parsed = renyi_syntax::parse(&file.text);
        let json = renyi_syntax::module_to_json(&file, &parsed.module);
        assert!(
            json.starts_with("{\n  \"node\": \"Module\""),
            "{}",
            path.display()
        );
        assert!(json.ends_with("}\n"), "{}", path.display());
        assert!(
            json_is_balanced(&json),
            "{}: unbalanced JSON",
            path.display()
        );
        assert!(
            json.contains("\"node\": \"Function\""),
            "{}: no function in the JSON",
            path.display()
        );
    }
}
