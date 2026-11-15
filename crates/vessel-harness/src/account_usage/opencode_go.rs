//! `OpenCode` Go reports three independent periods; rolling deliberately has no short name.
use super::{AccountUsageError, AccountUsageOutcome, common};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use vessel_proto::{
    limits::LimitPercent,
    usage_reports::{AvailableUsageReport, UsageSummary},
};
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Status {
    Ok,
    RateLimited,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Window {
    #[serde(rename = "status")]
    _status: Status,
    percent: LimitPercent,
    resets_at: DateTime<Utc>,
}
#[derive(Deserialize)]
struct Windows {
    rolling: Window,
    weekly: Window,
    monthly: Window,
}
#[derive(Deserialize)]
struct Response {
    usage: Windows,
}
pub(super) fn decode(body: &[u8]) -> Result<AccountUsageOutcome, AccountUsageError> {
    let response: Response = common::decode(body)?;
    let windows = [
        ("rolling", "Rolling", "", response.usage.rolling),
        ("weekly", "Weekly", "wk", response.usage.weekly),
        ("monthly", "Monthly", "mo", response.usage.monthly),
    ]
    .into_iter()
    .map(|(id, label, short, window)| {
        common::window(
            id,
            label,
            Some(short),
            Some(window.percent),
            Some(window.resets_at),
            UsageSummary::Unspecified,
        )
    })
    .collect::<Result<Vec<_>, _>>()?
    .try_into()?;
    Ok(AccountUsageOutcome::Available(AvailableUsageReport {
        plan: Some(common::name("Go")?),
        windows,
        ..Default::default()
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::usage_reports::{UsageShortLabel, UsageTone};
    #[test]
    fn rolling_weekly_and_monthly_are_never_combined_or_given_guessed_durations() {
        let body=br#"{"usage":{"rolling":{"status":"ok","percent":31,"resetsAt":"2026-12-01T01:00:00Z"},"weekly":{"status":"rate-limited","percent":100,"resetsAt":"2026-12-02T01:00:00Z"},"monthly":{"status":"ok","percent":70,"resetsAt":"2026-12-03T01:00:00Z"}}}"#;
        let AccountUsageOutcome::Available(report) = decode(body).unwrap() else {
            panic!("available");
        };
        let windows = report.windows.as_slice();
        assert_eq!(windows.len(), 3);
        assert_eq!(windows[0].short_label, Some(UsageShortLabel::Hidden));
        assert_eq!(windows[0].remaining.unwrap().get().to_string(), "69");
        assert_eq!(windows[1].tone, Some(UsageTone::Danger));
        assert_eq!(windows[2].tone, Some(UsageTone::Warning));
    }
}
