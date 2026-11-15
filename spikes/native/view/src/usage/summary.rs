//! Pure pinned-window selection; availability is supplied by the report adapter.
use super::{DisplayAs, DisplayPercent, Pin, PinSelection};
use vessel_proto::conversation::FiniteNumber;
/// An available account's independent window, without transport or view handles.
#[derive(Debug, Clone)]
pub struct SummaryWindow<W> {
    /// Source-owned identity shared across its accounts.
    pub id: W,
    /// Whole label for an absent abbreviated label.
    pub label: String,
    /// Some(empty) deliberately hides the label; None uses the full one.
    pub short_label: Option<String>,
    /// Explicit source pin default, never inferred from a period's name.
    pub summary: bool,
    /// Reported consumption, distinct from an absent figure.
    pub used: Option<FiniteNumber>,
    /// Source-reported remaining figure takes precedence in remaining mode.
    pub remaining: Option<FiniteNumber>,
}
impl<W> SummaryWindow<W> {
    fn percentage(&self, display: DisplayAs) -> Option<DisplayPercent> {
        DisplayPercent::from_reported(self.used, self.remaining, display)
    }
}
/// Only current available reports belong here; retained failures must be filtered first.
#[derive(Debug, Clone)]
pub struct SummaryAccount<S, W> {
    /// Stable source identity, never its icon or display label.
    pub source: S,
    /// Source report order is preserved.
    pub windows: Vec<SummaryWindow<W>>,
}
/// A report's position in one snapshot, used only to keep visual account groups.
/// It is never persisted, sent to a node, or treated as an account identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryGroup(usize);

/// The exact values selected for one line's rendering.
#[derive(Debug)]
pub struct PinnedWindow<'a, S, W> {
    /// Visual grouping within the currently borrowed report list.
    pub group: SummaryGroup,
    /// Source whose identity groups accounts.
    pub source: &'a S,
    /// Source window metadata.
    pub window: &'a SummaryWindow<W>,
    /// Bounded presentation share only.
    pub percent: DisplayPercent,
}
/// Source-marked defaults win; only a report with no marks uses its first percentage.
#[must_use]
pub fn default_pins<S: Clone + Eq, W: Clone + Eq>(
    reports: &[SummaryAccount<S, W>],
) -> Vec<Pin<S, W>> {
    let mut pins = Vec::new();
    for report in reports {
        let marked = report
            .windows
            .iter()
            .any(|window| window.summary && window.percentage(DisplayAs::Used).is_some());
        let eligible = report
            .windows
            .iter()
            .filter(|window| window.percentage(DisplayAs::Used).is_some());
        for window in eligible
            .filter(|window| !marked || window.summary)
            .take(if marked { report.windows.len() } else { 1 })
        {
            let pin = Pin {
                source: report.source.clone(),
                window: window.id.clone(),
            };
            if !pins.contains(&pin) {
                pins.push(pin);
            }
        }
    }
    pins
}
/// Explicit empty selections remain empty, and each pin matches every source account.
#[must_use]
pub fn pinned_windows<'a, S: Clone + Eq, W: Clone + Eq>(
    reports: &'a [SummaryAccount<S, W>],
    pins: &PinSelection<S, W>,
    display: DisplayAs,
) -> Vec<PinnedWindow<'a, S, W>> {
    let defaults = default_pins(reports);
    let effective = pins.effective(&defaults);
    let mut sources = Vec::new();
    for report in reports {
        if !sources.contains(&&report.source) {
            sources.push(&report.source);
        }
    }
    sources
        .into_iter()
        .flat_map(|source| {
            reports
                .iter()
                .enumerate()
                .filter(move |(_, report)| &report.source == source)
        })
        .flat_map(|(index, report)| {
            report.windows.iter().filter_map(move |window| {
                let pinned = effective
                    .iter()
                    .any(|pin| pin.source == report.source && pin.window == window.id);
                pinned
                    .then(|| window.percentage(display))
                    .flatten()
                    .map(|percent| PinnedWindow {
                        group: SummaryGroup(index),
                        source: &report.source,
                        window,
                        percent,
                    })
            })
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn window(id: u8, summary: bool, used: Option<f64>) -> SummaryWindow<u8> {
        SummaryWindow {
            id,
            label: "Full label".into(),
            short_label: None,
            summary,
            used: used.and_then(FiniteNumber::new),
            remaining: None,
        }
    }
    #[test]
    fn source_marks_win_and_one_pin_matches_all_accounts() {
        let reports = [
            SummaryAccount {
                source: 1,
                windows: vec![window(1, false, Some(31.0)), window(2, true, Some(70.0))],
            },
            SummaryAccount {
                source: 1,
                windows: vec![window(2, true, Some(40.0))],
            },
        ];
        assert_eq!(
            default_pins(&reports),
            [Pin {
                source: 1,
                window: 2
            }]
        );
        let chosen = pinned_windows(&reports, &PinSelection::default(), DisplayAs::Remaining);
        assert_eq!(
            chosen
                .iter()
                .map(|window| window.percent.text())
                .collect::<Vec<_>>(),
            ["30%", "60%"]
        );
        assert!(
            pinned_windows(
                &reports,
                &PinSelection::explicit(Vec::new()),
                DisplayAs::Used
            )
            .is_empty()
        );
    }
    #[test]
    fn an_unknown_marked_window_falls_back_to_the_first_reported_percentage() {
        let reports = [SummaryAccount {
            source: 1,
            windows: vec![window(1, true, None), window(2, false, Some(31.0))],
        }];
        assert_eq!(
            default_pins(&reports),
            [Pin {
                source: 1,
                window: 2
            }]
        );
        let reports = [SummaryAccount {
            source: 1,
            windows: vec![window(1, false, None), window(2, false, Some(0.0))],
        }];
        assert_eq!(
            default_pins(&reports),
            [Pin {
                source: 1,
                window: 2
            }]
        );
    }
    #[test]
    fn modes_prefer_their_explicit_side_and_unknown_stays_unknown() {
        let used = FiniteNumber::new(31.0);
        let remaining = FiniteNumber::new(68.0);
        assert_eq!(
            DisplayPercent::from_reported(used, remaining, DisplayAs::Used)
                .unwrap()
                .text(),
            "31%"
        );
        assert_eq!(
            DisplayPercent::from_reported(used, remaining, DisplayAs::Remaining)
                .unwrap()
                .text(),
            "68%"
        );
        assert!(DisplayPercent::from_reported(None, None, DisplayAs::Used).is_none());
    }
}
