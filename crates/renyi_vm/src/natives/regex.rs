//! `std.regex` and `Text.matches`, on the `regex` crate.

use regex::Regex;

use super::{arg, crash, text};
use crate::extension::Native;
use crate::value::Value;
use crate::vm::{Interrupt, Vm};

/// The natives of `std.regex` (decision AK2): the declared function
/// each implements, by module, name and the type of its first parameter.
pub(crate) const NATIVES: &[Native] = &[
    Native::method("std.regex", "find_all", "Pattern", find_all),
    Native::method("std.regex", "captures", "Pattern", captures),
    Native::method("std.regex", "replace_all", "Pattern", replace_all),
    Native::method("std.regex", "split", "Pattern", split),
    Native::method("std.regex", "problem", "Text", problem),
];

fn compile(pattern: &str) -> Result<Regex, Interrupt> {
    Regex::new(pattern).map_err(|error| crash(format!("invalid regular expression: {error}")))
}

/// `regex.problem(pattern)`: the engine's complaint about a pattern, as the
/// checker reports it for a literal (its report ends with the one line that
/// names the problem), or nothing when the pattern compiles.
fn problem(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let pattern = text(arg(args, 0))?;
    Ok(match Regex::new(pattern) {
        Ok(_) => Value::Nothing,
        Err(error) => {
            let report = error.to_string();
            let line = report.lines().last().unwrap_or(&report);
            Value::text(line.trim_start_matches("error: "))
        }
    })
}

/// `Text.matches(pattern)`: whether the whole text matches.
pub fn text_matches(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = text(arg(args, 0))?;
    let pattern = text(arg(args, 1))?;
    let regex = compile(&format!("^(?:{pattern})$"))?;
    Ok(Value::Boolean(regex.is_match(value)))
}

fn find_all(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let regex = compile(text(arg(args, 0))?)?;
    let value = text(arg(args, 1))?;
    Ok(Value::list(
        regex
            .find_iter(value)
            .map(|found| Value::text(found.as_str()))
            .collect(),
    ))
}

fn captures(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let regex = compile(text(arg(args, 0))?)?;
    let value = text(arg(args, 1))?;
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

fn replace_all(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let regex = compile(text(arg(args, 0))?)?;
    let value = text(arg(args, 1))?;
    let replacement = text(arg(args, 2))?;
    Ok(Value::text(regex.replace_all(value, replacement)))
}

fn split(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let regex = compile(text(arg(args, 0))?)?;
    let value = text(arg(args, 1))?;
    Ok(Value::list(regex.split(value).map(Value::text).collect()))
}
