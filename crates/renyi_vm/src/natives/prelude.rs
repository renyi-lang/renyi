//! The prelude's methods: numbers, text, bytes, lists, maps, sets, ranges
//! and durations.

use std::cmp::Ordering;
use std::rc::Rc;

use indexmap::IndexSet;
use num_bigint::BigInt;
use num_traits::FromPrimitive;

use sha2::{Digest, Sha256};

use super::{arg, bytes, crash, decimal, float, int, list, map, range, set, small, take, text};
use crate::decimal::Decimal;
use crate::extension::Native;
use crate::integer::Int;
use crate::value::{take_list, take_map, take_set, Value};
use crate::vm::{finite_float, range_items, Interrupt, Vm};

/// The natives of `std.prelude` (decision AK2): the declared function
/// each implements, by module, name and the type of its first parameter.
pub(crate) const NATIVES: &[Native] = &[
    Native::method("std.prelude", "to_decimal", "Integer", integer_to_decimal),
    Native::method("std.prelude", "to_float", "Integer", integer_to_float),
    Native::method("std.prelude", "to_text", "Integer", integer_to_text),
    Native::method("std.prelude", "quotient", "Integer", integer_quotient),
    Native::method("std.prelude", "absolute", "Integer", integer_absolute),
    Native::method("std.prelude", "at_least", "Integer", number_at_least),
    Native::method("std.prelude", "at_most", "Integer", number_at_most),
    Native::method("std.prelude", "rounded", "Decimal", decimal_rounded),
    Native::method("std.prelude", "truncated", "Decimal", decimal_truncated),
    Native::method("std.prelude", "to_float", "Decimal", decimal_to_float),
    Native::method("std.prelude", "to_text", "Decimal", any_to_text),
    Native::method("std.prelude", "absolute", "Decimal", decimal_absolute),
    Native::method("std.prelude", "at_least", "Decimal", number_at_least),
    Native::method("std.prelude", "at_most", "Decimal", number_at_most),
    Native::method("std.prelude", "rounded", "Float", float_rounded),
    Native::method("std.prelude", "truncated", "Float", float_truncated),
    Native::method("std.prelude", "square_root", "Float", float_square_root),
    Native::method("std.prelude", "to_decimal", "Float", float_to_decimal),
    Native::method("std.prelude", "to_text", "Float", any_to_text),
    Native::method("std.prelude", "absolute", "Float", float_absolute),
    Native::method("std.prelude", "at_least", "Float", number_at_least),
    Native::method("std.prelude", "at_most", "Float", number_at_most),
    Native::method("std.prelude", "to_text", "Boolean", any_to_text),
    Native::method("std.prelude", "length", "Text", text_length),
    Native::method("std.prelude", "is_empty", "Text", text_is_empty),
    Native::method("std.prelude", "trim", "Text", text_trim),
    Native::method("std.prelude", "trim_start", "Text", text_trim_start),
    Native::method("std.prelude", "trim_end", "Text", text_trim_end),
    Native::method("std.prelude", "to_lower", "Text", text_to_lower),
    Native::method("std.prelude", "to_upper", "Text", text_to_upper),
    Native::method("std.prelude", "split", "Text", text_split),
    Native::method("std.prelude", "lines", "Text", text_lines),
    Native::method("std.prelude", "characters", "Text", text_characters),
    Native::method("std.prelude", "contains", "Text", text_contains),
    Native::method("std.prelude", "starts_with", "Text", text_starts_with),
    Native::method("std.prelude", "ends_with", "Text", text_ends_with),
    Native::method("std.prelude", "index_of", "Text", text_index_of),
    Native::method("std.prelude", "replace", "Text", text_replace),
    Native::method("std.prelude", "pad_left", "Text", text_pad_left),
    Native::method("std.prelude", "pad_right", "Text", text_pad_right),
    Native::method("std.prelude", "repeat", "Text", text_repeat),
    Native::method("std.prelude", "take", "Text", text_take),
    Native::method("std.prelude", "drop", "Text", text_drop),
    Native::method("std.prelude", "reversed", "Text", text_reversed),
    Native::method("std.prelude", "matches", "Text", super::regex::text_matches),
    Native::method("std.prelude", "to_integer", "Text", text_to_integer),
    Native::method("std.prelude", "to_decimal", "Text", text_to_decimal),
    Native::method("std.prelude", "to_float", "Text", text_to_float),
    Native::method("std.prelude", "to_bytes", "Text", text_to_bytes),
    Native::method("std.prelude", "to_text", "Text", any_to_text),
    Native::method("std.prelude", "length", "Bytes", bytes_length),
    Native::method("std.prelude", "is_empty", "Bytes", bytes_is_empty),
    Native::method("std.prelude", "to_text", "Bytes", bytes_to_text),
    Native::method("std.prelude", "to_base64", "Bytes", bytes_to_base64),
    Native::method("std.prelude", "sha256", "Bytes", bytes_sha256),
    Native::method("std.prelude", "length", "List", list_length),
    Native::method("std.prelude", "is_empty", "List", list_is_empty),
    Native::method("std.prelude", "at", "List", list_at),
    Native::method("std.prelude", "first", "List", list_first),
    Native::method("std.prelude", "last", "List", list_last),
    Native::method("std.prelude", "rest", "List", list_rest),
    Native::method("std.prelude", "without_last", "List", list_without_last),
    Native::method("std.prelude", "without_index", "List", list_without_index),
    Native::method("std.prelude", "take", "List", list_take),
    Native::method("std.prelude", "drop", "List", list_drop),
    Native::method("std.prelude", "slice", "List", list_slice),
    Native::method("std.prelude", "append", "List", list_append),
    Native::method("std.prelude", "append_all", "List", list_append_all),
    Native::method("std.prelude", "reversed", "List", list_reversed),
    Native::method("std.prelude", "sorted", "List", list_sorted),
    Native::method("std.prelude", "distinct", "List", list_distinct),
    Native::method("std.prelude", "contains", "List", list_contains),
    Native::method("std.prelude", "index_of", "List", list_index_of),
    Native::method("std.prelude", "largest", "List", list_largest),
    Native::method("std.prelude", "smallest", "List", list_smallest),
    Native::method("std.prelude", "with_index", "List", list_with_index),
    Native::method("std.prelude", "to_set", "List", list_to_set),
    Native::method("std.prelude", "flattened", "List", list_flattened),
    Native::method("std.prelude", "join", "List", list_join),
    Native::method("std.prelude", "sum", "List of Integer", list_sum_integers),
    Native::method("std.prelude", "sum", "List of Decimal", list_sum_decimals),
    Native::method("std.prelude", "sum", "List of Float", list_sum_floats),
    Native::method("std.prelude", "length", "Map", map_length),
    Native::method("std.prelude", "is_empty", "Map", map_is_empty),
    Native::method("std.prelude", "get", "Map", map_get),
    Native::method("std.prelude", "set", "Map", map_set),
    Native::method("std.prelude", "without", "Map", map_without),
    Native::method("std.prelude", "contains_key", "Map", map_contains_key),
    Native::method("std.prelude", "keys", "Map", map_keys),
    Native::method("std.prelude", "values", "Map", map_values),
    Native::method("std.prelude", "entries", "Map", map_entries),
    Native::method("std.prelude", "merged", "Map", map_merged),
    Native::method("std.prelude", "length", "Set", set_length),
    Native::method("std.prelude", "is_empty", "Set", set_is_empty),
    Native::method("std.prelude", "contains", "Set", set_contains),
    Native::method("std.prelude", "add", "Set", set_add),
    Native::method("std.prelude", "without", "Set", set_without),
    Native::method("std.prelude", "union", "Set", set_union),
    Native::method("std.prelude", "intersection", "Set", set_intersection),
    Native::method("std.prelude", "difference", "Set", set_difference),
    Native::method("std.prelude", "is_subset_of", "Set", set_is_subset_of),
    Native::method("std.prelude", "sorted", "Set", set_sorted),
    Native::method("std.prelude", "to_list", "Set", set_to_list),
    Native::method("std.prelude", "to_list", "List", iterable_to_list),
    Native::method("std.prelude", "to_list", "Map", iterable_to_list),
    Native::method("std.prelude", "to_list", "Text", iterable_to_list),
    Native::method("std.prelude", "contains", "Range", range_contains),
    Native::method("std.prelude", "to_list", "Range", range_to_list),
    Native::method("std.prelude", "length", "Range", range_length),
    Native::method(
        "std.prelude",
        "to_milliseconds",
        "Duration",
        duration_to_milliseconds,
    ),
    Native::method("std.prelude", "to_seconds", "Duration", duration_to_seconds),
];

