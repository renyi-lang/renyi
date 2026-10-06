//! Layout checks the formatter would fix: line width and trailing whitespace.

use crate::diagnostics::Diagnostic;
use crate::span::{SourceFile, Span};

pub const MAX_COLUMNS: usize = 100;

pub fn check_layout(file: &SourceFile) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for number in 1..=file.line_count() {
        let line = file.line(number);
        let start = file.line_start(number);
        let width = line.chars().count();
        if width > MAX_COLUMNS {
            diagnostics.push(
                Diagnostic::warning(
                    "line-width",
                    format!("line is {width} columns; the limit is {MAX_COLUMNS}"),
                    Span::new(start, start + line.len()),
                )
                .with_fix(
                    "run `renyi format`, or break inside parentheses or before `where`, `and` or `or`",
                ),
            );
        }
        let trimmed = line.trim_end_matches([' ', '\t']);
        if trimmed.len() != line.len() {
            diagnostics.push(
                Diagnostic::warning(
                    "trailing-whitespace",
                    "trailing whitespace",
                    Span::new(start + trimmed.len(), start + line.len()),
                )
                .with_fix("run `renyi format`"),
            );
        }
    }
    diagnostics
}
