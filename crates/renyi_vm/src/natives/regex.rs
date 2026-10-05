//! `std.regex` and `Text.matches`, on the `regex` crate.

use regex::Regex;

use super::{arg, crash, text, NativeFn};
use crate::value::Value;
use crate::vm::{Interrupt, Vm};

pub fn lookup(name: &str, head: Option<&str>) -> Option<NativeFn> {
    Some(match (head, name) {
        (Some("Pattern"), "find_all") => find_all,
        (Some("Pattern"), "captures") => captures,
        (Some("Pattern"), "replace_all") => replace_all,
        (Some("Pattern"), "split") => split,
        _ => return None,
    })
}

fn compile(pattern: &str) -> Result<Regex, Interrupt> {
    Regex::new(pattern).map_err(|error| crash(format!("invalid regular expression: {error}")))
}

/// `Text.matches(pattern)`: whether the whole text matches.
pub fn text_matches(_: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let value = text(arg(&args, 0))?;
    let pattern = text(arg(&args, 1))?;
    let regex = compile(&format!("^(?:{pattern})$"))?;
    Ok(Value::Boolean(regex.is_match(value)))
}

fn find_all(_: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let regex = compile(text(arg(&args, 0))?)?;
    let value = text(arg(&args, 1))?;
    Ok(Value::list(
        regex
            .find_iter(value)
            .map(|found| Value::text(found.as_str()))
            .collect(),
    ))
}

fn captures(_: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let regex = compile(text(arg(&args, 0))?)?;
    let value = text(arg(&args, 1))?;
    Ok(match regex.captures(value) {
        Some(found) => Value::list(
            found
                .iter()
                .skip(1)
                .map(|group| Value::text(group.map(|g| g.as_str()).unwrap_or("")))
                .collect(),
        ),
        None => Value::Nothing,
    })
}

fn replace_all(_: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let regex = compile(text(arg(&args, 0))?)?;
    let value = text(arg(&args, 1))?;
    let replacement = text(arg(&args, 2))?;
    Ok(Value::text(regex.replace_all(value, replacement)))
}

fn split(_: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let regex = compile(text(arg(&args, 0))?)?;
    let value = text(arg(&args, 1))?;
    Ok(Value::list(regex.split(value).map(Value::text).collect()))
}
