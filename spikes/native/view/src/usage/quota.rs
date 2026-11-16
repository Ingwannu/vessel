//! Display independent vendor observations without estimating a quota or currency.
use super::{ReportFact, timestamp};
use crate::localization::Translator;
use chrono::{DateTime, FixedOffset, Utc};
use vessel_proto::{
    limits::{ExtraUsage, LimitPeriod, LimitScope, LimitSnapshot, LimitWindow, UsageLimits},
    usage::CostVisibility,
};

/// Scope names are preserved while only explicitly supplied durations are classified.
#[must_use]
pub fn window_label(window: &LimitWindow, words: &'static Translator) -> String {
    let text = words.usage();
    let period = match window.period {
        LimitPeriod::FiveHour => text.five_hour().to_owned(),
        LimitPeriod::Weekly => text.weekly().to_owned(),
        LimitPeriod::Other { duration } => text
            .duration_window(&duration.get().to_string())
            .unwrap_or_else(|_| text.other_window().to_owned()),
        LimitPeriod::Unknown => text.unknown_window().to_owned(),
    };
    let scope = match &window.scope {
        LimitScope::General => return period,
        LimitScope::Model { name, .. }
        | LimitScope::Surface { name, .. }
        | LimitScope::Feature { name, .. } => name.as_str(),
        LimitScope::CodeReview => text.code_review(),
    };
    text.scoped_window(&period, scope).unwrap_or(period)
}

/// A passed reset is explicitly stale, never converted into a renewed allowance.
#[must_use]
pub fn reset_label(
    reset: Option<DateTime<Utc>>,
    offset: FixedOffset,
    now: DateTime<Utc>,
    words: &'static Translator,
) -> String {
    let text = words.usage();
    match reset {
        None => text.reset_unknown().to_owned(),
        Some(at) if at <= now => text.reset_past().to_owned(),
        Some(at) => {
            let instant = timestamp(at, offset, words);
            let duration = reset_duration(at, now, words);
            text.reset_in(&duration, &instant).unwrap_or(instant)
        }
    }
}
pub(super) fn reset_duration(
    at: DateTime<Utc>,
    now: DateTime<Utc>,
    words: &'static Translator,
) -> String {
    const MINUTES_PER_HOUR: i64 = 60;
    const HOURS_PER_DAY: i64 = 24;
    let minutes = at.signed_duration_since(now).num_minutes().max(0);
    let hours = minutes.div_euclid(MINUTES_PER_HOUR);
    let days = hours.div_euclid(HOURS_PER_DAY);
    let text = words.usage();
    let (major, minor) = if days > 0 {
        (
            text.duration_days(&days.to_string())
                .unwrap_or_else(|_| days.to_string()),
            (hours % HOURS_PER_DAY > 0).then(|| {
                text.duration_hours(&(hours % HOURS_PER_DAY).to_string())
                    .unwrap_or_else(|_| (hours % HOURS_PER_DAY).to_string())
            }),
        )
    } else if hours > 0 {
        (
            text.duration_hours(&hours.to_string())
                .unwrap_or_else(|_| hours.to_string()),
            (minutes % MINUTES_PER_HOUR > 0).then(|| {
                text.duration_minutes(&(minutes % MINUTES_PER_HOUR).to_string())
                    .unwrap_or_else(|_| (minutes % MINUTES_PER_HOUR).to_string())
            }),
        )
    } else {
        (
            text.duration_minutes(&minutes.to_string())
                .unwrap_or_else(|_| minutes.to_string()),
            None,
        )
    };
    match minor {
        Some(minor) => text.duration_pair(&major, &minor).unwrap_or(major),
        None => major,
    }
}

/// Account metadata is separate from each allowance's percentage and unit.
#[must_use]
pub fn quota_facts(
    limits: &UsageLimits,
    snapshot: &LimitSnapshot,
    words: &'static Translator,
) -> Vec<ReportFact> {
    let mut facts = Vec::new();
    if let Some(plan) = &snapshot.plan {
        facts.push(fact(words.usage().plan(), plan.as_str()));
    }
    facts.extend(quota_details(limits, snapshot, words));
    facts
}

