//! Committed session mutation ordering, independent of log coverage and authority.

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Assignment requires the durable clock acknowledgements, all Info carrier
/// projections and the transactional attention/workspace display consumers.
/// Until then every peer receives Unknown rather than a partial clock contract.
pub const SESSION_MUTATIONS: Option<crate::ProtocolVersion> = None;

mod older;

/// A local writer's monotonic, millisecond mutation timestamp. It is data,
/// never a label-map revision, execution fence or caller attestation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct SessionUpdatedAt(DateTime<Utc>);

/// A clock value cannot be normalized, represented or advanced honestly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidSessionUpdatedAt {
    /// The canonical store timestamp representation cannot hold this value.
    #[error("session mutation time is outside the stored timestamp range")]
    OutOfRange,
    /// Wire/storage values must not silently discard a fractional tick.
    #[error("session mutation time is not a whole millisecond")]
    Submillisecond,
    /// Saturation would lose strict ordering, so the writer must refuse.
    #[error("session mutation time cannot advance within the stored timestamp range")]
    Exhausted,
}

impl SessionUpdatedAt {
    /// A repeated or backwards wall-clock sample still advances by this tick.
    pub const MIN_INCREMENT: TimeDelta = TimeDelta::milliseconds(1);

    /// The stored fact for display; this does not sample a clock or establish
    /// correspondence with a separately published actor snapshot.
    #[must_use]
    pub const fn time(self) -> DateTime<Utc> {
        self.0
    }

    /// Compute a writer-owned clock from its own wall-time sample. Callers
    /// cannot use this pure value operation to change a stored session.
    ///
    /// # Errors
    /// An out-of-range sample or an exhausted previous clock refuses rather
    /// than saturating and producing equal successive timestamps.
    pub fn advance(
        previous: Option<Self>,
        writer_now: DateTime<Utc>,
    ) -> Result<Self, InvalidSessionUpdatedAt> {
        let normalized = DateTime::from_timestamp_millis(writer_now.timestamp_millis())
            .ok_or(InvalidSessionUpdatedAt::OutOfRange)?;
        let now = Self::try_from(normalized)?;
        let Some(previous) = previous else {
            return Ok(now);
        };
        if now > previous {
            return Ok(now);
        }
        let next = previous
            .0
            .checked_add_signed(Self::MIN_INCREMENT)
            .ok_or(InvalidSessionUpdatedAt::Exhausted)?;
        Self::try_from(next).map_err(|_outside| InvalidSessionUpdatedAt::Exhausted)
    }
}

impl TryFrom<DateTime<Utc>> for SessionUpdatedAt {
    type Error = InvalidSessionUpdatedAt;

    fn try_from(time: DateTime<Utc>) -> Result<Self, Self::Error> {
        if time.timestamp_nanos_opt().is_none() {
            return Err(InvalidSessionUpdatedAt::OutOfRange);
        }
        let whole = DateTime::from_timestamp_millis(time.timestamp_millis())
            .ok_or(InvalidSessionUpdatedAt::OutOfRange)?;
        if whole != time {
            return Err(InvalidSessionUpdatedAt::Submillisecond);
        }
        Ok(Self(time))
    }
}

impl Serialize for SessionUpdatedAt {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SessionUpdatedAt {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let time = DateTime::<Utc>::deserialize(deserializer)?;
        Self::try_from(time).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
