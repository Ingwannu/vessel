//! Explicit pool percentages win; spend is a balance only when no pools are reported.
use super::{
    AccountUsageError, AccountUsageOutcome,
    common::{self, Number, Timestamp},
};
use serde::Deserialize;
use vessel_proto::{
    conversation::FiniteNumber,
    limits::LimitPercent,
    usage_reports::{AvailableUsageReport, UsageBalance, UsageBalanceUnit, UsageSummary},
};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Plan {
    total_spend: Option<Number>,
    remaining: Option<Number>,
    limit: Option<Number>,
    total_percent_used: Option<Number>,
    auto_percent_used: Option<Number>,
    api_percent_used: Option<Number>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    plan_usage: Option<Plan>,
    billing_cycle_end: Option<Timestamp>,
}
#[expect(
    clippy::float_arithmetic,
    reason = "The Cursor endpoint reports cents; dividing by 100 changes the unit, never estimates a price"
)]
fn dollars(value: Option<Number>) -> Option<FiniteNumber> {
    const CENTS_PER_DOLLAR: f64 = 100.0;
    FiniteNumber::new(value?.0.get() / CENTS_PER_DOLLAR)
}
pub(super) fn decode(body: &[u8]) -> Result<AccountUsageOutcome, AccountUsageError> {
    let response: Response = common::decode(body)?;
    let reset = response.billing_cycle_end.and_then(Timestamp::datetime);
    let mut windows = Vec::new();
    let mut balances = Vec::new();
    if let Some(plan) = response.plan_usage {
        for (id, label, used, summary) in [
            (
                "plan_usage",
                "Total",
                plan.total_percent_used,
                UsageSummary::Included,
            ),
            (
                "cursor_usage",
                "Cursor",
                plan.auto_percent_used,
                UsageSummary::Unspecified,
            ),
            (
                "third_party_usage",
                "Third Party",
                plan.api_percent_used,
                UsageSummary::Unspecified,
            ),
        ] {
            if let Some(used) = used {
                windows.push(common::window(
                    id,
                    label,
                    None,
                    Some(LimitPercent::reported(used.0)),
                    reset,
                    summary,
                )?);
            }
        }
        if windows.is_empty() {
            let (used, remaining, limit) = (
                dollars(plan.total_spend),
                dollars(plan.remaining),
                dollars(plan.limit),
            );
            balances.push(UsageBalance {
                id: "plan_usage".parse()?,
                label: common::name("Plan usage")?,
                used,
                remaining,
                limit,
                unit: UsageBalanceUnit::Usd,
                resets_at: reset,
                tone: Some(common::tone(common::percent(used, limit))),
            });
        }
    }
    Ok(AccountUsageOutcome::Available(AvailableUsageReport {
        windows: windows.try_into()?,
        balances: balances.try_into()?,
        ..Default::default()
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_pool_percentages_never_become_a_spend_ratio() {
        let AccountUsageOutcome::Available(report)=decode(br#"{"planUsage":{"totalSpend":900,"limit":1000,"totalPercentUsed":40,"autoPercentUsed":10},"billingCycleEnd":1800000000}"#).unwrap()else{panic!("available")};
        assert_eq!(
            report.windows.as_slice()[0].used.unwrap().get().to_string(),
            "40"
        );
        assert_eq!(report.windows.as_slice()[0].summary, UsageSummary::Included);
        assert_eq!(
            report.windows.as_slice()[0].resets_at.unwrap().timestamp(),
            1_800_000_000
        );
        assert!(report.balances.is_empty());
        let AccountUsageOutcome::Available(balance) =
            decode(br#"{"planUsage":{"totalSpend":"1234","remaining":null,"limit":2000}}"#)
                .unwrap()
        else {
            panic!("available")
        };
        assert_eq!(
            balance.balances.as_slice()[0]
                .used
                .unwrap()
                .get()
                .to_string(),
            "12.34"
        );
        assert!(balance.balances.as_slice()[0].remaining.is_none());
    }
}
