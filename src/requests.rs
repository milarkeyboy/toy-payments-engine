//! Types used in handling requests for the engine to action.
//!
//! Newtype structs are used for clearer type distinction. Serde shall treat
//! them as transparent i.e. treat them as their wrapped integral type. The
//! trouble is that all the functionality of these types has to be derived
//! explicitly. Perhaps the benefit in strong typing and readability was not
//! worth the extra boilerplate.

use serde::de;
use serde::{Deserialize, Deserializer, Serialize};

use std::fmt;
use std::str::FromStr;

/// Identifier used to identify a client of the payment system.
#[derive(Debug, Serialize, Deserialize, Eq, Hash, PartialEq, Clone, Copy, PartialOrd, Ord)]
#[serde(transparent)]
pub struct ClientId(pub u16);

/// Identifier used to identify a transaction within the payment system.
#[derive(Debug, Deserialize, Eq, Hash, PartialEq, Clone, Copy)]
#[serde(transparent)]
pub struct TransactionId(pub u32);

/// An exact payment amount, stored as ten-thousandths of a unit. Using this
/// allows fixed-point arithmetic within the engine.
#[derive(Debug, Eq, Hash, PartialEq, Clone, Copy)]
pub struct Amount(i64);

/// Types of requests that can be made on the engine.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum RequestType {
    Deposit { amount: Amount },
    Withdrawal { amount: Amount },
    Dispute,
    Resolve,
    Chargeback,
}

/// A request that can be made on the payment engine for it to action.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct Request {
    pub client: ClientId,

    #[serde(rename = "tx")]
    pub transaction: TransactionId,

    #[serde(flatten)]
    pub request_type: RequestType,
}

/// Error returned when parsing an invalid payment amount.
#[derive(Debug, Eq, PartialEq)]
pub struct AmountParseError;

impl Amount {
    /// Number of fixed-point units in one whole payment unit.
    pub const SCALE: i64 = 10_000;

    /// Return the stored fixed-point units.
    pub fn units(self) -> i64 {
        self.0
    }
}

impl FromStr for Amount {
    type Err = AmountParseError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let value = raw.trim();
        if value.is_empty() || value.starts_with('-') {
            return Err(AmountParseError);
        }

        let value = value.strip_prefix('+').unwrap_or(value);
        let (whole, fractional) = value.split_once('.').unwrap_or((value, ""));

        if whole.is_empty() && fractional.is_empty() {
            return Err(AmountParseError);
        }
        if !whole.bytes().all(|b| b.is_ascii_digit())
            || !fractional.bytes().all(|b| b.is_ascii_digit())
            || fractional.len() > 4
        {
            return Err(AmountParseError);
        }

        let whole_units = if whole.is_empty() {
            0
        } else {
            whole
                .parse::<i64>()
                .ok()
                .and_then(|whole| whole.checked_mul(Self::SCALE))
                .ok_or(AmountParseError)?
        };

        let mut fractional_units = 0;
        for digit in fractional.bytes() {
            fractional_units = (fractional_units * 10) + i64::from(digit - b'0');
        }
        for _ in fractional.len()..4 {
            fractional_units *= 10;
        }

        let units = whole_units
            .checked_add(fractional_units)
            .ok_or(AmountParseError)?;
        if units <= 0 {
            return Err(AmountParseError);
        }

        Ok(Self(units))
    }
}

impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AmountVisitor;

        impl de::Visitor<'_> for AmountVisitor {
            type Value = Amount;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a positive decimal amount with up to 4 decimal places")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value.parse().map_err(|_| {
                    E::custom(
                        "invalid amount: expected a positive decimal with up to 4 decimal places",
                    )
                })
            }
        }

        deserializer.deserialize_str(AmountVisitor)
    }
}
