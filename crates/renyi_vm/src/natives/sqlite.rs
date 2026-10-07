//! `std.sqlite`: database files through the `rusqlite` crate with SQLite
//! compiled in (decision S1). A query decodes each row into the record type
//! the context expects, columns matched to fields by name or `as` name,
//! with the type mapping of the library sketch (section 11).

use std::cell::RefCell;
use std::rc::Rc;

use renyi_check::types::Ty;
use rusqlite::types::{Value as SqlValue, ValueRef};

use super::{arg, crash, list, text};
use crate::decimal::Decimal;
use crate::extension::Native as Entry;
use crate::integer::Int;
use crate::render::float_text;
use crate::types::TypeShape;
use crate::value::{Native, Value};
use crate::vm::{Interrupt, Vm};

/// The natives of `std.sqlite` (decision AK2): the declared function
/// each implements, by module, name and the type of its first parameter.
pub(crate) const NATIVES: &[Entry] = &[
    Entry::function("std.sqlite", "open", open),
    Entry::function("std.sqlite", "integer", integer_parameter),
    Entry::function("std.sqlite", "decimal", decimal_parameter),
    Entry::function("std.sqlite", "text", text_parameter),
    Entry::function("std.sqlite", "boolean", boolean_parameter),
    Entry::function("std.sqlite", "absent", absent_parameter),
    Entry::method("std.sqlite", "query", "Connection", query),
    Entry::method("std.sqlite", "execute", "Connection", execute),
    Entry::method("std.sqlite", "close", "Connection", close),
];

fn error(vm: &Vm, variant: &str, fields: Vec<Value>) -> Result<Value, Interrupt> {
    vm.fail_variant("std.sqlite", "DbError", variant, fields)
}

fn open(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?.to_string();
    match rusqlite::Connection::open(&path) {
        Ok(connection) => Ok(Value::Native(Rc::new(Native::Connection {
            connection: RefCell::new(Some(connection)),
            path,
        }))),
        Err(detail) => error(
            vm,
            "CannotOpen",
            vec![Value::text(path), Value::text(detail.to_string())],
        ),
    }
}

fn parameter(vm: &Vm, variant: &str, fields: Vec<Value>) -> Result<Value, Interrupt> {
    vm.library_variant("std.sqlite", "Parameter", variant, fields)
}

fn integer_parameter(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    parameter(vm, "IntegerValue", vec![arg(args, 0).clone()])
}

fn decimal_parameter(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    parameter(vm, "DecimalValue", vec![arg(args, 0).clone()])
}

fn text_parameter(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    parameter(vm, "TextValue", vec![arg(args, 0).clone()])
}

fn boolean_parameter(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    parameter(vm, "BooleanValue", vec![arg(args, 0).clone()])
}

fn absent_parameter(vm: &mut Vm, _: &mut [Value]) -> Result<Value, Interrupt> {
    parameter(vm, "NullValue", Vec::new())
}

/// The native behind a `Connection` value.
fn connection_of(value: &Value) -> Result<Rc<Native>, Interrupt> {
    match value {
        Value::Native(native) if matches!(**native, Native::Connection { .. }) => {
            Ok(native.clone())
        }
        other => Err(crash(format!(
            "a database operation expected a Connection, found {}",
            other.kind_name()
        ))),
    }
}

/// The `Parameter` values as SQLite values: a Decimal travels as text, which
/// a numeric column converts and a text column keeps.
fn parameters(vm: &Vm, value: &Value) -> Result<Vec<SqlValue>, Interrupt> {
    let mut out = Vec::new();
    for item in list(value)?.iter() {
        let Value::Variant(variant) = item else {
            return Err(crash("a query parameter must be a `Parameter`"));
        };
        let name = match &vm.program.types.meta(variant.ty).shape {
            TypeShape::Sum(variants) => variants
                .get(variant.tag)
                .map(|v| v.name.clone())
                .unwrap_or_default(),
            _ => String::new(),
        };
        let field = variant.fields.first().cloned().unwrap_or(Value::Nothing);
        out.push(match (name.as_str(), field) {
            ("IntegerValue", Value::Integer(value)) => match value.to_i64() {
                Some(small) => SqlValue::Integer(small),
                None => SqlValue::Text(value.to_string()),
            },
            ("DecimalValue", Value::Decimal(value)) => SqlValue::Text(value.to_string()),
            ("TextValue", Value::Text(value)) => SqlValue::Text(value.to_string()),
            ("BooleanValue", Value::Boolean(value)) => SqlValue::Integer(i64::from(value)),
            ("NullValue", _) => SqlValue::Null,
            (name, other) => {
                return Err(crash(format!(
                    "a `{name}` parameter cannot carry {}",
                    other.kind_name()
                )))
            }
        });
    }
    Ok(out)
}

