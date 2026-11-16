//! A loss-aware projection of already sanitized limits; no provider schema is parsed twice.
use vessel_proto::{
    limits::{
        AccountLimitState, ExtraUsage, LimitName, LimitPeriod, LimitScope, LimitSnapshot,
        LimitWindow, UsageLimits,
    },
    usage_reports::{
        AvailableUsageReport, ReportLoginFailures, SourceReportEntry, SourceReportState,
        SuccessfulUsageReport, UsageBalance, UsageBalanceUnit, UsageCompleteness, UsageDetail,
        UsageDetailAccess, UsageProblem, UsageReportId, UsageSummary, UsageWindow,
    },
};
#[derive(Debug, Clone, thiserror::Error)]
pub(crate) enum ProjectionError {
    #[error("saved account metadata exceeds the validated label or identity shape")]
    Text(#[source] vessel_proto::limits::InvalidLimitText),
    #[error("saved account entries exceed their count or identity constraints")]
    Collection(#[source] vessel_proto::usage_reports::InvalidUsageCollection),
    #[error("this saved login has no report source")]
    Unsupported,
}
fn name(text: &str) -> Result<LimitName, ProjectionError> {
    text.parse().map_err(ProjectionError::Text)
}
pub(crate) fn entry(
    id: UsageReportId,
    label: Option<&str>,
    limits: UsageLimits,
) -> Result<SourceReportEntry, ProjectionError> {
    let source = super::source(&limits.account.harness).ok_or(ProjectionError::Unsupported)?;
    let state = match limits.state {
        AccountLimitState::Off | AccountLimitState::NotRequested => SourceReportState::NotRequested,
        AccountLimitState::Refreshing { retained } => SourceReportState::Refreshing {
            // The legacy limits DTO does not identify the immediately preceding attempt.
            origin: vessel_proto::usage_reports::UsageRefreshOrigin::Unavailable,
            started_at: None,
            retained: retained.map(success).transpose()?,
        },
        AccountLimitState::Ready { snapshot } => SourceReportState::Available {
            observed: success(snapshot)?,
        },
        AccountLimitState::Failed {
            reason,
            retained,
            retry_at,
        } => SourceReportState::Unavailable {
            attempted_at: None,
            retry_at,
            problem: UsageProblem::Login { reason },
            retained: retained.map(success).transpose()?,
            logins: ReportLoginFailures::default(),
        },
    };
    Ok(SourceReportEntry::new(
        id,
        name(source.label())?,
        Some(source.builtin()),
        limits.source,
        label.map(name).transpose()?,
        state,
    )
    .for_visibility(limits.cost_visibility))
}
fn success(snapshot: LimitSnapshot) -> Result<SuccessfulUsageReport, ProjectionError> {
    let at = snapshot.at;
    let windows = snapshot
        .windows
        .into_iter()
        .map(window)
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(ProjectionError::Collection)?;
    let mut details = Vec::new();
    if let Some(extra) = snapshot.extra_usage {
        details.push(UsageDetail {
            id: "extra_usage".parse().map_err(ProjectionError::Text)?,
            label: name("Additional paid use")?,
            value: name(match extra {
                ExtraUsage::Enabled => "Enabled",
                ExtraUsage::Disabled => "Disabled",
            })?,
            access: UsageDetailAccess::Operate,
            tone: None,
        });
    }
    let balances = if let Some(credits) = snapshot.credits {
        if let Some(value) = credits.has_credits {
            details.push(boolean("has_credits", "Credits available", value)?);
        }
        if let Some(value) = credits.unlimited {
            details.push(boolean("unlimited", "Unlimited credits", value)?);
        }
        vec![UsageBalance {
            id: "credits".parse().map_err(ProjectionError::Text)?,
            label: name("Reported credits")?,
            used: None,
            remaining: credits.balance,
            limit: None,
            unit: UsageBalanceUnit::Credits,
            resets_at: None,
            tone: None,
        }]
    } else {
        Vec::new()
    };
    if let Some(value) = snapshot.allowed {
        details.push(boolean("allowed", "Vendor allows use", value)?);
    }
    if let Some(value) = snapshot.limit_reached {
        details.push(boolean(
            "limit_reached",
            "Vendor reports limit reached",
            value,
        )?);
    }
    Ok(SuccessfulUsageReport {
        at,
        report: AvailableUsageReport {
            completeness: if snapshot.incomplete {
                UsageCompleteness::Partial
            } else {
                UsageCompleteness::Complete
            },
            plan: snapshot.plan,
            windows,
            balances: balances.try_into().map_err(ProjectionError::Collection)?,
            details: details.try_into().map_err(ProjectionError::Collection)?,
        },
    })
}
fn window(window: LimitWindow) -> Result<UsageWindow, ProjectionError> {
    let label = window_label(&window)?;
    Ok(UsageWindow {
        id: window.id,
        label,
        short_label: None,
        summary: UsageSummary::Unspecified,
        used: window.used,
        remaining: None,
        resets_at: window.resets_at,
        runs_out_at: None,
        shortfall: None,
        tone: None,
    })
}
fn window_label(window: &LimitWindow) -> Result<LimitName, ProjectionError> {
    let period = match window.period {
        LimitPeriod::FiveHour => "5-hour window".to_owned(),
        LimitPeriod::Weekly => "Weekly window".to_owned(),
        LimitPeriod::Other { duration } => format!("Reported window ({} ms)", duration.get()),
        LimitPeriod::Unknown => "Window duration not reported".to_owned(),
    };
    let scope = match &window.scope {
        LimitScope::General => return name(&period),
        LimitScope::Model { name, .. }
        | LimitScope::Surface { name, .. }
        | LimitScope::Feature { name, .. } => name.as_str(),
        LimitScope::CodeReview => "Code review",
    };
    name(&format!("{scope} · {period}"))
}

fn boolean(id: &str, label: &str, value: bool) -> Result<UsageDetail, ProjectionError> {
    Ok(UsageDetail {
        id: id.parse().map_err(ProjectionError::Text)?,
        label: name(label)?,
        value: name(if value { "Yes" } else { "No" })?,
        access: UsageDetailAccess::Read,
        tone: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{
        conversation::FiniteNumber,
        limits::{ExternalLookup, LimitAccount, LimitCredits, LimitFailure},
        usage::CostVisibility,
    };
    fn snapshot() -> LimitSnapshot {
        LimitSnapshot {
            at: chrono::DateTime::UNIX_EPOCH,
            plan: None,
            windows: Vec::new(),
            credits: Some(LimitCredits {
                balance: FiniteNumber::new(6.0),
                has_credits: Some(true),
                unlimited: Some(false),
            }),
            extra_usage: Some(ExtraUsage::Enabled),
            allowed: Some(false),
            limit_reached: Some(true),
            incomplete: true,
        }
    }
    fn limits(state: AccountLimitState, visibility: CostVisibility) -> UsageLimits {
        UsageLimits {
            policy: ExternalLookup::On,
            account: LimitAccount {
                harness: "codex".parse().unwrap(),
                profile: Some("fixture".parse().unwrap()),
            },
            source: Some(vessel_proto::limits::LimitLoginStore::Codex),
            state,
            cost_visibility: visibility,
            notice: None,
        }
    }
    #[test]
    fn failed_refresh_retains_actual_success_and_unknown_attempt_time() {
        let retry = chrono::DateTime::UNIX_EPOCH.checked_add_signed(chrono::Duration::hours(1));
        let entry = entry(
            "saved-codex:fixture".parse().unwrap(),
            Some("fixture"),
            limits(
                AccountLimitState::Failed {
                    reason: LimitFailure::RateLimited,
                    retained: Some(snapshot()),
                    retry_at: retry,
                },
                CostVisibility::Permitted,
            ),
        )
        .unwrap();
        let SourceReportState::Unavailable {
            attempted_at,
            retry_at,
            retained,
            ..
        } = entry.state
        else {
            panic!("unavailable");
        };
        assert_eq!(attempted_at, None);
        assert_eq!(retry_at, retry);
        let retained = retained.unwrap();
        assert_eq!(retained.at, chrono::DateTime::UNIX_EPOCH);
        assert_eq!(retained.report.completeness, UsageCompleteness::Partial);
        assert_eq!(
            retained.report.balances.as_slice()[0].remaining,
            FiniteNumber::new(6.0)
        );
        assert_eq!(retained.report.details.as_slice().len(), 5);
    }
    #[test]
    fn read_projection_redacts_credit_amounts_and_paid_details_but_keeps_factual_flags() {
        let entry = entry(
            "saved-codex:fixture".parse().unwrap(),
            Some("fixture"),
            limits(
                AccountLimitState::Ready {
                    snapshot: snapshot(),
                },
                CostVisibility::Withheld,
            ),
        )
        .unwrap();
        let SourceReportState::Available { observed } = entry.state else {
            panic!("available");
        };
        assert!(observed.report.balances.as_slice()[0].remaining.is_none());
        assert_eq!(observed.report.details.as_slice().len(), 4);
        assert!(
            observed
                .report
                .details
                .as_slice()
                .iter()
                .all(|detail| detail.access == UsageDetailAccess::Read)
        );
        assert!(
            observed
                .report
                .details
                .as_slice()
                .iter()
                .any(|detail| detail.id.as_str() == "allowed" && detail.value.as_str() == "No")
        );
    }
}
