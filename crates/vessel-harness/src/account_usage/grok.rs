//! A known monthly credit bucket and an independently reported period stay separate.
use super::{
    AccountUsageError, AccountUsageOutcome,
    common::{self, Number, Text},
};
use serde::Deserialize;
use vessel_proto::usage_reports::{
    AvailableUsageReport, UsageBalance, UsageBalanceUnit, UsageSummary,
};
#[derive(Default, Deserialize)]
struct Amount {
    val: Option<Number>,
}
#[derive(Deserialize)]
struct Period {
    r#type: Option<Text>,
    end: Option<chrono::DateTime<chrono::Utc>>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    monthly_limit: Option<Amount>,
    used: Option<Amount>,
    credit_usage_percent: Option<Number>,
    current_period: Option<Period>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Usage {
    credit_usage: Option<Number>,
}
#[derive(Deserialize)]
struct Response {
    config: Option<Config>,
    usage: Option<Usage>,
}
pub(super) fn decode(body: &[u8]) -> Result<AccountUsageOutcome, AccountUsageError> {
    let response: Response = common::decode(body)?;
    let config = response.config.unwrap_or_default();
    let limit = common::finite(config.monthly_limit.and_then(|value| value.val));
    let used = common::finite(
        config
            .used
            .and_then(|value| value.val)
            .or(response.usage.and_then(|value| value.credit_usage)),
    );
    let balances = if limit.is_some() || used.is_some() {
        vec![UsageBalance {
            id: if limit.is_some() {
                "monthly_credits"
            } else {
                "credits"
            }
            .parse()?,
            label: common::name(if limit.is_some() {
                "Monthly credits"
            } else {
                "Credits"
            })?,
            used,
            remaining: common::difference(limit, used),
            limit,
            unit: UsageBalanceUnit::Credits,
            resets_at: None,
            tone: Some(common::tone(common::percent(used, limit))),
        }]
    } else {
        Vec::new()
    };
    let windows = if let Some(percent) = config.credit_usage_percent {
        let (kind, reset) = config.current_period.map_or((None, None), |period| {
            (period.r#type.map(Text::value), period.end)
        });
        let unknown = format!("period:{}", kind.as_deref().unwrap_or("unknown"));
        let (id, label, short) = match kind.as_deref() {
            Some("USAGE_PERIOD_TYPE_WEEKLY") => ("weekly", "Weekly", "wk"),
            Some("USAGE_PERIOD_TYPE_MONTHLY") => ("monthly", "Monthly", "mo"),
            _ => (
                unknown.as_str(),
                kind.as_deref()
                    .filter(|text| !text.is_empty())
                    .unwrap_or("Current period"),
                "",
            ),
        };
        vec![common::window(
            id,
            label,
            Some(short),
            Some(vessel_proto::limits::LimitPercent::reported(percent.0)),
            reset,
            UsageSummary::Unspecified,
        )?]
    } else {
        Vec::new()
    };
    Ok(AccountUsageOutcome::Available(AvailableUsageReport {
        windows: windows.try_into()?,
        balances: balances.try_into()?,
        ..Default::default()
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::usage_reports::UsageShortLabel;
    #[test]
    fn unknown_period_does_not_become_monthly_and_missing_limit_stays_missing() {
        let AccountUsageOutcome::Available(report)=decode(br#"{"config":{"creditUsagePercent":"75","currentPeriod":{"type":"FUTURE"}},"usage":{"creditUsage":"4"}}"#).unwrap() else{panic!("available")};
        assert_eq!(report.windows.as_slice()[0].id.as_str(), "period:FUTURE");
        assert_eq!(
            report.windows.as_slice()[0].short_label,
            Some(UsageShortLabel::Hidden)
        );
        let balance = &report.balances.as_slice()[0];
        assert_eq!(balance.id.as_str(), "credits");
        assert!(balance.limit.is_none() && balance.remaining.is_none());
        assert_eq!(balance.used.unwrap().get().to_string(), "4");
    }
}