fn kind_of(cell: &ValueRef) -> &'static str {
    match cell {
        ValueRef::Null => "NULL",
        ValueRef::Integer(_) => "INTEGER",
        ValueRef::Real(_) => "REAL",
        ValueRef::Text(_) => "TEXT",
        ValueRef::Blob(_) => "BLOB",
    }
}

/// A cell as a value of the field's type, or the `Mismatch` error.
fn decode_cell(vm: &mut Vm, cell: ValueRef, ty: &Ty, column: &str) -> Result<Value, Interrupt> {
    let mismatch = |vm: &Vm, expected: &str| {
        error(
            vm,
            "Mismatch",
            vec![
                Value::text(column),
                Value::text(expected),
                Value::text(kind_of(&cell)),
            ],
        )
    };
    let b = vm.program.builtins.clone();
    let (id, args) = match ty {
        Ty::Maybe(inner) => {
            if cell == ValueRef::Null {
                return Ok(Value::Nothing);
            }
            return decode_cell(vm, cell, inner, column);
        }
        Ty::App(id, args) => (*id, args.clone()),
        _ => return Err(crash("a row field has a type a query cannot decode")),
    };
    let _ = args;
    if id == b.integer {
        return match cell {
            ValueRef::Integer(value) => Ok(Value::integer(value)),
            ValueRef::Text(bytes) => match Int::parse(&String::from_utf8_lossy(bytes)) {
                Some(value) => Ok(Value::Integer(value)),
                None => mismatch(vm, "Integer"),
            },
            _ => mismatch(vm, "Integer"),
        };
    }
    if id == b.float {
        return match cell {
            ValueRef::Real(value) => Ok(Value::Float(value)),
            ValueRef::Integer(value) => Ok(Value::Float(value as f64)),
            _ => mismatch(vm, "Float"),
        };
    }
    if id == b.decimal {
        let parsed = match cell {
            ValueRef::Integer(value) => Some(Decimal::from_int(&Int::Small(value))),
            ValueRef::Real(value) => Decimal::parse(&float_text(value)),
            ValueRef::Text(bytes) => Decimal::parse(String::from_utf8_lossy(bytes).trim()),
            _ => None,
        };
        return match parsed {
            Some(value) => Ok(Value::decimal(value)),
            None => mismatch(vm, "Decimal"),
        };
    }
    if id == b.text {
        return match cell {
            ValueRef::Text(bytes) => Ok(Value::text(String::from_utf8_lossy(bytes))),
            _ => mismatch(vm, "Text"),
        };
    }
    if id == b.boolean {
        return match cell {
            ValueRef::Integer(0) => Ok(Value::Boolean(false)),
            ValueRef::Integer(1) => Ok(Value::Boolean(true)),
            _ => mismatch(vm, "Boolean"),
        };
    }
    if id == b.bytes {
        return match cell {
            ValueRef::Blob(bytes) | ValueRef::Text(bytes) => Ok(Value::Bytes(Rc::from(bytes))),
            _ => mismatch(vm, "Bytes"),
        };
    }
    let meta = vm.program.types.meta(id).clone();
    if let TypeShape::Subtype { base } = &meta.shape {
        let value = decode_cell(vm, cell, base, column)?;
        if value.is_failure() {
            return Ok(value);
        }
        return match vm.construct(id, vec![value])? {
            Value::Failure(violation) => {
                let detail = violation_detail(&violation);
                error(
                    vm,
                    "Mismatch",
                    vec![
                        Value::text(column),
                        Value::text(format!("a {} ({detail})", meta.name)),
                        Value::text("a value outside its condition"),
                    ],
                )
            }
            built => Ok(built),
        };
    }
    mismatch(vm, &meta.name)
}

