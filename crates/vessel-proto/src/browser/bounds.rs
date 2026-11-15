//! Bounds are defined once so wire decoding and native/MCP preparation agree.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Prevent an agent from allocating an unbounded browser surface.
pub const MAX_VIEWPORT_SIDE: u32 = 8192;
/// Paseo's `browser_wait` maximum (tools.ts).
pub const MAX_WAIT_MS: u32 = 30_000;
/// Paseo's host default wait (service.ts).
pub const DEFAULT_WAIT_MS: u32 = 5_000;
/// Paseo's retained log/request maximum (service.ts / tools.ts).
pub const MAX_LOG_ENTRIES: u32 = 200;
/// Paseo returns this many log entries when omitted.
pub const DEFAULT_LOG_ENTRIES: u32 = 50;
/// Paseo's ordinary broker deadline, also used when a wait omits timeoutMs.
pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(15);
/// An explicit wait gets the same response allowance as Paseo's tool adapter.
const WAIT_RESPONSE_ALLOWANCE: Duration = Duration::from_secs(1);

/// Zero cannot be a viewport, a deadline or a log request.
const MIN_POSITIVE: u32 = 1;

/// Refused quantities carry no page-controlled text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidBrowserBound {
    /// Browser surfaces have a per-dimension allocation ceiling.
    #[error("viewport side must be {MIN_POSITIVE}..={MAX_VIEWPORT_SIDE}")]
    Viewport,
    /// A wait cannot consume actor/transport capacity indefinitely.
    #[error("wait timeout must be {MIN_POSITIVE}..={MAX_WAIT_MS} milliseconds")]
    Wait,
    /// Log calls cannot exceed the retained ring.
    #[error("log count must be {MIN_POSITIVE}..={MAX_LOG_ENTRIES}")]
    Logs,
}

macro_rules! bounded_number {
    ($name:ident, $max:ident, $error:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(try_from = "u32", into = "u32")]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
        pub struct $name(u32);

        impl $name {
            /// Explicit encoding adapters can obtain the checked quantity.
            #[must_use]
            pub const fn get(self) -> u32 {
                self.0
            }
        }

        impl TryFrom<u32> for $name {
            type Error = InvalidBrowserBound;
            fn try_from(value: u32) -> Result<Self, Self::Error> {
                if (MIN_POSITIVE..=$max).contains(&value) {
                    Ok(Self(value))
                } else {
                    Err(InvalidBrowserBound::$error)
                }
            }
        }

        impl From<$name> for u32 {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

bounded_number!(
    ViewportSide,
    MAX_VIEWPORT_SIDE,
    Viewport,
    "A positive bounded CSS-pixel dimension, independent of device scaling."
);
bounded_number!(
    WaitTimeout,
    MAX_WAIT_MS,
    Wait,
    "A bounded wait in milliseconds, not an arbitrary transport deadline."
);
bounded_number!(
    LogCount,
    MAX_LOG_ENTRIES,
    Logs,
    "The requested console/network tail, bounded by its retained ring."
);

impl Default for WaitTimeout {
    fn default() -> Self {
        Self(DEFAULT_WAIT_MS)
    }
}

impl Default for LogCount {
    fn default() -> Self {
        Self(DEFAULT_LOG_ENTRIES)
    }
}

/// Omitted and explicitly supplied wait budgets have different broker deadlines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", content = "timeout", rename_all = "snake_case")]
pub enum WaitBudget {
    /// The host waits its default while the broker retains the ordinary deadline.
    #[default]
    Default,
    /// The host waits this budget and the broker adds its response allowance.
    Explicit(WaitTimeout),
}

impl WaitBudget {
    /// Resolve host waiting without losing whether the caller supplied a timeout.
    #[must_use]
    pub fn host_timeout(self) -> WaitTimeout {
        match self {
            Self::Default => WaitTimeout::default(),
            Self::Explicit(timeout) => timeout,
        }
    }

    /// The outer deadline must exceed an explicit host wait, within the named cap.
    #[must_use]
    pub fn command_timeout(self) -> Duration {
        match self {
            Self::Default => COMMAND_TIMEOUT,
            Self::Explicit(timeout) => Duration::from_millis(u64::from(timeout.get()))
                .saturating_add(WAIT_RESPONSE_ALLOWANCE),
        }
    }
}

/// Both sides are validated before the browser receives a metrics change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ViewportSize {
    /// Horizontal CSS pixels; not scaled image pixels.
    pub width: ViewportSide,
    /// Vertical CSS pixels; not scaled image pixels.
    pub height: ViewportSide,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_and_explicit_default_waits_preserve_reference_deadlines() {
        let omitted = WaitBudget::Default;
        let explicit = WaitBudget::Explicit(WaitTimeout::default());
        assert_eq!(omitted.host_timeout(), explicit.host_timeout());
        assert_eq!(omitted.command_timeout(), COMMAND_TIMEOUT);
        assert_ne!(omitted.command_timeout(), explicit.command_timeout());
        assert_eq!(
            explicit.command_timeout(),
            Duration::from_millis(u64::from(DEFAULT_WAIT_MS))
                .saturating_add(WAIT_RESPONSE_ALLOWANCE)
        );
        assert_eq!(
            WaitBudget::Explicit(WaitTimeout::try_from(MAX_WAIT_MS).unwrap()).command_timeout(),
            Duration::from_millis(u64::from(MAX_WAIT_MS)).saturating_add(WAIT_RESPONSE_ALLOWANCE)
        );
    }

    #[test]
    fn all_decoding_paths_enforce_quantity_boundaries() {
        for value in [MIN_POSITIVE, MAX_VIEWPORT_SIDE] {
            let side = ViewportSide::try_from(value).unwrap();
            assert_eq!(
                serde_json::from_str::<ViewportSide>(&value.to_string()).unwrap(),
                side
            );
        }
        for value in [0, MAX_VIEWPORT_SIDE.saturating_add(MIN_POSITIVE)] {
            assert!(ViewportSide::try_from(value).is_err());
            assert!(serde_json::from_str::<ViewportSide>(&value.to_string()).is_err());
        }
        assert!(WaitTimeout::try_from(MAX_WAIT_MS).is_ok());
        assert!(WaitTimeout::try_from(MAX_WAIT_MS.saturating_add(MIN_POSITIVE)).is_err());
        assert!(LogCount::try_from(MAX_LOG_ENTRIES).is_ok());
        assert!(LogCount::try_from(MAX_LOG_ENTRIES.saturating_add(MIN_POSITIVE)).is_err());
        assert_eq!(WaitTimeout::default().get(), DEFAULT_WAIT_MS);
        assert_eq!(LogCount::default().get(), DEFAULT_LOG_ENTRIES);
    }
}
