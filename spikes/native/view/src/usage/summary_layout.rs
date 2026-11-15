//! The richest one-line summary that fits; unmeasured rows start with labels.
/// A meter narrower than this becomes visual noise (Paseo pinned.ts).
pub const MIN_SUMMARY_METER_WIDTH: f32 = 16.0;
/// Space between the optional meter and its percentage label.
pub const SUMMARY_METER_GAP: f32 = 4.0;
/// Finite nonnegative measured layout geometry, never an account amount.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SummaryWidth(f32);
impl SummaryWidth {
    /// Reject unusable geometry rather than treating it as zero room.
    #[must_use]
    pub fn measured(points: f32) -> Option<Self> {
        (points.is_finite() && points >= 0.0).then_some(Self(points))
    }
}
/// Window cardinality is separate from measured width.
#[derive(Debug, Clone, Copy)]
pub struct SummaryWindowCount(usize);
impl SummaryWindowCount {
    /// Counts the windows being rendered, after filtering unavailable reports.
    #[must_use]
    pub fn of<T>(windows: &[T]) -> Self {
        Self(windows.len())
    }
}
/// Every meter is shown or omitted together; a row never mixes densities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryLayout {
    /// Every window includes a quiet meter and its short label.
    Meters,
    /// Percentages and short names fit without meters.
    Labels,
    /// Only percentages are shorter than the short names.
    Percents,
}
/// Measurements can arrive independently; incomplete measurement stays stable.
#[derive(Debug, Clone, Copy)]
pub struct SummaryMeasurements {
    /// Width available inside the summary row.
    pub available: Option<SummaryWidth>,
    /// Shaped labels with icons and gaps, without meters.
    pub labels: Option<SummaryWidth>,
    /// Shaped percentages with icons and gaps, without short names.
    pub percents: Option<SummaryWidth>,
    /// Number of independent windows, not source/account count.
    pub windows: SummaryWindowCount,
}
/// Matches the source's all-meter, labels, then shorter-percent fallback order.
#[must_use]
#[expect(
    clippy::float_arithmetic,
    reason = "Finite measured widths are presentation geometry, never usage accounting"
)]
pub fn summary_layout(measured: SummaryMeasurements) -> SummaryLayout {
    let (Some(available), Some(labels), Some(percents)) =
        (measured.available, measured.labels, measured.percents)
    else {
        return SummaryLayout::Labels;
    };
    let with_meters = (0..measured.windows.0).fold(labels.0, |width, _| {
        width + MIN_SUMMARY_METER_WIDTH + SUMMARY_METER_GAP
    });
    if with_meters <= available.0 {
        SummaryLayout::Meters
    } else if labels.0 <= available.0 {
        SummaryLayout::Labels
    } else if percents.0 < labels.0 {
        SummaryLayout::Percents
    } else {
        SummaryLayout::Labels
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn chosen(available: Option<f32>, labels: f32, percents: f32) -> SummaryLayout {
        summary_layout(SummaryMeasurements {
            available: available.and_then(SummaryWidth::measured),
            labels: SummaryWidth::measured(labels),
            percents: SummaryWidth::measured(percents),
            windows: SummaryWindowCount::of(&[(), ()]),
        })
    }
    #[test]
    fn density_changes_all_meters_together_and_unknown_starts_with_labels() {
        assert_eq!(chosen(None, 60.0, 40.0), SummaryLayout::Labels);
        assert_eq!(chosen(Some(100.0), 60.0, 40.0), SummaryLayout::Meters);
        assert_eq!(chosen(Some(99.0), 60.0, 40.0), SummaryLayout::Labels);
        assert_eq!(chosen(Some(59.0), 60.0, 40.0), SummaryLayout::Percents);
        assert_eq!(chosen(Some(39.0), 40.0, 40.0), SummaryLayout::Labels);
        assert!(SummaryWidth::measured(f32::NAN).is_none());
        assert!(SummaryWidth::measured(-1.0).is_none());
    }
}
