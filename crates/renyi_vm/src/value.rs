//! Runtime values. Every value is immutable and reference counted (decision
//! O1); a collection is updated in place by `Rc::make_mut` when nothing else
//! holds it. A `maybe T` is the value itself or `Nothing`; the result of a
//! fallible call is the value itself or `Failure(error)`, which only exists
//! between the call and the `otherwise` or `match` that handles it. A value
//! that entered through a guarded capability (`only to`, decision P3) is
//! wrapped in `Guarded` with its origins; the wrapper is always outermost,
//! so that a container holds plain items and carries the union of their
//! origins, and every operation strips its operands and tags its result.

use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use indexmap::{IndexMap, IndexSet};
use renyi_check::{FunctionId, TypeId};

use crate::decimal::Decimal;
use crate::integer::Int;
use crate::pinned::Pinned;

// Decision AR4: the layout is fixed, so that the generated code reads the
// tag at offset 0 and the payload at offset 8 (the discriminants are the
// declaration order, `Rc` payloads are pointers to allocations whose first
// word is the strong count).
#[derive(Clone, Debug)]
#[repr(C, u8)]
pub enum Value {
    Nothing,
    Boolean(bool),
    Integer(Int),
    /// Boxed: a `Decimal` is 40 bytes wide, and a `Value` stays at 24
    /// (decision X3).
    Decimal(Rc<Decimal>),
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
    /// A value with its origins (decision P3): one bit per guarded
    /// capability of the run, in the order of the grant. Never wraps
    /// `Nothing`, a `Boolean`, a function, a native value or a `Failure`,
    /// whose error carries the origins instead.
    Guarded(Rc<(u64, Value)>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RangeValue {
    pub from: Int,
    pub to: Int,
    pub by: Int,
}

/// A record: its type and its fields. `tag` is always `usize::MAX`, so
/// that a record and a variant hold their type, their tag and their
/// fields at the same offsets and the generated code reads a field of
/// either on one path (decision AT2; the field cache has `usize::MAX` as
/// a record's tag).
#[derive(Clone, Debug)]
#[repr(C)]
pub struct Record {
    pub ty: TypeId,
    pub tag: usize,
    pub fields: Pinned<Value>,
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct Variant {
    pub ty: TypeId,
    pub tag: usize,
    pub fields: Pinned<Value>,
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
    /// A loop's position over its source: the list itself when the source
    /// was one, else a snapshot of the items.
    Iterator(RefCell<(Rc<Vec<Value>>, usize)>),
    /// A loop's position over a range of small Integers: the next value,
    /// the last one and the step, and whether it has run out; the shape
    /// the generated code keeps in registers (decision AG3), so that a
    /// frame can change hands in the middle of such a loop.
    RangeIterator {
        current: std::cell::Cell<i64>,
        to: i64,
        by: i64,
        done: std::cell::Cell<bool>,
    },
    /// A `within` deadline as an instant in milliseconds, with the duration.
    Deadline(i64, i64),
    /// An SQLite connection and the path it was opened on; `None` once
    /// closed, or on a replay, where no query reaches the file.
    Connection {
        connection: RefCell<Option<rusqlite::Connection>>,
        path: String,
    },
}

// Decision X3: the stack and every collection hold values by this width.
const _: () = assert!(std::mem::size_of::<Value>() <= 24);

/// The layout of a `Value` as the generated code reads it (decision AR4):
/// the tag at offset 0, the payload at offset 8; an `Rc` payload is a
/// pointer to an allocation whose first word is the strong count; an
/// `Integer` holds an `Int`, whose own tag lies at offset 8 and whose
/// payload at 16. The test below holds these to the types.
pub mod layout {
    pub const SIZE: usize = 24;
    pub const PAYLOAD: i32 = 8;
    pub const TAG_NOTHING: u8 = 0;
    pub const TAG_BOOLEAN: u8 = 1;
    pub const TAG_INTEGER: u8 = 2;
    pub const TAG_FLOAT: u8 = 4;
    pub const TAG_FAILURE: u8 = 18;
    /// One bit per tag whose payload is an `Rc` at `PAYLOAD`.
    pub const RC_TAGS: u64 = (1 << 3)
        | (1 << 5)
        | (1 << 6)
        | (1 << 7)
        | (1 << 8)
        | (1 << 9)
        | (1 << 10)
        | (1 << 11)
        | (1 << 12)
        | (1 << 13)
        | (1 << 17)
        | (1 << 18)
        | (1 << 19);
    /// Inside an `Integer`: the `Int` tag and its payload.
    pub const INT_TAG: i32 = 8;
    pub const INT_PAYLOAD: i32 = 16;
    pub const INT_SMALL: u8 = 0;
    pub const INT_BIG: u8 = 1;
    pub const TAG_RECORD: u8 = 12;
    pub const TAG_VARIANT: u8 = 13;
    /// Inside an `Rc` allocation: the value, after the two counts.
    pub const RC_VALUE: i32 = 16;
    /// Inside a `Record` and a `Variant`: the type, the tag and the
    /// fields (a pinned vector: pointer, length, capacity).
    pub const RECORD_TY: i32 = std::mem::offset_of!(super::Record, ty) as i32;
    pub const RECORD_TAG: i32 = std::mem::offset_of!(super::Record, tag) as i32;
    pub const RECORD_FIELDS: i32 = std::mem::offset_of!(super::Record, fields) as i32;
    pub const VARIANT_TY: i32 = std::mem::offset_of!(super::Variant, ty) as i32;
    pub const VARIANT_TAG: i32 = std::mem::offset_of!(super::Variant, tag) as i32;
    pub const VARIANT_FIELDS: i32 = std::mem::offset_of!(super::Variant, fields) as i32;
    // a record and a variant share the prefix the generated code reads
    // (decision AT2)
    const _: () = assert!(RECORD_TY == VARIANT_TY);
    const _: () = assert!(RECORD_TAG == VARIANT_TAG);
    const _: () = assert!(RECORD_FIELDS == VARIANT_FIELDS);
}

thread_local! {
    /// The one-character texts of the ASCII range (`Value::character`).
    static ASCII: [Rc<str>; 128] = std::array::from_fn(|code| {
        let mut buffer = [0; 4];
        Rc::from(&*(code as u8 as char).encode_utf8(&mut buffer))
    });
}

impl Value {
    pub fn text(text: impl AsRef<str>) -> Value {
        Value::Text(Rc::from(text.as_ref()))
    }

    /// A text of one character. The ASCII ones come from a table kept per
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

    pub fn decimal(value: Decimal) -> Value {
        Value::Decimal(Rc::new(value))
    }

    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(items))
    }

    pub fn pair(left: Value, right: Value) -> Value {
        Value::Pair(Rc::new((left, right)))
    }

    pub fn record(ty: TypeId, fields: Vec<Value>) -> Value {
        Value::Record(Rc::new(Record {
            ty,
            tag: usize::MAX,
            fields: fields.into(),
        }))
    }

    pub fn variant(ty: TypeId, tag: usize, fields: Vec<Value>) -> Value {
        Value::Variant(Rc::new(Variant {
            ty,
            tag,
            fields: fields.into(),
        }))
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
        match self.plain() {
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
        match self.plain() {
            Value::Integer(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.as_int().and_then(Int::to_i64)
    }

    pub fn as_list(&self) -> Option<&Rc<Vec<Value>>> {
        match self.plain() {
            Value::List(items) => Some(items),
            _ => None,
        }
    }

    /// The runtime type of a record, variant or subtype-carrying value, when
    /// it has one of its own.
    pub fn type_id(&self) -> Option<TypeId> {
        match self.plain() {
            Value::Record(record) => Some(record.ty),
            Value::Variant(variant) => Some(variant.ty),
            _ => None,
        }
    }

    // ------------------------------------------------------------- origins

    /// Whether the value is wrapped with origins: the test the fast paths
    /// make before they strip anything.
    #[inline]
    pub fn is_guarded(&self) -> bool {
        matches!(self, Value::Guarded(_))
    }

    /// The guarded capabilities the value came through (decision P3), one
    /// bit each; `0` for a plain value. A `Failure` answers for its error.
    pub fn origins(&self) -> u64 {
        match self {
            Value::Guarded(guarded) => guarded.0 | guarded.1.origins(),
            Value::Failure(error) => error.origins(),
            _ => 0,
        }
    }

    /// The value under its guard wrappers.
    pub fn plain(&self) -> &Value {
        let mut value = self;
        while let Value::Guarded(guarded) = value {
            value = &guarded.1;
        }
        value
    }

    pub fn into_plain(self) -> Value {
        let mut value = self;
        while let Value::Guarded(guarded) = value {
            value = match Rc::try_unwrap(guarded) {
                Ok((_, inner)) => inner,
                Err(shared) => shared.1.clone(),
            };
        }
        value
    }

    /// The value with the origins added. Nothing, a Boolean, a function and
    /// a native value carry none: a decision taken on guarded data is an
    /// implicit flow, which guards do not track (`06-runtime-guarantees.md`
    /// section 3.4). A `Failure` stays outermost and tags its error.
    pub fn guarded(self, origins: u64) -> Value {
        if origins == 0 {
            return self;
        }
        match self {
            Value::Nothing | Value::Boolean(_) | Value::Function(_) | Value::Native(_) => self,
            Value::Failure(error) => {
                let error = Rc::try_unwrap(error).unwrap_or_else(|shared| (*shared).clone());
                Value::failure(error.guarded(origins))
            }
            Value::Guarded(guarded) => {
                let (inner_origins, inner) =
                    Rc::try_unwrap(guarded).unwrap_or_else(|shared| (*shared).clone());
                Value::Guarded(Rc::new((inner_origins | origins, inner)))
            }
            other => Value::Guarded(Rc::new((origins, other))),
        }
    }

    /// What the value is, for messages.
    pub fn kind_name(&self) -> &'static str {
        match self.plain() {
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
            Value::Guarded(_) => "guarded value",
        }
    }
}

/// The origins of several values and the values themselves made plain: the
/// operands of an operation whose result carries their union. Plain
/// operands, the usual case, pass through untouched.
pub fn plain_all(values: Vec<Value>) -> (u64, Vec<Value>) {
    if !values.iter().any(Value::is_guarded) {
        return (0, values);
    }
    let mut origins = 0;
    let values = values
        .into_iter()
        .map(|value| {
            origins |= value.origins();
            value.into_plain()
        })
        .collect();
    (origins, values)
}

/// The origins of several values, which are made plain in place: the
/// arguments of a primitive, in the VM's scratch buffer.
pub fn plain_in_place(values: &mut [Value]) -> u64 {
    let mut origins = 0;
    for value in values.iter_mut() {
        if value.is_guarded() {
            origins |= value.origins();
            let plain = std::mem::replace(value, Value::Nothing).into_plain();
            *value = plain;
        }
    }
    origins
}

/// Structural equality: the derived `Equal` of every data type. Numbers
/// compare by value, a `Float` by its value with `-0.0` equal to `0.0`; a
/// guard wrapper is transparent.
impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self.plain(), other.plain()) {
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
        let value = self.plain();
        std::mem::discriminant(value).hash(state);
        match value {
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
            Value::Guarded(guarded) => guarded.1.hash(state),
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

#[cfg(test)]
mod layout_tests {
    use super::layout::*;
    use super::*;

    fn tag(value: &Value) -> u8 {
        // SAFETY: `Value` is `repr(C, u8)`: its first byte is the tag.
        unsafe { *(value as *const Value as *const u8) }
    }

    fn payload_word(value: &Value, offset: usize) -> usize {
        // SAFETY: the value is 24 bytes wide and the word lies inside it.
        unsafe { *((value as *const Value as *const u8).add(offset) as *const usize) }
    }

    #[test]
    fn the_constants_match_the_types() {
        assert_eq!(std::mem::size_of::<Value>(), SIZE);
        assert_eq!(std::mem::size_of::<Int>(), 16);
        assert_eq!(tag(&Value::Nothing), TAG_NOTHING);
        assert_eq!(tag(&Value::Boolean(true)), TAG_BOOLEAN);
        assert_eq!(tag(&Value::integer(5)), TAG_INTEGER);
        assert_eq!(tag(&Value::Float(1.5)), TAG_FLOAT);
        assert_eq!(tag(&Value::failure(Value::Nothing)), TAG_FAILURE);
        assert_eq!(
            payload_word(&Value::Boolean(true), PAYLOAD as usize) as u8,
            1
        );
        assert_eq!(
            payload_word(&Value::Boolean(false), PAYLOAD as usize) as u8,
            0
        );
        assert_eq!(
            payload_word(&Value::Float(1.5), PAYLOAD as usize),
            1.5f64.to_bits() as usize
        );
        let big = Int::from_big(num_bigint::BigInt::from(i64::MAX) * 4);
        let values = [
            Value::decimal(Decimal::parse("1.5").unwrap()),
            Value::text("t"),
            Value::Bytes(Rc::from(&b"b"[..])),
            Value::list(vec![]),
            Value::Map(Rc::new(IndexMap::new())),
            Value::Set(Rc::new(IndexSet::new())),
            Value::Range(Rc::new(RangeValue {
                from: Int::Small(1),
                to: Int::Small(2),
                by: Int::Small(1),
            })),
            Value::pair(Value::Nothing, Value::Nothing),
            Value::record(0, vec![]),
            Value::variant(0, 0, vec![]),
            Value::Native(Rc::new(Native::Deadline(0, 0))),
            Value::failure(Value::Nothing),
            Value::Guarded(Rc::new((1, Value::Nothing))),
        ];
        let mut rc_tags = 0u64;
        for value in &values {
            rc_tags |= 1 << tag(value);
        }
        assert_eq!(rc_tags, RC_TAGS);
        for plain in [
            Value::Nothing,
            Value::Boolean(false),
            Value::integer(1),
            Value::Float(0.0),
            Value::Duration(1),
            Value::Instant(1),
            Value::Function(0),
        ] {
            assert_eq!((RC_TAGS >> tag(&plain)) & 1, 0, "{plain:?}");
        }
        // an `Rc` payload points at its allocation, whose first word is the
        // strong count (`Rc::as_ptr` points two words past it)
        let list = Rc::new(vec![Value::Nothing]);
        let value = Value::List(list.clone());
        assert_eq!(Rc::strong_count(&list), 2);
        let allocation = payload_word(&value, PAYLOAD as usize);
        assert_eq!(allocation + 16, Rc::as_ptr(&list) as usize);
        assert_eq!(unsafe { *(allocation as *const usize) }, 2);
        // a text's payload is a fat pointer whose address is the allocation
        let text: Rc<str> = Rc::from("abc");
        let value = Value::Text(text.clone());
        let allocation = payload_word(&value, PAYLOAD as usize);
        assert_eq!(allocation + 16, Rc::as_ptr(&text) as *const u8 as usize);
        assert_eq!(unsafe { *(allocation as *const usize) }, 2);
        // an Integer: the Int's tag and payload
        let small = Value::integer(7);
        assert_eq!(payload_word(&small, INT_TAG as usize) as u8, INT_SMALL);
        assert_eq!(payload_word(&small, INT_PAYLOAD as usize), 7);
        let value = Value::Integer(big.clone());
        assert_eq!(payload_word(&value, INT_TAG as usize) as u8, INT_BIG);
        if let Int::Big(rc) = &big {
            let allocation = payload_word(&value, INT_PAYLOAD as usize);
            assert_eq!(allocation + 16, Rc::as_ptr(rc) as usize);
        }
        // a record's and a variant's type, tag and fields inside the
        // allocation
        let word = |address: usize| unsafe { *(address as *const usize) };
        let record = Value::record(3, vec![Value::integer(1), Value::text("a")]);
        let Value::Record(rc) = &record else {
            unreachable!()
        };
        let allocation = payload_word(&record, PAYLOAD as usize);
        let inner = allocation + RC_VALUE as usize;
        assert_eq!(inner, Rc::as_ptr(rc) as usize);
        assert_eq!(word(inner + RECORD_TY as usize), 3);
        assert_eq!(
            word(inner + RECORD_FIELDS as usize),
            rc.fields.as_ptr() as usize
        );
        assert_eq!(word(inner + RECORD_FIELDS as usize + 8), 2);
        let variant = Value::variant(4, 2, vec![Value::Nothing]);
        let Value::Variant(rc) = &variant else {
            unreachable!()
        };
        let inner = payload_word(&variant, PAYLOAD as usize) + RC_VALUE as usize;
        assert_eq!(inner, Rc::as_ptr(rc) as usize);
        assert_eq!(word(inner + VARIANT_TY as usize), 4);
        assert_eq!(word(inner + VARIANT_TAG as usize), 2);
        assert_eq!(
            word(inner + VARIANT_FIELDS as usize),
            rc.fields.as_ptr() as usize
        );
        assert_eq!(word(inner + VARIANT_FIELDS as usize + 8), 1);
    }
}
