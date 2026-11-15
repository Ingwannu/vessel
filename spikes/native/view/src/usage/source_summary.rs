//! Available observations, including their pending refresh, feed pins; source/account labels cannot define identities.
use super::{Inventory, LimitStatus, SummaryAccount, SummaryWindow, limit_view, window_label};
use crate::{agent_mark::AgentMark, localization::Translator};
use vessel_proto::{
    conversation::FiniteNumber,
    limits::{LimitKey, LimitPeriod},
    usage_reports::{
        BuiltinUsageSource, SavedUsageSource, SourceReportState, UsageShortLabel, UsageSourceId,
        UsageSummary,
    },
};
/// One canonical source identity works across the legacy adapter and generic report transport.
#[must_use]
pub fn saved_source(harness: &vessel_proto::HarnessRef) -> Option<UsageSourceId> {
    SavedUsageSource::for_harness(harness).and_then(|source| source.id().ok())
}
/// Source defaults and independent explicit used/remaining values remain pure view data.
#[must_use]
pub fn summary_candidates(
    inventory: &Inventory,
    words: &'static Translator,
) -> Vec<SummaryAccount<UsageSourceId, LimitKey>> {
    if let Some(reports) = inventory.source_reports() {
        if reports.policy == vessel_proto::limits::ExternalLookup::Off {
            return Vec::new();
        }
        return reports
            .reports
            .as_slice()
            .iter()
            .filter_map(|entry| {
                if reports
                    .sources
                    .as_slice()
                    .iter()
                    .find(|source| &source.id == entry.source())
                    .is_some_and(|source| {
                        source.availability
                            != vessel_proto::usage_reports::UsageSourceAvailability::Enabled
                    })
                {
                    return None;
                }
                let observed = match &entry.state {
                    SourceReportState::Available { observed }
                    | SourceReportState::Refreshing {
                        origin: vessel_proto::usage_reports::UsageRefreshOrigin::Available,
                        retained: Some(observed),
                        ..
                    } => observed,
                    SourceReportState::NotRequested
                    | SourceReportState::Refreshing { .. }
                    | SourceReportState::Unavailable { .. }
                    | SourceReportState::Failed { .. } => return None,
                };
                Some(SummaryAccount {
                    source: entry.source().clone(),
                    windows: observed
                        .report
                        .windows
                        .as_slice()
                        .iter()
                        .map(|window| SummaryWindow {
                            id: window.id.clone(),
                            label: window.label.as_str().to_owned(),
                            short_label: window.short_label.as_ref().map(|label| match label {
                                UsageShortLabel::Hidden => String::new(),
                                UsageShortLabel::Text(text) => text.as_str().to_owned(),
                            }),
                            summary: window.summary == UsageSummary::Included,
                            used: window.used.and_then(|value| FiniteNumber::new(value.get())),
                            remaining: window
                                .remaining
                                .and_then(|value| FiniteNumber::new(value.get())),
                        })
                        .collect(),
                })
            })
            .collect();
    }
    inventory
        .accounts()
        .iter()
        .filter_map(|account| {
            let limits = account.result.as_ref().ok()?;
            let view = limit_view(limits);
            if view.status != LimitStatus::Current {
                return None;
            }
            let snapshot = view.snapshot?;
            Some(SummaryAccount {
                source: saved_source(account.account.harness())?,
                windows: snapshot
                    .windows
                    .iter()
                    .map(|window| SummaryWindow {
                        id: window.id.clone(),
                        label: window_label(window, words),
                        short_label: match window.period {
                            LimitPeriod::FiveHour => {
                                Some(words.usage().short_five_hour().to_owned())
                            }
                            LimitPeriod::Weekly => Some(words.usage().short_weekly().to_owned()),
                            LimitPeriod::Unknown => Some(String::new()),
                            LimitPeriod::Other { .. } => None,
                        },
                        summary: false,
                        used: window.used.and_then(|value| FiniteNumber::new(value.get())),
                        remaining: None,
                    })
                    .collect(),
            })
        })
        .collect()
}
/// The node's typed built-in mark is authoritative; arbitrary source labels/ids never brand a card.
#[must_use]
pub fn source_mark(inventory: &Inventory, source: &UsageSourceId) -> AgentMark {
    if let Some(reports) = inventory.source_reports() {
        let builtin = reports
            .sources
            .as_slice()
            .iter()
            .find(|entry| &entry.id == source)
            .and_then(|entry| entry.builtin)
            .or_else(|| {
                reports
                    .reports
                    .as_slice()
                    .iter()
                    .find(|entry| entry.source() == source)
                    .and_then(|entry| entry.builtin)
            });
        return builtin_source_mark(builtin);
    }
    inventory
        .accounts()
        .iter()
        .find(|account| saved_source(account.account.harness()).as_ref() == Some(source))
        .map_or(AgentMark::Generic, |account| {
            AgentMark::of(account.account.harness())
        })
}

