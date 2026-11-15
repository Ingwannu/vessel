//! Inclusive row ranges have Paseo's negative-from-end and clamping semantics.
/// Upper bound for one returned text projection, including row separators.
pub const MAX_CAPTURE_BYTES: usize = 1024 * 1024;
/// A signed requested line position; negative positions count from the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineOffset(i64);
impl From<i64> for LineOffset {
    fn from(value: i64) -> Self {
        Self(value)
    }
}
/// Absolute zero-based physical row in the active retained buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineIndex(pub u32);
/// Retained primary history rows, excluding the live screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryLines(pub(crate) usize);
impl HistoryLines {
    /// Count for viewport calculations; never an event-log sequence.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}
/// Number of physical rows retained, including visible blank rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCount(pub(crate) u32);
impl LineCount {
    /// Total retained physical rows.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
/// Omitted ends select the complete retained buffer; reversed ranges are empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CaptureRange {
    /// Inclusive first row.
    pub start: Option<LineOffset>,
    /// Inclusive last row.
    pub end: Option<LineOffset>,
}
impl CaptureRange {
    pub(crate) fn resolve(self, total: LineCount) -> Option<std::ops::RangeInclusive<u32>> {
        let last = total.0.checked_sub(1)?;
        let index = |value: Option<LineOffset>, default: u32| {
            let resolved = value.map_or(i64::from(default), |offset| {
                if offset.0 < 0 {
                    i64::from(total.0).saturating_add(offset.0)
                } else {
                    offset.0
                }
            });
            u32::try_from(resolved.clamp(0, i64::from(last))).unwrap_or_default()
        };
        let start = index(self.start, 0);
        let end = index(self.end, last);
        (start <= end).then_some(start..=end)
    }
}
/// A translated text projection, never an event-log byte slice or synthesized ANSI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    /// Captured physical rows, with trailing JS whitespace removed as Paseo does.
    pub lines: Vec<String>,
    /// Total retained rows, independent of the requested range.
    pub total: LineCount,
}
