//! `Integer`: arbitrary precision (decision B9), with a machine-word fast
//! path. A `Big` value never fits an `i64`; every operation keeps that
//! invariant so that equality and hashing can compare representations.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{Pow, Signed, ToPrimitive, Zero};

// Decision AR4: the layout is fixed, like `Value`'s.
#[derive(Clone, Debug)]
#[repr(C, u8)]
pub enum Int {
    Small(i64),
    Big(Rc<BigInt>),
}

impl Int {
    pub fn from_big(value: BigInt) -> Int {
        match value.to_i64() {
            Some(small) => Int::Small(small),
            None => Int::Big(Rc::new(value)),
        }
    }

    pub fn to_big(&self) -> BigInt {
        match self {
            Int::Small(value) => BigInt::from(*value),
            Int::Big(value) => (**value).clone(),
        }
    }

    pub fn parse(text: &str) -> Option<Int> {
        let digits: String = text.chars().filter(|c| *c != '_').collect();
        if let Ok(small) = digits.parse::<i64>() {
            return Some(Int::Small(small));
        }
        digits.parse::<BigInt>().ok().map(Int::from_big)
    }

    pub fn is_zero(&self) -> bool {
        matches!(self, Int::Small(0))
    }

    pub fn is_negative(&self) -> bool {
        match self {
            Int::Small(value) => *value < 0,
            Int::Big(value) => value.is_negative(),
        }
    }

    pub fn to_i64(&self) -> Option<i64> {
        match self {
            Int::Small(value) => Some(*value),
            Int::Big(_) => None,
        }
    }

    pub fn to_f64(&self) -> f64 {
        match self {
            Int::Small(value) => *value as f64,
            Int::Big(value) => value.to_f64().unwrap_or(f64::INFINITY),
        }
    }

    pub fn add(&self, other: &Int) -> Int {
        match (self, other) {
            (Int::Small(a), Int::Small(b)) => match a.checked_add(*b) {
                Some(sum) => Int::Small(sum),
                None => Int::from_big(BigInt::from(*a) + BigInt::from(*b)),
            },
            _ => Int::from_big(self.to_big() + other.to_big()),
        }
    }

    pub fn subtract(&self, other: &Int) -> Int {
        match (self, other) {
            (Int::Small(a), Int::Small(b)) => match a.checked_sub(*b) {
                Some(difference) => Int::Small(difference),
                None => Int::from_big(BigInt::from(*a) - BigInt::from(*b)),
            },
            _ => Int::from_big(self.to_big() - other.to_big()),
        }
    }

    pub fn multiply(&self, other: &Int) -> Int {
        match (self, other) {
            (Int::Small(a), Int::Small(b)) => match a.checked_mul(*b) {
                Some(product) => Int::Small(product),
                None => Int::from_big(BigInt::from(*a) * BigInt::from(*b)),
            },
            _ => Int::from_big(self.to_big() * other.to_big()),
        }
    }

    pub fn negate(&self) -> Int {
        Int::Small(0).subtract(self)
    }

    /// Division rounded toward zero; `None` for a zero divisor.
    pub fn quotient(&self, other: &Int) -> Option<Int> {
        if other.is_zero() {
            return None;
        }
        match (self, other) {
            (Int::Small(a), Int::Small(b)) => match a.checked_div(*b) {
                Some(quotient) => Some(Int::Small(quotient)),
                None => Some(Int::from_big(BigInt::from(*a) / BigInt::from(*b))),
            },
            _ => Some(Int::from_big(self.to_big() / other.to_big())),
        }
    }

    /// The remainder of the truncating division: its sign is the dividend's.
    pub fn remainder(&self, other: &Int) -> Option<Int> {
        if other.is_zero() {
            return None;
        }
        match (self, other) {
            (Int::Small(a), Int::Small(b)) => match a.checked_rem(*b) {
                Some(remainder) => Some(Int::Small(remainder)),
                None => Some(Int::Small(0)),
            },
            _ => Some(Int::from_big(self.to_big() % other.to_big())),
        }
    }

