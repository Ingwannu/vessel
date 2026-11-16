//! Z.ai's subscription metadata is not a numerical quota.
use super::{
    AccountUsageError, AccountUsageOutcome,
    common::{self, Text},
};
use serde::Deserialize;
use vessel_proto::usage_reports::{
    AvailableUsageReport, NoQuotaReason, UsageDetailAccess, UsageProblem,
};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Subscription {
    product_name: Option<Text>,
    status: Option<Text>,
    purchase_time: Option<Text>,
    valid: Option<Text>,
}
#[derive(Deserialize)]
struct Response {
    data: Option<common::Rows<Subscription, MAX_SUBSCRIPTIONS>>,
}
const MAX_SUBSCRIPTIONS: usize = 128;
pub(super) fn decode(body: &[u8]) -> Result<AccountUsageOutcome, AccountUsageError> {
    let response: Response = common::decode(body)?;
    let Some(subscription) = response
        .data
        .and_then(|data| data.into_values().into_iter().next())
    else {
        return Ok(AccountUsageOutcome::Unavailable(UsageProblem::NoQuota {
            reason: NoQuotaReason::NoActivePlan,
        }));
    };
    let mut details = Vec::new();
    for (id, label, value, access) in [
        (
            "status",
            "Status",
            subscription.status,
            UsageDetailAccess::Read,
        ),
        (
            "valid",
            "Valid",
            subscription.valid,
            UsageDetailAccess::Read,
        ),
        (
            "purchase_time",
            "Purchased",
            subscription.purchase_time,
            UsageDetailAccess::Operate,
        ),
    ] {
        if let Some(value) = common::optional(value)? {
            details.push(common::detail(id, label, value, access)?);
        }
    }
    Ok(AccountUsageOutcome::Available(AvailableUsageReport {
        plan: common::optional(subscription.product_name)?,
        details: details.try_into()?,
        ..Default::default()
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_subscription_is_no_quota_while_valid_false_is_a_reported_fact() {
        assert!(matches!(
            decode(br#"{"data":[]}"#).unwrap(),
            AccountUsageOutcome::Unavailable(UsageProblem::NoQuota {
                reason: NoQuotaReason::NoActivePlan
            })
        ));
        let AccountUsageOutcome::Available(report)=decode(br#"{"data":[{"productName":"Coding","status":"active","valid":false,"purchaseTime":123}]}"#).unwrap() else {panic!("available");};
        assert!(report.windows.is_empty());
        assert_eq!(report.details.as_slice()[1].value.as_str(), "false");
        assert_eq!(
            report.details.as_slice()[2].access,
            UsageDetailAccess::Operate
        );
    }
}