// ----------------------------------------------------------------- numbers

fn integer_to_decimal(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::decimal(Decimal::from_int(int(arg(args, 0))?)))
}

fn integer_to_float(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    finite_float(int(arg(args, 0))?.to_f64())
}

fn integer_to_text(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::text(int(arg(args, 0))?.to_string()))
}

fn integer_quotient(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let dividend = int(arg(args, 0))?;
    let divisor = int(arg(args, 1))?;
    dividend
        .quotient(divisor)
        .map(Value::Integer)
        .ok_or_else(|| crash("division by zero"))
}

fn integer_absolute(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Integer(int(arg(args, 0))?.absolute()))
}

fn number_at_least(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let (a, b) = (arg(args, 0).clone(), arg(args, 1).clone());
    Ok(if vm.compare(&a, &b)? == Ordering::Less {
        b
    } else {
        a
    })
}

fn number_at_most(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let (a, b) = (arg(args, 0).clone(), arg(args, 1).clone());
    Ok(if vm.compare(&a, &b)? == Ordering::Greater {
        b
    } else {
        a
    })
}

fn any_to_text(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = arg(args, 0).clone();
    Ok(Value::text(vm.render(&value, false)?))
}

fn decimal_rounded(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = decimal(arg(args, 0))?;
    let places = small(arg(args, 1))?;
    value
        .rounded(places)
        .map(Value::decimal)
        .map_err(|_| crash("cannot round to that many places"))
}

