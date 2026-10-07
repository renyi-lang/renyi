//! `std.json`: a reader and a writer for JSON text, decoding by the type the
//! context expects (recorded by the checker as `Target::Result`) and
//! encoding by the value's shape, with the derivation rules of the library
//! sketch (section 7).

use std::rc::Rc;

use indexmap::{IndexMap, IndexSet};
use renyi_check::types::Ty;

use super::{arg, base64_decode, base64_encode, crash, text};
use crate::decimal::Decimal;
use crate::extension::Native;
use crate::integer::Int;
use crate::natives::time::{instant_text, parse_instant_text};
use crate::render::float_text;
use crate::types::{FieldMeta, TypeShape};
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

    fn key(self, field: &FieldMeta) -> String {
        if let Some(external) = &field.external_name {
            return external.clone();
        }
        match self {
            Naming::Exact => field.name.clone(),
            Naming::Kebab => field.name.replace('_', "-"),
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
                out
            }
        }
    }
}

// ---------------------------------------------------------------- decoding

/// The outcome of decoding: a value, or the `JsonError` to fail with.
pub type Decoded = Result<Value, Value>;

fn json_error(vm: &Vm, variant: &str, fields: Vec<Value>) -> Result<Decoded, Interrupt> {
    Ok(Err(vm.library_variant(
        "std.json",
        "JsonError",
        variant,
        fields,
    )?))
}

