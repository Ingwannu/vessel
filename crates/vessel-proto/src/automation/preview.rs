//! Read-only schedule previews (protocol 13, ADR 0033). The effective
//! cadence comes back so an interval's preview anchor survives saving.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Cadence, OutOfRange};

/// How many next runs to compute: 1 to 10, default 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(try_from = "u32", into = "u32")]
pub struct PreviewCount(u32);

impl PreviewCount {
    /// The number of runs.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Default for PreviewCount {
    fn default() -> Self {
        Self(5)
    }
}

impl TryFrom<u32> for PreviewCount {
    type Error = OutOfRange;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if (1..=10).contains(&value) {
            Ok(Self(value))
        } else {
            Err(OutOfRange {
                what: "preview run count",
                min: 1,
                max: 10,
            })
        }
    }
}

impl From<PreviewCount> for u32 {
    fn from(value: PreviewCount) -> Self {
        value.0
    }
}

/// `schedule.preview`: compute without saving or starting anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PreviewSchedule {
    /// When it runs. An interval without an anchor is anchored at `after`.
    pub cadence: Cadence,
    /// Strictly after this instant; absent uses one node clock reading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<DateTime<Utc>>,
    /// At most ten runs; absent means five.
    #[serde(default)]
    pub count: PreviewCount,
}

/// A preview's instants and the exact cadence to reuse when saving it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SchedulePreview {
    /// The resolved starting instant.
    pub after: DateTime<Utc>,
    /// Includes the resolved interval anchor; reuse in create/update.
    pub cadence: Cadence,
    /// Strictly increasing UTC instants, exactly the requested count.
    pub runs: Vec<DateTime<Utc>>,
}

/// Why a preview could not be computed, carried in `WireError.detail`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SchedulePreviewError {
    /// Zone absent from the bundled database.
    #[error("choose an IANA time zone this node knows, such as Europe/London")]
    UnknownZone,
    /// A cadence from a newer client.
    #[error("update this node before using this kind of schedule")]
    UnknownCadence,
    /// No matching day within eight years or insufficient future instants.
    #[error("choose a schedule with enough future runs; this expression or time has none")]
    NoFutureRun,
    /// Chrono can represent dates beyond the scheduler's time database.
    #[error("choose a time between years -9999 and 9999 without a leap second")]
    TimeOutOfRange,
    /// A future reason; keep its human message readable.
    #[serde(other)]
    #[error("update this client to read the schedule preview reason")]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Call, ProtocolVersion, Request, Scope};

    #[test]
    fn preview_is_bounded_round_trips_and_requires_protocol_13() {
        assert!(serde_json::from_str::<PreviewCount>("0").is_err());
        assert!(serde_json::from_str::<PreviewCount>("11").is_err());
        let request: PreviewSchedule =
            serde_json::from_str(r#"{"cadence":{"kind":"every","seconds":60}}"#).unwrap();
        assert_eq!(request.count, PreviewCount::default());
        let call = request.into_call();
        assert_eq!(
            call.method()
                .scope()
                .expect("legacy method requires a scope"),
            Scope::Read
        );
        assert_eq!(call.since(), Some(ProtocolVersion::new(13)));
        let json = serde_json::to_string(&call).unwrap();
        assert_eq!(serde_json::from_str::<Call>(&json).unwrap(), call);
    }
}
