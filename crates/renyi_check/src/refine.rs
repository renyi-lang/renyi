//! Compile-time evaluation of refinement conditions on literal values
//! (syntax sketch section 4): `Port(8080)` is checked here, so the
//! construction needs no `otherwise`.

use renyi_syntax::ast::{BinaryOp, Expr, ExprKind, TextPiece};

/// A literal value the evaluator understands.
#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    Integer(i128),
    /// A decimal as digits and a scale: 19.99 is (1999, 2).
    Decimal(i128, u32),
    Text(String),
    Boolean(bool),
}

/// The outcome of a compile-time check.
#[derive(Debug, PartialEq)]
pub enum Verdict {
    Holds,
    Fails,
    /// The condition uses something the evaluator cannot run (a call it does
    /// not know, a value that is not a literal); the construction is checked
    /// at run time instead.
    Unknown,
}

/// The literal value of an expression, when it is one. A text literal must
/// have no interpolation holes; a negative number is a literal.
pub fn literal_of(expr: &Expr) -> Option<Literal> {
    match &expr.kind {
        ExprKind::Integer(digits) => digits.replace('_', "").parse().ok().map(Literal::Integer),
        ExprKind::Decimal(digits) => parse_decimal(&digits.replace('_', "")),
        ExprKind::Text { pieces, .. } => {
            let mut out = String::new();
            for piece in pieces {
                match piece {
                    TextPiece::Text(value) => out.push_str(value),
                    TextPiece::Hole(_) => return None,
                }
            }
            Some(Literal::Text(out))
        }
        ExprKind::RawText(value) => Some(Literal::Text(value.clone())),
        ExprKind::Boolean(value) => Some(Literal::Boolean(*value)),
        ExprKind::Paren(inner) => literal_of(inner),
        _ => None,
    }
}

fn parse_decimal(text: &str) -> Option<Literal> {
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, text),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let scale = fraction.len() as u32;
    let mantissa: i128 = format!("{whole}{fraction}").parse().ok()?;
    Some(Literal::Decimal(sign * mantissa, scale))
}

/// Evaluate a refinement condition with `name` bound to `value`.
pub fn evaluate(condition: &Expr, name: &str, value: &Literal) -> Verdict {
    match eval(condition, name, value) {
        Some(Literal::Boolean(true)) => Verdict::Holds,
        Some(Literal::Boolean(false)) => Verdict::Fails,
        _ => Verdict::Unknown,
    }
}

fn eval(expr: &Expr, name: &str, value: &Literal) -> Option<Literal> {
    match &expr.kind {
        ExprKind::Name(n) if n.text == name => Some(value.clone()),
        ExprKind::Name(_) => None,
        ExprKind::Paren(inner) => eval(inner, name, value),
        ExprKind::Not(inner) => match eval(inner, name, value)? {
            Literal::Boolean(b) => Some(Literal::Boolean(!b)),
            _ => None,
        },
        ExprKind::Binary { op, left, right } => {
            let left = eval(left, name, value)?;
            let right = eval(right, name, value)?;
            binary(*op, &left, &right)
        }
        ExprKind::Call { callee, args } => {
            let ExprKind::Member { base, name: method } = &callee.kind else {
                return None;
            };
            let receiver = eval(base, name, value)?;
            let arguments: Option<Vec<Literal>> = args
                .iter()
                .map(|arg| eval(&arg.value, name, value))
                .collect();
            method_call(&receiver, &method.text, &arguments?)
        }
        _ => literal_of(expr),
    }
}

fn binary(op: BinaryOp, left: &Literal, right: &Literal) -> Option<Literal> {
    use Literal::*;
    match (op, left, right) {
        (BinaryOp::And, Boolean(a), Boolean(b)) => Some(Boolean(*a && *b)),
        (BinaryOp::Or, Boolean(a), Boolean(b)) => Some(Boolean(*a || *b)),
        (BinaryOp::Is, a, b) => Some(Boolean(equal(a, b)?)),
        (BinaryOp::IsNot, a, b) => Some(Boolean(!equal(a, b)?)),
        (BinaryOp::IsLessThan, a, b) => Some(Boolean(compare(a, b)?.is_lt())),
        (BinaryOp::IsAtMost, a, b) => Some(Boolean(compare(a, b)?.is_le())),
        (BinaryOp::IsGreaterThan, a, b) => Some(Boolean(compare(a, b)?.is_gt())),
        (BinaryOp::IsAtLeast, a, b) => Some(Boolean(compare(a, b)?.is_ge())),
        (BinaryOp::Add, Integer(a), Integer(b)) => Some(Integer(a.checked_add(*b)?)),
        (BinaryOp::Subtract, Integer(a), Integer(b)) => Some(Integer(a.checked_sub(*b)?)),
        (BinaryOp::Multiply, Integer(a), Integer(b)) => Some(Integer(a.checked_mul(*b)?)),
        (BinaryOp::Remainder, Integer(a), Integer(b)) if *b != 0 => Some(Integer(a % b)),
        _ => None,
    }
}

fn equal(a: &Literal, b: &Literal) -> Option<bool> {
    match (a, b) {
        (Literal::Text(x), Literal::Text(y)) => Some(x == y),
        (Literal::Boolean(x), Literal::Boolean(y)) => Some(x == y),
        _ => Some(compare(a, b)?.is_eq()),
    }
}

