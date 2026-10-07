//! Byte spans and their translation to lines and columns.

/// A half-open byte range `start..end` into a source text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    /// The smallest span covering both.
    pub fn join(self, other: Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }

    pub fn len(self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.end == self.start
    }
}

/// A one-based line and column. Columns count characters, not bytes, so that
/// the 100-column limit means what a reader sees.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// A source file with an index of line starts.
#[derive(Clone, Debug)]
pub struct SourceFile {
    pub name: String,
    pub text: String,
    line_starts: Vec<usize>,
    /// The package the file belongs to (decision AC1); `None` for a file
    /// of the program itself.
    pub package: Option<Package>,
}

/// A dependency a file was read from: its name and its version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub version: String,
}

impl SourceFile {
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> SourceFile {
        let text = text.into();
        let mut line_starts = vec![0];
        for (offset, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(offset + 1);
            }
        }
        SourceFile {
            name: name.into(),
            text,
            line_starts,
            package: None,
        }
    }

    /// The same file, tagged with the package it belongs to.
    pub fn in_package(mut self, package: Package) -> SourceFile {
        self.package = Some(package);
        self
    }

    /// The line and column of a byte offset.
    pub fn position(&self, offset: usize) -> Position {
        let offset = offset.min(self.text.len());
        let line = match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index - 1,
        };
        let column = self.text[self.line_starts[line]..offset].chars().count() + 1;
        Position {
            line: line + 1,
            column,
        }
    }

    /// The number of lines, counting a final line without a line break.
    pub fn line_count(&self) -> usize {
        let last_start = *self.line_starts.last().unwrap_or(&0);
        if last_start == self.text.len() && !self.text.is_empty() {
            self.line_starts.len() - 1
        } else {
            self.line_starts.len()
        }
    }

    /// The text of a one-based line, without its line break.
    pub fn line(&self, number: usize) -> &str {
        let start = self.line_starts[number - 1];
        let end = self
            .line_starts
            .get(number)
            .map_or(self.text.len(), |next| next - 1);
        &self.text[start..end]
    }

    /// The byte offset where a one-based line starts.
    pub fn line_start(&self, number: usize) -> usize {
        self.line_starts[number - 1]
    }

    pub fn slice(&self, span: Span) -> &str {
        &self.text[span.start..span.end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_count_characters() {
        let file = SourceFile::new("t.ry", "ab\nc\u{e9}d\n");
        assert_eq!(file.position(0), Position { line: 1, column: 1 });
        assert_eq!(file.position(3), Position { line: 2, column: 1 });
        assert_eq!(file.position(6), Position { line: 2, column: 3 });
        assert_eq!(file.line(2), "c\u{e9}d");
        assert_eq!(file.line_count(), 2);
    }
}
