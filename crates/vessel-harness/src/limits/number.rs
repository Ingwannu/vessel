//! Bounded vendor numeric spellings, normalized once at the external edge.
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::time::Duration;
use vessel_proto::DurationMs;
use vessel_proto::conversation::FiniteNumber;
use vessel_proto::limits::{LimitPercent, LimitPeriod};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum Number {
    Numeric(FiniteNumber),
    Text(String),
}
impl Number {
    pub(super) fn finite(&self) -> Option<FiniteNumber> {
        match self {
            Self::Numeric(value) => Some(*value),
            Self::Text(value) => value.trim().parse().ok().and_then(FiniteNumber::new),
        }
    }
    pub(super) fn percent(&self) -> Option<LimitPercent> {
        self.finite().map(LimitPercent::reported)
    }
    fn duration(&self) -> Option<Duration> {
        Duration::try_from_secs_f64(self.finite()?.get()).ok()
    }
    pub(super) fn epoch(&self) -> Option<DateTime<Utc>> {
        let duration = self.duration()?;
        DateTime::from_timestamp(
            i64::try_from(duration.as_secs()).ok()?,
            duration.subsec_nanos(),
        )
    }
    pub(super) fn period(&self) -> LimitPeriod {
        let duration = self.duration();
        let millis = duration
            .filter(|value| value.subsec_nanos().is_multiple_of(1_000_000))
            .and_then(|value| u64::try_from(value.as_millis()).ok());
        match millis {
            Some(18_000_000) => LimitPeriod::FiveHour,
            Some(604_800_000) => LimitPeriod::Weekly,
            Some(0) | None => LimitPeriod::Unknown,
            Some(value) => LimitPeriod::Other {
                duration: DurationMs::from_duration(Duration::from_millis(value)),
            },
        }
    }
}
pub(super) fn reset(text: Option<&str>) -> Option<DateTime<Utc>> {
    text.and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|at| at.with_timezone(&Utc))
}
