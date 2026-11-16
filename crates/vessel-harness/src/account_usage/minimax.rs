//! `MiniMax`'s explicit remaining percentage takes priority over counts for each model/period.
use super::{
    AccountUsageError, AccountUsageOutcome,
    common::{self, Number, Text},
};
use serde::Deserialize;
use vessel_proto::{
    conversation::FiniteNumber,
    limits::LimitPercent,
    usage_reports::{
        AvailableUsageReport, NoQuotaReason, UsageProblem, UsageSummary, UsageTone, UsageWindow,
    },
};
const MAX_MODELS: usize = 64;
#[derive(Deserialize)]
struct Model {
    #[serde(rename = "model_name")]
    name: Option<Text>,
    end_time: Option<Number>,
    weekly_end_time: Option<Number>,
    current_interval_total_count: Option<Number>,
    current_interval_usage_count: Option<Number>,
    current_interval_remaining_percent: Option<Number>,
    current_interval_status: Option<Number>,
    current_weekly_total_count: Option<Number>,
    current_weekly_usage_count: Option<Number>,
    current_weekly_remaining_percent: Option<Number>,
    current_weekly_status: Option<Number>,
}
#[derive(Deserialize)]
struct Status {
    status_code: Option<Number>,
}
#[derive(Deserialize)]
struct Response {
    model_remains: Option<common::Rows<Model, MAX_MODELS>>,
    base_resp: Option<Status>,
}
#[expect(
    clippy::float_arithmetic,
    reason = "MiniMax explicitly reports remaining percentage or same-unit used/total"
)]
fn used(
    remaining: Option<Number>,
    used: Option<Number>,
    total: Option<Number>,
) -> Option<LimitPercent> {
    let raw = if let Some(remaining) = remaining {
        100.0 - remaining.0.get()
    } else {
        common::percent(common::finite(used), common::finite(total))?.get()
    };
    FiniteNumber::new(raw.clamp(0.0, 100.0)).map(LimitPercent::reported)
}
fn tone(status: Option<Number>, used: LimitPercent) -> UsageTone {
    match status.map(|status| status.0.get().to_string()).as_deref() {
        Some("2") => UsageTone::Danger,
        Some("3") => UsageTone::Default,
        Some("0" | "1") => UsageTone::Ok,
        // An unfamiliar/missing status is not a vendor assertion of healthy quota.
        _ => common::tone(Some(used)),
    }
}
fn windows(model: Model) -> Result<Vec<UsageWindow>, AccountUsageError> {
    let label = common::optional(model.name)?.unwrap_or(common::name("MiniMax")?);
    let mut windows = Vec::new();
    for (period, title, used, status, reset) in [
        (
            "interval",
            "Interval",
            used(
                model.current_interval_remaining_percent,
                model.current_interval_usage_count,
                model.current_interval_total_count,
            ),
            model.current_interval_status,
            model.end_time,
        ),
        (
            "weekly",
            "Weekly",
            used(
                model.current_weekly_remaining_percent,
                model.current_weekly_usage_count,
                model.current_weekly_total_count,
            ),
            model.current_weekly_status,
            model.weekly_end_time,
        ),
    ] {
        if let Some(used) = used {
            let mut window = common::window(
                &format!("{period}_{}", label.as_str()),
                &format!("{} · {title}", label.as_str()),
                None,
                Some(used),
                common::milliseconds(common::finite(reset)),
                UsageSummary::Unspecified,
            )?;
            window.tone = Some(tone(status, used));
            windows.push(window);
        }
    }
    Ok(windows)
}
pub(super) fn decode(body: &[u8]) -> Result<AccountUsageOutcome, AccountUsageError> {
    let response: Response = common::decode(body)?;
    if let Some(code) = response.base_resp.and_then(|status| status.status_code) {
        if code.0.get().to_string() == "2062" {
            return Ok(no_plan());
        }
        if code.0.get().to_string() != "0" {
            return Err(AccountUsageError::ChangedResponse);
        }
    }
    let Some(models) = response.model_remains else {
        return Err(AccountUsageError::ChangedResponse);
    };
    let models = models.into_values();
    if models.is_empty() {
        return Ok(no_plan());
    }
    let mut result = Vec::new();
    for model in models {
        result.extend(windows(model)?);
    }
    Ok(AccountUsageOutcome::Available(AvailableUsageReport {
        windows: result.try_into()?,
        ..Default::default()
    }))
}
fn no_plan() -> AccountUsageOutcome {
    AccountUsageOutcome::Unavailable(UsageProblem::NoQuota {
        reason: NoQuotaReason::NoActivePlan,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remaining_percent_wins_and_each_period_retains_its_status() {
        let AccountUsageOutcome::Available(report)=decode(br#"{"model_remains":[{"model_name":"M2","current_interval_total_count":100,"current_interval_usage_count":99,"current_interval_remaining_percent":80,"current_interval_status":2,"current_weekly_usage_count":70,"current_weekly_total_count":100,"current_weekly_status":3}]}"#).unwrap()else{panic!("available")};
        assert_eq!(
            report.windows.as_slice()[0].used.unwrap().get().to_string(),
            "20"
        );
        assert_eq!(report.windows.as_slice()[0].tone, Some(UsageTone::Danger));
        assert_eq!(report.windows.as_slice()[1].tone, Some(UsageTone::Default));
    }
    #[test]
    fn only_explicit_no_subscription_is_no_plan() {
        assert!(matches!(
            decode(br#"{"base_resp":{"status_code":2062},"model_remains":null}"#).unwrap(),
            AccountUsageOutcome::Unavailable(_)
        ));
        assert!(
            decode(br#"{"base_resp":{"status_code":9999,"status_msg":"private vendor message"}}"#)
                .is_err()
        );
        assert!(decode(br"{}").is_err());
    }
}
