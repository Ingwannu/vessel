//! `ChatGPT` quota responses, after Paseo's codex-usage-source and GOAT's
//! codexUsageLimits. A slot is not evidence of its length: duration names
//! identity so the weekly window survives moving from primary to secondary.
use super::number::Number;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use vessel_proto::limits::{
    LimitCredits, LimitFailure, LimitKey, LimitName, LimitPeriod, LimitScope, LimitSlot,
    LimitSnapshot, LimitWindow,
};

#[derive(Debug, Deserialize)]
struct Response {
    plan_type: Option<Plan>,
    rate_limit: Option<Rate>,
    additional_rate_limits: Option<Vec<Entry>>,
    code_review_rate_limit: Option<Rate>,
    credits: Option<Credits>,
}
#[derive(Debug, Deserialize)]
struct Rate {
    primary_window: Option<Window>,
    secondary_window: Option<Window>,
    allowed: Option<bool>,
    limit_reached: Option<bool>,
}
#[derive(Debug, Deserialize)]
struct Window {
    used_percent: Option<Number>,
    reset_at: Option<Number>,
    limit_window_seconds: Option<Number>,
}
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Entry {
    Known(Additional),
    Other(serde::de::IgnoredAny),
}
#[derive(Debug, Deserialize)]
struct Additional {
    limit_name: Option<String>,
    metered_feature: Option<String>,
    rate_limit: Option<Rate>,
}
#[derive(Debug, Deserialize)]
struct Credits {
    balance: Option<Number>,
    #[serde(rename = "has_credits")]
    available: Option<bool>,
    unlimited: Option<bool>,
}

pub(super) fn parse(body: &[u8], now: DateTime<Utc>) -> Result<LimitSnapshot, LimitFailure> {
    let response: Response =
        serde_json::from_slice(body).map_err(|_changed| LimitFailure::ChangedResponse)?;
    let mut windows = Vec::new();
    let mut incomplete = false;
    let allowed = response.rate_limit.as_ref().and_then(|rate| rate.allowed);
    let limit_reached = response
        .rate_limit
        .as_ref()
        .and_then(|rate| rate.limit_reached);
    append(
        &mut windows,
        response.rate_limit,
        "general",
        &LimitScope::General,
        &mut incomplete,
    );
    for entry in response.additional_rate_limits.into_iter().flatten() {
        let Entry::Known(limit) = entry else {
            incomplete = true;
            continue;
        };
        let identity = limit
            .metered_feature
            .as_deref()
            .filter(|id| !id.trim().is_empty())
            .or(limit.limit_name.as_deref());
        let Some(identity) = identity else {
            incomplete = true;
            continue;
        };
        let Ok(id) = identity.parse::<LimitKey>() else {
            incomplete = true;
            continue;
        };
        let Some(name) = limit
            .limit_name
            .as_deref()
            .or(Some(identity))
            .and_then(|name| name.parse::<LimitName>().ok())
        else {
            incomplete = true;
            continue;
        };
        append(
            &mut windows,
            limit.rate_limit,
            &format!("feature:{id}"),
            &LimitScope::Feature { id, name },
            &mut incomplete,
        );
    }
    append(
        &mut windows,
        response.code_review_rate_limit,
        "code_review",
        &LimitScope::CodeReview,
        &mut incomplete,
    );
    Ok(LimitSnapshot {
        at: now,
        plan: plan(response.plan_type),
        windows,
        credits: response.credits.map(|credits| LimitCredits {
            balance: credits.balance.and_then(|number| number.finite()),
            has_credits: credits.available,
            unlimited: credits.unlimited,
        }),
        extra_usage: None,
        allowed,
        limit_reached,
        incomplete,
    })
}
fn append(
    windows: &mut Vec<LimitWindow>,
    rate: Option<Rate>,
    prefix: &str,
    scope: &LimitScope,
    incomplete: &mut bool,
) {
    let Some(rate) = rate else { return };
    for (slot, value) in [
        (LimitSlot::Primary, rate.primary_window),
        (LimitSlot::Secondary, rate.secondary_window),
    ] {
        let Some(value) = value else { continue };
        if windows.len() >= 128 {
            *incomplete = true;
            break;
        }
        let period = value
            .limit_window_seconds
            .as_ref()
            .map_or(LimitPeriod::Unknown, Number::period);
        if value.limit_window_seconds.is_some() && period == LimitPeriod::Unknown {
            *incomplete = true;
        }
        let suffix = match period {
            LimitPeriod::FiveHour => "five_hour".to_owned(),
            LimitPeriod::Weekly => "weekly".to_owned(),
            LimitPeriod::Other { duration } => format!("duration_{}", duration.get()),
            LimitPeriod::Unknown => match slot {
                LimitSlot::Primary => "unknown_primary",
                LimitSlot::Secondary => "unknown_secondary",
                LimitSlot::Only => "unknown",
            }
            .to_owned(),
        };
        let candidate = format!("{prefix}:{suffix}");
        let id = match super::unique_key(&candidate, windows) {
            Ok(id) => id,
            Err(_unrepresentable) => {
                *incomplete = true;
                continue;
            }
        };
        let used = value.used_percent.as_ref().and_then(Number::percent);
        let resets_at = value.reset_at.as_ref().and_then(Number::epoch);
        if value.used_percent.is_some() && used.is_none()
            || value.reset_at.is_some() && resets_at.is_none()
        {
            *incomplete = true;
        }
        windows.push(LimitWindow {
            id,
            period,
            slot,
            scope: scope.clone(),
            used,
            resets_at,
        });
    }
}
// Account strings are not echoed from this sensitive endpoint. Unknown future
// subscription labels remain absent until explicitly understood here.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Plan {
    Free,
    Go,
    Plus,
    Pro,
    Team,
    Business,
    Enterprise,
    Edu,
    #[serde(other)]
    Unknown,
}
fn plan(value: Option<Plan>) -> Option<LimitName> {
    let name = match value? {
        Plan::Free => "Free",
        Plan::Go => "Go",
        Plan::Plus => "Plus",
        Plan::Pro => "Pro",
        Plan::Team => "Team",
        Plan::Business => "Business",
        Plan::Enterprise => "Enterprise",
        Plan::Edu => "Education",
        Plan::Unknown => return None,
    };
    name.parse().ok()
}