fn decimal_truncated(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Integer(decimal(arg(args, 0))?.truncated()))
}

fn decimal_to_float(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    finite_float(decimal(arg(args, 0))?.to_f64())
}

fn decimal_absolute(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::decimal(decimal(arg(args, 0))?.absolute()))
}

fn float_rounded(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = float(arg(args, 0))?;
    let places = small(arg(args, 1))?;
    let factor = 10f64.powi(places.clamp(-300, 300) as i32);
    Ok(Value::Float((value * factor).round() / factor))
}

fn float_truncated(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = float(arg(args, 0))?;
    BigInt::from_f64(value.trunc())
        .map(|big| Value::Integer(Int::from_big(big)))
        .ok_or_else(|| crash("cannot truncate a Float that is not finite"))
}

fn float_square_root(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = float(arg(args, 0))?;
    if value < 0.0 {
        return Err(crash("the square root of a negative number"));
    }
    Ok(Value::Float(value.sqrt()))
}

fn float_to_decimal(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Decimal::from_f64(float(arg(args, 0))?)
        .map(Value::decimal)
        .ok_or_else(|| crash("cannot convert a Float that is not finite to a Decimal"))
}

fn float_absolute(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Float(float(arg(args, 0))?.abs()))
}

// -------------------------------------------------------------------- text

fn text_length(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::integer(text(arg(args, 0))?.chars().count() as i64))
}

fn text_is_empty(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Boolean(text(arg(args, 0))?.is_empty()))
}

fn text_trim(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::text(text(arg(args, 0))?.trim()))
}

fn text_trim_start(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::text(text(arg(args, 0))?.trim_start()))
}

fn text_trim_end(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::text(text(arg(args, 0))?.trim_end()))
}

fn text_to_lower(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::text(text(arg(args, 0))?.to_lowercase()))
}

fn text_to_upper(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::text(text(arg(args, 0))?.to_uppercase()))
}

