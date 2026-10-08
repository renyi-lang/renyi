"""The profile-guided round on strings and JSON, written and not yet applied.

The handoff's section "The profile-guided round on strings and JSON"
describes the change; this script makes it: it rewrites
`crates/renyi_vm/src/natives/json.rs` and patches `crates/renyi_json`,
`value.rs`, the prelude natives, `vm.rs`, the regex natives and the
machine-code runtime. Every anchor is verified before any file is
written; a mismatch writes nothing and names the anchor. Delete this
script in the commit that lands the change.

usage: python tools/apply_perf_round.py
"""

import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

JSON_RS = r'''//! `std.json`: a reader and a writer for JSON text, decoding by the type the
//! context expects (recorded by the checker as `Target::Result`) and
//! encoding by the value's shape, with the derivation rules of the library
//! sketch (section 7). The encoder walks a value once into a sink: the tree
//! of `encode`, which the recording, the HTTP client, the server and the
//! Python bridge keep, or the text of `render`, written as the value is
//! walked.

use std::borrow::Cow;
use std::fmt::{Display, Write};
use std::rc::Rc;

use indexmap::{IndexMap, IndexSet};
use renyi_check::types::Ty;

use super::{arg, base64_decode, base64_encode, crash, text};
use crate::decimal::Decimal;
use crate::extension::Native;
use crate::integer::Int;
use crate::natives::time::{instant_text, parse_instant_text};
use crate::render::float_text;
use crate::types::{FieldMeta, TypeMeta, TypeShape};
use crate::value::Value;
use crate::vm::{Interrupt, Vm};

/// The natives of `std.json` (decision AK2): the declared function
/// each implements, by module, name and the type of its first parameter.
pub(crate) const NATIVES: &[Native] = &[
    Native::function("std.json", "parse", parse),
    Native::function("std.json", "parse_with", parse_with),
    Native::function("std.json", "render", render),
    Native::function("std.json", "render_indented", render_indented),
    Native::function("std.json", "render_with", render_with),
];

pub use renyi_json::{read_json, write_json, Json, ReadError};

// ------------------------------------------------------------------ naming

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Naming {
    Exact,
    Camel,
    Kebab,
}

impl Naming {
    fn of(value: &Value) -> Naming {
        match value {
            Value::Variant(variant) if variant.tag == 1 => Naming::Camel,
            Value::Variant(variant) if variant.tag == 2 => Naming::Kebab,
            _ => Naming::Exact,
        }
    }

    /// The key of a field: its external name when it declares one, else its
    /// name as this naming spells it; the exact spelling is borrowed.
    fn key(self, field: &FieldMeta) -> Cow<'_, str> {
        if let Some(external) = &field.external_name {
            return Cow::Borrowed(external);
        }
        match self {
            Naming::Exact => Cow::Borrowed(&field.name),
            Naming::Kebab => Cow::Owned(field.name.replace('_', "-")),
            Naming::Camel => {
                let mut out = String::new();
                for (index, part) in field.name.split('_').enumerate() {
                    if index == 0 {
                        out.push_str(part);
                    } else {
                        let mut chars = part.chars();
                        if let Some(first) = chars.next() {
                            out.extend(first.to_uppercase());
                            out.push_str(chars.as_str());
                        }
                    }
                }
                Cow::Owned(out)
            }
        }
    }
}

// ---------------------------------------------------------------- decoding

/// The outcome of decoding: a value, or the `JsonError` to fail with.
pub type Decoded = Result<Value, Value>;

/// Where a value stands in the document, as a chain from the root that is
/// rendered only for a message: `$`, `$[3]`, `$.name`.
enum Path<'a> {
    Root(&'a str),
    Index(&'a Path<'a>, usize),
    Key(&'a Path<'a>, &'a str),
}

impl Path<'_> {
    fn render(&self) -> String {
        match self {
            Path::Root(root) => root.to_string(),
            Path::Index(parent, index) => format!("{}[{index}]", parent.render()),
            Path::Key(parent, key) => format!("{}.{key}", parent.render()),
        }
    }
}

fn json_error(vm: &Vm, variant: &str, fields: Vec<Value>) -> Result<Decoded, Interrupt> {
    Ok(Err(vm.library_variant(
        "std.json",
        "JsonError",
        variant,
        fields,
    )?))
}

fn mismatch(vm: &Vm, path: &Path, expected: &str, found: &Json) -> Result<Decoded, Interrupt> {
    json_error(
        vm,
        "Mismatch",
        vec![
            Value::text(path.render()),
            Value::text(expected),
            Value::text(found.kind()),
        ],
    )
}

fn decimal_of(text: &str) -> Option<Decimal> {
    match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => {
            let shift: i64 = exponent.parse().ok()?;
            Decimal::parse(mantissa)?.scaled(shift).ok()
        }
        None => Decimal::parse(text),
    }
}

/// The type of a field of the type: as declared, or with the type's
/// arguments put in for its parameters when it has any.
fn field_type<'t>(ty: &'t Ty, meta: &TypeMeta, args: &[Ty]) -> Cow<'t, Ty> {
    if meta.params.is_empty() {
        return Cow::Borrowed(ty);
    }
    Cow::Owned(ty.substitute(&|param| {
        meta.params
            .iter()
            .position(|p| *p == param)
            .and_then(|index| args.get(index).cloned())
    }))
}

/// Decode a document into a value of the type, with the derivation rules of
/// the library sketch; `path` names the root for error messages (`$`).
pub fn decode(
    vm: &mut Vm,
    json: &Json,
    ty: &Ty,
    path: &str,
    naming: Naming,
) -> Result<Decoded, Interrupt> {
    decode_at(vm, json, ty, &Path::Root(path), naming)
}

fn decode_at(
    vm: &mut Vm,
    json: &Json,
    ty: &Ty,
    path: &Path,
    naming: Naming,
) -> Result<Decoded, Interrupt> {
    let program = vm.program;
    let b = &program.builtins;
    let (id, args) = match ty {
        Ty::Maybe(inner) => {
            if *json == Json::Null {
                return Ok(Ok(Value::Nothing));
            }
            return decode_at(vm, json, inner, path, naming);
        }
        Ty::App(id, args) => (*id, args.as_slice()),
        Ty::Unit => {
            return Ok(match json {
                Json::Null => Ok(Value::Nothing),
                _ => return mismatch(vm, path, "null", json),
            })
        }
        _ => {
            return Err(crash(
                "cannot decode JSON into a type the context leaves open",
            ))
        }
    };
    if id == b.duration {
        return Ok(match json {
            Json::Number(text) => match text.parse::<i64>() {
                Ok(ms) => Ok(Value::Duration(ms)),
                Err(_) => return mismatch(vm, path, "a Duration in milliseconds", json),
            },
            _ => return mismatch(vm, path, "a Duration in milliseconds", json),
        });
    }
    if id == b.integer {
        return Ok(match json {
            Json::Number(text) if !text.contains(['.', 'e', 'E']) => match Int::parse(text) {
                Some(value) => Ok(Value::Integer(value)),
                None => return mismatch(vm, path, "Integer", json),
            },
            _ => return mismatch(vm, path, "Integer", json),
        });
    }
    if id == b.decimal {
        return Ok(match json {
            Json::Number(text) => match decimal_of(text) {
                Some(value) => Ok(Value::decimal(value)),
                None => return mismatch(vm, path, "Decimal", json),
            },
            _ => return mismatch(vm, path, "Decimal", json),
        });
    }
    if id == b.float {
        return Ok(match json {
            Json::Number(text) => match text.parse::<f64>() {
                // a Float never holds infinity: a number past its range
                // does not fit
                Ok(value) if value.is_finite() => Ok(Value::Float(value)),
                _ => return mismatch(vm, path, "Float", json),
            },
            _ => return mismatch(vm, path, "Float", json),
        });
    }
    if id == b.text {
        return Ok(match json {
            Json::Text(text) => Ok(Value::text(text)),
            _ => return mismatch(vm, path, "Text", json),
        });
    }
    if id == b.boolean {
        return Ok(match json {
            Json::Boolean(value) => Ok(Value::Boolean(*value)),
            _ => return mismatch(vm, path, "Boolean", json),
        });
    }
    if id == b.bytes {
        return Ok(match json {
            Json::Text(text) => match base64_decode(text) {
                Some(bytes) => Ok(Value::Bytes(Rc::from(bytes))),
                None => return mismatch(vm, path, "Bytes as base64", json),
            },
            _ => return mismatch(vm, path, "Bytes as base64", json),
        });
    }
    let open = Ty::Error;
    if id == b.list || id == b.set {
        let Json::Array(items) = json else {
            return mismatch(vm, path, "List", json);
        };
        let item_ty = args.first().unwrap_or(&open);
        let mut values = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            match decode_at(vm, item, item_ty, &Path::Index(path, index), naming)? {
                Ok(value) => values.push(value),
                Err(error) => return Ok(Err(error)),
            }
        }
        return Ok(Ok(if id == b.list {
            Value::list(values)
        } else {
            Value::Set(Rc::new(values.into_iter().collect::<IndexSet<Value>>()))
        }));
    }
    if id == b.map {
        let Json::Object(fields) = json else {
            return mismatch(vm, path, "Map", json);
        };
        let value_ty = args.get(1).unwrap_or(&open);
        let mut map = IndexMap::with_capacity(fields.len());
        for (key, item) in fields {
            match decode_at(vm, item, value_ty, &Path::Key(path, key), naming)? {
                Ok(value) => {
                    map.insert(Value::text(key), value);
                }
                Err(error) => return Ok(Err(error)),
            }
        }
        return Ok(Ok(Value::Map(Rc::new(map))));
    }
    let meta = program.types.meta(id);
    match (meta.module.as_str(), meta.name.as_str()) {
        ("std.json", "JsonValue") => return Ok(Ok(json_value(vm, json)?)),
        ("std.time", "Date") => {
            return Ok(match json {
                Json::Text(text) => match parse_instant_text(&format!("{text}T00:00:00Z")) {
                    Some(ms) => Ok(vm.date_from_days(ms.div_euclid(86_400_000))?),
                    None => return mismatch(vm, path, "a date", json),
                },
                _ => return mismatch(vm, path, "a date", json),
            })
        }
        ("std.time", "Instant") => {
            return Ok(match json {
                Json::Text(text) => match parse_instant_text(text) {
                    Some(ms) => Ok(Value::Instant(ms)),
                    None => return mismatch(vm, path, "an instant", json),
                },
                _ => return mismatch(vm, path, "an instant", json),
            })
        }
        _ => {}
    }
    match &meta.shape {
        TypeShape::Subtype { base } => {
            let value = match decode_at(vm, json, &field_type(base, meta, args), path, naming)? {
                Ok(value) => value,
                Err(error) => return Ok(Err(error)),
            };
            let built = vm.construct(id, vec![value])?;
            constrained(vm, built, path)
        }
        TypeShape::Record(fields) => {
            let Json::Object(entries) = json else {
                return mismatch(vm, path, &meta.name, json);
            };
            let mut values = Vec::with_capacity(fields.len());
            for field in fields {
                let key = naming.key(field);
                let field_path = Path::Key(path, &key);
                let found = entries.iter().find(|(k, _)| k.as_str() == &*key).map(|(_, v)| v);
                match found {
                    Some(item) => {
                        let ty = field_type(&field.ty, meta, args);
                        match decode_at(vm, item, &ty, &field_path, naming)? {
                            Ok(value) => values.push(value),
                            Err(error) => return Ok(Err(error)),
                        }
                    }
                    None if field.optional => values.push(Value::Nothing),
                    None => return mismatch(vm, &field_path, "a value", &Json::Null),
                }
            }
            let built = vm.construct(id, values)?;
            constrained(vm, built, path)
        }
        TypeShape::Sum(variants) => {
            let Json::Object(entries) = json else {
                return mismatch(vm, path, &meta.name, json);
            };
            let kind_path = Path::Key(path, "kind");
            let kind = entries.iter().find(|(k, _)| k == "kind").map(|(_, v)| v);
            let Some(Json::Text(kind)) = kind else {
                return mismatch(vm, &kind_path, "the variant name", &Json::Null);
            };
            let Some(tag) = variants.iter().position(|v| v.name == *kind) else {
                return mismatch(vm, &kind_path, &meta.name, &Json::Text(kind.clone()));
            };
            let mut values = Vec::new();
            for field in &variants[tag].fields {
                let key = naming.key(field);
                let field_path = Path::Key(path, &key);
                let found = entries.iter().find(|(k, _)| k.as_str() == &*key).map(|(_, v)| v);
                match found {
                    Some(item) => {
                        let ty = field_type(&field.ty, meta, args);
                        match decode_at(vm, item, &ty, &field_path, naming)? {
                            Ok(value) => values.push(value),
                            Err(error) => return Ok(Err(error)),
                        }
                    }
                    None if field.optional => values.push(Value::Nothing),
                    None => return mismatch(vm, &field_path, "a value", &Json::Null),
                }
            }
            let built = vm.construct_variant(id, tag, values)?;
            constrained(vm, built, path)
        }
        TypeShape::Opaque => mismatch(vm, path, &meta.name, json),
    }
}

/// A construction's failure becomes a `Constraint` error.
fn constrained(vm: &Vm, built: Value, path: &Path) -> Result<Decoded, Interrupt> {
    match built {
        Value::Failure(error) => {
            let detail = match &*error {
                Value::Record(record) => record
                    .fields
                    .get(1)
                    .and_then(Value::as_text)
                    .unwrap_or("its condition")
                    .to_string(),
                _ => "its condition".to_string(),
            };
            json_error(
                vm,
                "Constraint",
                vec![Value::text(path.render()), Value::text(detail)],
            )
        }
        value => Ok(Ok(value)),
    }
}

fn json_value(vm: &Vm, json: &Json) -> Result<Value, Interrupt> {
    let make = |variant: &str, fields: Vec<Value>| {
        vm.library_variant("std.json", "JsonValue", variant, fields)
    };
    match json {
        Json::Null => make("JsonNull", Vec::new()),
        Json::Boolean(value) => make("JsonBoolean", vec![Value::Boolean(*value)]),
        Json::Number(text) => {
            let value = decimal_of(text).ok_or_else(|| crash("a JSON number is out of range"))?;
            make("JsonNumber", vec![Value::decimal(value)])
        }
        Json::Text(text) => make("JsonText", vec![Value::text(text)]),
        Json::Array(items) => {
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                values.push(json_value(vm, item)?);
            }
            make("JsonArray", vec![Value::list(values)])
        }
        Json::Object(fields) => {
            let mut map = IndexMap::with_capacity(fields.len());
            for (key, value) in fields {
                map.insert(Value::text(key), json_value(vm, value)?);
            }
            make("JsonObject", vec![Value::Map(Rc::new(map))])
        }
    }
}

// ---------------------------------------------------------------- encoding

/// Where an encoding goes, in document order: `item` comes before every
/// item of an array and `key` before every value of an object.
trait Sink {
    fn null(&mut self);
    fn boolean(&mut self, value: bool);
    fn number(&mut self, value: &dyn Display);
    fn text(&mut self, value: &str);
    fn begin_array(&mut self);
    fn item(&mut self);
    fn end_array(&mut self);
    fn begin_object(&mut self);
    fn key(&mut self, key: &str);
    fn end_object(&mut self);
}

/// The tree of `encode`.
#[derive(Default)]
struct TreeSink {
    /// The containers being built, innermost last.
    open: Vec<Node>,
    result: Option<Json>,
}

enum Node {
    Array(Vec<Json>),
    /// The fields so far and the key the next value goes under.
    Object(Vec<(String, Json)>, String),
}

impl TreeSink {
    fn put(&mut self, json: Json) {
        match self.open.last_mut() {
            Some(Node::Array(items)) => items.push(json),
            Some(Node::Object(fields, key)) => fields.push((std::mem::take(key), json)),
            None => self.result = Some(json),
        }
    }
}

impl Sink for TreeSink {
    fn null(&mut self) {
        self.put(Json::Null);
    }

    fn boolean(&mut self, value: bool) {
        self.put(Json::Boolean(value));
    }

    fn number(&mut self, value: &dyn Display) {
        self.put(Json::Number(value.to_string()));
    }

    fn text(&mut self, value: &str) {
        self.put(Json::Text(value.to_string()));
    }

    fn begin_array(&mut self) {
        self.open.push(Node::Array(Vec::new()));
    }

    fn item(&mut self) {}

    fn end_array(&mut self) {
        if let Some(Node::Array(items)) = self.open.pop() {
            self.put(Json::Array(items));
        }
    }

    fn begin_object(&mut self) {
        self.open.push(Node::Object(Vec::new(), String::new()));
    }

    fn key(&mut self, key: &str) {
        if let Some(Node::Object(_, pending)) = self.open.last_mut() {
            *pending = key.to_string();
        }
    }

    fn end_object(&mut self) {
        if let Some(Node::Object(fields, _)) = self.open.pop() {
            self.put(Json::Object(fields));
        }
    }
}

/// The text of `render`, written as the value is walked, in the layout of
/// `write_json`: one line, or indented by `indent` per level.
struct TextSink {
    out: String,
    indent: Option<usize>,
    /// One entry per open container: whether it holds an item yet.
    open: Vec<bool>,
}

impl TextSink {
    /// Before an item or a key: the comma after the previous one, and the
    /// line break of the indented layout.
    fn next(&mut self) {
        let depth = self.open.len();
        if let Some(started) = self.open.last_mut() {
            if *started {
                self.out.push(',');
            }
            *started = true;
        }
        renyi_json::newline(&mut self.out, self.indent, depth);
    }

    fn close(&mut self, bracket: char) {
        let started = self.open.pop().unwrap_or(false);
        if started {
            renyi_json::newline(&mut self.out, self.indent, self.open.len());
        }
        self.out.push(bracket);
    }
}

impl Sink for TextSink {
    fn null(&mut self) {
        self.out.push_str("null");
    }

    fn boolean(&mut self, value: bool) {
        self.out.push_str(if value { "true" } else { "false" });
    }

    fn number(&mut self, value: &dyn Display) {
        let _ = write!(self.out, "{value}");
    }

    fn text(&mut self, value: &str) {
        renyi_json::write_string(value, &mut self.out);
    }

    fn begin_array(&mut self) {
        self.out.push('[');
        self.open.push(false);
    }

    fn item(&mut self) {
        self.next();
    }

    fn end_array(&mut self) {
        self.close(']');
    }

    fn begin_object(&mut self) {
        self.out.push('{');
        self.open.push(false);
    }

    fn key(&mut self, key: &str) {
        self.next();
        renyi_json::write_string(key, &mut self.out);
        self.out.push(':');
        if self.indent.is_some() {
            self.out.push(' ');
        }
    }

    fn end_object(&mut self) {
        self.close('}');
    }
}

/// Walk a value into the sink by its shape, with the derivation rules of
/// the library sketch.
fn walk(vm: &mut Vm, value: &Value, naming: Naming, sink: &mut dyn Sink) -> Result<(), Interrupt> {
    match value {
        // the origins of a guarded value are checked at the boundary, not
        // written out
        Value::Guarded(guarded) => walk(vm, &guarded.1, naming, sink),
        Value::Nothing => {
            sink.null();
            Ok(())
        }
        Value::Boolean(value) => {
            sink.boolean(*value);
            Ok(())
        }
        Value::Integer(value) => {
            sink.number(value);
            Ok(())
        }
        Value::Decimal(value) => {
            sink.number(&**value);
            Ok(())
        }
        Value::Float(value) => {
            if !value.is_finite() {
                return Err(crash("cannot render a Float that is not finite as JSON"));
            }
            sink.number(&float_text(*value));
            Ok(())
        }
        Value::Text(text) => {
            sink.text(text);
            Ok(())
        }
        Value::Bytes(bytes) => {
            sink.text(&base64_encode(bytes));
            Ok(())
        }
        Value::List(items) => sequence(vm, items.iter(), naming, sink),
        Value::Set(items) => sequence(vm, items.iter(), naming, sink),
        Value::Map(entries) => {
            sink.begin_object();
            for (key, item) in entries.iter() {
                let key = vm.to_text(key)?;
                sink.key(&key);
                walk(vm, item, naming, sink)?;
            }
            sink.end_object();
            Ok(())
        }
        Value::Pair(pair) => sequence(vm, [&pair.0, &pair.1].into_iter(), naming, sink),
        Value::Duration(ms) => {
            sink.number(ms);
            Ok(())
        }
        Value::Instant(ms) => {
            sink.text(&instant_text(*ms));
            Ok(())
        }
        Value::Record(record) => {
            let meta = vm.program.types.meta(record.ty);
            if meta.module == "std.time" && meta.name == "Date" {
                let (year, month, day) = vm.date_parts(value)?;
                sink.text(&format!("{year:04}-{month:02}-{day:02}"));
                return Ok(());
            }
            let TypeShape::Record(fields) = &meta.shape else {
                return Err(crash(format!("cannot render a `{}` as JSON", meta.name)));
            };
            sink.begin_object();
            for (field, item) in fields.iter().zip(&record.fields) {
                sink.key(&naming.key(field));
                walk(vm, item, naming, sink)?;
            }
            sink.end_object();
            Ok(())
        }
        Value::Variant(variant) => {
            let meta = vm.program.types.meta(variant.ty);
            let TypeShape::Sum(variants) = &meta.shape else {
                return Err(crash(format!("cannot render a `{}` as JSON", meta.name)));
            };
            let shape = &variants[variant.tag];
            if meta.module == "std.json" && meta.name == "JsonValue" {
                let inner = variant.fields.first().cloned().unwrap_or(Value::Nothing);
                return match shape.name.as_str() {
                    "JsonNull" => {
                        sink.null();
                        Ok(())
                    }
                    "JsonBoolean" => {
                        sink.boolean(inner.as_bool().unwrap_or(false));
                        Ok(())
                    }
                    "JsonText" => {
                        sink.text(inner.as_text().unwrap_or(""));
                        Ok(())
                    }
                    _ => walk(vm, &inner, naming, sink),
                };
            }
            sink.begin_object();
            sink.key("kind");
            sink.text(&shape.name);
            for (field, item) in shape.fields.iter().zip(&variant.fields) {
                sink.key(&naming.key(field));
                walk(vm, item, naming, sink)?;
            }
            sink.end_object();
            Ok(())
        }
        other => Err(crash(format!(
            "cannot render {} as JSON",
            other.kind_name()
        ))),
    }
}

fn sequence<'a>(
    vm: &mut Vm,
    items: impl Iterator<Item = &'a Value>,
    naming: Naming,
    sink: &mut dyn Sink,
) -> Result<(), Interrupt> {
    sink.begin_array();
    for item in items {
        sink.item();
        walk(vm, item, naming, sink)?;
    }
    sink.end_array();
    Ok(())
}

/// Encode a value by its shape, with the derivation rules of the library
/// sketch.
pub fn encode(vm: &mut Vm, value: &Value, naming: Naming) -> Result<Json, Interrupt> {
    let mut tree = TreeSink::default();
    walk(vm, value, naming, &mut tree)?;
    Ok(tree.result.unwrap_or(Json::Null))
}

// ---------------------------------------------------------------- natives

fn parse_into(vm: &mut Vm, content: &str, naming: Naming) -> Result<Value, Interrupt> {
    let Some(ty) = vm.take_expected() else {
        return Err(crash(
            "`json.parse` does not know the type to decode into; give the binding a type",
        ));
    };
    let json = match read_json(content) {
        Ok(json) => json,
        Err((detail, line)) => {
            return vm.fail_variant(
                "std.json",
                "JsonError",
                "Malformed",
                vec![Value::text(detail), Value::integer(line)],
            )
        }
    };
    match decode(vm, &json, &ty, "$", naming)? {
        Ok(value) => Ok(value),
        Err(error) => Ok(Value::failure(error)),
    }
}

fn parse(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    parse_into(vm, text(arg(args, 0))?, Naming::Exact)
}

fn parse_with(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let naming = Naming::of(arg(args, 1));
    parse_into(vm, text(arg(args, 0))?, naming)
}

fn render_as(
    vm: &mut Vm,
    value: &Value,
    naming: Naming,
    indent: Option<usize>,
) -> Result<Value, Interrupt> {
    let mut sink = TextSink {
        out: String::new(),
        indent,
        open: Vec::new(),
    };
    walk(vm, value, naming, &mut sink)?;
    Ok(Value::text(sink.out))
}

fn render(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = arg(args, 0).clone();
    render_as(vm, &value, Naming::Exact, None)
}

fn render_indented(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = arg(args, 0).clone();
    render_as(vm, &value, Naming::Exact, Some(2))
}

fn render_with(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = arg(args, 0).clone();
    let naming = Naming::of(arg(args, 1));
    render_as(vm, &value, naming, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documents_read_and_write() {
        let json = read_json(r#"{"a": [1, 2.5, "x\n"], "b": null, "c": true}"#).unwrap();
        let mut out = String::new();
        write_json(&json, &mut out, None, 0);
        assert_eq!(out, r#"{"a":[1,2.5,"x\n"],"b":null,"c":true}"#);
        assert_eq!(read_json("{\n  \"a\": 1,\n").unwrap_err().1, 3);
    }

    #[test]
    fn a_path_renders_only_when_asked() {
        let root = Path::Root("$");
        let item = Path::Index(&root, 1);
        let field = Path::Key(&item, "score");
        assert_eq!(field.render(), "$[1].score");
        assert_eq!(Path::Key(&root, "kind").render(), "$.kind");
    }

    #[test]
    fn the_text_sink_writes_the_layout_of_write_json() {
        let json = read_json(r#"{"a":[1,{"b":[]},{}],"c":"q\"\\"}"#).unwrap();
        for indent in [None, Some(2)] {
            let mut expected = String::new();
            write_json(&json, &mut expected, indent, 0);
            let mut sink = TextSink {
                out: String::new(),
                indent,
                open: Vec::new(),
            };
            replay(&json, &mut sink);
            assert_eq!(sink.out, expected);
        }
    }

    /// The tree walked into a sink, as `walk` walks a value.
    fn replay(json: &Json, sink: &mut dyn Sink) {
        match json {
            Json::Null => sink.null(),
            Json::Boolean(value) => sink.boolean(*value),
            Json::Number(text) => sink.number(text),
            Json::Text(text) => sink.text(text),
            Json::Array(items) => {
                sink.begin_array();
                for item in items {
                    sink.item();
                    replay(item, sink);
                }
                sink.end_array();
            }
            Json::Object(fields) => {
                sink.begin_object();
                for (key, value) in fields {
                    sink.key(key);
                    replay(value, sink);
                }
                sink.end_object();
            }
        }
    }
}
'''

