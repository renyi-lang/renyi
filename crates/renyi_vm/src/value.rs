//! Runtime values. Every value is immutable and reference counted (decision
//! O1); a collection is updated in place by `Rc::make_mut` when nothing else
//! holds it. A `maybe T` is the value itself or `Nothing`; the result of a
//! fallible call is the value itself or `Failure(error)`, which only exists
//! between the call and the `otherwise` or `match` that handles it. A value
//! that entered through a guarded capability (`only to`, decision P3) is
//! wrapped in `Guarded` with its origins; the wrapper is always outermost,
//! so that a container holds plain items and carries the union of their
//! origins, and every operation strips its operands and tags its result.

use std::alloc::Layout;
use std::cell::{Cell, RefCell};
use std::hash::{Hash, Hasher};
use std::ptr::NonNull;
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
    Record(Composite),
    Variant(Composite),
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
    /// A text of at most `SMALL_TEXT` bytes, held in the value itself
    /// (decision AU26); a longer one is a `Text`. Every text is made by
    /// `Value::text` or `Value::character`, which choose by the length,
    /// so that a text has one form for its characters.
    SmallText(SmallText),
}

/// The longest text a value holds in itself (decision AU26): the sixteen
/// bytes past the tag hold its length and its bytes.
pub const SMALL_TEXT: usize = 15;

/// A text of at most `SMALL_TEXT` bytes in place (decision AU26): the
/// length, then the bytes, zero after them, so that two hold the same
/// characters exactly when their sixteen bytes are equal, which the
/// generated code compares as two words.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct SmallText {
    len: u8,
    bytes: [u8; SMALL_TEXT],
}

impl SmallText {
    /// The text in place, when it is short enough.
    #[inline]
    pub fn new(text: &str) -> Option<SmallText> {
        let len = text.len();
        if len > SMALL_TEXT {
            return None;
        }
        let mut bytes = [0; SMALL_TEXT];
        copy_short(&mut bytes, text.as_bytes());
        Some(SmallText {
            len: len as u8,
            bytes,
        })
    }

    /// The text of one character.
    #[inline]
    pub fn character(c: char) -> SmallText {
        let mut bytes = [0; SMALL_TEXT];
        let len = c.encode_utf8(&mut bytes[..4]).len();
        SmallText {
            len: len as u8,
            bytes,
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: the bytes up to the length were copied from a `str`
        // (`new`), so they are UTF-8.
        unsafe { std::str::from_utf8_unchecked(&self.bytes[..self.len as usize]) }
    }
}

/// At most `SMALL_TEXT` bytes copied to the start of `to` without a call:
/// two words, two halves or two quarters that overlap in the middle, as
/// the length asks, so that nothing past either end is read or written.
#[inline]
fn copy_short(to: &mut [u8; SMALL_TEXT], from: &[u8]) {
    let len = from.len();
    debug_assert!(len <= SMALL_TEXT);
    let (source, target) = (from.as_ptr(), to.as_mut_ptr());
    // SAFETY: each read lies within `from` and each write within `to`,
    // whose length is at least `len`; the reads and writes are unaligned.
    unsafe {
        if len >= 8 {
            let head = (source as *const u64).read_unaligned();
            let tail = (source.add(len - 8) as *const u64).read_unaligned();
            (target as *mut u64).write_unaligned(head);
            (target.add(len - 8) as *mut u64).write_unaligned(tail);
        } else if len >= 4 {
            let head = (source as *const u32).read_unaligned();
            let tail = (source.add(len - 4) as *const u32).read_unaligned();
            (target as *mut u32).write_unaligned(head);
            (target.add(len - 4) as *mut u32).write_unaligned(tail);
        } else if len >= 2 {
            let head = (source as *const u16).read_unaligned();
            let tail = (source.add(len - 2) as *const u16).read_unaligned();
            (target as *mut u16).write_unaligned(head);
            (target.add(len - 2) as *mut u16).write_unaligned(tail);
        } else if len == 1 {
            *target = *source;
        }
    }
}

impl std::fmt::Debug for SmallText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RangeValue {
    pub from: Int,
    pub to: Int,
    pub by: Int,
}

/// A record or a variant in one counted block (decision AU27): the
/// count, the number of fields, the type, the tag and the fields
/// themselves, so that making one is one allocation and the generated
/// code reads a field at `layout::RECORD_FIELDS` past the block's start.
/// A record's tag is `usize::MAX`, so that a record and a variant read
/// alike (decision AT2; the field cache has `usize::MAX` as a record's
/// tag). `Deref` gives the `Shape`; a clone shares the block, as an `Rc`
/// does, and `make_mut` copies it before an update unless it is held once.
pub struct Composite {
    block: NonNull<Head>,
}

/// The two words of a composite's block before its shape.
#[repr(C)]
struct Head {
    count: Cell<usize>,
    len: usize,
}

/// What a composite holds past its head: the type, the tag and the
/// fields, a slice of the length the head keeps.
#[repr(C)]
pub struct Shape {
    pub ty: TypeId,
    pub tag: usize,
    pub fields: [Value],
}

impl Composite {
    /// The layout of a block of `len` fields.
    fn layout(len: usize) -> Layout {
        let size = layout::RECORD_FIELDS as usize + len * std::mem::size_of::<Value>();
        Layout::from_size_align(size, std::mem::align_of::<Value>().max(8))
            .expect("the layout of a record")
    }

