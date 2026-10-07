//! JSON as a value, with a reader and a writer: what `std.json` decodes and
//! encodes through, what a recording, a run manifest, a bytecode file and
//! the project map are written and read as, and what the package manifests
//! of decision AC1 are read as. One reader for every file the toolchain
//! reads, so that every toolchain reads the same documents the same way.

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Boolean(bool),
    /// The number as written.
    Number(String),
    Text(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    /// What the value is, for a message: `a string`, `an object`.
    pub fn kind(&self) -> &'static str {
        match self {
            Json::Null => "null",
            Json::Boolean(_) => "a boolean",
            Json::Number(_) => "a number",
            Json::Text(_) => "a string",
            Json::Array(_) => "an array",
            Json::Object(_) => "an object",
        }
    }
}

// ----------------------------------------------------------------- reading

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    line: i64,
}

/// What went wrong and on which line.
pub type ReadError = (String, i64);

/// Read a JSON document; an error carries its detail and line.
pub fn read_json(text: &str) -> Result<Json, ReadError> {
    let mut reader = Reader {
        bytes: text.as_bytes(),
        pos: 0,
        line: 1,
    };
    reader.skip_space();
    let value = reader.value()?;
    reader.skip_space();
    if reader.pos < reader.bytes.len() {
        return Err(("text after the document".to_string(), reader.line));
    }
    Ok(value)
}

impl Reader<'_> {
    fn error<T>(&self, detail: &str) -> Result<T, ReadError> {
        Err((detail.to_string(), self.line))
    }

    fn skip_space(&mut self) {
        while let Some(&byte) = self.bytes.get(self.pos) {
            match byte {
                b'\n' => {
                    self.line += 1;
                    self.pos += 1;
                }
                b' ' | b'\t' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), ReadError> {
        if self.bytes.get(self.pos) == Some(&byte) {
            self.pos += 1;
            Ok(())
        } else {
            self.error(&format!("expected `{}`", byte as char))
        }
    }

    fn value(&mut self) -> Result<Json, ReadError> {
        match self.bytes.get(self.pos) {
            None => self.error("the document ends early"),
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::Text),
            Some(b't') => self.literal("true", Json::Boolean(true)),
            Some(b'f') => self.literal("false", Json::Boolean(false)),
            Some(b'n') => self.literal("null", Json::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(&other) => self.error(&format!("unexpected `{}`", other as char)),
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Result<Json, ReadError> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(value)
        } else {
            self.error(&format!("expected `{word}`"))
        }
    }

    fn number(&mut self) -> Result<Json, ReadError> {
        let start = self.pos;
        while let Some(&byte) = self.bytes.get(self.pos) {
            if byte.is_ascii_digit() || matches!(byte, b'-' | b'+' | b'.' | b'e' | b'E') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap_or("");
        if text.parse::<f64>().is_err() {
            return self.error("malformed number");
        }
        Ok(Json::Number(text.to_string()))
    }

    fn string(&mut self) -> Result<String, ReadError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let Some(&byte) = self.bytes.get(self.pos) else {
                return self.error("a string is not closed");
            };
            self.pos += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(&escaped) = self.bytes.get(self.pos) else {
                        return self.error("a string is not closed");
                    };
                    self.pos += 1;
                    match escaped {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let mut code = self.hex4()?;
                            if (0xD800..0xDC00).contains(&code)
                                && self.bytes[self.pos..].starts_with(b"\\u")
                            {
                                self.pos += 2;
                                let low = self.hex4()?;
                                code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                            }
                            out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        }
                        _ => return self.error("unknown escape"),
                    }
                }
                b'\n' => return self.error("a line break inside a string"),
                _ => {
                    // copy the whole UTF-8 sequence
                    let start = self.pos - 1;
                    let width = utf8_width(byte);
                    let end = (start + width).min(self.bytes.len());
                    out.push_str(
                        std::str::from_utf8(&self.bytes[start..end]).unwrap_or("\u{FFFD}"),
                    );
                    self.pos = end;
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, ReadError> {
        let end = self.pos + 4;
        if end > self.bytes.len() {
            return self.error("a short unicode escape");
        }
        let digits = std::str::from_utf8(&self.bytes[self.pos..end]).unwrap_or("");
        self.pos = end;
        u32::from_str_radix(digits, 16).or_else(|_| self.error("a bad unicode escape"))
    }

    fn array(&mut self) -> Result<Json, ReadError> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_space();
        if self.bytes.get(self.pos) == Some(&b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            self.skip_space();
            items.push(self.value()?);
            self.skip_space();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Json::Array(items));
                }
                _ => return self.error("expected `,` or `]`"),
            }
        }
    }

    fn object(&mut self) -> Result<Json, ReadError> {
        self.expect(b'{')?;
        let mut fields = Vec::new();
        self.skip_space();
        if self.bytes.get(self.pos) == Some(&b'}') {
            self.pos += 1;
            return Ok(Json::Object(fields));
        }
        loop {
            self.skip_space();
            let key = self.string()?;
            self.skip_space();
            self.expect(b':')?;
            self.skip_space();
            let value = self.value()?;
            fields.push((key, value));
            self.skip_space();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Json::Object(fields));
                }
                _ => return self.error("expected `,` or `}`"),
            }
        }
    }
}

fn utf8_width(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

// ----------------------------------------------------------------- writing

pub fn write_json(json: &Json, out: &mut String, indent: Option<usize>, depth: usize) {
    match json {
        Json::Null => out.push_str("null"),
        Json::Boolean(true) => out.push_str("true"),
        Json::Boolean(false) => out.push_str("false"),
        Json::Number(text) => out.push_str(text),
        Json::Text(text) => write_string(text, out),
        Json::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                newline(out, indent, depth + 1);
                write_json(item, out, indent, depth + 1);
            }
            newline(out, indent, depth);
            out.push(']');
        }
        Json::Object(fields) => {
            if fields.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (index, (key, value)) in fields.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                newline(out, indent, depth + 1);
                write_string(key, out);
                out.push(':');
                if indent.is_some() {
                    out.push(' ');
                }
                write_json(value, out, indent, depth + 1);
            }
            newline(out, indent, depth);
            out.push('}');
        }
    }
}

fn newline(out: &mut String, indent: Option<usize>, depth: usize) {
    if let Some(width) = indent {
        out.push('\n');
        out.push_str(&" ".repeat(width * depth));
    }
}

fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
