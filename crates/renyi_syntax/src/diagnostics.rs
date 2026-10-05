//! Diagnostics with a location, a stable code and a suggested fix, rendered
//! as text for people and as JSON for tools (decision D3).

use crate::span::{SourceFile, Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub span: Span,
    pub fix: Option<String>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>, span: Span) -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            code,
            message: message.into(),
            span,
            fix: None,
        }
    }

    pub fn warning(code: &'static str, message: impl Into<String>, span: Span) -> Diagnostic {
        Diagnostic {
            severity: Severity::Warning,
            code,
            message: message.into(),
            span,
            fix: None,
        }
    }

    pub fn with_fix(mut self, fix: impl Into<String>) -> Diagnostic {
        self.fix = Some(fix.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

/// `file:line:column: severity [code]: message`, with the fix on a second line.
pub fn render_text(file: &SourceFile, diagnostics: &[Diagnostic]) -> String {
    let mut out = String::new();
    for diagnostic in diagnostics {
        let position = file.position(diagnostic.span.start);
        out.push_str(&format!(
            "{}:{}:{}: {} [{}]: {}\n",
            file.name,
            position.line,
            position.column,
            severity_name(diagnostic.severity),
            diagnostic.code,
            diagnostic.message
        ));
        if let Some(fix) = &diagnostic.fix {
            out.push_str(&format!("  fix: {fix}\n"));
        }
    }
    out
}

/// A JSON array with one object per diagnostic.
pub fn render_json(file: &SourceFile, diagnostics: &[Diagnostic]) -> String {
    let mut out = String::from("[");
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let start = file.position(diagnostic.span.start);
        let end = file.position(diagnostic.span.end);
        out.push_str(&format!(
            "\n  {{\"file\": {}, \"line\": {}, \"column\": {}, \"end_line\": {}, \"end_column\": {}, \
             \"severity\": {}, \"code\": {}, \"message\": {}, \"fix\": {}}}",
            json_string(&file.name),
            start.line,
            start.column,
            end.line,
            end.column,
            json_string(severity_name(diagnostic.severity)),
            json_string(diagnostic.code),
            json_string(&diagnostic.message),
            diagnostic.fix.as_deref().map_or("null".to_string(), json_string),
        ));
    }
    if !diagnostics.is_empty() {
        out.push('\n');
    }
    out.push_str("]\n");
    out
}

pub fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