    /// A block held once with the type and the tag written, its `len`
    /// fields for the caller to write.
    fn allocate(ty: TypeId, tag: usize, len: usize) -> NonNull<Head> {
        let layout = Composite::layout(len);
        // SAFETY: the layout is never empty: the head, the type and the
        // tag take 32 bytes.
        let raw = unsafe { std::alloc::alloc(layout) };
        let Some(block) = NonNull::new(raw as *mut Head) else {
            std::alloc::handle_alloc_error(layout)
        };
        // SAFETY: the block is fresh and holds the head, then the type and
        // the tag at the offsets `Shape` has them (`layout::RECORD_TY`).
        unsafe {
            block.as_ptr().write(Head {
                count: Cell::new(1),
                len,
            });
            (raw.add(layout::RECORD_TY as usize) as *mut TypeId).write(ty);
            (raw.add(layout::RECORD_TAG as usize) as *mut usize).write(tag);
        }
        block
    }

    /// Where a block's fields begin.
    fn fields_of(block: NonNull<Head>) -> *mut Value {
        // SAFETY: the fields lie within the block, `RECORD_FIELDS` past
        // its start.
        unsafe { (block.as_ptr() as *mut u8).add(layout::RECORD_FIELDS as usize) as *mut Value }
    }

    /// A composite of the fields, moved in.
    pub fn new(ty: TypeId, tag: usize, mut fields: Vec<Value>) -> Composite {
        let len = fields.len();
        let block = Composite::allocate(ty, tag, len);
        // SAFETY: the block has room for `len` fields; the values move
        // there and the vector forgets them before it frees its buffer.
        unsafe {
            std::ptr::copy_nonoverlapping(fields.as_ptr(), Composite::fields_of(block), len);
            fields.set_len(0);
        }
        Composite { block }
    }

    /// A composite of the top `len` values of a stack, moved in and the
    /// stack cut below them: a construction with no vector between.
    pub fn from_top(stack: &mut Pinned<Value>, len: usize, ty: TypeId, tag: usize) -> Composite {
        let at = stack.len() - len;
        let block = Composite::allocate(ty, tag, len);
        // SAFETY: the values move into the block's room for `len` fields,
        // and the stack forgets them.
        unsafe {
            std::ptr::copy_nonoverlapping(
                stack.as_slice().as_ptr().add(at),
                Composite::fields_of(block),
                len,
            );
        }
        stack.forget_from(at);
        Composite { block }
    }

    fn head(&self) -> &Head {
        // SAFETY: the block lives while a composite points at it.
        unsafe { self.block.as_ref() }
    }

    /// The shape as a pointer with the length of its fields.
    fn shape(&self) -> *mut Shape {
        let len = self.head().len;
        // SAFETY: the shape lies two words into the block.
        let data = unsafe { (self.block.as_ptr() as *mut u8).add(std::mem::size_of::<Head>()) };
        std::ptr::slice_from_raw_parts_mut(data, len) as *mut Shape
    }

