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
            text.reset_at(&instant).unwrap_or(instant)
        }
    }
}

/// Account metadata is separate from each allowance's percentage and unit.
#[must_use]
pub fn quota_facts(
    limits: &UsageLimits,
    snapshot: &LimitSnapshot,
    words: &'static Translator,
) -> Vec<ReportFact> {
    let text = words.usage();
    let mut facts = Vec::new();
    if let Some(plan) = &snapshot.plan {
        facts.push(fact(text.plan(), plan.as_str()));
    }
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
