//! `Decimal`: IEEE 754 decimal128 arithmetic (decision J16) on a coefficient
//! of at most 34 digits and an exponent. The exponent is kept, not
//! normalized, so that `2.50` prints as `2.50` and `9.50 + 5.00` as `14.50`;
//! comparison and hashing ignore it (decision R5). Addition, subtraction and
//! multiplication are exact until the result exceeds 34 digits, when they
//! round half-even; division rounds half-even at the 34th digit; the
//! adjusted exponent is limited to decimal128's range and an overflow is an
//! error the VM turns into a crash.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{Pow, Signed, ToPrimitive, Zero};

use crate::integer::Int;

pub const DIGITS: usize = 34;
const MAX_ADJUSTED_EXPONENT: i64 = 6144;
const MIN_ADJUSTED_EXPONENT: i64 = -6176;

#[derive(Clone, Debug)]
pub struct Decimal {
    /// Signed, at most 34 digits.
    coefficient: BigInt,
    exponent: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecimalError {
    Overflow,
    DivisionByZero,
    InvalidExponent,
}

fn pow10(n: u64) -> BigInt {
    Pow::pow(BigInt::from(10), n)
}

fn digit_count(value: &BigInt) -> usize {
    if value.is_zero() {
        1
    } else {
        value.abs().to_string().len()
    }
}

/// Divide and round half-even: `value / divisor`.
fn divide_half_even(value: &BigInt, divisor: &BigInt) -> BigInt {
    let (quotient, remainder) = value.div_rem(divisor);
    let twice = &remainder.abs() * BigInt::from(2);
    let away = match twice.cmp(&divisor.abs()) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => quotient.is_odd(),
    };
    if !away {
        quotient
    } else if value.is_negative() != divisor.is_negative() && !remainder.is_zero() {
        quotient - 1
    } else {
        quotient + 1
    }
}

impl Decimal {
    pub fn zero() -> Decimal {
        Decimal {
            coefficient: BigInt::zero(),
            exponent: 0,
        }
    }

    /// Build a value, rounding the coefficient to 34 digits half-even and
    /// checking the exponent range.
    pub fn new(coefficient: BigInt, exponent: i64) -> Result<Decimal, DecimalError> {
        let mut coefficient = coefficient;
        let mut exponent = exponent;
        let digits = digit_count(&coefficient);
        if digits > DIGITS {
            let drop = (digits - DIGITS) as u64;
            coefficient = divide_half_even(&coefficient, &pow10(drop));
            exponent += drop as i64;
            // rounding may carry into a 35th digit (9.99.. to 10.00..)
            if digit_count(&coefficient) > DIGITS {
                coefficient = divide_half_even(&coefficient, &BigInt::from(10));
                exponent += 1;
            }
        }
        let adjusted = exponent + digit_count(&coefficient) as i64 - 1;
        if !coefficient.is_zero() && adjusted > MAX_ADJUSTED_EXPONENT {
            return Err(DecimalError::Overflow);
        }
        if !coefficient.is_zero() && adjusted < MIN_ADJUSTED_EXPONENT {
            return Err(DecimalError::Overflow);
        }
        Ok(Decimal {
            coefficient,
            exponent,
        })
    }

    pub fn from_int(value: &Int) -> Decimal {
        Decimal::new(value.to_big(), 0).unwrap_or_else(|_| Decimal::zero())
    }

    /// Parse a literal or text: digits with an optional point and sign, no
    /// exponent notation.
    pub fn parse(text: &str) -> Option<Decimal> {
        let text: String = text.chars().filter(|c| *c != '_').collect();
        let (sign, body) = match text.strip_prefix('-') {
            Some(rest) => (-1, rest),
            None => (1, text.strip_prefix('+').unwrap_or(&text)),
        };
        if body.is_empty() {
            return None;
        }
        let (whole, fraction) = match body.split_once('.') {
            Some((w, f)) => (w, f),
            None => (body, ""),
        };
        if (whole.is_empty() && fraction.is_empty())
            || !whole.chars().all(|c| c.is_ascii_digit())
            || !fraction.chars().all(|c| c.is_ascii_digit())
        {
            return None;
        }
        let digits = format!("{whole}{fraction}");
        let digits = if digits.is_empty() { "0" } else { &digits };
        let coefficient = digits.parse::<BigInt>().ok()? * sign;
        Decimal::new(coefficient, -(fraction.len() as i64)).ok()
    }