    /// The shape to update in place when the block is held once.
    pub fn get_mut(&mut self) -> Option<&mut Shape> {
        if self.head().count.get() != 1 {
            return None;
        }
        // SAFETY: held once, so nothing else reads the block.
        Some(unsafe { &mut *self.shape() })
    }

    /// The shape to update in place, the block copied first when another
    /// value holds it too.
    pub fn make_mut(&mut self) -> &mut Shape {
        if self.head().count.get() != 1 {
            let len = self.fields.len();
            let block = Composite::allocate(self.ty, self.tag, len);
            let fields = Composite::fields_of(block);
            for (index, field) in self.fields.iter().enumerate() {
                // SAFETY: the copy has room for `len` fields.
                unsafe { fields.add(index).write(field.clone()) };
            }
            *self = Composite { block };
        }
        // SAFETY: held once now.
        unsafe { &mut *self.shape() }
    }

    /// How many values hold the block.
    pub fn count(&self) -> usize {
        self.head().count.get()
    }
}

impl Clone for Composite {
    /// One holder more, as the generated code counts one in place (decision
    /// AU24): without the check `Rc` makes on a count past `usize::MAX`,
    /// which no run reaches (every holder is a value of 24 bytes or a
    /// reference the VM releases) and whose call to `abort` would cost
    /// every clone of every value a frame of its own.
    #[inline(always)]
    fn clone(&self) -> Composite {
        let count = self.head().count.get();
        debug_assert!(count < usize::MAX, "the count of a record");
        self.head().count.set(count.wrapping_add(1));
        Composite { block: self.block }
    }
}

impl Drop for Composite {
    /// One holder fewer, in place; the last frees the block out of line,
    /// as an `Rc` does.
    #[inline(always)]
    fn drop(&mut self) {
        let count = self.head().count.get() - 1;
        self.head().count.set(count);
        if count == 0 {
            self.free();
        }
    }
}

impl Composite {
    /// The last holder's drop: the fields dropped and the block freed with
    /// the layout it was made with.
    #[inline(never)]
    fn free(&mut self) {
        let len = self.head().len;
        // SAFETY: nothing holds the block any more.
        unsafe {
            std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(
                Composite::fields_of(self.block),
                len,
            ));
            std::alloc::dealloc(self.block.as_ptr() as *mut u8, Composite::layout(len));
        }
    }
}

impl std::ops::Deref for Composite {
    type Target = Shape;

    #[inline]
    fn deref(&self) -> &Shape {
        // SAFETY: the shape lives while the block does.
        unsafe { &*self.shape() }
    }
}

impl std::fmt::Debug for Composite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Composite")
            .field("ty", &self.ty)
            .field("tag", &self.tag)
            .field("fields", &&self.fields)
            .finish()
    }
}

/// Values of the library that carry more than their declared fields. The
/// layout is fixed (decision AU1, stage ii): the tag at offset 0, the
/// payload at offset 8, so that the generated code reads a list
/// iterator in place (`layout::NATIVE_TAG`, `NATIVE_PAYLOAD`).
#[derive(Debug)]
#[repr(C, u8)]
pub enum Native {
    /// A row of a CSV file with a header: the line, the header names and the
    /// cells.
    CsvRow {
        line: i64,
        header: Rc<Vec<String>>,
        cells: Vec<String>,
    },
    /// A loop's position over its source: the list itself when the source
    /// was one, else a snapshot of the items; read and advanced by the
    /// generated code in place (decision AU1, stage ii).
    Iterator(ListIter),
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

/// A loop's position over a list (decision AU1, stage ii): the items'
/// address and count, which the generated code reads, the position, which
/// it advances, and the list itself, which keeps the items where they
/// are: a list another holder changes is copied first (`Rc::make_mut`),
/// and a list only the iterator holds is changed by nobody.
#[derive(Debug)]
#[repr(C)]
pub struct ListIter {
    pub items: *const Value,
    pub len: usize,
    pub position: std::cell::Cell<usize>,
    pub list: Rc<Vec<Value>>,
}

impl ListIter {
    pub fn new(list: Rc<Vec<Value>>) -> ListIter {
        ListIter {
            items: list.as_ptr(),
            len: list.len(),
            position: std::cell::Cell::new(0),
            list,
        }
    }