    /// `self power exponent` for an exponent of at least 0.
    pub fn power(&self, exponent: &Int) -> Option<Int> {
        let exponent = exponent.to_i64()?;
        if exponent < 0 {
            return None;
        }
        if let Int::Small(base) = self {
            if let Ok(small) = u32::try_from(exponent) {
                if let Some(result) = base.checked_pow(small) {
                    return Some(Int::Small(result));
                }
            }
        }
        let exponent = u64::try_from(exponent).ok()?;
        if exponent > 1_000_000 {
            return None;
        }
        Some(Int::from_big(Pow::pow(self.to_big(), exponent)))
    }

    pub fn absolute(&self) -> Int {
        if self.is_negative() {
            self.negate()
        } else {
            self.clone()
        }
    }

    pub fn compare(&self, other: &Int) -> Ordering {
        match (self, other) {
            (Int::Small(a), Int::Small(b)) => a.cmp(b),
            _ => self.to_big().cmp(&other.to_big()),
        }
    }

    /// The number of decimal digits, ignoring the sign (0 has one digit).
    pub fn digit_count(&self) -> usize {
        match self {
            Int::Small(value) => value.unsigned_abs().to_string().len(),
            Int::Big(value) => value.abs().to_string().len(),
        }
    }

    /// Greatest common divisor, for exact arithmetic elsewhere.
    pub fn gcd(&self, other: &Int) -> Int {
        Int::from_big(self.to_big().gcd(&other.to_big()))
    }
}

impl PartialEq for Int {
    fn eq(&self, other: &Int) -> bool {
        self.compare(other) == Ordering::Equal
    }
}

impl Eq for Int {}

impl Hash for Int {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Int::Small(value) => value.hash(state),
            Int::Big(value) => value.to_string().hash(state),
        }
    }
}

/// The decimal digits of a machine-word Integer appended to the text,
/// without `core::fmt` (decision AU2's profile: the formatter machinery
/// was most of the cost of writing an Integer into a text).
pub fn push_digits(text: &mut String, value: i64) {
    let mut buffer = [0u8; 20];
    let mut at = buffer.len();
    let mut rest = value.unsigned_abs();
    loop {
        at -= 1;
        buffer[at] = b'0' + (rest % 10) as u8;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    if value < 0 {
        text.push('-');
    }
    text.extend(buffer[at..].iter().map(|digit| *digit as char));
}

/// The decimal digits of a machine-word Integer as a text.
pub fn digits(value: i64) -> String {
    let mut text = String::with_capacity(20);
    push_digits(&mut text, value);
    text
}

impl fmt::Display for Int {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Int::Small(value) => write!(f, "{value}"),
            Int::Big(value) => write!(f, "{value}"),
        }
    }
}

impl From<i64> for Int {
    fn from(value: i64) -> Int {
        Int::Small(value)
    }
}

impl From<usize> for Int {
    fn from(value: usize) -> Int {
        match i64::try_from(value) {
            Ok(small) => Int::Small(small),
            Err(_) => Int::from_big(BigInt::from(value)),
        }
    }
}

impl Default for Int {
    fn default() -> Int {
        Int::Small(0)
    }
}

/// Whether a big integer is zero (for callers holding a `BigInt`).
pub fn big_is_zero(value: &BigInt) -> bool {
    value.is_zero()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_promotes_and_demotes() {
        let max = Int::Small(i64::MAX);
        let big = max.add(&Int::Small(1));
        assert!(matches!(big, Int::Big(_)));
        let back = big.subtract(&Int::Small(1));
        assert_eq!(back, max);
        assert!(matches!(back, Int::Small(_)));
    }

    #[test]
    fn quotient_and_remainder_truncate_toward_zero() {
        let seven = Int::Small(-7);
        let two = Int::Small(2);
        assert_eq!(seven.quotient(&two), Some(Int::Small(-3)));
        assert_eq!(seven.remainder(&two), Some(Int::Small(-1)));
        assert_eq!(seven.quotient(&Int::Small(0)), None);
    }

    #[test]
    fn power_and_digits() {
        assert_eq!(Int::Small(2).power(&Int::Small(10)), Some(Int::Small(1024)));
        let huge = Int::Small(10).power(&Int::Small(40)).unwrap();
        assert_eq!(huge.digit_count(), 41);
        assert_eq!(huge.to_string(), format!("1{}", "0".repeat(40)));
    }
}
