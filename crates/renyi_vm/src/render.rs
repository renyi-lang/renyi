//! `ToText` and `Compare` as the VM performs them: a declared `to_text` or
//! `compare` method of the value's type (a user implementation or a library
//! primitive) when there is one, the derived form otherwise. A derived
//! `ToText` prints a variant as its bare name and a record in constructor
//! form; a derived `Compare` orders records by the fields `can Compare by`
//! names, or by every field, and variants by position.

use std::cmp::Ordering;

use crate::natives::time::{civil_from_days, instant_text};
use crate::types::TypeShape;
use crate::value::{Native, Value};
use crate::vm::{Interrupt, Vm};

impl Vm<'_> {
    /// The text a value prints as in interpolation and `console.print`.
    pub fn to_text(&mut self, value: &Value) -> Result<String, Interrupt> {
        if let Some(ty) = value.type_id() {
            if let Some(function) = self.program.method(ty, "to_text") {
                return match self.call_function(function, vec![value.clone()])? {
                    Value::Text(text) => Ok(text.to_string()),
                    Value::Failure(error) => {
                        let shown = self.render(&error, false)?;
                        Err(Interrupt::crash(format!("`to_text` failed: {shown}")))
                    }
                    other => self.render(&other, false),
                };
            }
        }
        self.render(value, false)
    }

    /// The derived rendering; `nested` quotes text, as inside a list or a
    /// constructor form.
    pub fn render(&mut self, value: &Value, nested: bool) -> Result<String, Interrupt> {
        Ok(match value {
            Value::Nothing => "nothing".to_string(),
            Value::Boolean(true) => "true".to_string(),
            Value::Boolean(false) => "false".to_string(),
            Value::Integer(value) => value.to_string(),
            Value::Decimal(value) => value.to_string(),
            Value::Float(value) => float_text(*value),
            Value::Text(text) => {
                if nested {
                    quoted(text)
                } else {
                    text.to_string()
                }
            }
            Value::Bytes(bytes) => format!("{} bytes", bytes.len()),
            Value::List(items) => {
                let shown = self.render_all(items.iter(), true)?;
                format!("[{}]", shown.join(", "))
            }
            Value::Set(items) => {
                let shown = self.render_all(items.iter(), true)?;
                format!("[{}].to_set()", shown.join(", "))
            }
            Value::Map(entries) => {
                let mut shown = Vec::with_capacity(entries.len());
                for (key, value) in entries.iter() {
                    shown.push(format!(
                        "{}: {}",
                        self.nested_text(key)?,
                        self.nested_text(value)?
                    ));
                }
                format!("{{{}}}", shown.join(", "))
            }
            Value::Range(range) => {
                if range.by == crate::integer::Int::Small(1) {
                    format!("{} to {}", range.from, range.to)
                } else {
                    format!("{} to {} by {}", range.from, range.to, range.by)
                }
            }
            Value::Pair(pair) => format!(
                "Pair(left: {}, right: {})",
                self.nested_text(&pair.0)?,
                self.nested_text(&pair.1)?
            ),
            Value::Record(record) => {
                let meta = self.program.types.meta(record.ty);
                let names: Vec<String> = match &meta.shape {
                    TypeShape::Record(fields) => fields.iter().map(|f| f.name.clone()).collect(),
                    _ => Vec::new(),
                };
                let name = meta.name.clone();
                self.constructor_form(&name, &names, &record.fields)?
            }
            Value::Variant(variant) => {
                let meta = self.program.types.meta(variant.ty);
                let (name, names): (String, Vec<String>) = match &meta.shape {
                    TypeShape::Sum(variants) => match variants.get(variant.tag) {
                        Some(v) => (
                            v.name.clone(),
                            v.fields.iter().map(|f| f.name.clone()).collect(),
                        ),
                        None => (meta.name.clone(), Vec::new()),
                    },
                    _ => (meta.name.clone(), Vec::new()),
                };
                self.constructor_form(&name, &names, &variant.fields)?
            }
            Value::Duration(ms) => duration_text(*ms),
            Value::Instant(ms) => instant_text(*ms),
            Value::Function(id) => format!("<function {}>", self.qualified(*id)),
            Value::Native(native) => match &**native {
                Native::CsvRow { line, .. } => format!("Row(line: {line})"),
                Native::Iterator(_) => "<iterator>".to_string(),
                Native::Deadline(_, limit) => format!("<deadline after {}>", duration_text(*limit)),
                Native::Connection { path, .. } => format!("<connection {path}>"),
            },
            Value::Failure(error) => format!("failure({})", self.nested_text(error)?),
        })
    }

    fn render_all<'a>(
        &mut self,
        items: impl Iterator<Item = &'a Value>,
        nested: bool,
    ) -> Result<Vec<String>, Interrupt> {
        let mut shown = Vec::new();
        for item in items {
            shown.push(if nested {
                self.nested_text(item)?
            } else {
                self.render(item, false)?
            });
        }
        Ok(shown)
    }

    /// A value inside another: its own `to_text` when it has one, else the
    /// derived form with text quoted.
    fn nested_text(&mut self, value: &Value) -> Result<String, Interrupt> {
        if let Some(ty) = value.type_id() {
            if self.program.method(ty, "to_text").is_some() {
                return self.to_text(value);
            }
        }
        self.render(value, true)
    }

    fn constructor_form(
        &mut self,
        name: &str,
        field_names: &[String],
        fields: &[Value],
    ) -> Result<String, Interrupt> {
        if fields.is_empty() {
            return Ok(name.to_string());
        }
        let mut shown = Vec::with_capacity(fields.len());
        for (index, value) in fields.iter().enumerate() {
            let field = field_names
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("field{index}"));
            shown.push(format!("{field}: {}", self.nested_text(value)?));
        }
        Ok(format!("{name}({})", shown.join(", ")))
    }

    // ------------------------------------------------------------ ordering

    /// The order of two values: numbers, text, booleans, durations and
    /// instants by value, lists and pairs lexicographically, records and
    /// variants by a declared `compare` or the derived one.
    pub fn compare(&mut self, left: &Value, right: &Value) -> Result<Ordering, Interrupt> {
        Ok(match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => a.compare(b),
            (Value::Decimal(a), Value::Decimal(b)) => a.compare(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
            (Value::Integer(a), Value::Decimal(b)) => {
                crate::decimal::Decimal::from_int(a).compare(b)
            }
            (Value::Decimal(a), Value::Integer(b)) => {
                a.compare(&crate::decimal::Decimal::from_int(b))
            }
            (Value::Float(a), Value::Integer(b)) => {
                a.partial_cmp(&b.to_f64()).unwrap_or(Ordering::Equal)
            }
            (Value::Integer(a), Value::Float(b)) => {
                a.to_f64().partial_cmp(b).unwrap_or(Ordering::Equal)
            }
            (Value::Float(a), Value::Decimal(b)) => {
                a.partial_cmp(&b.to_f64()).unwrap_or(Ordering::Equal)
            }
            (Value::Decimal(a), Value::Float(b)) => {
                a.to_f64().partial_cmp(b).unwrap_or(Ordering::Equal)
            }
            (Value::Text(a), Value::Text(b)) => a.cmp(b),
            (Value::Boolean(a), Value::Boolean(b)) => a.cmp(b),
            (Value::Duration(a), Value::Duration(b)) | (Value::Instant(a), Value::Instant(b)) => {
                a.cmp(b)
            }
            (Value::List(a), Value::List(b)) => {
                let (a, b) = (a.clone(), b.clone());
                self.compare_sequences(&a, &b)?
            }
            (Value::Pair(a), Value::Pair(b)) => {
                let (a, b) = (a.clone(), b.clone());
                match self.compare(&a.0, &b.0)? {
                    Ordering::Equal => self.compare(&a.1, &b.1)?,
                    other => other,
                }
            }
            (Value::Record(a), Value::Record(b)) if a.ty == b.ty => {
                if let Some(function) = self.program.method(a.ty, "compare") {
                    return self.declared_compare(function, left, right);
                }
                let (a, b) = (a.clone(), b.clone());
                let indices: Vec<usize> = match &self.program.types.meta(a.ty).compare_by {
                    Some(indices) => indices.clone(),
                    None => (0..a.fields.len()).collect(),
                };
                let mut ordering = Ordering::Equal;
                for index in indices {
                    let (x, y) = (&a.fields[index], &b.fields[index]);
                    ordering = self.compare(x, y)?;
                    if ordering != Ordering::Equal {
                        break;
                    }
                }
                ordering
            }
            (Value::Variant(a), Value::Variant(b)) if a.ty == b.ty => {
                if let Some(function) = self.program.method(a.ty, "compare") {
                    return self.declared_compare(function, left, right);
                }
                let (a, b) = (a.clone(), b.clone());
                match a.tag.cmp(&b.tag) {
                    Ordering::Equal => self.compare_sequences(&a.fields, &b.fields)?,
                    other => other,
                }
            }
            (Value::Nothing, Value::Nothing) => Ordering::Equal,
            (Value::Nothing, _) => Ordering::Less,
            (_, Value::Nothing) => Ordering::Greater,
            (left, right) => {
                return Err(Interrupt::crash(format!(
                    "cannot order {} and {}",
                    self.describe(left),
                    self.describe(right)
                )))
            }
        })
    }

    fn compare_sequences(&mut self, a: &[Value], b: &[Value]) -> Result<Ordering, Interrupt> {
        for (x, y) in a.iter().zip(b.iter()) {
            let ordering = self.compare(x, y)?;
            if ordering != Ordering::Equal {
                return Ok(ordering);
            }
        }
        Ok(a.len().cmp(&b.len()))
    }

    fn declared_compare(
        &mut self,
        function: usize,
        left: &Value,
        right: &Value,
    ) -> Result<Ordering, Interrupt> {
        match self.call_function(function, vec![left.clone(), right.clone()])? {
            Value::Variant(variant) if variant.ty == self.program.builtins.ordering => {
                Ok(match variant.tag {
                    0 => Ordering::Less,
                    1 => Ordering::Equal,
                    _ => Ordering::Greater,
                })
            }
            other => Err(Interrupt::crash(format!(
                "`compare` produced {}, not an Ordering",
                other.kind_name()
            ))),
        }
    }
}

/// The shortest text that reads back as the value, always with a decimal
/// point.
pub fn float_text(value: f64) -> String {
    if !value.is_finite() {
        return format!("{value}");
    }
    let text = format!("{value:?}");
    if text.contains('e') {
        let plain = format!("{value}");
        if plain.contains('.') {
            plain
        } else {
            format!("{plain}.0")
        }
    } else {
        text
    }
}

pub fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '{' => out.push_str("\\{"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

pub fn duration_text(ms: i64) -> String {
    if ms.abs() < 1000 {
        format!("{ms}ms")
    } else if ms % 1000 == 0 {
        format!("{}s", ms / 1000)
    } else {
        format!("{}.{:03}s", ms / 1000, (ms % 1000).abs())
    }
}

/// `YYYY-MM-DD` of a day count since 1970-01-01.
pub fn date_text(days: i64) -> String {
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}
