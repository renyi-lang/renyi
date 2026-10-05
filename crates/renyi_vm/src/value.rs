//! Runtime values. Every value is immutable and reference counted (decision
//! O1); a collection is updated in place by `Rc::make_mut` when nothing else
//! holds it. A `maybe T` is the value itself or `Nothing`; the result of a
//! fallible call is the value itself or `Failure(error)`, which only exists
//! between the call and the `otherwise` or `match` that handles it.

use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use indexmap::{IndexMap, IndexSet};
use renyi_check::{FunctionId, TypeId};

use crate::decimal::Decimal;
use crate::integer::Int;

#[derive(Clone, Debug)]
pub enum Value {
    Nothing,
    Boolean(bool),
    Integer(Int),
    Decimal(Decimal),
    Float(f64),
    Text(Rc<str>),
    Bytes(Rc<[u8]>),
    List(Rc<Vec<Value>>),
    Map(Rc<IndexMap<Value, Value>>),
    Set(Rc<IndexSet<Value>>),
    Range(Rc<RangeValue>),
    Pair(Rc<(Value, Value)>),
    Record(Rc<Record>),
    Variant(Rc<Variant>),
    /// Milliseconds.
    Duration(i64),
    /// Milliseconds since the Unix epoch, UTC.
    Instant(i64),
    Function(FunctionId),
    Native(Rc<Native>),
    /// The error of a failed fallible call, on its way to a handler.
    Failure(Rc<Value>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RangeValue {
    pub from: Int,
    pub to: Int,
    pub by: Int,
}

#[derive(Clone, Debug)]
pub struct Record {
    pub ty: TypeId,
    pub fields: Vec<Value>,
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub ty: TypeId,
    pub tag: usize,
    pub fields: Vec<Value>,
}

/// Values of the library that carry more than their declared fields.
#[derive(Debug)]
pub enum Native {
    /// A row of a CSV file with a header: the line, the header names and the
    /// cells.
    CsvRow {
        line: i64,
        header: Rc<Vec<String>>,
        cells: Vec<String>,
    },
    /// A loop's position over a snapshot of its source.
    Iterator(RefCell<(Vec<Value>, usize)>),
    /// A `within` deadline as an instant in milliseconds, with the duration.
    Deadline(i64, i64),
    /// An open SQLite connection (a placeholder until the driver exists).
    Connection(RefCell<Option<String>>),
}

impl Value {
    pub fn text(text: impl AsRef<str>) -> Value {
        Value::Text(Rc::from(text.as_ref()))
    }

    pub fn integer(value: i64) -> Value {
        Value::Integer(Int::Small(value))
    }

    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(items))
    }

    pub fn pair(left: Value, right: Value) -> Value {
        Value::Pair(Rc::new((left, right)))
    }

    pub fn record(ty: TypeId, fields: Vec<Value>) -> Value {
        Value::Record(Rc::new(Record { ty, fields }))
    }

    pub fn variant(ty: TypeId, tag: usize, fields: Vec<Value>) -> Value {
        Value::Variant(Rc::new(Variant { ty, tag, fields }))
    }

    pub fn failure(error: Value) -> Value {
        Value::Failure(Rc::new(error))
    }

    pub fn is_nothing(&self) -> bool {
        matches!(self, Value::Nothing)
    }

