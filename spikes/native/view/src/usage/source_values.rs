//! Derived drawing shares never overwrite source amounts or bypass monetary visibility.
use super::{DisplayAs, DisplayPercent};
use chrono::{DateTime, Utc};
use vessel_proto::{
    conversation::FiniteNumber,
    usage::CostVisibility,
    usage_reports::{UsageBalance, UsageWindow},
};

/// Paseo `balance-bar.tsx`: positive allowance and a known used amount are needed for a bar.
/// Source amounts remain independent facts; only this bounded drawing ratio is derived.
#[must_use]
#[expect(
    clippy::float_arithmetic,
    reason = "A source's finite amounts are divided only for a display share, never billing or persistence"
)]
pub fn balance_percent(
    balance: &UsageBalance,
    visibility: CostVisibility,
) -> Option<DisplayPercent> {
    const FULL_PERCENT: f64 = 100.0;
    if visibility == CostVisibility::Withheld {
        return None;
    }
    let limit = balance.limit?.get();
    if limit <= 0.0 {
        return None;
    }
    let used = balance
        .used
        .map(FiniteNumber::get)
        .or_else(|| balance.remaining.map(|remaining| limit - remaining.get()))?;
    let percent = FiniteNumber::new(used / limit * FULL_PERCENT)?;
    Some(DisplayPercent::reported(percent, DisplayAs::Used))
}

/// A forecast is not a reset; retained forecasts stop claiming risk once their time passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceWindowTiming {
    /// Show the actual reset separately from any absent/incomplete forecast.
    Reset(Option<DateTime<Utc>>),
    /// The source supplies both forecast and shortfall, never inferred by the view.
    Risk {
        /// Actual source forecast.
        at: DateTime<Utc>,
        /// Actual source shortfall, without drawing clamps.
        shortfall: vessel_proto::limits::LimitPercent,
    },
}
/// Match `window-bar.tsx`'s risk condition, including retained-past suppression.
#[must_use]
pub fn source_window_timing(
    window: &UsageWindow,
    retained: bool,
    now: DateTime<Utc>,
) -> SourceWindowTiming {
    match (window.runs_out_at, window.shortfall) {
        (Some(at), Some(shortfall)) if !retained || at > now => {
            SourceWindowTiming::Risk { at, shortfall }
        }
        _ => SourceWindowTiming::Reset(window.resets_at),
    }
}

/// Risk countdown deliberately excludes reset time, which would imply the wrong event.
#[must_use]
pub fn source_risk_label(
    at: DateTime<Utc>,
    now: DateTime<Utc>,
    words: &'static crate::localization::Translator,
) -> String {
    if at <= now {
        return words.usage().runs_out_now().to_owned();
    }
    let duration = super::quota::reset_duration(at, now, words);
    words.usage().runs_out_in(&duration).unwrap_or(duration)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::usage_reports::{UsageBalanceUnit, UsageSummary};
    fn number(value: f64) -> FiniteNumber {
        FiniteNumber::new(value).unwrap()
    }
    #[test]
    fn balance_drawing_requires_visibility_and_a_positive_known_allowance() {
        let mut balance = UsageBalance {
            id: "credits".parse().unwrap(),
            label: "Credits".parse().unwrap(),
            used: Some(number(30.0)),
            remaining: Some(number(95.0)),
            limit: Some(number(100.0)),
            unit: UsageBalanceUnit::Credits,
            resets_at: None,
            tone: None,
        };
        assert_eq!(
            balance_percent(&balance, CostVisibility::Permitted)
                .unwrap()
                .whole(),
            30
        );
        assert!(balance_percent(&balance, CostVisibility::Withheld).is_none());
        balance.used = None;
        assert_eq!(
            balance_percent(&balance, CostVisibility::Permitted)
                .unwrap()
                .whole(),
            5
        );
        balance.limit = Some(number(0.0));
        assert!(balance_percent(&balance, CostVisibility::Permitted).is_none());
        balance.limit = None;
        assert!(balance_percent(&balance, CostVisibility::Permitted).is_none());
    }
    #[test]
    fn risk_needs_both_source_fields_and_does_not_relabel_reset_time() {
        let at = DateTime::UNIX_EPOCH;
        let mut window = UsageWindow {
            id: "window".parse().unwrap(),
            label: "Window".parse().unwrap(),
            short_label: None,
            summary: UsageSummary::Included,
            used: None,
            remaining: None,
            resets_at: Some(at),
            runs_out_at: Some(at),
            shortfall: None,
            tone: None,
        };
        assert_eq!(
            source_window_timing(&window, false, at),
            SourceWindowTiming::Reset(Some(at))
        );
        window.shortfall = Some(vessel_proto::limits::LimitPercent::reported(number(17.0)));
        assert!(matches!(
            source_window_timing(&window, false, at),
            SourceWindowTiming::Risk { .. }
        ));
        assert_eq!(
            source_window_timing(&window, true, at),
            SourceWindowTiming::Reset(Some(at))
        );
        assert!(matches!(
            source_window_timing(&window, true, at - chrono::TimeDelta::seconds(1)),
            SourceWindowTiming::Risk { .. }
        ));
    }
}
