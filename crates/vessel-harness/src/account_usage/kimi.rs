//! Kimi's duration/name metadata establishes periods; array order never guesses a week.
use super::{
    AccountUsageError, AccountUsageOutcome,
    common::{self, Number, Text},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashSet;
use vessel_proto::{
    conversation::FiniteNumber,
    limits::LimitPercent,
    usage_reports::{AvailableUsageReport, MAX_USAGE_WINDOWS, UsageCompleteness, UsageSummary},
};
#[derive(Deserialize)]
#[serde(untagged)]
enum Maybe<T> {
    Known(T),
    Unknown(serde::de::IgnoredAny),
}
impl<T> Maybe<T> {
    fn known(self) -> Option<T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Unknown(_) => None,
        }
    }
}
#[derive(Clone, Default, Deserialize)]
struct Fields {
    limit: Option<Number>,
    used: Option<Number>,
    remaining: Option<Number>,
    #[serde(rename = "resetTime")]
    reset_time_camel: Option<Text>,
    #[serde(rename = "resetAt")]
    reset_at_camel: Option<Text>,
    reset_time: Option<Text>,
    reset_at: Option<Text>,
    name: Option<Text>,
    title: Option<Text>,
    scope: Option<Text>,
    duration: Option<Number>,
    #[serde(rename = "timeUnit")]
    time_unit: Option<Text>,
}
impl Fields {
    fn has_values(&self) -> bool {
        self.limit.is_some() || self.used.is_some() || self.remaining.is_some()
    }
    fn label(&self) -> Option<String> {
        self.name
            .as_ref()
            .or(self.title.as_ref())
            .or(self.scope.as_ref())
            .cloned()
            .map(Text::value)
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().to_owned())
    }
    fn reset(&self) -> Option<DateTime<Utc>> {
        self.reset_time_camel
            .as_ref()
            .or(self.reset_at_camel.as_ref())
            .or(self.reset_time.as_ref())
            .or(self.reset_at.as_ref())
            .cloned()
            .map(Text::value)
            .and_then(|text| DateTime::parse_from_rfc3339(&text).ok())
            .map(|date| date.with_timezone(&Utc))
    }
    #[expect(
        clippy::float_arithmetic,
        reason = "Kimi's explicit limit plus used/remaining define this ratio"
    )]
    fn percent(&self) -> Option<LimitPercent> {
        let limit = common::finite(self.limit)?;
        let used = common::finite(self.used)
            .or_else(|| FiniteNumber::new(limit.get() - common::finite(self.remaining)?.get()))?;
        let raw = common::percent(Some(used), Some(limit))?;
        FiniteNumber::new(raw.get().clamp(0.0, 100.0)).map(LimitPercent::reported)
    }
}
#[derive(Deserialize)]
struct Limit {
    #[serde(flatten)]
    metadata: Fields,
    detail: Option<Maybe<Fields>>,
    window: Option<Maybe<Fields>>,
}
#[derive(Deserialize)]
struct Response {
    usage: Option<Maybe<Fields>>,
    limits: Option<common::Rows<Maybe<Limit>, MAX_USAGE_WINDOWS>>,
}
fn slug(text: &str) -> String {
    text.to_ascii_lowercase()
        .split(|letter: char| !letter.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}
#[expect(
    clippy::float_arithmetic,
    reason = "Only source-reported exact minute multiples convert to hours"
)]
fn duration_label(duration: Number, unit: &str) -> Option<String> {
    const MINUTES_PER_HOUR: f64 = 60.0;
    let value = duration.0.get();
    if value <= 0.0 {
        return None;
    }
    let uppercase = unit.to_ascii_uppercase();
    let unit = uppercase.strip_prefix("TIME_UNIT_").unwrap_or(&uppercase);
    let (value, name) = match unit {
        "MINUTE" if value % MINUTES_PER_HOUR == 0.0 => (value / MINUTES_PER_HOUR, "hour"),
        "MINUTE" => (value, "minute"),
        "HOUR" => (value, "hour"),
        "DAY" => (value, "day"),
        "WEEK" => (value, "week"),
        "SECOND" => (value, "second"),
        _ => return None,
    };
    Some(format!("{value}-{name} limit"))
}
fn identity(
    metadata: &Fields,
    fields: &Fields,
    window: Option<&Fields>,
    index: usize,
) -> (String, String) {
    let label = metadata.label().or_else(|| fields.label());
    let duration = window
        .and_then(|window| window.duration)
        .or(metadata.duration)
        .or(fields.duration)
        .filter(|duration| duration.0.get() > 0.0);
    let unit = window
        .and_then(|window| window.time_unit.as_ref())
        .or(metadata.time_unit.as_ref())
        .or(fields.time_unit.as_ref())
        .cloned()
        .map(Text::value);
    let generated = duration
        .zip(unit.as_deref())
        .and_then(|(duration, unit)| duration_label(duration, unit));
    let id = if let Some((duration, unit)) = duration.zip(unit.as_deref()) {
        format!(
            "coding_limit_{}_{}",
            slug(&duration.0.get().to_string()),
            slug(unit)
        )
    } else if let Some(label) = &label {
        format!("coding_limit_{}", slug(label))
    } else {
        format!("coding_limit_{index}")
    };
    (
        id,
        label
            .or(generated)
            .unwrap_or_else(|| format!("Limit {index}")),
    )
}
fn unique(base: &str, seen: &mut HashSet<String>) -> String {
    let mut id = base.to_owned();
    for suffix in 2..=MAX_USAGE_WINDOWS.saturating_add(1) {
        if !seen.contains(&id) {
            break;
        }
        id = format!("{base}_{suffix}");
    }
    let _new = seen.insert(id.clone());
    id
}
pub(super) fn decode(body: &[u8]) -> Result<AccountUsageOutcome, AccountUsageError> {
    let response: Response = common::decode(body)?;
    let mut windows = Vec::new();
    let mut seen = HashSet::new();
    let mut incomplete = false;
    if let Some(usage) = response.usage {
        if let Some(fields) = usage.known() {
            if fields.has_values() {
                let label = fields
                    .label()
                    .or_else(|| {
                        fields
                            .duration
                            .zip(fields.time_unit.clone().map(Text::value))
                            .and_then(|(duration, unit)| duration_label(duration, &unit))
                    })
                    .unwrap_or_else(|| "Usage limit".to_owned());
                windows.push(common::window(
                    &unique("coding_usage", &mut seen),
                    &label,
                    None,
                    fields.percent(),
                    fields.reset(),
                    UsageSummary::Unspecified,
                )?);
            }
        } else {
            incomplete = true;
        }
    }
    let limits = response
        .limits
        .map(common::Rows::into_values)
        .unwrap_or_default();
    for (index, limit) in limits.into_iter().enumerate() {
        let Some(limit) = limit.known() else {
            incomplete = true;
            continue;
        };
        let detail = known_or_partial(limit.detail, &mut incomplete);
        let fields = detail
            .as_ref()
            .filter(|fields| fields.has_values())
            .unwrap_or(&limit.metadata);
        if !fields.has_values() {
            continue;
        }
        let window = known_or_partial(limit.window, &mut incomplete);
        let (id, label) = identity(
            &limit.metadata,
            fields,
            window.as_ref(),
            index.saturating_add(1),
        );
        windows.push(common::window(
            &unique(&id, &mut seen),
            &label,
            None,
            fields.percent(),
            fields.reset(),
            UsageSummary::Unspecified,
        )?);
    }
    let completeness = if incomplete {
        UsageCompleteness::Partial
    } else {
        UsageCompleteness::Complete
    };
    Ok(AccountUsageOutcome::Available(AvailableUsageReport {
        completeness,
        windows: windows.try_into()?,
        ..Default::default()
    }))
}
fn known_or_partial<T>(value: Option<Maybe<T>>, incomplete: &mut bool) -> Option<T> {
    match value {
        Some(Maybe::Known(value)) => Some(value),
        Some(Maybe::Unknown(_)) => {
            *incomplete = true;
            None
        }
        None => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn periods_come_from_metadata_and_equal_duration_ids_remain_unique() {
        let AccountUsageOutcome::Available(report)=decode(br#"{"usage":{"limit":"100","remaining":"25"},"limits":[{"window":{"duration":300,"timeUnit":"TIME_UNIT_MINUTE"},"detail":{"limit":200,"used":100}},{"window":{"duration":300,"timeUnit":"TIME_UNIT_MINUTE"},"detail":{"limit":200,"used":150}},false]}"#).unwrap()else{panic!("available")};
        let windows = report.windows.as_slice();
        assert_eq!(windows[0].label.as_str(), "Usage limit");
        assert_eq!(windows[0].used.unwrap().get().to_string(), "75");
        assert_eq!(windows[1].label.as_str(), "5-hour limit");
        assert_eq!(
            windows[2].id.as_str(),
            "coding_limit_300_time_unit_minute_2"
        );
        assert_eq!(report.completeness, UsageCompleteness::Partial);
    }
    #[test]
    fn missing_or_zero_denominator_keeps_percentage_unknown() {
        let AccountUsageOutcome::Available(report) =
            decode(br#"{"usage":{"limit":0,"used":10}}"#).unwrap()
        else {
            panic!("available")
        };
        assert!(report.windows.as_slice()[0].used.is_none());
    }
}
