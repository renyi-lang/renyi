//! The foreign function interface (decision AF1): the types that cross the
//! boundary to a C library, recognised on a checked signature, and their
//! spellings in the bytecode file, which the VM reads back.

use crate::types::Ty;
use crate::world::World;

/// A C type of the boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CType {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    Size,
    F64,
    Bool,
    Text,
    Bytes,
}

/// What a foreign function returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CResult {
    Void,
    Value(CType),
    /// `maybe Text`: a `char *` that may be null.
    TextOrNull,
}

/// The width types of `std.foreign` by name.
pub const WIDTHS: [(&str, CType); 9] = [
    ("Int8", CType::I8),
    ("UInt8", CType::U8),
    ("Int16", CType::I16),
    ("UInt16", CType::U16),
    ("Int32", CType::I32),
    ("UInt32", CType::U32),
    ("Int64", CType::I64),
    ("UInt64", CType::U64),
    ("Size", CType::Size),
];

/// The most words of the calling convention a foreign function takes.
pub const MAX_WORDS: usize = 6;

/// The types a parameter may have, for the diagnostic.
pub const PARAMETER_TYPES: &str = "Int8, UInt8, Int16, UInt16, Int32, UInt32, Int64, UInt64 or Size of `std.foreign`, Float, Boolean, Text or Bytes";

/// The types a result may have, for the diagnostic.
pub const RESULT_TYPES: &str =
    "a type of `std.foreign`, Float, Boolean, Text or `maybe Text`, or nothing";

impl CType {
    pub fn spelling(self) -> &'static str {
        match self {
            CType::I8 => "i8",
            CType::U8 => "u8",
            CType::I16 => "i16",
            CType::U16 => "u16",
            CType::I32 => "i32",
            CType::U32 => "u32",
            CType::I64 => "i64",
            CType::U64 => "u64",
            CType::Size => "size",
            CType::F64 => "f64",
            CType::Bool => "bool",
            CType::Text => "text",
            CType::Bytes => "bytes",
        }
    }

    pub fn parse(text: &str) -> Option<CType> {
        [
            CType::I8,
            CType::U8,
            CType::I16,
            CType::U16,
            CType::I32,
            CType::U32,
            CType::I64,
            CType::U64,
            CType::Size,
            CType::F64,
            CType::Bool,
            CType::Text,
            CType::Bytes,
        ]
        .into_iter()
        .find(|c| c.spelling() == text)
    }

    /// The words of the calling convention the type takes: `Bytes` is a
    /// pointer and a length.
    pub fn words(self) -> usize {
        match self {
            CType::Bytes => 2,
            _ => 1,
        }
    }
}

impl CResult {
    pub fn spelling(self) -> &'static str {
        match self {
            CResult::Void => "void",
            CResult::Value(c) => c.spelling(),
            CResult::TextOrNull => "text_or_null",
        }
    }

    pub fn parse(text: &str) -> Option<CResult> {
        match text {
            "void" => Some(CResult::Void),
            "text_or_null" => Some(CResult::TextOrNull),
            other => CType::parse(other).map(CResult::Value),
        }
    }
}

impl World {
    /// The C type of a parameter's type: a width type of `std.foreign`, or
    /// `Float`, `Boolean`, `Text` or `Bytes` of the prelude.
    pub fn c_type_of(&self, ty: &Ty) -> Option<CType> {
        let Ty::App(id, args) = ty else {
            return None;
        };
        if !args.is_empty() {
            return None;
        }
        let info = &self.types[*id];
        match self.modules[info.module].name.as_str() {
            "std.foreign" => WIDTHS
                .iter()
                .find(|(name, _)| *name == info.name)
                .map(|(_, c)| *c),
            "std.prelude" => match info.name.as_str() {
                "Float" => Some(CType::F64),
                "Boolean" => Some(CType::Bool),
                "Text" => Some(CType::Text),
                "Bytes" => Some(CType::Bytes),
                _ => None,
            },
            _ => None,
        }
    }

    /// The C result of a declared result type: nothing, a type of the
    /// boundary other than `Bytes` (whose length no C function reports), or
    /// `maybe Text` for a `char *` that may be null.
    pub fn c_result_of(&self, returns: Option<&Ty>) -> Option<CResult> {
        match returns {
            None => Some(CResult::Void),
            Some(Ty::Maybe(inner)) => match self.c_type_of(inner) {
                Some(CType::Text) => Some(CResult::TextOrNull),
                _ => None,
            },
            Some(ty) => match self.c_type_of(ty)? {
                CType::Bytes => None,
                c => Some(CResult::Value(c)),
            },
        }
    }
}