    /// The next item, the position advanced past it; `None` at the end,
    /// where the position stays.
    pub fn next(&self) -> Option<Value> {
        let at = self.position.get();
        let item = self.list.get(at).cloned();
        if item.is_some() {
            self.position.set(at + 1);
        }
        item
    }
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
    pub const TAG_TEXT: u8 = 5;
    pub const TAG_FAILURE: u8 = 18;
    /// A text in the value itself (decision AU26): its length at
    /// `PAYLOAD`, its bytes after it, zero to the end of the value.
    pub const TAG_SMALL_TEXT: u8 = 20;
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
    /// The tags a retain or a release in place tests (decisions AU29 and
    /// AU32): `RC_TAGS` and the Integer's, whose payload holds a counted
    /// block only when the Integer is big.
    pub const COUNTED_OR_INTEGER: u64 = RC_TAGS | 1 << TAG_INTEGER;
    /// Inside an `Integer`: the `Int` tag and its payload.
    pub const INT_TAG: i32 = 8;
    pub const INT_PAYLOAD: i32 = 16;
    pub const INT_SMALL: u8 = 0;
    pub const INT_BIG: u8 = 1;
    pub const TAG_RECORD: u8 = 12;
    pub const TAG_VARIANT: u8 = 13;
    /// Inside an `Rc` allocation: the value, after the two counts.
    pub const RC_VALUE: i32 = 16;
    /// Inside the block of a record or a variant (`Composite`, decision
    /// AU27), from its start: the count, the number of fields, the type,
    /// the tag and the fields themselves. The type and the tag lie where
    /// an `Rc` of the former layout held them, so that a field read
    /// changes only in where the fields are.
    pub const RECORD_LEN: i32 = 8;
    pub const RECORD_TY: i32 = 16;
    pub const RECORD_TAG: i32 = 24;
    pub const RECORD_FIELDS: i32 = 32;
    /// A `Native` value's tag, and inside a `Native` (`repr(C, u8)`) its
    /// own tag and its payload (decision AU1, stage ii).
    pub const TAG_NATIVE: u8 = 17;
    pub const NATIVE_TAG: i32 = 0;
    pub const NATIVE_PAYLOAD: i32 = 8;
    /// The tag of `Native::Iterator`.
    pub const NATIVE_ITERATOR: u8 = 1;
    /// Inside a `ListIter`: the items' address, their count, the position.
    pub const ITER_ITEMS: i32 = std::mem::offset_of!(super::ListIter, items) as i32;
    pub const ITER_LEN: i32 = std::mem::offset_of!(super::ListIter, len) as i32;
    pub const ITER_POSITION: i32 = std::mem::offset_of!(super::ListIter, position) as i32;
}

thread_local! {
    /// The empty list, which every list made without items shares
    /// (`Value::list`, decision AU25): a list that is appended to later is
    /// copied out of it first, as any list held twice is.
    static EMPTY_LIST: Rc<Vec<Value>> = Rc::new(Vec::new());
}

impl Value {
    /// A text: in the value itself when it is short (decision AU26),
    /// counted otherwise.
    #[inline]
    pub fn text(text: impl AsRef<str>) -> Value {
        let text = text.as_ref();
        match SmallText::new(text) {
            Some(small) => Value::SmallText(small),
            None => Value::Text(Rc::from(text)),
        }
    }

    /// A text of one character, always in the value itself.
    #[inline]
    pub fn character(c: char) -> Value {
        Value::SmallText(SmallText::character(c))
    }

    pub fn integer(value: i64) -> Value {
        Value::Integer(Int::Small(value))
    }

    pub fn decimal(value: Decimal) -> Value {
        Value::Decimal(Rc::new(value))
    }

    pub fn list(items: Vec<Value>) -> Value {
        if items.is_empty() {
            return EMPTY_LIST.with(|empty| Value::List(empty.clone()));
        }
        Value::List(Rc::new(items))
    }