    pub fn from_f64(value: f64) -> Option<Decimal> {
        if !value.is_finite() {
            return None;
        }
        // the shortest text that reads back as the value is the natural
        // decimal; an integral value drops its `.0`
        let text = format!("{value:?}");
        if let Some((mantissa, exponent)) = text.split_once('e') {
            let base = Decimal::parse(mantissa.strip_suffix(".0").unwrap_or(mantissa))?;
            let shift: i64 = exponent.parse().ok()?;
            return Decimal::new(base.coefficient, base.exponent + shift).ok();
        }
        Decimal::parse(text.strip_suffix(".0").unwrap_or(&text))
    }

    pub fn to_f64(&self) -> f64 {
        self.to_string().parse().unwrap_or(0.0)
    }

    pub fn is_zero(&self) -> bool {
        self.coefficient.is_zero()
    }

    pub fn is_negative(&self) -> bool {
        self.coefficient.is_negative()
    }

    fn aligned(&self, other: &Decimal) -> (BigInt, BigInt, i64) {
        let exponent = self.exponent.min(other.exponent);
        let left = &self.coefficient * pow10((self.exponent - exponent) as u64);
        let right = &other.coefficient * pow10((other.exponent - exponent) as u64);
        (left, right, exponent)
    }

    pub fn add(&self, other: &Decimal) -> Result<Decimal, DecimalError> {
        let (left, right, exponent) = self.aligned(other);
        Decimal::new(left + right, exponent)
    }

    pub fn subtract(&self, other: &Decimal) -> Result<Decimal, DecimalError> {
        let (left, right, exponent) = self.aligned(other);
        Decimal::new(left - right, exponent)
    }

    pub fn multiply(&self, other: &Decimal) -> Result<Decimal, DecimalError> {
        Decimal::new(
            &self.coefficient * &other.coefficient,
            self.exponent + other.exponent,
        )
    }

    /// Division: exact when the quotient terminates within 34 digits (with
    /// the ideal exponent, `10.00 / 4` is `2.50`), else rounded half-even at
    /// the 34th digit.
    pub fn divide(&self, other: &Decimal) -> Result<Decimal, DecimalError> {
        if other.is_zero() {
            return Err(DecimalError::DivisionByZero);
        }
        if self.is_zero() {
            return Decimal::new(BigInt::zero(), self.exponent - other.exponent);
        }
        let ideal = self.exponent - other.exponent;
        let extra = (DIGITS + 1 + digit_count(&other.coefficient)) as i64
            - digit_count(&self.coefficient) as i64;
        let extra = extra.max(0) as u64;
        let scaled = &self.coefficient * pow10(extra);
        let (mut quotient, remainder) = scaled.div_rem(&other.coefficient);
        let mut exponent = ideal - extra as i64;
        if remainder.is_zero() {
            // strip trailing zeros down to the ideal exponent
            let ten = BigInt::from(10);
            while exponent < ideal && (&quotient % &ten).is_zero() {
                quotient /= &ten;
                exponent += 1;
            }
            Decimal::new(quotient, exponent)
        } else {
            let digits = digit_count(&quotient);
            if digits > DIGITS {
                let drop = (digits - DIGITS) as u64;
                let divisor = pow10(drop);
                let (head, tail) = quotient.div_rem(&divisor);
                // the discarded tail plus the nonzero remainder decides the rounding
                let twice = &tail.abs() * BigInt::from(2);
                let away = match twice.cmp(&divisor) {
                    Ordering::Greater => true,
                    Ordering::Less => false,
                    Ordering::Equal => true, // the remainder makes it more than half
                };
                let rounded = if away {
                    if head.is_negative() {
                        head - 1
                    } else {
                        head + 1
                    }
                } else {
                    head
                };
                Decimal::new(rounded, exponent + drop as i64)
            } else {
                Decimal::new(quotient, exponent)
            }
        }
    }

