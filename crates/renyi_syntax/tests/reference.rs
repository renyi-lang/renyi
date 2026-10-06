//! The language reference against the grammar file and the diagnostics.
//!
//! `docs/reference.md` is normative, so what it quotes must be what exists:
//! every rule in one of its ```ebnf blocks is a rule of `docs/grammar.ebnf`,
//! character for character once whitespace is folded, and every rule of the
//! grammar file is quoted; the codes of its appendix A are exactly the codes
//! the crates emit, each with the severity it is emitted with.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

fn read(relative: &str) -> String {
    let path = root().join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

// ------------------------------------------------------------ the grammar

/// The text without its `/* ... */` comments.
fn without_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find("*/").expect("a comment that does not end");
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

/// The rules of an EBNF text in order: a rule starts at a line whose first
/// character is not a space and which holds `::=`; the indented lines after
/// it continue it. Each body has its whitespace folded to single spaces.
fn rules(text: &str) -> Vec<(String, String)> {
    let mut rules: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        if !line.starts_with(' ') && line.contains("::=") {
            let (name, body) = line.split_once("::=").expect("checked above");
            rules.push((name.trim().to_string(), body.to_string()));
        } else if line.trim().is_empty() {
            continue;
        } else {
            let (_, body) = rules
                .last_mut()
                .unwrap_or_else(|| panic!("text before the first rule: {line:?}"));
            body.push(' ');
            body.push_str(line);
        }
    }
    rules
        .into_iter()
        .map(|(name, body)| (name, fold(&body)))
        .collect()
}

fn fold(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The content of every ```ebnf block of a Markdown text.
fn ebnf_blocks(markdown: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for line in markdown.lines() {
        if inside {
            if line.starts_with("```") {
                inside = false;
            } else {
                out.push_str(line);
                out.push('\n');
            }
        } else if line.trim_end() == "```ebnf" {
            inside = true;
        }
    }
    assert!(!inside, "a ```ebnf block that does not end");
    out
}

#[test]
fn the_reference_quotes_every_rule_of_the_grammar_verbatim() {
    let grammar: BTreeMap<String, String> = rules(&without_comments(&read("docs/grammar.ebnf")))
        .into_iter()
        .collect();
    let quoted = rules(&ebnf_blocks(&read("docs/reference.md")));
    assert!(
        grammar.len() >= 60,
        "the grammar file has {} rules",
        grammar.len()
    );
    assert!(!quoted.is_empty(), "the reference quotes no rule");
    let mut seen = BTreeSet::new();
    for (name, body) in &quoted {
        match grammar.get(name) {
            None => panic!("the reference quotes `{name}`, which the grammar file does not define"),
            Some(expected) => assert!(
                body == expected,
                "the reference's `{name}` differs from the grammar file's:\n  reference: {body}\n  grammar:   {expected}"
            ),
        }
        assert!(
            seen.insert(name.clone()),
            "the reference quotes `{name}` twice"
        );
    }
    let missing: Vec<&String> = grammar
        .keys()
        .filter(|name| !seen.contains(*name))
        .collect();
    assert!(
        missing.is_empty(),
        "rules of the grammar file the reference does not quote: {missing:?}"
    );
}

// ---------------------------------------------------------- the diagnostics

#[derive(Default)]
struct Emitted {
    errors: BTreeSet<String>,
    warnings: BTreeSet<String>,
}

fn source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(dir).unwrap_or_else(|error| panic!("cannot list {}: {error}", dir.display()))
    {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            source_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

fn is_code(text: &str) -> bool {
    !text.is_empty()
        && text
            .split('-')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_lowercase()))
}

fn is_identifier_byte(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The codes a source file emits: the first text argument of a call whose
/// name holds `error` or `warning` (`Diagnostic::error`, `self.error_fix`,
/// `symbol_error`, `warning_with_fix`, ...), or the argument after a single
/// identifier (`error_with_fix(module, "code", ...)`). The tests of the file
/// (`#[cfg(test)]`) are not read.
fn codes_in(text: &str, emitted: &mut Emitted) {
    let text = text.split("#[cfg(test)]").next().expect("a first piece");
    for (index, _) in text.match_indices('(') {
        let name_start = text[..index]
            .rfind(|c: char| !is_identifier_byte(c))
            .map_or(0, |at| at + 1);
        let callee = &text[name_start..index];
        let warning = callee.contains("warning");
        if !warning && !callee.contains("error") {
            continue;
        }
        let mut rest = text[index + 1..].trim_start();
        if !rest.starts_with('"') {
            let ident_end = rest.find(|c: char| !is_identifier_byte(c)).unwrap_or(0);
            if ident_end == 0 || !rest[ident_end..].starts_with(',') {
                continue;
            }
            rest = rest[ident_end + 1..].trim_start();
        }
        let Some(literal) = rest.strip_prefix('"') else {
            continue;
        };
        let Some(end) = literal.find('"') else {
            continue;
        };
        let code = &literal[..end];
        if is_code(code) {
            let set = if warning {
                &mut emitted.warnings
            } else {
                &mut emitted.errors
            };
            set.insert(code.to_string());
        }
    }
}

fn emitted_codes() -> Emitted {
    let mut files = Vec::new();
    for entry in fs::read_dir(root().join("crates")).expect("the crates directory") {
        let src = entry.expect("a directory entry").path().join("src");
        if src.is_dir() {
            source_files(&src, &mut files);
        }
    }
    assert!(files.len() >= 20, "{} source files", files.len());
    let mut emitted = Emitted::default();
    for file in files {
        let text = fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", file.display()));
        codes_in(&text, &mut emitted);
    }
    emitted
}

/// The rows of appendix A: the code and its severity letter.
fn appendix_codes(markdown: &str) -> BTreeMap<String, char> {
    let appendix = markdown
        .split("## Appendix A")
        .nth(1)
        .expect("appendix A")
        .split("## Appendix B")
        .next()
        .expect("appendix B after appendix A");
    let mut codes = BTreeMap::new();
    for line in appendix.lines() {
        let Some(rest) = line.strip_prefix("| `") else {
            continue;
        };
        let Some((code, rest)) = rest.split_once("` | ") else {
            continue;
        };
        let severity = rest.chars().next().expect("a severity");
        assert!(matches!(severity, 'E' | 'W'), "appendix A: {line}");
        assert!(
            codes.insert(code.to_string(), severity).is_none(),
            "appendix A lists `{code}` twice"
        );
    }
    codes
}

#[test]
fn appendix_a_lists_every_diagnostic_code_the_crates_emit() {
    let emitted = emitted_codes();
    let listed = appendix_codes(&read("docs/reference.md"));
    let all: BTreeSet<&String> = emitted.errors.iter().chain(&emitted.warnings).collect();
    assert!(all.len() >= 90, "{} codes emitted", all.len());
    let missing: Vec<&&String> = all
        .iter()
        .filter(|code| !listed.contains_key(**code))
        .collect();
    let extra: Vec<&String> = listed.keys().filter(|code| !all.contains(code)).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "appendix A differs from the crates:\n  emitted but not listed: {missing:?}\n  listed but not emitted: {extra:?}"
    );
    let both: Vec<&String> = emitted.errors.intersection(&emitted.warnings).collect();
    assert!(
        both.is_empty(),
        "emitted both as an error and as a warning: {both:?}"
    );
    for (code, severity) in &listed {
        let expected = if emitted.warnings.contains(code) {
            'W'
        } else {
            'E'
        };
        assert!(
            *severity == expected,
            "appendix A marks `{code}` as {severity}; the crates emit it as {expected}"
        );
    }
}