    pub fn pair(left: Value, right: Value) -> Value {
        Value::Pair(Rc::new((left, right)))
    }

    pub fn record(ty: TypeId, fields: Vec<Value>) -> Value {
        Value::Record(Composite::new(ty, usize::MAX, fields))
    }

    pub fn variant(ty: TypeId, tag: usize, fields: Vec<Value>) -> Value {
        Value::Variant(Composite::new(ty, tag, fields))
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
            Value::SmallText(text) => Some(text.as_str()),
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
            Value::Text(_) | Value::SmallText(_) => "Text",
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
            (Value::SmallText(a), Value::SmallText(b)) => a == b,
            (Value::Text(a), Value::SmallText(b)) | (Value::SmallText(b), Value::Text(a)) => {
                **a == *b.as_str()
            }
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
        // a text hashes as its characters, in the value or counted
        if let Some(text) = value.as_text() {
            layout::TAG_TEXT.hash(state);
            text.hash(state);
            return;
        }
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
            Value::Text(_) | Value::SmallText(_) => unreachable!("a text hashed above"),
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
        Value::text(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Value {
        Value::text(value)
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
    fn a_list_iterator_lies_where_the_generated_code_reads_it() {
        let list = Rc::new(vec![Value::integer(7), Value::Nothing]);
        let value = Value::Native(Rc::new(Native::Iterator(ListIter::new(list.clone()))));
        assert_eq!(tag(&value), TAG_NATIVE);
        let allocation = payload_word(&value, PAYLOAD as usize);
        let native = allocation + RC_VALUE as usize;
        // SAFETY: the words lie inside the allocation of the native value.
        unsafe {
            assert_eq!(
                *((native + NATIVE_TAG as usize) as *const u8),
                NATIVE_ITERATOR
            );
            let iterator = native + NATIVE_PAYLOAD as usize;
            assert_eq!(
                *((iterator + ITER_ITEMS as usize) as *const usize),
                list.as_ptr() as usize
            );
            assert_eq!(*((iterator + ITER_LEN as usize) as *const usize), 2);
            assert_eq!(*((iterator + ITER_POSITION as usize) as *const usize), 0);
        }
        let Value::Native(native) = &value else {
            unreachable!()
        };
        let Native::Iterator(iterator) = &**native else {
            unreachable!()
        };
        assert_eq!(iterator.next(), Some(Value::integer(7)));
        assert_eq!(iterator.next(), Some(Value::Nothing));
        assert_eq!(iterator.next(), None);
        assert_eq!(iterator.position.get(), 2);
    }

    #[test]
    fn a_short_text_holds_its_bytes_and_zero_after_them_at_every_length() {
        let source = "abcdefghijklmnop";
        for len in 0..=SMALL_TEXT {
            let small = SmallText::new(&source[..len]).expect("short enough");
            assert_eq!(small.as_str(), &source[..len]);
            assert!(small.bytes[len..].iter().all(|byte| *byte == 0), "{len}");
        }
        assert!(SmallText::new(source).is_none());
        for c in ['a', 'é', '中', '😀'] {
            let small = SmallText::character(c);
            assert_eq!(small.as_str(), c.to_string());
            assert!(small.bytes[c.len_utf8()..].iter().all(|byte| *byte == 0));
        }
        // a text has one form for its characters, and both forms compare
        // and hash by them
        assert_eq!(Value::text("ab"), Value::from("ab".to_string()));
        assert_ne!(Value::text("ab"), Value::text("abc"));
        assert_eq!(
            Value::text("sixteen bytes!!!"),
            Value::text(String::from("sixteen bytes!!!"))
        );
        let mut set = IndexSet::new();
        set.insert(Value::text("ab"));
        set.insert(Value::text("sixteen bytes!!!"));
        assert!(!set.contains(&Value::character('a')));
        assert!(set.contains(&Value::text("ab")));
        assert!(set.contains(&Value::text("sixteen bytes!!!")));
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
            Value::text("a text past sixteen bytes"),
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
            Value::text("t"),
        ] {
            assert_eq!((RC_TAGS >> tag(&plain)) & 1, 0, "{plain:?}");
        }
        // a short text in the value (decision AU26): the tag, the length at
        // the payload, the bytes after it and zero to the end
        let small = Value::text("fifteen bytes!!");
        assert_eq!(tag(&small), TAG_SMALL_TEXT);
        assert_eq!(tag(&Value::text("sixteen bytes!!!")), TAG_TEXT);
        assert_eq!(tag(&Value::character('é')), TAG_SMALL_TEXT);
        // SAFETY: the value is 24 bytes wide.
        let bytes =
            unsafe { std::slice::from_raw_parts(&small as *const Value as *const u8, SIZE) };
        assert_eq!(bytes[PAYLOAD as usize], 15);
        assert_eq!(&bytes[PAYLOAD as usize + 1..], b"fifteen bytes!!");
        let short = Value::text("ab");
        // SAFETY: as above.
        let bytes =
            unsafe { std::slice::from_raw_parts(&short as *const Value as *const u8, SIZE) };
        assert_eq!(bytes[PAYLOAD as usize], 2);
        assert_eq!(&bytes[PAYLOAD as usize + 1..PAYLOAD as usize + 3], b"ab");
        assert!(bytes[PAYLOAD as usize + 3..].iter().all(|byte| *byte == 0));
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
        // a record's and a variant's count, length, type, tag and fields
        // in their one block (decision AU27)
        let word = |address: usize| unsafe { *(address as *const usize) };
        let record = Value::record(3, vec![Value::integer(1), Value::text("a")]);
        let Value::Record(composite) = &record else {
            unreachable!()
        };
        let block = payload_word(&record, PAYLOAD as usize);
        assert_eq!(word(block), 1);
        assert_eq!(word(block + RECORD_LEN as usize), 2);
        assert_eq!(word(block + RECORD_TY as usize), 3);
        assert_eq!(word(block + RECORD_TAG as usize), usize::MAX);
        assert_eq!(
            block + RECORD_FIELDS as usize,
            composite.fields.as_ptr() as usize
        );
        assert_eq!(composite.fields, [Value::integer(1), Value::text("a")][..]);
        let copy = record.clone();
        assert_eq!(word(block), 2);
        drop(copy);
        assert_eq!(word(block), 1);
        let variant = Value::variant(4, 2, vec![Value::Nothing]);
        let block = payload_word(&variant, PAYLOAD as usize);
        assert_eq!(word(block + RECORD_LEN as usize), 1);
        assert_eq!(word(block + RECORD_TY as usize), 4);
        assert_eq!(word(block + RECORD_TAG as usize), 2);
        assert_eq!(word(block + RECORD_FIELDS as usize) as u8, TAG_NOTHING);
    }

    #[test]
    fn a_composite_is_copied_before_an_update_unless_it_is_held_once() {
        let mut record = Composite::new(7, usize::MAX, vec![Value::integer(1), Value::text("a")]);
        let shared = record.clone();
        assert_eq!(record.count(), 2);
        assert!(record.get_mut().is_none());
        record.make_mut().fields[0] = Value::integer(2);
        assert_eq!(record.count(), 1);
        assert_eq!(shared.count(), 1);
        assert_eq!(shared.fields[0], Value::integer(1));
        assert_eq!(record.fields[0], Value::integer(2));
        assert_eq!(record.ty, 7);
        assert_eq!(record.tag, usize::MAX);
        let unique = record.get_mut().expect("held once");
        unique.fields[1] = Value::text("a text past sixteen bytes");
        assert_eq!(record.fields[1], Value::text("a text past sixteen bytes"));
        // from the top of a stack, moved: the stack forgets the values
        let mut stack: Pinned<Value> =
            vec![Value::Nothing, Value::integer(5), Value::text("b")].into();
        let moved = Composite::from_top(&mut stack, 2, 9, 1);
        assert_eq!(stack.len(), 1);
        assert_eq!(moved.fields, [Value::integer(5), Value::text("b")][..]);
        assert_eq!((moved.ty, moved.tag), (9, 1));
        let empty = Composite::new(1, 0, Vec::new());
        assert!(empty.fields.is_empty());
    }
}