    pub fn negate(&self) -> Decimal {
        Decimal {
            coefficient: -&self.coefficient,
            exponent: self.exponent,
        }
    }

    pub fn absolute(&self) -> Decimal {
        Decimal {
            coefficient: self.coefficient.abs(),
            exponent: self.exponent,
        }
    }

    /// `self power n` for an integer exponent of at least 0.
    pub fn power(&self, exponent: &Int) -> Result<Decimal, DecimalError> {
        let Some(n) = exponent.to_i64() else {
            return Err(DecimalError::InvalidExponent);
        };
        if n < 0 {
            return Err(DecimalError::InvalidExponent);
        }
        let mut result = Decimal::new(BigInt::from(1), 0)?;
        for _ in 0..n {
            result = result.multiply(self)?;
        }
        Ok(result)
    }

    /// Rounded half away from zero to `places` decimal places; the result has
    /// exactly that many (decision: the library sketch, 1.1).
    pub fn rounded(&self, places: i64) -> Result<Decimal, DecimalError> {
        let target = -places;
        if self.exponent >= target {
            let coefficient = &self.coefficient * pow10((self.exponent - target) as u64);
            return Decimal::new(coefficient, target);
        }
        let divisor = pow10((target - self.exponent) as u64);
        let (quotient, remainder) = self.coefficient.div_rem(&divisor);
        let twice = &remainder.abs() * BigInt::from(2);
        let coefficient = if twice >= divisor {
            if self.coefficient.is_negative() {
                quotient - 1
            } else {
                quotient + 1
            }
        } else {
            quotient
        };
        Decimal::new(coefficient, target)
    }

    /// The integer part, toward zero.
    pub fn truncated(&self) -> Int {
        if self.exponent >= 0 {
            Int::from_big(&self.coefficient * pow10(self.exponent as u64))
        } else {
            Int::from_big(&self.coefficient / pow10((-self.exponent) as u64))
        }
    }

    pub fn compare(&self, other: &Decimal) -> Ordering {
        let (left, right, _) = self.aligned(other);
        left.cmp(&right)
    }

    /// The coefficient and exponent with trailing zeros removed, so that equal
    /// values hash alike.
    fn normalized(&self) -> (BigInt, i64) {
        if self.coefficient.is_zero() {
            return (BigInt::zero(), 0);
        }
        let ten = BigInt::from(10);
        let mut coefficient = self.coefficient.clone();
        let mut exponent = self.exponent;
        while (&coefficient % &ten).is_zero() {
            coefficient /= &ten;
            exponent += 1;
        }
        (coefficient, exponent)
    }

    pub fn exponent(&self) -> i64 {
        self.exponent
    }

    /// The same digits with the exponent moved by `shift`: `1.5` scaled by
    /// 2 is `150`.
    pub fn scaled(&self, shift: i64) -> Result<Decimal, DecimalError> {
        Decimal::new(self.coefficient.clone(), self.exponent + shift)
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Decimal) -> bool {
        self.compare(other) == Ordering::Equal
    }
}

impl Eq for Decimal {}

impl Hash for Decimal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (coefficient, exponent) = self.normalized();
        coefficient.to_string().hash(state);
        exponent.hash(state);
    }
}

impl fmt::Display for Decimal {
    /// Plain decimal notation, never exponent notation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.coefficient.abs().to_string();
        let sign = if self.coefficient.is_negative() {
            "-"
        } else {
            ""
        };
        if self.exponent >= 0 {
            let zeros = "0".repeat(self.exponent.min(100_000) as usize);
            return write!(f, "{sign}{digits}{zeros}");
        }
        let places = (-self.exponent) as usize;
        if digits.len() > places {
            let (whole, fraction) = digits.split_at(digits.len() - places);
            write!(f, "{sign}{whole}.{fraction}")
        } else {
            let padding = "0".repeat(places - digits.len());
            write!(f, "{sign}0.{padding}{digits}")
        }
    }
}