    pub fn is_failure(&self) -> bool {
        matches!(self, Value::Failure(_))
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<&Int> {
        match self {
            Value::Integer(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.as_int().and_then(Int::to_i64)
    }

    pub fn as_list(&self) -> Option<&Rc<Vec<Value>>> {
        match self {
            Value::List(items) => Some(items),
            _ => None,
        }
    }

    /// The runtime type of a record, variant or subtype-carrying value, when
    /// it has one of its own.
    pub fn type_id(&self) -> Option<TypeId> {
        match self {
            Value::Record(record) => Some(record.ty),
            Value::Variant(variant) => Some(variant.ty),
            _ => None,
        }
    }

    /// What the value is, for messages.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Value::Nothing => "nothing",
            Value::Boolean(_) => "Boolean",
            Value::Integer(_) => "Integer",
            Value::Decimal(_) => "Decimal",
            Value::Float(_) => "Float",
            Value::Text(_) => "Text",
            Value::Bytes(_) => "Bytes",
            Value::List(_) => "List",
            Value::Map(_) => "Map",
            Value::Set(_) => "Set",
            Value::Range(_) => "Range",
            Value::Pair(_) => "Pair",
            Value::Record(_) => "record",
            Value::Variant(_) => "variant",
            Value::Duration(_) => "Duration",
            Value::Instant(_) => "Instant",
            Value::Function(_) => "function",
            Value::Native(_) => "native value",
            Value::Failure(_) => "failure",
        }
    }
}

/// Structural equality: the derived `Equal` of every data type. Numbers
/// compare by value, a `Float` by its value with `-0.0` equal to `0.0`.
impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Nothing, Value::Nothing) => true,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Decimal(a), Value::Decimal(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => {
                a.len() == b.len() && a.iter().all(|(k, v)| b.get(k) == Some(v))
            }
            (Value::Set(a), Value::Set(b)) => a.len() == b.len() && a.iter().all(|v| b.contains(v)),
            (Value::Range(a), Value::Range(b)) => a == b,
            (Value::Pair(a), Value::Pair(b)) => a == b,
            (Value::Record(a), Value::Record(b)) => a.ty == b.ty && a.fields == b.fields,
            (Value::Variant(a), Value::Variant(b)) => {
                a.ty == b.ty && a.tag == b.tag && a.fields == b.fields
            }
            (Value::Duration(a), Value::Duration(b)) => a == b,
            (Value::Instant(a), Value::Instant(b)) => a == b,
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::Native(a), Value::Native(b)) => Rc::ptr_eq(a, b),
            (Value::Failure(a), Value::Failure(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Nothing => {}
            Value::Boolean(value) => value.hash(state),
            Value::Integer(value) => value.hash(state),
            Value::Decimal(value) => value.hash(state),
            Value::Float(value) => {
                let bits = if *value == 0.0 { 0.0f64 } else { *value };
                bits.to_bits().hash(state)
            }
            Value::Text(value) => value.hash(state),
            Value::Bytes(value) => value.hash(state),
            Value::List(items) => items.hash(state),
            Value::Map(entries) => {
                entries.len().hash(state);
                for (key, value) in entries.iter() {
                    key.hash(state);
                    value.hash(state);
                }
            }
            Value::Set(items) => {
                items.len().hash(state);
                for item in items.iter() {
                    item.hash(state);
                }
            }
            Value::Range(range) => range.hash(state),
            Value::Pair(pair) => pair.hash(state),
            Value::Record(record) => {
                record.ty.hash(state);
                record.fields.hash(state);
            }
            Value::Variant(variant) => {
                variant.ty.hash(state);
                variant.tag.hash(state);
                variant.fields.hash(state);
            }
            Value::Duration(value) | Value::Instant(value) => value.hash(state),
            Value::Function(id) => id.hash(state),
            Value::Native(native) => Rc::as_ptr(native).hash(state),
            Value::Failure(error) => error.hash(state),
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Value {
        Value::Boolean(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Value {
        Value::integer(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Value {
        Value::Text(Rc::from(value))
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Value {
        Value::Text(Rc::from(value))
    }
}

/// An Rc-held list whose contents are taken out when unique and cloned
/// otherwise: the in-place update of decision O1.
pub fn take_list(list: Rc<Vec<Value>>) -> Vec<Value> {
    Rc::try_unwrap(list).unwrap_or_else(|shared| (*shared).clone())
}

pub fn take_map(map: Rc<IndexMap<Value, Value>>) -> IndexMap<Value, Value> {
    Rc::try_unwrap(map).unwrap_or_else(|shared| (*shared).clone())
}

pub fn take_set(set: Rc<IndexSet<Value>>) -> IndexSet<Value> {
    Rc::try_unwrap(set).unwrap_or_else(|shared| (*shared).clone())
}
