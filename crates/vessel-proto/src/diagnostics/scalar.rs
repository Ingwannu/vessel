//! Quantities of a report, with no text from the node's configuration.
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use super::{InvalidDiagnostic, MAX_DIAGNOSTIC_AGENTS};

/// Largest exactly representable count in every client, including JSON numbers.
/// Larger observed counts are explicitly saturated rather than rounded by JS.
pub const MAX_DIAGNOSTIC_COUNT: u64 = 9_007_199_254_740_991;

/// A count whose arithmetic never wraps or silently loses overflow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DiagnosticCount {
    value: u64,
    saturated: bool,
}
impl DiagnosticCount {
    /// Wrap an observed count, saturating above the client-exact ceiling.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        if value > MAX_DIAGNOSTIC_COUNT {
            Self {
                value: MAX_DIAGNOSTIC_COUNT,
                saturated: true,
            }
        } else {
            Self {
                value,
                saturated: false,
            }
        }
    }
    /// Its count, with [`Self::is_saturated`] saying whether it overflowed.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.value
    }
    /// Whether a previous addition overflowed.
    #[must_use]
    pub const fn is_saturated(self) -> bool {
        self.saturated
    }
    /// Add with an explicit saturation flag.
    #[must_use]
    pub const fn saturating_add(self, other: Self) -> Self {
        match self.value.checked_add(other.value) {
            Some(value) if value <= MAX_DIAGNOSTIC_COUNT => Self {
                value,
                saturated: self.saturated || other.saturated,
            },
            Some(_) | None => Self {
                value: MAX_DIAGNOSTIC_COUNT,
                saturated: true,
            },
        }
    }
}
impl<'de> Deserialize<'de> for DiagnosticCount {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            value: u64,
            saturated: bool,
        }
        let fields = Fields::deserialize(input)
            .map_err(|_invalid| D::Error::custom(InvalidDiagnostic::Count))?;
        if fields.value > MAX_DIAGNOSTIC_COUNT
            || (fields.saturated && fields.value != MAX_DIAGNOSTIC_COUNT)
        {
            return Err(D::Error::custom(InvalidDiagnostic::Count));
        }
        Ok(Self {
            value: fields.value,
            saturated: fields.saturated,
        })
    }
}

/// `SQLite`'s nonnegative user-version ceiling.
pub const MAX_DIAGNOSTIC_SCHEMA: u32 = i32::MAX.unsigned_abs();

/// A schema version, not a wire protocol number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct DiagnosticSchemaVersion(u32);
impl DiagnosticSchemaVersion {
    /// Its `SQLite` schema number.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
impl TryFrom<u32> for DiagnosticSchemaVersion {
    type Error = InvalidDiagnostic;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value > MAX_DIAGNOSTIC_SCHEMA {
            return Err(InvalidDiagnostic::Schema);
        }
        Ok(Self(value))
    }
}
impl<'de> Deserialize<'de> for DiagnosticSchemaVersion {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let value = u32::deserialize(input)
            .map_err(|_invalid| D::Error::custom(InvalidDiagnostic::Schema))?;
        Self::try_from(value).map_err(D::Error::custom)
    }
}

/// First report-local custom slot. It is not a persistent agent identity.
pub const MIN_CUSTOM_AGENT_SLOT: usize = 1;

/// A custom agent's slot in this report, never its person-chosen name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct CustomAgentSlot(usize);
impl CustomAgentSlot {
    /// The report-local ordinal.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}
impl TryFrom<usize> for CustomAgentSlot {
    type Error = InvalidDiagnostic;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if !(MIN_CUSTOM_AGENT_SLOT..=MAX_DIAGNOSTIC_AGENTS).contains(&value) {
            return Err(InvalidDiagnostic::CustomSlot);
        }
        Ok(Self(value))
    }
}
impl<'de> Deserialize<'de> for CustomAgentSlot {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let value = usize::deserialize(input)
            .map_err(|_invalid| D::Error::custom(InvalidDiagnostic::CustomSlot))?;
        Self::try_from(value).map_err(D::Error::custom)
    }
}