# (file, [(old, new), ...]); each old must occur exactly once
EDITS = [
    (
        "crates/renyi_json/src/lib.rs",
        [
            (
                r'''#[derive(Clone, Debug, PartialEq)]
pub enum Json {''',
                r'''use std::fmt::Write;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {''',
            ),
            (
                r'''struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    line: i64,
}''',
                r'''struct Reader<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
    line: i64,
}''',
            ),
            (
                r'''    let mut reader = Reader {
        bytes: text.as_bytes(),
        pos: 0,
        line: 1,
    };''',
                r'''    let mut reader = Reader {
        text,
        bytes: text.as_bytes(),
        pos: 0,
        line: 1,
    };''',
            ),
            (
                r'''    fn string(&mut self) -> Result<String, ReadError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let Some(&byte) = self.bytes.get(self.pos) else {
                return self.error("a string is not closed");
            };
            self.pos += 1;
            match byte {''',
                r'''    fn string(&mut self) -> Result<String, ReadError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            // a run of plain bytes is copied at once: the bytes that end it
            // are ASCII, never inside a multibyte sequence, so the run is
            // a text of its own
            let start = self.pos;
            while let Some(&byte) = self.bytes.get(self.pos) {
                if byte == b'"' || byte == b'\\' || byte < 0x20 {
                    break;
                }
                self.pos += 1;
            }
            out.push_str(&self.text[start..self.pos]);
            let Some(&byte) = self.bytes.get(self.pos) else {
                return self.error("a string is not closed");
            };
            self.pos += 1;
            match byte {''',
            ),
            (
                r'''                b'\n' => return self.error("a line break inside a string"),
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
    }''',
                r'''                b'\n' => return self.error("a line break inside a string"),
                // another control byte is kept as it is
                other => out.push(other as char),
            }
        }
    }''',
            ),
            (
                r'''fn utf8_width(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

''',
                r'''''',
            ),
            (
                r'''fn newline(out: &mut String, indent: Option<usize>, depth: usize) {''',
                r'''/// The line break before an item of the indented layout, with the
/// indentation of its depth; nothing on one line.
pub fn newline(out: &mut String, indent: Option<usize>, depth: usize) {''',
            ),
            (
                r'''fn write_string(text: &str, out: &mut String) {
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
}''',
                r'''/// A JSON string: the text quoted, with the escapes JSON needs.
pub fn write_string(text: &str, out: &mut String) {
    out.reserve(text.len() + 2);
    out.push('"');
    // a run of plain characters is copied at once: the bytes that need an
    // escape are ASCII, never inside a multibyte sequence
    let mut start = 0;
    for (index, byte) in text.bytes().enumerate() {
        let escaped = match byte {
            b'"' => "\\\"",
            b'\\' => "\\\\",
            b'\n' => "\\n",
            b'\r' => "\\r",
            b'\t' => "\\t",
            byte if byte >= 0x20 => continue,
            _ => "",
        };
        out.push_str(&text[start..index]);
        if escaped.is_empty() {
            let _ = write!(out, "\\u{byte:04x}");
        } else {
            out.push_str(escaped);
        }
        start = index + 1;
    }
    out.push_str(&text[start..]);
    out.push('"');
}''',
            ),
        ],
    ),
    (
        "crates/renyi_vm/src/value.rs",
        [
            (
                r'''    pub fn integer(value: i64) -> Value {
        Value::Integer(Int::Small(value))
    }
''',
                r'''    /// A text of one character. The ASCII ones come from a table kept per
    /// thread, so that the characters of a text are listed without an
    /// allocation each.
    pub fn character(c: char) -> Value {
        if c.is_ascii() {
            return ASCII.with(|table| Value::Text(table[c as usize].clone()));
        }
        Value::Text(Rc::from(&*c.encode_utf8(&mut [0; 4])))
    }

    pub fn integer(value: i64) -> Value {
        Value::Integer(Int::Small(value))
    }
''',
            ),
            (
                r'''// Decision X3: the stack and every collection hold values by this width.
const _: () = assert!(std::mem::size_of::<Value>() <= 24);
''',
                r'''// Decision X3: the stack and every collection hold values by this width.
const _: () = assert!(std::mem::size_of::<Value>() <= 24);

thread_local! {
    /// The one-character texts of the ASCII range (`Value::character`).
    static ASCII: [Rc<str>; 128] = std::array::from_fn(|code| {
        let mut buffer = [0; 4];
        Rc::from(&*(code as u8 as char).encode_utf8(&mut buffer))
    });
}
''',
            ),
        ],
    ),
    (
        "crates/renyi_vm/src/natives/prelude.rs",
        [
            (
                r'''    if separator.is_empty() {
        // nothing lies between empty separators but the characters
        return Ok(Value::list(
            value.chars().map(|c| Value::text(c.to_string())).collect(),
        ));
    }''',
                r'''    if separator.is_empty() {
        // nothing lies between empty separators but the characters
        return Ok(Value::list(characters(value)));
    }''',
            ),
            (
                r'''fn text_characters(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::list(
        text(arg(args, 0))?
            .chars()
            .map(|c| Value::text(c.to_string()))
            .collect(),
    ))
}''',
                r'''/// The characters of a text as one-character texts, listed in one
/// allocation (what `characters`, `split("")` and a loop over a text walk).
pub(crate) fn characters(text: &str) -> Vec<Value> {
    let mut items = Vec::with_capacity(text.chars().count());
    items.extend(text.chars().map(Value::character));
    items
}

fn text_characters(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::list(characters(text(arg(args, 0))?)))
}''',
            ),
        ],
    ),
    (
        "crates/renyi_vm/src/vm.rs",
        [
            (
                r'''            Value::Text(text) => {
                Rc::new(text.chars().map(|c| Value::text(c.to_string())).collect())
            }''',
                r'''            Value::Text(text) => Rc::new(natives::prelude::characters(&text)),''',
            ),
        ],
    ),
    (
        "crates/renyi_vm/src/natives/regex.rs",
        [
            (
                r'''//! `std.regex` and `Text.matches`, on the `regex` crate.

use regex::Regex;
''',
                r'''//! `std.regex` and `Text.matches`, on the `regex` crate. A pattern is
//! compiled once per thread and kept, so that a loop applying it pays the
//! compilation once.

use std::cell::RefCell;
use std::collections::HashMap;

use regex::Regex;
''',
            ),
            (
                r'''fn compile(pattern: &str) -> Result<Regex, Interrupt> {
    Regex::new(pattern).map_err(|error| crash(format!("invalid regular expression: {error}")))
}''',
                r'''thread_local! {
    /// The compiled patterns by their text; emptied when it holds
    /// `CACHE_LIMIT` of them.
    static CACHE: RefCell<HashMap<String, Regex>> = RefCell::new(HashMap::new());
}

const CACHE_LIMIT: usize = 256;

fn compile(pattern: &str) -> Result<Regex, Interrupt> {
    if let Some(found) = CACHE.with(|cache| cache.borrow().get(pattern).cloned()) {
        return Ok(found);
    }
    let regex = Regex::new(pattern)
        .map_err(|error| crash(format!("invalid regular expression: {error}")))?;
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(pattern.to_string(), regex.clone());
    });
    Ok(regex)
}''',
            ),
        ],
    ),
    (
        "crates/renyi_vm/src/native/runtime.rs",
        [
            (
                r'''use std::rc::Rc;
''',
                r'''use std::fmt::Write;
use std::rc::Rc;
''',
            ),
            (
                r'''    pub(crate) fn op_concat(&mut self, count: usize) -> Result<Value, Interrupt> {
        let (origins, pieces) = plain_all(self.pop_n(count));
        let mut text = String::new();
        for piece in &pieces {
            match piece {
                Value::Text(part) => text.push_str(part),
                other => text.push_str(&self.render(other, false)?),
            }
        }
        Ok(Value::text(text).guarded(origins))
    }''',
                r'''    /// The pieces are joined where they lie on the stack, an Integer
    /// written straight into the text, and the stack cut below them after.
    pub(crate) fn op_concat(&mut self, count: usize) -> Result<Value, Interrupt> {
        let at = self.stack.len().saturating_sub(count);
        let mut origins = 0;
        let mut length = 0;
        for piece in &self.stack[at..] {
            origins |= piece.origins();
            length += match piece.plain() {
                Value::Text(part) => part.len(),
                _ => 20,
            };
        }
        let mut text = String::with_capacity(length);
        for index in at..self.stack.len() {
            match self.stack[index].plain() {
                Value::Text(part) => text.push_str(part),
                Value::Integer(value) => {
                    let _ = write!(text, "{value}");
                }
                _ => {
                    let piece = self.stack[index].clone();
                    text.push_str(&self.render(piece.plain(), false)?);
                }
            }
        }
        self.stack.truncate(at);
        Ok(Value::text(text).guarded(origins))
    }''',
            ),
        ],
    ),
]


def read(path):
    with open(path, "r", encoding="utf-8", newline="") as handle:
        return handle.read()


def write(path, content, crlf):
    if crlf:
        content = content.replace("\n", "\r\n")
    with open(path, "w", encoding="utf-8", newline="") as handle:
        handle.write(content)


def main():
    planned = []
    problems = []
    for relative, edits in EDITS:
        path = os.path.join(ROOT, relative)
        original = read(path)
        crlf = "\r\n" in original
        content = original.replace("\r\n", "\n")
        for old, new in edits:
            count = content.count(old)
            if count != 1:
                problems.append(f"{relative}: anchor found {count} times: {old[:60]!r}")
                continue
            content = content.replace(old, new)
        planned.append((path, content, crlf))
    json_path = os.path.join(ROOT, "crates/renyi_vm/src/natives/json.rs")
    json_original = read(json_path)
    planned.append((json_path, JSON_RS, "\r\n" in json_original))
    if problems:
        for problem in problems:
            print(problem)
        sys.exit(1)
    for path, content, crlf in planned:
        write(path, content, crlf)
        print("patched", os.path.relpath(path, ROOT))


if __name__ == "__main__":
    main()