/// Match only the node's typed built-in source. New vendor artwork remains Generic
/// until K/operator approval; labels and source-id spellings never select a brand.
#[must_use]
pub const fn builtin_source_mark(builtin: Option<BuiltinUsageSource>) -> AgentMark {
    match builtin {
        Some(BuiltinUsageSource::Claude) => AgentMark::ClaudeCode,
        Some(BuiltinUsageSource::Codex) => AgentMark::Codex,
        Some(BuiltinUsageSource::Copilot) => AgentMark::Copilot,
        Some(BuiltinUsageSource::OpenCodeGo) => AgentMark::OpenCode,
        Some(
            BuiltinUsageSource::Cursor
            | BuiltinUsageSource::Grok
            | BuiltinUsageSource::Kimi
            | BuiltinUsageSource::MiniMax
            | BuiltinUsageSource::Zai,
        )
        | None => AgentMark::Generic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::ENGLISH;
    use vessel_proto::{
        limits::{ExternalLookup, LimitPercent},
        usage::CostVisibility,
        usage_reports::{
            AvailableUsageReport, SourceReportEntry, SuccessfulUsageReport, UsageReports,
            UsageWindow,
        },
    };
    #[test]
    fn only_existing_typed_artwork_is_selected() {
        for (source, mark) in [
            (BuiltinUsageSource::Claude, AgentMark::ClaudeCode),
            (BuiltinUsageSource::Codex, AgentMark::Codex),
            (BuiltinUsageSource::Copilot, AgentMark::Copilot),
            (BuiltinUsageSource::OpenCodeGo, AgentMark::OpenCode),
            (BuiltinUsageSource::Cursor, AgentMark::Generic),
            (BuiltinUsageSource::Grok, AgentMark::Generic),
            (BuiltinUsageSource::Kimi, AgentMark::Generic),
            (BuiltinUsageSource::MiniMax, AgentMark::Generic),
            (BuiltinUsageSource::Zai, AgentMark::Generic),
        ] {
            assert_eq!(builtin_source_mark(Some(source)), mark);
        }
        assert_eq!(builtin_source_mark(None), AgentMark::Generic);
    }
    #[test]
    fn equal_labels_do_not_merge_accounts_and_retained_failures_never_enter_summary() {
        let report = AvailableUsageReport {
            windows: vec![UsageWindow {
                id: "week".parse().unwrap(),
                label: "Weekly".parse().unwrap(),
                short_label: Some(UsageShortLabel::Hidden),
                summary: UsageSummary::Included,
                used: Some(LimitPercent::reported(FiniteNumber::new(30.0).unwrap())),
                remaining: Some(LimitPercent::reported(FiniteNumber::new(65.0).unwrap())),
                resets_at: None,
                runs_out_at: None,
                shortfall: None,
                tone: None,
            }]
            .try_into()
            .unwrap(),
            ..Default::default()
        };
        let observed = SuccessfulUsageReport {
            at: chrono::DateTime::UNIX_EPOCH,
            report,
        };
        let mut entries = Vec::new();
        for account in ["first", "second"] {
            entries.push(SourceReportEntry::new(
                format!("copilot:{account}").parse().unwrap(),
                "Same label".parse().unwrap(),
                Some(BuiltinUsageSource::Copilot),
                None,
                Some("Same account label".parse().unwrap()),
                SourceReportState::Available {
                    observed: observed.clone(),
                },
            ));
        }
        for (account, origin) in [
            (
                "pending-current",
                vessel_proto::usage_reports::UsageRefreshOrigin::Available,
            ),
            (
                "pending-failed",
                vessel_proto::usage_reports::UsageRefreshOrigin::Unavailable,
            ),
        ] {
            entries.push(SourceReportEntry::new(
                format!("copilot:{account}").parse().unwrap(),
                "Same label".parse().unwrap(),
                Some(BuiltinUsageSource::Copilot),
                None,
                None,
                SourceReportState::Refreshing {
                    origin,
                    started_at: None,
                    retained: Some(observed.clone()),
                },
            ));
        }
        entries.push(SourceReportEntry::new(
            "copilot:failed".parse().unwrap(),
            "Same label".parse().unwrap(),
            Some(BuiltinUsageSource::Copilot),
            None,
            None,
            SourceReportState::Failed {
                attempted_at: None,
                reason: vessel_proto::usage_reports::UsageFetchFailure::Unreachable,
                retry_at: None,
                retained: Some(observed),
                logins: Default::default(),
            },
        ));
        let mut reports = UsageReports {
            at: chrono::DateTime::UNIX_EPOCH,
            policy: ExternalLookup::On,
            cost_visibility: CostVisibility::Permitted,
            sources: Default::default(),
            reports: entries.try_into().unwrap(),
            source_failures: Default::default(),
        };
        let words = Translator::get(ENGLISH).unwrap();
        let inventory = Inventory::Sources {
            reports: reports.clone(),
            refresh_problem: None,
        };
        let candidates = summary_candidates(&inventory, words);
        assert_eq!(candidates.len(), 3);
        let pinned = super::super::pinned_windows(
            &candidates,
            &super::super::PinSelection::default(),
            super::super::DisplayAs::Remaining,
        );
        assert_eq!(pinned.len(), 3);
        assert_eq!(pinned[0].percent.text(), "65%");
        assert_ne!(pinned[0].group, pinned[1].group);
        reports.policy = ExternalLookup::Off;
        assert!(
            summary_candidates(
                &Inventory::Sources {
                    reports,
                    refresh_problem: None
                },
                words
            )
            .is_empty()
        );
    }
}