fn violation_detail(violation: &Value) -> String {
    match violation {
        Value::Record(record) => record
            .fields
            .get(1)
            .and_then(Value::as_text)
            .unwrap_or("its condition")
            .to_string(),
        _ => "its condition".to_string(),
    }
}

fn query(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let native = connection_of(arg(args, 0))?;
    let sql = text(arg(args, 1))?.to_string();
    let parameters = parameters(vm, arg(args, 2))?;
    let b = vm.program.builtins.clone();
    let row_ty = match vm.take_expected() {
        Some(Ty::App(id, items)) if id == b.list => items.into_iter().next(),
        _ => None,
    };
    let Some(Ty::App(row_id, _)) = row_ty else {
        return Err(crash(
            "`query` does not know the row type to decode into; give the binding a type",
        ));
    };
    let meta = vm.program.types.meta(row_id).clone();
    let TypeShape::Record(fields) = &meta.shape else {
        return Err(crash(format!(
            "`query` decodes rows into records; `{}` is not one",
            meta.name
        )));
    };
    let Native::Connection { connection, .. } = &*native else {
        unreachable!("checked by connection_of");
    };
    let guard = connection.borrow();
    let Some(connection) = guard.as_ref() else {
        return Err(crash("the connection is closed"));
    };
    let mut statement = match connection.prepare(&sql) {
        Ok(statement) => statement,
        Err(detail) => {
            return error(
                vm,
                "Failed",
                vec![Value::text(&sql), Value::text(detail.to_string())],
            )
        }
    };
    let columns: Vec<String> = statement
        .column_names()
        .iter()
        .map(|name| name.to_string())
        .collect();
    let mut rows = match statement.query(rusqlite::params_from_iter(parameters)) {
        Ok(rows) => rows,
        Err(detail) => {
            return error(
                vm,
                "Failed",
                vec![Value::text(&sql), Value::text(detail.to_string())],
            )
        }
    };
    let mut out = Vec::new();
    loop {
        let row = match rows.next() {
            Ok(Some(row)) => row,
            Ok(None) => break,
            Err(detail) => {
                return error(
                    vm,
                    "Failed",
                    vec![Value::text(&sql), Value::text(detail.to_string())],
                )
            }
        };
        let mut values = Vec::with_capacity(fields.len());
        for field in fields {
            let column = field
                .external_name
                .clone()
                .unwrap_or_else(|| field.name.clone());
            let Some(index) = columns.iter().position(|name| *name == column) else {
                return error(
                    vm,
                    "Mismatch",
                    vec![
                        Value::text(&column),
                        Value::text("a column"),
                        Value::text("no such column"),
                    ],
                );
            };
            let cell = match row.get_ref(index) {
                Ok(cell) => cell,
                Err(detail) => {
                    return error(
                        vm,
                        "Failed",
                        vec![Value::text(&sql), Value::text(detail.to_string())],
                    )
                }
            };
            let value = decode_cell(vm, cell, &field.ty, &column)?;
            if value.is_failure() {
                return Ok(value);
            }
            values.push(value);
        }
        match vm.construct(row_id, values)? {
            Value::Failure(violation) => {
                let detail = violation_detail(&violation);
                return error(
                    vm,
                    "Mismatch",
                    vec![
                        Value::text(&meta.name),
                        Value::text(format!("a row where {detail}")),
                        Value::text("a row outside the condition"),
                    ],
                );
            }
            built => out.push(built),
        }
    }
    Ok(Value::list(out))
}

fn execute(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let native = connection_of(arg(args, 0))?;
    let sql = text(arg(args, 1))?.to_string();
    let parameters = parameters(vm, arg(args, 2))?;
    let Native::Connection { connection, .. } = &*native else {
        unreachable!("checked by connection_of");
    };
    let guard = connection.borrow();
    let Some(connection) = guard.as_ref() else {
        return Err(crash("the connection is closed"));
    };
    match connection.execute(&sql, rusqlite::params_from_iter(parameters)) {
        Ok(changed) => Ok(Value::integer(changed as i64)),
        Err(detail) => error(
            vm,
            "Failed",
            vec![Value::text(&sql), Value::text(detail.to_string())],
        ),
    }
}

fn close(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let native = connection_of(arg(args, 0))?;
    if let Native::Connection { connection, .. } = &*native {
        connection.borrow_mut().take();
    }
    Ok(Value::Nothing)
}
