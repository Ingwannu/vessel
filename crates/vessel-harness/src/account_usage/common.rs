//! Scalar coercion stays at the vendor edge; reports receive validated finite values.
use super::AccountUsageError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, de::DeserializeOwned};
use vessel_proto::{
    conversation::FiniteNumber,
    limits::{LimitName, LimitPercent},
    usage_reports::{
        UsageDetail, UsageDetailAccess, UsageShortLabel, UsageSummary, UsageTone, UsageWindow,
    },
};
/// A vendor may encode a number as JSON numeric text; null is handled by Option.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Number(pub(crate) FiniteNumber);
impl<'de> Deserialize<'de> for Number {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Input {
            Number(FiniteNumber),
            Text(String),
        }
        let value = match Input::deserialize(decoder)? {
            Input::Number(value) => Some(value),
            Input::Text(value) => value.trim().parse().ok().and_then(FiniteNumber::new),
        };
        value
            .map(Self)
            .ok_or_else(|| serde::de::Error::custom("expected a finite account usage number"))
    }
}
pub(super) fn finite(value: Option<Number>) -> Option<FiniteNumber> {
    value.map(|value| value.0)
}
#[expect(
    clippy::float_arithmetic,
    reason = "Source-reported same-unit quota operands define this percentage"
)]
pub(super) fn percent(
    used: Option<FiniteNumber>,
    limit: Option<FiniteNumber>,
) -> Option<LimitPercent> {
    let (used, limit) = (used?, limit?);
    if limit.get() <= 0.0 {
        return None;
    }
    FiniteNumber::new(used.get() / limit.get() * 100.0).map(LimitPercent::reported)
}
#[expect(
    clippy::float_arithmetic,
    reason = "Reported same-unit balance subtraction; no cost estimate"
)]
pub(super) fn difference(
    total: Option<FiniteNumber>,
    used: Option<FiniteNumber>,
) -> Option<FiniteNumber> {
    FiniteNumber::new((total?.get() - used?.get()).max(0.0))
}
pub(super) fn milliseconds(value: Option<FiniteNumber>) -> Option<DateTime<Utc>> {
    // Decimal conversion is checked; neither a saturating float cast nor an overflowing epoch is accepted.
    let millis = value?.get().trunc().to_string().parse::<i64>().ok()?;
    DateTime::from_timestamp_millis(millis)
}
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum Timestamp {
    Text(String),
    Number(FiniteNumber),
}
impl Timestamp {
    #[expect(
        clippy::float_arithmetic,
        reason = "Cursor explicitly reports epoch seconds or milliseconds"
    )]
    pub(super) fn datetime(self) -> Option<DateTime<Utc>> {
        const SECONDS_BOUND: f64 = 10_000_000_000.0;
        const MILLIS_PER_SECOND: f64 = 1_000.0;
        let number = match self {
            Self::Number(value) => value,
            Self::Text(text) => {
                if let Ok(date) = DateTime::parse_from_rfc3339(&text) {
                    return Some(date.with_timezone(&Utc));
                }
                FiniteNumber::new(text.trim().parse().ok()?)?
            }
        };
        let millis = if number.get().abs() < SECONDS_BOUND {
            FiniteNumber::new(number.get() * MILLIS_PER_SECOND)?
        } else {
            number
        };
        milliseconds(Some(millis))
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum Text {
    Literal(String),
    Number(FiniteNumber),
    Bool(bool),
}
impl Text {
    pub(super) fn value(self) -> String {
        match self {
            Self::Literal(value) => value,
            Self::Number(value) => value.get().to_string(),
            Self::Bool(value) => value.to_string(),
        }
    }
}
pub(super) fn decode<T: DeserializeOwned>(body: &[u8]) -> Result<T, AccountUsageError> {
    serde_json::from_slice(body)
        .map_err(|source| AccountUsageError::Decode(super::AccountUsageDecodeFailure::new(source)))
}
pub(super) fn name(text: &str) -> Result<LimitName, AccountUsageError> {
    text.parse().map_err(AccountUsageError::Text)
}
pub(super) fn optional(text: Option<Text>) -> Result<Option<LimitName>, AccountUsageError> {
    text.map(Text::value)
        .filter(|text| !text.trim().is_empty())
        .map(|text| name(&text))
        .transpose()
}
pub(super) fn detail(
    id: &str,
    label: &str,
    value: LimitName,
    access: UsageDetailAccess,
) -> Result<UsageDetail, AccountUsageError> {
    Ok(UsageDetail {
        id: id.parse()?,
        label: name(label)?,
        value,
        access,
        tone: None,
    })
}
pub(super) fn tone(used: Option<LimitPercent>) -> UsageTone {
    match used {
        None => UsageTone::Default,
        Some(value) if value.get() > 90.0 => UsageTone::Danger,
        Some(value) if value.get() >= 70.0 => UsageTone::Warning,
        Some(_) => UsageTone::Ok,
    }
}
#[expect(
    clippy::float_arithmetic,
    reason = "The vendor's reported percentage has a presentation complement, never a token or cost estimate"
)]
pub(super) fn window(
    id: &str,
    label: &str,
    short: Option<&str>,
    used: Option<LimitPercent>,
    reset: Option<DateTime<Utc>>,
    summary: UsageSummary,
) -> Result<UsageWindow, AccountUsageError> {
    let remaining = used
        .and_then(|used| FiniteNumber::new((100.0 - used.get()).max(0.0)))
        .map(LimitPercent::reported);
    Ok(UsageWindow {
        id: id.parse()?,
        label: name(label)?,
        short_label: short
            .map(|short| {
                if short.is_empty() {
                    Ok(UsageShortLabel::Hidden)
                } else {
                    name(short).map(UsageShortLabel::Text)
                }
            })
            .transpose()?,
        summary,
        used,
        remaining,
        resets_at: reset,
        runs_out_at: None,
        shortfall: None,
        tone: Some(tone(used)),
    })
}

/// Reject excess source rows before decoding their values; never silently truncate.
pub(super) struct Rows<T, const N: usize>(Vec<T>);
impl<T, const N: usize> Rows<T, N> {
    pub(super) fn into_values(self) -> Vec<T> {
        self.0
    }
}
impl<'de, T: Deserialize<'de>, const N: usize> Deserialize<'de> for Rows<T, N> {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Visitor<T, const N: usize>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>, const N: usize> serde::de::Visitor<'de> for Visitor<T, N> {
            type Value = Rows<T, N>;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "at most {N} source rows")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut rows: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while values.len() < N {
                    match rows.next_element()? {
                        Some(value) => values.push(value),
                        None => return Ok(Rows(values)),
                    }
                }
                let _end = rows.next_element_seed(Excess)?;
                Ok(Rows(values))
            }
        }
        decoder.deserialize_seq(Visitor::<T, N>(std::marker::PhantomData))
    }
}

struct Excess;
impl<'de> serde::de::DeserializeSeed<'de> for Excess {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, _decoder: D) -> Result<(), D::Error> {
        Err(serde::de::Error::custom(
            "too many account usage source rows",
        ))
    }
}
