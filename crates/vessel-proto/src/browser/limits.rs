//! Sizes the browser tools bound: a viewport, a wait, a log page, a
//! scroll, and the text and image caps.

use std::time::Duration;

use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::conversation::FiniteNumber;

use super::InvalidInput;

/// The most bytes of snapshot or evaluate text this slice keeps.
pub const MAX_TEXT_BYTES: usize = 80_000;

/// The most decoded screenshot bytes. Paseo has no cap.
pub const MAX_SCREENSHOT_BYTES: usize = 8_388_608;

/// The longest viewport side. Paseo has no cap.
pub const MAX_VIEWPORT_SIDE: u16 = 8192;

/// The default `browser_wait`, when the caller sets no `timeoutMs`.
pub const DEFAULT_WAIT_MS: u32 = 5_000;

/// The longest `timeoutMs`. The broker deadline is this plus one second.
pub const MAX_WAIT_MS: u32 = 30_000;

/// Whether a capped string was cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TextCap {
    /// The whole text fit.
    Whole,
    /// The cap bound and the rest was dropped.
    Cut,
}

/// `text` cut to at most `max` bytes, on a character boundary.
pub(crate) fn cap_units(text: &str, max: usize) -> (String, TextCap) {
    if text.len() <= max {
        return (text.to_owned(), TextCap::Whole);
    }
    let mut kept = String::new();
    for ch in text.chars() {
        let next = kept.len().saturating_add(ch.len_utf8());
        if next > max {
            break;
        }
        kept.push(ch);
    }
    (kept, TextCap::Cut)
}

/// One side of a viewport, from 1 to [`MAX_VIEWPORT_SIDE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
pub struct ViewportSide(u16);

impl ViewportSide {
    /// `n` when it is in range.
    ///
    /// # Errors
    /// [`InvalidInput::Viewport`] for 0, for a side above the cap, and for
    /// a number that does not fit the side.
    pub fn new(n: u32) -> Result<Self, InvalidInput> {
        let side = u16::try_from(n).map_err(|_too_wide| InvalidInput::Viewport)?;
        if (1..=MAX_VIEWPORT_SIDE).contains(&side) {
            Ok(Self(side))
        } else {
            Err(InvalidInput::Viewport)
        }
    }

    /// The side.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl Serialize for ViewportSide {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u16(self.0)
    }
}

impl<'de> Deserialize<'de> for ViewportSide {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(u32::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// A viewport. Each side is a [`ViewportSide`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ViewportSize {
    /// The width.
    pub width: ViewportSide,
    /// The height.
    pub height: ViewportSide,
}

/// How many console and network entries `browser_logs` returns: 1 to 200,
/// default 50.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
pub struct LogLimit(u16);

impl LogLimit {
    /// The default page.
    pub const DEFAULT: Self = Self(50);

    /// `n` when it is from 1 to 200.
    ///
    /// # Errors
    /// [`InvalidInput::LogLimit`].
    pub fn new(n: u32) -> Result<Self, InvalidInput> {
        let n = u16::try_from(n).map_err(|_too_wide| InvalidInput::LogLimit)?;
        if (1..=200).contains(&n) {
            Ok(Self(n))
        } else {
            Err(InvalidInput::LogLimit)
        }
    }

    /// The limit.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl Default for LogLimit {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Serialize for LogLimit {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u16(self.0)
    }
}

impl<'de> Deserialize<'de> for LogLimit {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(u32::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// `browser_wait`'s `timeoutMs`: 1 to 30_000. Absent means the page waits
/// the default 5 seconds and the broker keeps its 15 second deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
pub struct WaitMs(u32);

impl WaitMs {
    /// The default page wait, used when the caller sets no timeout.
    pub const DEFAULT: Self = Self(DEFAULT_WAIT_MS);

    /// `ms` when it is in range.
    ///
    /// # Errors
    /// [`InvalidInput::WaitTimeout`].
    pub const fn new(ms: u32) -> Result<Self, InvalidInput> {
        if ms >= 1 && ms <= MAX_WAIT_MS {
            Ok(Self(ms))
        } else {
            Err(InvalidInput::WaitTimeout)
        }
    }

    /// The milliseconds.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// The page wait as a duration.
    #[must_use]
    pub fn as_duration(self) -> Duration {
        Duration::from_millis(u64::from(self.0))
    }
}

impl Serialize for WaitMs {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.0)
    }
}

impl<'de> Deserialize<'de> for WaitMs {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(u32::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// A scroll's x and y, in CSS pixels. Both are finite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CssDelta {
    /// Horizontal CSS pixels.
    pub x: FiniteNumber,
    /// Vertical CSS pixels.
    pub y: FiniteNumber,
}

impl CssDelta {
    /// Both numbers, when each is finite.
    ///
    /// # Errors
    /// [`InvalidInput::Scroll`] when either is NaN or infinite.
    pub fn new(x: f64, y: f64) -> Result<Self, InvalidInput> {
        let x = FiniteNumber::new(x).ok_or(InvalidInput::Scroll)?;
        let y = FiniteNumber::new(y).ok_or(InvalidInput::Scroll)?;
        Ok(Self { x, y })
    }
}

/// A point on the page, in CSS pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CssPoint {
    /// Horizontal CSS pixels.
    pub x: FiniteNumber,
    /// Vertical CSS pixels.
    pub y: FiniteNumber,
}

impl CssPoint {
    /// A finite point.
    #[must_use]
    pub fn new(x: f64, y: f64) -> Option<Self> {
        Some(Self {
            x: FiniteNumber::new(x)?,
            y: FiniteNumber::new(y)?,
        })
    }
}
