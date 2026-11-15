//! Rendered-line capture has inclusive clamped indices, independent of event cursors.

use crate::TerminalId;
use serde::{Deserialize, Serialize};

/// A signed screen-line index: negatives count backwards from retained rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct CaptureLine(i64);
impl CaptureLine {
    /// Preserve the signed row offset when adapting to the shared pure screen core.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}
impl std::str::FromStr for CaptureLine {
    type Err = std::num::ParseIntError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse().map(Self)
    }
}
/// Retained line count is not a byte count or event sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct CaptureLineCount(u32);
impl CaptureLineCount {
    /// Screen admission makes this count bounded before it crosses the wire.
    #[must_use]
    pub const fn new(lines: u32) -> Self {
        Self(lines)
    }
    /// Consumers can compare counts without interpreting serialized text.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
/// Both variants return xterm-compatible translated text; style is not synthesized.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CaptureStyle {
    /// Default plain-text result.
    #[default]
    Plain,
    /// Retain any controls the screen translation actually returns.
    Preserve,
}
/// Paseo indices are zero-based, inclusive, clamped and relative to retained history.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CaptureRange {
    /// None selects the first retained line.
    pub start: Option<CaptureLine>,
    /// None selects the final retained line.
    pub end: Option<CaptureLine>,
}
impl CaptureRange {
    /// Empty buffers/reversed ranges return None; no index is a sentinel.
    #[must_use]
    pub fn resolve(self, total: CaptureLineCount) -> Option<std::ops::RangeInclusive<u32>> {
        let last = total.get().checked_sub(1)?;
        let index = |line: Option<CaptureLine>, fallback: u32| -> Option<u32> {
            let Some(CaptureLine(line)) = line else {
                return Some(fallback);
            };
            let raw = if line < 0 {
                i64::from(total.get()).saturating_add(line)
            } else {
                line
            };
            u32::try_from(raw.clamp(0, i64::from(last))).ok()
        };
        let start = index(self.start, 0)?;
        let end = index(self.end, last)?;
        (start <= end).then_some(start..=end)
    }
}
/// The CLI translates --scrollback to start=0 before constructing this request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CaptureOptions {
    /// Inclusive retained-row selection, never event offsets.
    pub range: CaptureRange,
    /// Preserve or remove controls from translated text.
    pub style: CaptureStyle,
}
/// Pending method registration uses `host_registry::CLI_PARITY_PROTOCOL`, never a literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CaptureTerminal {
    /// Existing terminal identity; the node checks its ordinary ownership policy.
    pub terminal: TerminalId,
    /// One bounded, point-in-time screen projection.
    pub options: CaptureOptions,
}
/// Parsed rows retain blank lines and report their entire retained-buffer count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TerminalCapture {
    /// The terminal actually resolved by the node.
    pub terminal: TerminalId,
    /// Requested translated screen lines, not raw output chunks.
    pub lines: Vec<String>,
    /// All retained rows before the range was applied.
    pub total_lines: CaptureLineCount,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn range(start: &str, end: &str) -> CaptureRange {
        CaptureRange {
            start: Some(start.parse().unwrap()),
            end: Some(end.parse().unwrap()),
        }
    }
    #[test]
    fn negative_clamped_empty_and_reverse_ranges_match_the_reference() {
        assert_eq!(
            range("-2", "-1").resolve(CaptureLineCount::new(5)),
            Some(3..=4)
        );
        assert_eq!(
            range("-99", "99").resolve(CaptureLineCount::new(5)),
            Some(0..=4)
        );
        assert!(range("4", "1").resolve(CaptureLineCount::new(5)).is_none());
        assert!(
            CaptureRange::default()
                .resolve(CaptureLineCount::new(0))
                .is_none()
        );
    }
}