fn text_split(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = text(arg(args, 0))?;
    let separator = text(arg(args, 1))?;
    if separator.is_empty() {
        // nothing lies between empty separators but the characters
        return Ok(Value::list(
            value.chars().map(|c| Value::text(c.to_string())).collect(),
        ));
    }
    Ok(Value::list(
        value.split(separator).map(Value::text).collect(),
    ))
}

fn text_lines(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::list(
        text(arg(args, 0))?.lines().map(Value::text).collect(),
    ))
}

fn text_characters(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::list(
        text(arg(args, 0))?
            .chars()
            .map(|c| Value::text(c.to_string()))
            .collect(),
    ))
}

fn text_contains(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Boolean(
        text(arg(args, 0))?.contains(text(arg(args, 1))?),
    ))
}

fn text_starts_with(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Boolean(
        text(arg(args, 0))?.starts_with(text(arg(args, 1))?),
    ))
}

fn text_ends_with(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Boolean(
        text(arg(args, 0))?.ends_with(text(arg(args, 1))?),
    ))
}

fn text_index_of(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = text(arg(args, 0))?;
    Ok(match value.find(text(arg(args, 1))?) {
        Some(byte) => Value::integer(value[..byte].chars().count() as i64),
        None => Value::Nothing,
    })
}

fn text_replace(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = text(arg(args, 0))?;
    let old = text(arg(args, 1))?;
    if old.is_empty() {
        // an empty part occurs nowhere: nothing changes
        return Ok(Value::text(value));
    }
    Ok(Value::text(value.replace(old, text(arg(args, 2))?)))
}

fn text_pad_left(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = text(arg(args, 0))?;
    let width = small(arg(args, 1))?.max(0) as usize;
    let length = value.chars().count();
    Ok(Value::text(format!(
        "{}{value}",
        " ".repeat(width.saturating_sub(length))
    )))
}

fn text_pad_right(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = text(arg(args, 0))?;
    let width = small(arg(args, 1))?.max(0) as usize;
    let length = value.chars().count();
    Ok(Value::text(format!(
        "{value}{}",
        " ".repeat(width.saturating_sub(length))
    )))
}

fn text_repeat(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = text(arg(args, 0))?;
    let times = small(arg(args, 1))?.max(0) as usize;
    Ok(Value::text(value.repeat(times)))
}

/// A count of items to take or drop: at most everything, at least nothing,
/// whatever the Integer's size.
fn count(value: &Value) -> Result<usize, Interrupt> {
    Ok(match int(value)? {
        Int::Small(count) => (*count).max(0) as usize,
        big if big.is_negative() => 0,
        _ => usize::MAX,
    })
}

fn text_take(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = text(arg(args, 0))?;
    let length = count(arg(args, 1))?;
    Ok(Value::text(value.chars().take(length).collect::<String>()))
}

fn text_drop(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = text(arg(args, 0))?;
    let length = count(arg(args, 1))?;
    Ok(Value::text(value.chars().skip(length).collect::<String>()))
}

fn text_reversed(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::text(
        text(arg(args, 0))?.chars().rev().collect::<String>(),
    ))
}

fn text_to_integer(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = text(arg(args, 0))?;
    let digits = value.strip_prefix('+').unwrap_or(value);
    let plain = digits.strip_prefix('-').unwrap_or(digits);
    if plain.is_empty() || !plain.chars().all(|c| c.is_ascii_digit()) {
        return vm.fail_record("std.prelude", "InvalidNumber", vec![Value::text(value)]);
    }
    match Int::parse(digits) {
        Some(parsed) => Ok(Value::Integer(parsed)),
        None => vm.fail_record("std.prelude", "InvalidNumber", vec![Value::text(value)]),
    }
}

fn text_to_decimal(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = text(arg(args, 0))?;
    match Decimal::parse(value) {
        Some(parsed) if !value.contains('_') => Ok(Value::decimal(parsed)),
        _ => vm.fail_record("std.prelude", "InvalidNumber", vec![Value::text(value)]),
    }
}

fn text_to_float(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = text(arg(args, 0))?;
    match value.parse::<f64>() {
        Ok(parsed) if parsed.is_finite() => Ok(Value::Float(parsed)),
        _ => vm.fail_record("std.prelude", "InvalidNumber", vec![Value::text(value)]),
    }
}

