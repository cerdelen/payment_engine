use std::fmt;
use std::ops::SubAssign;

use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize};

/// Fixed precision amount type for financial transactions (precise for 4 decimal places).
///
/// Wraps an i128 values which is scaled by 10_000. This enables exact and efficient decimal
/// arithmatics without the rounding problems that come with using floating point numbers in Rust.
///
/// # Scaling
///
/// Multiply Money amounts by [`Self::SCALE`] to initialize.
/// ```rust
/// let mut balance = Amt::from(10_0000); // $10.00
/// balance -= Amt::from(2500); // $2.50
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amt(i128);

impl SubAssign for Amt {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0
    }
}

#[allow(unused)]
impl Amt {
    pub const SCALE: i128 = 10_000;

    /// Creates a new [`Amt`] with 0 value.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Amt(0)
    }

    /// Creates a new [`Amt`] with maximum possible value.
    #[inline]
    #[must_use]
    pub fn max() -> Self {
        Amt(i128::MAX)
    }

    /// Instantiate an Amt. Be aware to scale the values accordingly.
    #[inline]
    #[must_use]
    pub fn from(from: i128) -> Self {
        Amt(from)
    }

    #[must_use]
    /// performs checked_add on underlying i128 values
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        Some(Amt(self.0.checked_add(rhs.0)?))
    }

    #[must_use]
    /// performs checked_sub on underlying i128 values
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        Some(Amt(self.0.checked_sub(rhs.0)?))
    }
}

impl fmt::Display for Amt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let absolute_v = self.0.abs();
        let whole = absolute_v / Amt::SCALE;
        let mut frac = absolute_v % Amt::SCALE;
        // enforce normalization to 1 trailing 0 after decimal point
        if frac == 0 {
            return write!(f, "{}{}.0", sign, whole);
        }

        // trim trailing 0's
        let mut width = 4;
        while frac % 10 == 0 {
            frac /= 10;
            width -= 1;
        }

        write!(
            f,
            "{}{}.{}",
            sign,
            whole,
            format_args!("{:0width$}", frac, width = width)
        )
    }
}

impl Serialize for Amt {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

fn parse_amt(s: &str) -> std::result::Result<Amt, &'static str> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty string");
    }

    // record if the value is prefixed with a '-'
    let negative = s.starts_with('-');
    // remove 1 prefixed '-' or '+'
    let s = s
        .strip_prefix('-')
        .or_else(|| s.strip_prefix('+'))
        .unwrap_or(s);

    let (whole_value_str, fraction_value_str) = match s.split_once('.') {
        Some((w, f)) => {
            // empty fractional part when a decimal point is present will not be allowed (eg. "1.")
            if f.is_empty() {
                return Err("empty fractional part");
            }
            (w, f)
        }
        // empty fractional part when no decimal point is present will  be allowed (eg. "1")
        None => (s, ""),
    };

    if whole_value_str.is_empty() || !whole_value_str.chars().all(|c| c.is_ascii_digit()) {
        return Err("invalid integer part");
    }

    if fraction_value_str.len() > 4 || !fraction_value_str.chars().all(|c| c.is_ascii_digit()) {
        return Err("invalid fractional part");
    }

    let whole_value: i128 = whole_value_str.parse().map_err(|_| "integer overflow")?;
    let fractional_value: i128 = fraction_value_str.parse().unwrap_or(0);
    let fractional_value: i128 =
        fractional_value * 10_i128.pow(4 - fraction_value_str.len() as u32);

    let mut value = whole_value
        .checked_mul(Amt::SCALE)
        .and_then(|v| v.checked_add(fractional_value))
        .ok_or("value overflow")?;

    if negative {
        value = value.checked_neg().ok_or("value underflow")?;
    }
    Ok(Amt(value))
}
// A general assumption is that we dont use any amounts bigger than (2^127 - 1) / 10 ^ 4

struct AmtVisitor;
impl<'de> Visitor<'de> for AmtVisitor {
    type Value = Amt;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "a string representing a decimal value with a precision of up to 4 digits after the decimal point"
        )
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        parse_amt(v).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for Amt {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(AmtVisitor)
    }
}

#[test]
fn test_invalid_amt_strings() {
    assert!(parse_amt("1.23456").is_err());
    assert!(parse_amt("1a1.23456").is_err());
    assert!(parse_amt("1.2a34").is_err());
    assert!(parse_amt("1.").is_err());
    assert!(parse_amt("a").is_err());
    assert!(parse_amt("").is_err());
    assert!(
        parse_amt(
            "9999999999999999999999999999999999999999999999999999999999999999999999999999999"
        )
        .is_err()
    );
    assert!(parse_amt("--1.2345").is_err());
    assert!(parse_amt("++1.2345").is_err());
    assert!(parse_amt("+-1.2345").is_err());
    assert!(parse_amt("-+1.2345").is_err());
}

#[test]
fn test_valid_amt_strings() {
    assert_eq!(parse_amt("1.2345").unwrap(), Amt(12345));
    assert_eq!(parse_amt("-1.2345").unwrap(), Amt(-12345));
    assert_eq!(parse_amt("+1.2345").unwrap(), Amt(12345));
    assert_eq!(
        parse_amt("19053420985320985.2345").unwrap(),
        Amt(190534209853209852345)
    );
    assert_eq!(parse_amt("1").unwrap(), Amt(10000));
    assert_eq!(parse_amt("1.2").unwrap(), Amt(12000));
    assert_eq!(parse_amt("1.23").unwrap(), Amt(12300));
    assert_eq!(parse_amt("1.234").unwrap(), Amt(12340));
}

#[test]
fn test_valid_amt_string_serialize_into_deserialze() {
    let test_strings = [
        "1.234",
        "1.23",
        "1.2",
        "19053420985320985.2345",
        "-1.2345",
        "1.2345",
        "0.0",
    ];
    let non_normalized_values = [
        ("1.00", "1.0"),
        ("-1", "-1.0"),
        ("+1", "1.0"),
        ("1", "1.0"),
        ("-0", "0.0"),
        ("+0", "0.0"),
        ("0", "0.0"),
        ("+1.2345", "1.2345"),
    ];

    for test_string in test_strings {
        let amt: Amt = parse_amt(test_string).unwrap();
        assert_eq!(amt.to_string(), test_string, "failed for '{test_string}'");
    }
    for (test_string_non_normalized, expected_normalization_string) in non_normalized_values {
        let amt: Amt = parse_amt(test_string_non_normalized).unwrap();
        assert_eq!(
            amt.to_string(),
            expected_normalization_string,
            "failed for '{test_string_non_normalized}'"
        );
    }
}