/// The facts without the plan, for a card whose header carries the plan as a
/// badge (Paseo card.tsx): credits, flags and limit state as detail rows.
#[must_use]
pub fn quota_details(
    limits: &UsageLimits,
    snapshot: &LimitSnapshot,
    words: &'static Translator,
) -> Vec<ReportFact> {
    let text = words.usage();
    let mut facts = Vec::new();
    if let Some(credits) = &snapshot.credits {
        let balance = match limits.cost_visibility {
            CostVisibility::Withheld => text.withheld().to_owned(),
            CostVisibility::Permitted => credits
                .balance
                .map_or_else(|| text.none().to_owned(), |value| value.get().to_string()),
        };
        facts.push(fact(text.credits(), &balance));
        if let Some(value) = credits.has_credits {
            facts.push(fact(text.has_credits(), boolean(value, words)));
        }
        if let Some(value) = credits.unlimited {
            facts.push(fact(text.unlimited(), boolean(value, words)));
        }
    }
    if let Some(extra) = snapshot.extra_usage {
        facts.push(fact(
            text.extra_usage(),
            match extra {
                ExtraUsage::Enabled => text.enabled(),
                ExtraUsage::Disabled => text.disabled(),
            },
        ));
    }
    if let Some(value) = snapshot.allowed {
        facts.push(fact(text.allowed(), boolean(value, words)));
    }
    if let Some(value) = snapshot.limit_reached {
        facts.push(fact(text.limit_reached(), boolean(value, words)));
    }
    facts
}
fn fact(label: &str, value: &str) -> ReportFact {
    ReportFact {
        label: label.to_owned(),
        value: value.to_owned(),
        detail: None,
    }
}
fn boolean(value: bool, words: &'static Translator) -> &'static str {
    if value {
        words.usage().yes()
    } else {
        words.usage().no()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::ENGLISH;
    use vessel_proto::{
        DurationMs,
        limits::{LimitKey, LimitSlot},
    };
    #[test]
    fn unknown_duration_and_code_review_scope_are_not_inferred_from_slots() {
        let words = Translator::get(ENGLISH).unwrap();
        let mut window = LimitWindow {
            id: "window".parse::<LimitKey>().unwrap(),
            period: LimitPeriod::Unknown,
            slot: LimitSlot::Primary,
            scope: LimitScope::CodeReview,
            used: None,
            resets_at: None,
        };
        assert_eq!(
            window_label(&window, words),
            "\u{2068}Code review\u{2069} · \u{2068}Window duration not reported\u{2069}"
        );
        window.period = LimitPeriod::Other {
            duration: DurationMs::from_duration(std::time::Duration::from_secs(42)),
        };
        assert_eq!(
            window_label(&window, words),
            "\u{2068}Code review\u{2069} · \u{2068}Reported window (\u{2068}42000\u{2069} ms)\u{2069}"
        );
        assert_eq!(
            reset_label(
                None,
                FixedOffset::east_opt(0).unwrap(),
                DateTime::UNIX_EPOCH,
                words
            ),
            words.usage().reset_unknown()
        );
        assert_eq!(
            reset_label(
                Some(DateTime::UNIX_EPOCH),
                FixedOffset::east_opt(0).unwrap(),
                DateTime::UNIX_EPOCH,
                words
            ),
            words.usage().reset_past()
        );
    }
    #[test]
    fn reset_countdown_floors_two_units_and_never_invents_a_renewed_quota() {
        let words = Translator::get(ENGLISH).unwrap();
        let now = DateTime::UNIX_EPOCH;
        for (seconds, expected) in [
            (59, "\u{2068}0\u{2069}m"),
            (60, "\u{2068}1\u{2069}m"),
            (3600, "\u{2068}1\u{2069}h"),
            (
                3660,
                "\u{2068}\u{2068}1\u{2069}h\u{2069} \u{2068}\u{2068}1\u{2069}m\u{2069}",
            ),
            (172_860, "\u{2068}2\u{2069}d"),
        ] {
            assert_eq!(
                reset_duration(now + chrono::TimeDelta::seconds(seconds), now, words),
                expected
            );
        }
    }
}
