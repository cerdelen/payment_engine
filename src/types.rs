use std::collections::HashMap;
use std::fmt;

use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize};

pub type ClientId = u16;
pub type TxId = u32;
pub type AccountBalances = HashMap<ClientId, ClientAccountBalance>;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Transaction {
    #[serde(rename = "type")]
    tx_type: TransactionType,
    #[serde(rename = "client")]
    client_id: u16,
    #[serde(rename = "tx")]
    tx_id: TxId,
    /// We will be scaling the amount values ourselves by the factor of 10 ^ 4
    #[serde(rename = "amount")]
    amt: Amt,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
}

#[derive(Debug, Serialize)]
pub struct ClientAccountBalance {
    #[serde(rename = "client")]
    pub id: ClientId,
    pub available: Amt,
    pub held: Amt,
    // probably unnecessary as it can be computed when needed (held + available)
    pub total: Amt,
    pub locked: bool,
}

#[allow(unused)]
impl ClientAccountBalance {
    pub fn new(id: ClientId) -> Self {
        Self {
            id,
            available: Amt::new(),
            held: Amt::new(),
            total: Amt::new(),
            locked: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amt(i128);

impl Amt {
    const SCALE: i128 = 10_000;

    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Amt(0)
    }

    /// Instantiate an Amt. Be aware to scale the values accordingly.
    #[inline]
    #[must_use]
    pub fn from(from: i128) -> Self {
        Amt(from)
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
            format!("{:0width$}", frac, width = width)
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

    // record if the value is prefixed with a -
    let negative = s.starts_with('-');
    // remove 1 prefixed - or +
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