fn text_to_bytes(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Bytes(Rc::from(text(arg(args, 0))?.as_bytes())))
}

// ------------------------------------------------------------------- bytes

fn bytes_length(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::integer(bytes(arg(args, 0))?.len() as i64))
}

fn bytes_is_empty(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Boolean(bytes(arg(args, 0))?.is_empty()))
}

fn bytes_to_text(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    match std::str::from_utf8(bytes(arg(args, 0))?) {
        Ok(decoded) => Ok(Value::text(decoded)),
        Err(error) => vm.fail_record(
            "std.prelude",
            "InvalidEncoding",
            vec![Value::text(error.to_string())],
        ),
    }
}

fn bytes_to_base64(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::text(super::base64_encode(bytes(arg(args, 0))?)))
}

/// The SHA-256 digest as 64 hex digits (decision AC1: the front end written
/// in Renyi verifies a package's hash with it).
fn bytes_sha256(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::text(format!(
        "{:x}",
        Sha256::digest(bytes(arg(args, 0))?)
    )))
}

// ------------------------------------------------------------------- lists

fn index_in(length: usize, index: i64) -> Option<usize> {
    if index < 0 || index as usize >= length {
        None
    } else {
        Some(index as usize)
    }
}

fn list_length(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::integer(list(arg(args, 0))?.len() as i64))
}

fn list_is_empty(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Boolean(list(arg(args, 0))?.is_empty()))
}

fn list_at(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    // an index beyond a machine word is out of range like any other
    let Some(index) = int(arg(args, 1))?.to_i64() else {
        return Ok(Value::Nothing);
    };
    Ok(index_in(items.len(), index)
        .map(|i| items[i].clone())
        .unwrap_or(Value::Nothing))
}

fn list_first(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(list(arg(args, 0))?
        .first()
        .cloned()
        .unwrap_or(Value::Nothing))
}

fn list_last(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(list(arg(args, 0))?
        .last()
        .cloned()
        .unwrap_or(Value::Nothing))
}

fn list_rest(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    Ok(Value::list(items.iter().skip(1).cloned().collect()))
}

fn list_without_last(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    let keep = items.len().saturating_sub(1);
    Ok(Value::list(items[..keep].to_vec()))
}

fn list_without_index(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    let index = small(arg(args, 1))?;
    let mut out: Vec<Value> = (**items).clone();
    if let Some(i) = index_in(out.len(), index) {
        out.remove(i);
    }
    Ok(Value::list(out))
}

fn list_take(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    let length = count(arg(args, 1))?;
    Ok(Value::list(items.iter().take(length).cloned().collect()))
}

fn list_drop(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    let length = count(arg(args, 1))?;
    Ok(Value::list(items.iter().skip(length).cloned().collect()))
}

/// `slice(start, stop)`: the items from `start` to `stop`, the end exclusive,
/// both clamped to the list (decision X2).
fn list_slice(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    let start = count(arg(args, 1))?.min(items.len());
    let stop = count(arg(args, 2))?.min(items.len()).max(start);
    Ok(Value::list(items[start..stop].to_vec()))
}

fn list_append(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let item = take(args, 1);
    match take(args, 0) {
        Value::List(items) => {
            let mut items = take_list(items);
            items.push(item);
            Ok(Value::list(items))
        }
        other => Err(crash(format!(
            "`append` needs a List, found {}",
            other.kind_name()
        ))),
    }
}

fn list_append_all(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let others = take(args, 1);
    let others = list(&others)?.clone();
    match take(args, 0) {
        Value::List(items) => {
            let mut items = take_list(items);
            items.extend(others.iter().cloned());
            Ok(Value::list(items))
        }
        other => Err(crash(format!(
            "`append_all` needs a List, found {}",
            other.kind_name()
        ))),
    }
}

fn list_reversed(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    Ok(Value::list(items.iter().rev().cloned().collect()))
}

