//! Copilot's plan/reset response has no invented token or percentage allowance.
use super::{
    AccountUsageError, AccountUsageOutcome,
    common::{self, Text},
};
use serde::Deserialize;
use vessel_proto::usage_reports::{AvailableUsageReport, UsageDetailAccess};
#[derive(Deserialize)]
struct Response {
    copilot_plan: Option<Text>,
    quota_reset_date: Option<Text>,
}
pub(super) fn decode(body: &[u8]) -> Result<AccountUsageOutcome, AccountUsageError> {
    let response: Response = common::decode(body)?;
    let details = common::optional(response.quota_reset_date)?
        .map(|value| common::detail("reset", "Quota reset", value, UsageDetailAccess::Read))
        .transpose()?
        .into_iter()
        .collect::<Vec<_>>()
        .try_into()?;
    Ok(AccountUsageOutcome::Available(AvailableUsageReport {
        plan: common::optional(response.copilot_plan)?,
        details,
        ..Default::default()
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_does_not_fabricate_quota_and_scalar_fields_are_bounded() {
        let AccountUsageOutcome::Available(report) =
            decode(br#"{"copilot_plan":"business","quota_reset_date":"2026-12-01"}"#).unwrap()
        else {
            panic!("available");
        };
        assert_eq!(report.plan.unwrap().as_str(), "business");
        assert!(report.windows.is_empty());
        assert!(report.balances.is_empty());
        assert_eq!(report.details.as_slice()[0].value.as_str(), "2026-12-01");
        let body = format!("{{\"copilot_plan\":\"{}\"}}", "x".repeat(257));
        assert!(decode(body.as_bytes()).is_err());
    }
}
