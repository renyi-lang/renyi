//! The library's primitives in Rust, behind one boundary: a declared library
//! function is looked up by module, name and receiver type, and runs as a
//! `NativeFn`. A function the build does not implement has no entry, and a
//! call to it crashes with a message that says so.

pub mod csv;
pub mod filesystem;
pub mod http;
pub mod json;
pub mod prelude;
pub mod regex;
pub mod server;
pub mod sqlite;
pub mod system;
pub mod time;

use std::rc::Rc;

use indexmap::{IndexMap, IndexSet};
use renyi_check::TypeId;

use crate::decimal::Decimal;
use crate::integer::Int;
use crate::value::{RangeValue, Value};
use crate::vm::{Interrupt, Vm};

/// A primitive takes its arguments as a slice of the VM's scratch buffer,
/// so that a call allocates nothing (decision X3); one that builds on an
/// argument in place takes it out with `take`.
pub type NativeFn = fn(&mut Vm, &mut [Value]) -> Result<Value, Interrupt>;

/// The primitive behind a library function: its module, its name and the
/// type of its first parameter as the checker spells it (`List of Item`).
pub fn lookup(module: &str, name: &str, receiver: Option<&str>) -> Option<NativeFn> {
    let head = receiver.map(|r| r.split(' ').next().unwrap_or(r));
    match module {
        "std.prelude" => prelude::lookup(name, head, receiver),
        "std.console" | "std.environment" | "std.random" => system::lookup(module, name),
        "std.time" => time::lookup(name, head),
        "std.filesystem" => filesystem::lookup(name, head),
        "std.json" => json::lookup(name),
        "std.csv" => csv::lookup(name, head),
        "std.regex" => regex::lookup(name, head),
        "std.http" => http::lookup(name),
        "std.server" => server::lookup(name, head),
        "std.sqlite" => sqlite::lookup(name, head),
        _ => None,
    }
}

pub fn crash(message: impl Into<String>) -> Interrupt {
    Interrupt::crash(message)
}

/// Milliseconds since the Unix epoch.
pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------- arguments

/// The i-th argument; the checker has verified the count, so a missing one
/// is a bug of the compiler.
pub fn arg(args: &[Value], index: usize) -> &Value {
    args.get(index)
        .expect("the checker verified the argument count")
}

/// The i-th argument taken out, `Nothing` left in its place: for a
/// primitive that updates a collection in place when nothing else holds
/// it (decision O1).
pub fn take(args: &mut [Value], index: usize) -> Value {
    match args.get_mut(index) {
        Some(value) => std::mem::replace(value, Value::Nothing),
        None => Value::Nothing,
    }
}

fn wrong(expected: &str, found: &Value) -> Interrupt {
    crash(format!(
        "a library function expected {expected}, found {}",
        found.kind_name()
    ))
}

pub fn text(value: &Value) -> Result<&str, Interrupt> {
    value.as_text().ok_or_else(|| wrong("Text", value))
}

pub fn int(value: &Value) -> Result<&Int, Interrupt> {
    value.as_int().ok_or_else(|| wrong("an Integer", value))
}

/// An Integer that fits a machine word.
pub fn small(value: &Value) -> Result<i64, Interrupt> {
    int(value)?
        .to_i64()
        .ok_or_else(|| crash("this Integer is too large for the operation"))
}

pub fn decimal(value: &Value) -> Result<&Decimal, Interrupt> {
    match value {
        Value::Decimal(value) => Ok(value),
        other => Err(wrong("a Decimal", other)),
    }
}

pub fn float(value: &Value) -> Result<f64, Interrupt> {
    match value {
        Value::Float(value) => Ok(*value),
        other => Err(wrong("a Float", other)),
    }
}

pub fn boolean(value: &Value) -> Result<bool, Interrupt> {
    value.as_bool().ok_or_else(|| wrong("a Boolean", value))
}

pub fn list(value: &Value) -> Result<&Rc<Vec<Value>>, Interrupt> {
    value.as_list().ok_or_else(|| wrong("a List", value))
}

pub fn map(value: &Value) -> Result<&Rc<IndexMap<Value, Value>>, Interrupt> {
    match value {
        Value::Map(value) => Ok(value),
        other => Err(wrong("a Map", other)),
    }
}

pub fn set(value: &Value) -> Result<&Rc<IndexSet<Value>>, Interrupt> {
    match value {
        Value::Set(value) => Ok(value),
        other => Err(wrong("a Set", other)),
    }
}

pub fn range(value: &Value) -> Result<&Rc<RangeValue>, Interrupt> {
    match value {
        Value::Range(value) => Ok(value),
        other => Err(wrong("a Range", other)),
    }
}

pub fn bytes(value: &Value) -> Result<&Rc<[u8]>, Interrupt> {
    match value {
        Value::Bytes(value) => Ok(value),
        other => Err(wrong("Bytes", other)),
    }
}

pub fn duration(value: &Value) -> Result<i64, Interrupt> {
    match value {
        Value::Duration(ms) => Ok(*ms),
        other => Err(wrong("a Duration", other)),
    }
}

pub fn instant(value: &Value) -> Result<i64, Interrupt> {
    match value {
        Value::Instant(ms) => Ok(*ms),
        other => Err(wrong("an Instant", other)),
    }
}

// ---------------------------------------------------------- library values

impl Vm<'_> {
    pub fn library_type(&self, module: &str, name: &str) -> Result<TypeId, Interrupt> {
        self.program.types.find(module, name).ok_or_else(|| {
            crash(format!(
                "the library type `{module}.{name}` is not declared"
            ))
        })
    }

    pub fn library_record(
        &self,
        module: &str,
        name: &str,
        fields: Vec<Value>,
    ) -> Result<Value, Interrupt> {
        Ok(Value::record(self.library_type(module, name)?, fields))
    }

    pub fn library_variant(
        &self,
        module: &str,
        name: &str,
        variant: &str,
        fields: Vec<Value>,
    ) -> Result<Value, Interrupt> {
        let ty = self.library_type(module, name)?;
        let tag = self
            .program
            .types
            .variant_tag(ty, variant)
            .ok_or_else(|| crash(format!("`{name}` has no variant `{variant}`")))?;
        Ok(Value::variant(ty, tag, fields))
    }

    /// A failure with a library record as the error.
    pub fn fail_record(
        &self,
        module: &str,
        name: &str,
        fields: Vec<Value>,
    ) -> Result<Value, Interrupt> {
        Ok(Value::failure(self.library_record(module, name, fields)?))
    }

    /// A failure with a variant of a library sum type as the error.
    pub fn fail_variant(
        &self,
        module: &str,
        name: &str,
        variant: &str,
        fields: Vec<Value>,
    ) -> Result<Value, Interrupt> {
        Ok(Value::failure(
            self.library_variant(module, name, variant, fields)?,
        ))
    }
}

// ----------------------------------------------------------------- base64

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(triple >> 18) as usize & 63] as char);
        out.push(ALPHABET[(triple >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(triple >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[triple as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut buffer = 0u32;
    let mut bits = 0;
    for byte in text.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return None,
        } as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(out)
}