/// A decimal coefficient as an `Int`, for callers that need the digits.
pub fn coefficient_digits(value: &BigInt) -> usize {
    digit_count(value)
}

/// The nearest `i64` of a big integer, saturating.
pub fn big_to_i64(value: &BigInt) -> i64 {
    value.to_i64().unwrap_or(if value.is_negative() {
        i64::MIN
    } else {
        i64::MAX
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(text: &str) -> Decimal {
        Decimal::parse(text).unwrap()
    }

    #[test]
    fn exponents_are_kept_through_sums_and_products() {
        assert_eq!(d("9.50").add(&d("5.00")).unwrap().to_string(), "14.50");
        assert_eq!(d("2").multiply(&d("2.50")).unwrap().to_string(), "5.00");
        assert_eq!(d("10.00").subtract(&d("1.00")).unwrap().to_string(), "9.00");
        assert_eq!(d("-10").multiply(&d("9")).unwrap().to_string(), "-90");
    }

    #[test]
    fn division_is_exact_when_it_terminates() {
        assert_eq!(d("10.00").divide(&d("4")).unwrap().to_string(), "2.50");
        assert_eq!(d("-90").divide(&d("5")).unwrap().to_string(), "-18");
        assert_eq!(d("1").divide(&d("8")).unwrap().to_string(), "0.125");
        assert_eq!(d("10.00").divide(&d("100")).unwrap().to_string(), "0.10");
        let third = d("1").divide(&d("3")).unwrap().to_string();
        assert_eq!(third, format!("0.{}", "3".repeat(34)));
        let two_thirds = d("2").divide(&d("3")).unwrap().to_string();
        assert_eq!(two_thirds, format!("0.{}7", "6".repeat(33)));
        assert_eq!(d("1").divide(&d("0")), Err(DecimalError::DivisionByZero));
    }

    #[test]
    fn rounding_is_half_away_from_zero_with_fixed_places() {
        assert_eq!(d("7.065").rounded(2).unwrap().to_string(), "7.07");
        assert_eq!(d("-7.065").rounded(2).unwrap().to_string(), "-7.07");
        assert_eq!(d("6").rounded(2).unwrap().to_string(), "6.00");
        assert_eq!(d("0").rounded(2).unwrap().to_string(), "0.00");
        assert_eq!(d("2.5").rounded(0).unwrap().to_string(), "3");
        assert_eq!(d("1234.5678").rounded(3).unwrap().to_string(), "1234.568");
    }

    #[test]
    fn comparison_ignores_scale_and_hashing_agrees() {
        use std::collections::hash_map::DefaultHasher;
        assert_eq!(d("32.0"), d("32.00"));
        assert_eq!(d("0.00"), d("0"));
        assert_eq!(d("1.5").compare(&d("1.25")), Ordering::Greater);
        let hash = |value: &Decimal| {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        };
        assert_eq!(hash(&d("32.0")), hash(&d("32.00")));
    }

    #[test]
    fn thirty_four_digits_round_half_even() {
        let big = d(&"9".repeat(34));
        let sum = big.add(&d("1")).unwrap();
        assert_eq!(sum.to_string(), format!("1{}", "0".repeat(34)));
        let product = d(&"1".repeat(20)).multiply(&d(&"1".repeat(20))).unwrap();
        assert_eq!(product.to_string().len(), 39);
    }

    #[test]
    fn floats_convert_through_their_shortest_text() {
        assert_eq!(Decimal::from_f64(0.1).unwrap().to_string(), "0.1");
        assert_eq!(
            Decimal::from_f64(2.5e10).unwrap().to_string(),
            "25000000000"
        );
        assert_eq!(Decimal::from_f64(1e-7).unwrap().to_string(), "0.0000001");
        assert_eq!(d("7.07").truncated(), Int::Small(7));
        assert_eq!(d("-7.9").truncated(), Int::Small(-7));
    }
}