fn compare(a: &Literal, b: &Literal) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Literal::Integer(x), Literal::Integer(y)) => Some(x.cmp(y)),
        (Literal::Text(x), Literal::Text(y)) => Some(x.cmp(y)),
        (Literal::Integer(_), Literal::Decimal(..))
        | (Literal::Decimal(..), Literal::Integer(_))
        | (Literal::Decimal(..), Literal::Decimal(..)) => {
            let (x, sx) = as_decimal(a)?;
            let (y, sy) = as_decimal(b)?;
            let scale = sx.max(sy);
            let x = x.checked_mul(10i128.checked_pow(scale - sx)?)?;
            let y = y.checked_mul(10i128.checked_pow(scale - sy)?)?;
            Some(x.cmp(&y))
        }
        _ => None,
    }
}

fn as_decimal(value: &Literal) -> Option<(i128, u32)> {
    match value {
        Literal::Integer(x) => Some((*x, 0)),
        Literal::Decimal(m, s) => Some((*m, *s)),
        _ => None,
    }
}

fn method_call(receiver: &Literal, method: &str, args: &[Literal]) -> Option<Literal> {
    use Literal::*;
    match (receiver, method, args) {
        (Text(text), "length", []) => Some(Integer(text.chars().count() as i128)),
        (Text(text), "is_empty", []) => Some(Boolean(text.is_empty())),
        (Text(text), "trim", []) => Some(Text(text.trim().to_string())),
        (Text(text), "to_upper", []) => Some(Text(text.to_uppercase())),
        (Text(text), "to_lower", []) => Some(Text(text.to_lowercase())),
        (Text(text), "starts_with", [Text(prefix)]) => Some(Boolean(text.starts_with(prefix))),
        (Text(text), "ends_with", [Text(suffix)]) => Some(Boolean(text.ends_with(suffix))),
        (Text(text), "contains", [Text(part)]) => Some(Boolean(text.contains(part))),
        (Text(text), "matches", [Text(pattern)]) => {
            let anchored = format!("^(?:{pattern})$");
            let regex = regex::Regex::new(&anchored).ok()?;
            Some(Boolean(regex.is_match(text)))
        }
        (Integer(x), "absolute", []) => Some(Integer(x.abs())),
        _ => None,
    }
}

/// Whether a regular expression literal is valid.
pub fn regex_error(pattern: &str) -> Option<String> {
    regex::Regex::new(pattern).err().map(|error| {
        // the engine's report ends with the one line that names the problem
        let text = error.to_string();
        text.lines()
            .last()
            .unwrap_or(&text)
            .trim_start_matches("error: ")
            .to_string()
    })
}

/// Whether a literal is an absolute URL (decision N2): a scheme, `://` and
/// a host.
pub fn is_url(text: &str) -> bool {
    let Some((scheme, rest)) = text.split_once("://") else {
        return false;
    };
    let scheme_ok = scheme
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c));
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    scheme_ok && !host.is_empty() && !host.chars().any(char::is_whitespace)
}

/// The days of a month in the proleptic Gregorian calendar.
pub fn days_in_month(year: i128, month: i128) -> i128 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renyi_syntax::parse;

    fn condition(source: &str) -> Expr {
        // wrap the condition in a module so the real parser reads it
        let module = format!(
            "module demo\n\npublic type Checked is Integer where {source}\n  purpose: Test.\n"
        );
        let parsed = parse(&module);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        match &parsed.module.items[0] {
            renyi_syntax::ast::Item::Type(def) => match &def.kind {
                renyi_syntax::ast::TypeKind::Subtype { refinement, .. } => {
                    refinement.clone().expect("refinement")
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    #[test]
    fn numeric_bounds() {
        let port = condition("value is at least 1 and value is at most 65535");
        assert_eq!(
            evaluate(&port, "value", &Literal::Integer(8080)),
            Verdict::Holds
        );
        assert_eq!(
            evaluate(&port, "value", &Literal::Integer(0)),
            Verdict::Fails
        );
        let positive = condition("value is greater than 0");
        assert_eq!(
            evaluate(&positive, "value", &Literal::Decimal(25, 1)),
            Verdict::Holds
        );
        assert_eq!(
            evaluate(&positive, "value", &Literal::Decimal(0, 2)),
            Verdict::Fails
        );
    }

    #[test]
    fn text_methods_and_patterns() {
        let currency = condition("value.length() is 3 and value is value.to_upper()");
        assert_eq!(
            evaluate(&currency, "value", &Literal::Text("EUR".into())),
            Verdict::Holds
        );
        assert_eq!(
            evaluate(&currency, "value", &Literal::Text("eur".into())),
            Verdict::Fails
        );
        let permission = condition("value.matches(\"^[a-z]+:[a-z_]+$\")");
        assert_eq!(
            evaluate(&permission, "value", &Literal::Text("orders:read".into())),
            Verdict::Holds
        );
        assert_eq!(
            evaluate(&permission, "value", &Literal::Text("Orders".into())),
            Verdict::Fails
        );
        let unknown = condition("value.reversed() is \"abc\"");
        assert_eq!(
            evaluate(&unknown, "value", &Literal::Text("cba".into())),
            Verdict::Unknown
        );
    }

    #[test]
    fn literals_are_recognized() {
        assert_eq!(parse_decimal("19.99"), Some(Literal::Decimal(1999, 2)));
        assert_eq!(parse_decimal("-40"), Some(Literal::Decimal(-40, 0)));
    }
}