fn list_sorted(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let mut items: Vec<Value> = (**list(arg(args, 0))?).clone();
    vm.sort_values(&mut items)?;
    Ok(Value::list(items))
}

fn list_distinct(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    let distinct: IndexSet<Value> = items.iter().cloned().collect();
    Ok(Value::list(distinct.into_iter().collect()))
}

fn list_contains(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    Ok(Value::Boolean(items.contains(arg(args, 1))))
}

fn list_index_of(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    Ok(items
        .iter()
        .position(|item| item == arg(args, 1))
        .map(|i| Value::integer(i as i64))
        .unwrap_or(Value::Nothing))
}

fn extreme(vm: &mut Vm, args: &[Value], wanted: Ordering) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?.clone();
    let mut best: Option<Value> = None;
    for item in items.iter() {
        best = Some(match best {
            None => item.clone(),
            Some(current) => {
                if vm.compare(item, &current)? == wanted {
                    item.clone()
                } else {
                    current
                }
            }
        });
    }
    Ok(best.unwrap_or(Value::Nothing))
}

fn list_largest(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    extreme(vm, args, Ordering::Greater)
}

fn list_smallest(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    extreme(vm, args, Ordering::Less)
}

fn list_with_index(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    Ok(Value::list(
        items
            .iter()
            .enumerate()
            .map(|(index, item)| Value::pair(item.clone(), Value::integer(index as i64)))
            .collect(),
    ))
}

fn list_to_set(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    Ok(Value::Set(Rc::new(items.iter().cloned().collect())))
}

fn list_flattened(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    let mut out = Vec::new();
    for item in items.iter() {
        out.extend(list(item)?.iter().cloned());
    }
    Ok(Value::list(out))
}

fn list_join(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    let separator = text(arg(args, 1))?;
    let mut parts = Vec::with_capacity(items.len());
    for item in items.iter() {
        parts.push(text(item)?);
    }
    Ok(Value::text(parts.join(separator)))
}

fn list_sum_integers(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    let mut total = Int::Small(0);
    for item in items.iter() {
        total = total.add(int(item)?);
    }
    Ok(Value::Integer(total))
}

fn list_sum_decimals(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    let mut total = Decimal::zero();
    for item in items.iter() {
        total = total
            .add(decimal(item)?)
            .map_err(|_| crash("the Decimal sum is out of range"))?;
    }
    Ok(Value::decimal(total))
}

fn list_sum_floats(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?;
    let mut total = 0.0;
    for item in items.iter() {
        total += float(item)?;
    }
    Ok(Value::Float(total))
}

// -------------------------------------------------------------------- maps

fn map_length(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::integer(map(arg(args, 0))?.len() as i64))
}

fn map_is_empty(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Boolean(map(arg(args, 0))?.is_empty()))
}

fn map_get(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(map(arg(args, 0))?
        .get(arg(args, 1))
        .cloned()
        .unwrap_or(Value::Nothing))
}

fn map_set(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = take(args, 2);
    let key = take(args, 1);
    match take(args, 0) {
        Value::Map(entries) => {
            let mut entries = take_map(entries);
            entries.insert(key, value);
            Ok(Value::Map(Rc::new(entries)))
        }
        other => Err(crash(format!(
            "`set` needs a Map, found {}",
            other.kind_name()
        ))),
    }
}

fn map_without(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let key = take(args, 1);
    match take(args, 0) {
        Value::Map(entries) => {
            let mut entries = take_map(entries);
            entries.shift_remove(&key);
            Ok(Value::Map(Rc::new(entries)))
        }
        other => Err(crash(format!(
            "`without` needs a Map, found {}",
            other.kind_name()
        ))),
    }
}

fn map_contains_key(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Boolean(
        map(arg(args, 0))?.contains_key(arg(args, 1)),
    ))
}

fn map_keys(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::list(map(arg(args, 0))?.keys().cloned().collect()))
}

fn map_values(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::list(map(arg(args, 0))?.values().cloned().collect()))
}

fn map_entries(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::list(
        map(arg(args, 0))?
            .iter()
            .map(|(key, value)| Value::pair(key.clone(), value.clone()))
            .collect(),
    ))
}