fn mismatch(vm: &Vm, path: &str, expected: &str, found: &Json) -> Result<Decoded, Interrupt> {
    json_error(
        vm,
        "Mismatch",
        vec![
            Value::text(path),
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

/// Decode a document into a value of the type, with the derivation rules of
/// the library sketch; `path` names the position for error messages.
pub fn decode(
    vm: &mut Vm,
    json: &Json,
    ty: &Ty,
    path: &str,
    naming: Naming,
) -> Result<Decoded, Interrupt> {
    let b = vm.program.builtins.clone();
    let (id, args) = match ty {
        Ty::Maybe(inner) => {
            if *json == Json::Null {
                return Ok(Ok(Value::Nothing));
            }
            return decode(vm, json, inner, path, naming);
        }
        Ty::App(id, args) => (*id, args.clone()),
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
    if id == b.list || id == b.set {
        let Json::Array(items) = json else {
            return mismatch(vm, path, "List", json);
        };
        let item_ty = args.first().cloned().unwrap_or(Ty::Error);
        let mut values = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            match decode(vm, item, &item_ty, &format!("{path}[{index}]"), naming)? {
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
        let value_ty = args.get(1).cloned().unwrap_or(Ty::Error);
        let mut map = IndexMap::with_capacity(fields.len());
        for (key, item) in fields {
            match decode(vm, item, &value_ty, &format!("{path}.{key}"), naming)? {
                Ok(value) => {
                    map.insert(Value::text(key), value);
                }
                Err(error) => return Ok(Err(error)),
            }
        }
        return Ok(Ok(Value::Map(Rc::new(map))));
    }
    let meta = vm.program.types.meta(id).clone();
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
    let substitute = |ty: &Ty| {
        ty.substitute(&|param| {
            meta.params
                .iter()
                .position(|p| *p == param)
                .and_then(|index| args.get(index).cloned())
        })
    };
    match &meta.shape {
        TypeShape::Subtype { base } => {
            let value = match decode(vm, json, &substitute(base), path, naming)? {
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
                let field_path = format!("{path}.{key}");
                let found = entries.iter().find(|(k, _)| *k == key).map(|(_, v)| v);
                match found {
                    Some(item) => {
                        match decode(vm, item, &substitute(&field.ty), &field_path, naming)? {
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
            let kind = entries.iter().find(|(k, _)| k == "kind").map(|(_, v)| v);
            let Some(Json::Text(kind)) = kind else {
                return mismatch(vm, &format!("{path}.kind"), "the variant name", &Json::Null);
            };
            let Some(tag) = variants.iter().position(|v| v.name == *kind) else {
                return mismatch(
                    vm,
                    &format!("{path}.kind"),
                    &meta.name,
                    &Json::Text(kind.clone()),
                );
            };
            let mut values = Vec::new();
            for field in &variants[tag].fields {
                let key = naming.key(field);
                let field_path = format!("{path}.{key}");
                let found = entries.iter().find(|(k, _)| *k == key).map(|(_, v)| v);
                match found {
                    Some(item) => {
                        match decode(vm, item, &substitute(&field.ty), &field_path, naming)? {
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
fn constrained(vm: &Vm, built: Value, path: &str) -> Result<Decoded, Interrupt> {
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
                vec![Value::text(path), Value::text(detail)],
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

/// Encode a value by its shape, with the derivation rules of the library
/// sketch.
pub fn encode(vm: &mut Vm, value: &Value, naming: Naming) -> Result<Json, Interrupt> {
    Ok(match value {
        // the origins of a guarded value are checked at the boundary, not
        // written out
        Value::Guarded(guarded) => return encode(vm, &guarded.1, naming),
        Value::Nothing => Json::Null,
        Value::Boolean(value) => Json::Boolean(*value),
        Value::Integer(value) => Json::Number(value.to_string()),
        Value::Decimal(value) => Json::Number(value.to_string()),
        Value::Float(value) => {
            if !value.is_finite() {
                return Err(crash("cannot render a Float that is not finite as JSON"));
            }
            Json::Number(float_text(*value))
        }
        Value::Text(text) => Json::Text(text.to_string()),
        Value::Bytes(bytes) => Json::Text(base64_encode(bytes)),
        Value::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items.iter() {
                out.push(encode(vm, item, naming)?);
            }
            Json::Array(out)
        }
        Value::Set(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items.iter() {
                out.push(encode(vm, item, naming)?);
            }
            Json::Array(out)
        }
        Value::Map(entries) => {
            let mut out = Vec::with_capacity(entries.len());
            for (key, item) in entries.iter() {
                let key = vm.to_text(key)?;
                out.push((key, encode(vm, item, naming)?));
            }
            Json::Object(out)
        }
        Value::Pair(pair) => Json::Array(vec![
            encode(vm, &pair.0, naming)?,
            encode(vm, &pair.1, naming)?,
        ]),
        Value::Duration(ms) => Json::Number(ms.to_string()),
        Value::Instant(ms) => Json::Text(instant_text(*ms)),
        Value::Record(record) => {
            let meta = vm.program.types.meta(record.ty).clone();
            if meta.module == "std.time" && meta.name == "Date" {
                let (year, month, day) = vm.date_parts(value)?;
                return Ok(Json::Text(format!("{year:04}-{month:02}-{day:02}")));
            }
            let TypeShape::Record(fields) = &meta.shape else {
                return Err(crash(format!("cannot render a `{}` as JSON", meta.name)));
            };
            let mut out = Vec::with_capacity(fields.len());
            for (field, item) in fields.iter().zip(&record.fields) {
                out.push((naming.key(field), encode(vm, item, naming)?));
            }
            Json::Object(out)
        }
        Value::Variant(variant) => {
            let meta = vm.program.types.meta(variant.ty).clone();
            let TypeShape::Sum(variants) = &meta.shape else {
                return Err(crash(format!("cannot render a `{}` as JSON", meta.name)));
            };
            let shape = &variants[variant.tag];
            if meta.module == "std.json" && meta.name == "JsonValue" {
                let inner = variant.fields.first().cloned().unwrap_or(Value::Nothing);
                return Ok(match shape.name.as_str() {
                    "JsonNull" => Json::Null,
                    "JsonBoolean" => Json::Boolean(inner.as_bool().unwrap_or(false)),
                    "JsonNumber" => encode(vm, &inner, naming)?,
                    "JsonText" => Json::Text(inner.as_text().unwrap_or("").to_string()),
                    _ => encode(vm, &inner, naming)?,
                });
            }
            let mut out = Vec::with_capacity(shape.fields.len() + 1);
            out.push(("kind".to_string(), Json::Text(shape.name.clone())));
            for (field, item) in shape.fields.iter().zip(&variant.fields) {
                out.push((naming.key(field), encode(vm, item, naming)?));
            }
            Json::Object(out)
        }
        other => {
            return Err(crash(format!(
                "cannot render {} as JSON",
                other.kind_name()
            )))
        }
    })
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
    let content = text(arg(args, 0))?.to_string();
    parse_into(vm, &content, Naming::Exact)
}

fn parse_with(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let content = text(arg(args, 0))?.to_string();
    let naming = Naming::of(arg(args, 1));
    parse_into(vm, &content, naming)
}

fn render_as(
    vm: &mut Vm,
    value: &Value,
    naming: Naming,
    indent: Option<usize>,
) -> Result<Value, Interrupt> {
    let json = encode(vm, value, naming)?;
    let mut out = String::new();
    write_json(&json, &mut out, indent, 0);
    Ok(Value::text(out))
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
}
