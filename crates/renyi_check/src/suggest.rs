//! Suggested fixes (decision D3): the closest name in scope, the Renyi
//! spelling of names other languages use (decision C4), and the conversion
//! between two types when the library has one.

/// The candidate within two edits of the name, when there is one.
pub(crate) fn closest<'a>(name: &str, candidates: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    for candidate in candidates {
        let distance = edit_distance(name, candidate);
        if distance <= 2 && best.is_none_or(|(d, _)| distance < d) {
            best = Some((distance, candidate));
        }
    }
    best.map(|(_, c)| c.to_string())
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            current.push(
                (previous[j] + cost)
                    .min(previous[j + 1] + 1)
                    .min(current[j] + 1),
            );
        }
        previous = current;
    }
    previous[b.len()]
}

/// `` `a`, `b` and `c` ``.
pub(crate) fn quoted(names: &[String]) -> String {
    let quoted: Vec<String> = names.iter().map(|n| format!("`{n}`")).collect();
    match quoted.len() {
        0 => String::new(),
        1 => quoted[0].clone(),
        n => format!("{} and {}", quoted[..n - 1].join(", "), quoted[n - 1]),
    }
}

/// The Renyi spelling of a value name of another language.
pub(crate) fn foreign_value(name: &str) -> Option<&'static str> {
    Some(match name {
        "null" | "nil" | "undefined" | "None" | "Null" | "Nil" => "write `nothing`",
        "True" => "write `true`",
        "False" => "write `false`",
        "this" => "write `self`",
        _ => return None,
    })
}

/// The Renyi form of a function name of another language.
pub(crate) fn foreign_function(name: &str) -> Option<&'static str> {
    Some(match name {
        "print" | "println" | "printf" | "puts" | "echo" | "log" | "console_log" => {
            "write `console.print(text)`, with `import std.console`"
        }
        "len" | "length" | "size" => "write `value.length()`",
        "str" | "to_string" | "string" => "write `value.to_text()`",
        "int" | "parse_int" => "write `text.to_integer()`, with `otherwise`",
        "float" | "parse_float" => "write `text.to_float()`, with `otherwise`",
        "input" | "readline" | "gets" => "write `console.read_line()`, with `import std.console`",
        "assert" => {
            "write `check condition` in a test, or `if not condition then fail with ... end`"
        }
        "range" => "write `from 1 to 9`",
        _ => return None,
    })
}

/// The Renyi spelling of a type name of another language.
pub(crate) fn foreign_type(name: &str) -> Option<&'static str> {
    Some(match name {
        "Int" | "int" | "Int32" | "Int64" | "Integer64" | "i32" | "i64" | "Long" | "long"
        | "Short" | "Byte" => "write `Integer`",
        "String" | "Str" | "str" | "string" | "Char" | "char" => "write `Text`",
        "Bool" | "bool" => "write `Boolean`",
        "Double" | "double" | "Float32" | "Float64" | "f32" | "f64" | "Number" | "number" => {
            "write `Float`, or `Decimal` for exact arithmetic"
        }
        "Array" | "array" | "Vec" | "Vector" | "ArrayList" | "list" | "Seq" | "Sequence" => {
            "write `List of Item`"
        }
        "Dict" | "dict" | "HashMap" | "Hash" | "Object" | "map" => "write `Map of Key to Value`",
        "Option" | "Optional" | "Nullable" | "Maybe" => "write `maybe Type`",
        "Void" | "void" | "Unit" | "None" | "Nothing" => {
            "drop the `returns` clause; a function without one returns nothing"
        }
        "Tuple" | "Pair" => "write a record type with named fields",
        _ => return None,
    })
}

/// A conversion from the shown type to the expected one, when the prelude
/// declares it (`library/std/prelude.ry`).
pub(crate) fn conversion_fix(actual: &str, expected: &str) -> String {
    match (actual, expected) {
        ("Integer" | "Decimal" | "Float" | "Boolean", "Text") => {
            "write `.to_text()` after the value".to_string()
        }
        ("Integer" | "Float", "Decimal") => "write `.to_decimal()` after the value".to_string(),
        ("Integer" | "Decimal", "Float") => "write `.to_float()` after the value".to_string(),
        ("Text", "Integer") => {
            "write `.to_integer()` after the value, with `otherwise`".to_string()
        }
        ("Text", "Decimal") => {
            "write `.to_decimal()` after the value, with `otherwise`".to_string()
        }
        ("Text", "Float") => "write `.to_float()` after the value, with `otherwise`".to_string(),
        _ if actual.strip_prefix("maybe ") == Some(expected) => {
            "write `otherwise default` after the value".to_string()
        }
        _ => format!("give it a `{expected}`, or change the declared type to `{actual}`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closest_within_two_edits() {
        assert_eq!(
            closest("totl", ["total", "other"].into_iter()),
            Some("total".into())
        );
        assert_eq!(closest("xyz", ["total"].into_iter()), None);
    }

    #[test]
    fn quoted_lists() {
        let names = ["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(quoted(&names), "`a`, `b` and `c`");
        assert_eq!(quoted(&names[..1]), "`a`");
    }
}
