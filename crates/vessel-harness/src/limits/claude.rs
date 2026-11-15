//! Anthropic OAuth usage windows; model and surface identities stay distinct.
use super::number::{Number, reset};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use vessel_proto::limits::{
    ExtraUsage, LimitFailure, LimitKey, LimitName, LimitPeriod, LimitScope, LimitSlot,
    LimitSnapshot, LimitWindow,
};

#[derive(Debug, Deserialize)]
struct Response {
    five_hour: Option<Window>,
    seven_day: Option<Window>,
    seven_day_opus: Option<Window>,
    seven_day_omelette: Option<Window>,
    limits: Option<Vec<Entry>>,
    extra_usage: Option<Extra>,
}
#[derive(Debug, Deserialize)]
struct Window {
    utilization: Number,
    resets_at: Option<String>,
}
#[derive(Debug, Deserialize)]
struct Extra {
    is_enabled: Option<bool>,
}
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Entry {
    Known(Scoped),
    Other(serde::de::IgnoredAny),
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    WeeklyScoped,
    #[serde(other)]
    Unknown,
}
#[derive(Debug, Deserialize)]
struct Scoped {
    kind: Kind,
    percent: Option<Number>,
    resets_at: Option<String>,
    scope: Option<Scope>,
}
#[derive(Debug, Deserialize)]
struct Scope {
    model: Option<Label>,
    surface: Option<Label>,
}
#[derive(Debug, Deserialize)]
struct Label {
    id: Option<String>,
    display_name: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dimension {
    Model,
    Surface,
}
#[derive(Debug)]
struct Value {
    dimension: Dimension,
    id: Option<LimitKey>,
    name: LimitName,
    used: Option<vessel_proto::limits::LimitPercent>,
    reset: Option<DateTime<Utc>>,
}

pub(super) fn parse(body: &[u8], now: DateTime<Utc>) -> Result<LimitSnapshot, LimitFailure> {
    let response: Response =
        serde_json::from_slice(body).map_err(|_changed| LimitFailure::ChangedResponse)?;
    let mut incomplete = false;
    let mut windows = general(response.five_hour, response.seven_day, &mut incomplete)?;
    let mut scoped = legacy(
        response.seven_day_opus,
        response.seven_day_omelette,
        &mut incomplete,
    )?;
    for entry in response.limits.into_iter().flatten() {
        match entry {
            Entry::Known(entry) => {
                if entry
                    .percent
                    .as_ref()
                    .is_some_and(|value| value.percent().is_none())
                    || entry
                        .resets_at
                        .as_deref()
                        .is_some_and(|text| reset(Some(text)).is_none())
                {
                    incomplete = true;
                }
                let Some(value) = scope(entry) else {
                    incomplete = true;
                    continue;
                };
                if let Some(twin) = scoped.iter_mut().find(|twin| same(twin, &value)) {
                    *twin = Value {
                        used: value.used.or(twin.used),
                        reset: value.reset.or(twin.reset),
                        ..value
                    };
                } else if scoped.len() < 126 {
                    scoped.push(value);
                } else {
                    incomplete = true;
                }
            }
            Entry::Other(_) => {
                incomplete = true;
            }
        }
    }
    for value in scoped {
        match window(value, &windows) {
            Ok(value) => windows.push(value),
            Err(_unrepresentable) => {
                incomplete = true;
            }
        }
    }
    Ok(LimitSnapshot {
        at: now,
        plan: None,
        windows,
        credits: None,
        allowed: None,
        limit_reached: None,
        incomplete,
        extra_usage: response
            .extra_usage
            .and_then(|extra| extra.is_enabled)
            .map(|enabled| {
                if enabled {
                    ExtraUsage::Enabled
                } else {
                    ExtraUsage::Disabled
                }
            }),
    })
}
fn general(
    five_hour: Option<Window>,
    seven_day: Option<Window>,
    incomplete: &mut bool,
) -> Result<Vec<LimitWindow>, LimitFailure> {
    let mut windows = Vec::new();
    for (id, period, value) in [
        ("five_hour", LimitPeriod::FiveHour, five_hour),
        ("weekly", LimitPeriod::Weekly, seven_day),
    ] {
        if let Some(value) = value {
            if value.utilization.percent().is_none()
                || value
                    .resets_at
                    .as_deref()
                    .is_some_and(|text| reset(Some(text)).is_none())
            {
                *incomplete = true;
            }
            windows.push(LimitWindow {
                id: id
                    .parse()
                    .map_err(|_invalid| LimitFailure::ChangedResponse)?,
                period,
                slot: LimitSlot::Only,
                scope: LimitScope::General,
                used: value.utilization.percent(),
                resets_at: reset(value.resets_at.as_deref()),
            });
        }
    }
    Ok(windows)
}
fn legacy(
    opus: Option<Window>,
    omelette: Option<Window>,
    incomplete: &mut bool,
) -> Result<Vec<Value>, LimitFailure> {
    let mut scoped = Vec::new();
    for (name, value) in [("Opus", opus), ("Omelette", omelette)] {
        if let Some(value) = value {
            if value.utilization.percent().is_none()
                || value
                    .resets_at
                    .as_deref()
                    .is_some_and(|text| reset(Some(text)).is_none())
            {
                *incomplete = true;
            }
            scoped.push(Value {
                dimension: Dimension::Model,
                id: None,
                name: name
                    .parse()
                    .map_err(|_invalid| LimitFailure::ChangedResponse)?,
                used: value.utilization.percent(),
                reset: reset(value.resets_at.as_deref()),
            });
        }
    }
    Ok(scoped)
}
fn scope(value: Scoped) -> Option<Value> {
    match value.kind {
        Kind::WeeklyScoped => {}
        Kind::Unknown => return None,
    }
    let scope = value.scope?;
    for (dimension, label) in [
        (Dimension::Model, scope.model),
        (Dimension::Surface, scope.surface),
    ] {
        if let Some(label) = label {
            let id = label
                .id
                .as_deref()
                .map(str::trim)
                .filter(|id| !id.is_empty());
            let name = label
                .display_name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .or(id);
            if let Some(name) = name.and_then(|name| name.parse().ok()) {
                return Some(Value {
                    dimension,
                    id: match id {
                        Some(id) => Some(id.parse().ok()?),
                        None => None,
                    },
                    name,
                    used: value.percent.and_then(|value| value.percent()),
                    reset: reset(value.resets_at.as_deref()),
                });
            }
        }
    }
    None
}
fn same(a: &Value, b: &Value) -> bool {
    a.dimension == b.dimension
        && match (&a.id, &b.id) {
            (Some(a), Some(b)) => a == b,
            (Some(_) | None, None) | (None, Some(_)) => {
                normalized(a.name.as_str()) == normalized(b.name.as_str())
            }
        }
}
fn normalized(name: &str) -> String {
    let mut result = String::new();
    let mut separator = false;
    for character in name.chars() {
        if character.is_alphanumeric() {
            if separator && !result.is_empty() {
                result.push('_');
            }
            separator = false;
            result.extend(character.to_lowercase());
        } else {
            separator = true;
        }
    }
    if result.is_empty() {
        name.to_lowercase()
    } else {
        result
    }
}
fn window(value: Value, previous: &[LimitWindow]) -> Result<LimitWindow, LimitFailure> {
    let dimension = match value.dimension {
        Dimension::Model => "model",
        Dimension::Surface => "surface",
    };
    let identity = value
        .id
        .as_ref()
        .map_or_else(|| normalized(value.name.as_str()), ToString::to_string);
    let candidate = format!("weekly_{dimension}_{identity}");
    let id = super::unique_key(&candidate, previous)?;
    let scope = match value.dimension {
        Dimension::Model => LimitScope::Model {
            id: value.id,
            name: value.name,
        },
        Dimension::Surface => LimitScope::Surface {
            id: value.id,
            name: value.name,
        },
    };
    Ok(LimitWindow {
        id,
        period: LimitPeriod::Weekly,
        slot: LimitSlot::Only,
        scope,
        used: value.used,
        resets_at: value.reset,
    })
}