fn map_merged(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let other = take(args, 1);
    let other = map(&other)?.clone();
    match take(args, 0) {
        Value::Map(entries) => {
            let mut entries = take_map(entries);
            for (key, value) in other.iter() {
                entries.insert(key.clone(), value.clone());
            }
            Ok(Value::Map(Rc::new(entries)))
        }
        other => Err(crash(format!(
            "`merged` needs a Map, found {}",
            other.kind_name()
        ))),
    }
}

// -------------------------------------------------------------------- sets

fn set_length(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::integer(set(arg(args, 0))?.len() as i64))
}

fn set_is_empty(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Boolean(set(arg(args, 0))?.is_empty()))
}

fn set_contains(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Boolean(set(arg(args, 0))?.contains(arg(args, 1))))
}

fn set_add(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let item = take(args, 1);
    match take(args, 0) {
        Value::Set(items) => {
            let mut items = take_set(items);
            items.insert(item);
            Ok(Value::Set(Rc::new(items)))
        }
        other => Err(crash(format!(
            "`add` needs a Set, found {}",
            other.kind_name()
        ))),
    }
}

fn set_without(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let item = take(args, 1);
    match take(args, 0) {
        Value::Set(items) => {
            let mut items = take_set(items);
            items.shift_remove(&item);
            Ok(Value::Set(Rc::new(items)))
        }
        other => Err(crash(format!(
            "`without` needs a Set, found {}",
            other.kind_name()
        ))),
    }
}

fn set_union(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let a = set(arg(args, 0))?;
    let b = set(arg(args, 1))?;
    Ok(Value::Set(Rc::new(a.union(b).cloned().collect())))
}

fn set_intersection(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let a = set(arg(args, 0))?;
    let b = set(arg(args, 1))?;
    Ok(Value::Set(Rc::new(a.intersection(b).cloned().collect())))
}

fn set_difference(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let a = set(arg(args, 0))?;
    let b = set(arg(args, 1))?;
    Ok(Value::Set(Rc::new(a.difference(b).cloned().collect())))
}

fn set_is_subset_of(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let a = set(arg(args, 0))?;
    let b = set(arg(args, 1))?;
    Ok(Value::Boolean(a.is_subset(b)))
}

fn set_sorted(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let mut items: Vec<Value> = set(arg(args, 0))?.iter().cloned().collect();
    vm.sort_values(&mut items)?;
    Ok(Value::list(items))
}

fn set_to_list(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::list(set(arg(args, 0))?.iter().cloned().collect()))
}

/// `to_list` of the prelude's `Iterable` for a list (itself), a map (its
/// pairs) and a text (its characters): what a loop walks (decision AB1).
fn iterable_to_list(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::List(vm.iterate(take(args, 0))?))
}

// ------------------------------------------------------------------ ranges

fn range_contains(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let range = range(arg(args, 0))?;
    let value = int(arg(args, 1))?;
    if range.by.is_zero() {
        return Err(crash("a range cannot step by 0"));
    }
    let (low, high) = if range.by.is_negative() {
        (&range.to, &range.from)
    } else {
        (&range.from, &range.to)
    };
    if value.compare(low) == Ordering::Less || value.compare(high) == Ordering::Greater {
        return Ok(Value::Boolean(false));
    }
    let offset = value.subtract(&range.from);
    let aligned = offset
        .remainder(&range.by)
        .map(|r| r.is_zero())
        .unwrap_or(false);
    Ok(Value::Boolean(aligned))
}

fn range_to_list(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::list(range_items(range(arg(args, 0))?)?))
}

fn range_length(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::integer(
        range_items(range(arg(args, 0))?)?.len() as i64
    ))
}

// --------------------------------------------------------------- durations

fn duration_to_milliseconds(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::integer(super::duration(arg(args, 0))?))
}

fn duration_to_seconds(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let ms = super::duration(arg(args, 0))?;
    Decimal::new(BigInt::from(ms), -3)
        .map(Value::decimal)
        .map_err(|_| crash("the duration is out of range"))
}
