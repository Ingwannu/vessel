//! Presentation projections borrow the authoritative generic DTO; no provider schema lives here.
use super::limit_failure;
use crate::localization::Translator;
use chrono::{DateTime, Utc};
use vessel_proto::{
    usage::CostVisibility,
    usage_reports::{
        SourceReportState, SuccessfulUsageReport, UsageBalance, UsageCredentialStore,
        UsageFetchFailure, UsageProblem, UsageRecipient, UsageSourceAvailability,
        UsageSourceDisclosure,
    },
};
/// Current attempt status is independent of an immutable successful observation.
#[derive(Debug)]
pub struct SourceView<'a> {
    /// Success, including a retained one, never gets the attempt time.
    pub observed: Option<&'a SuccessfulUsageReport>,
    /// Current safe status text; source/provider errors cannot supply it.
    pub status: Option<String>,
    /// The success belongs to a failed/unavailable attempt, including its next refresh.
    /// A pending refresh of an available attempt remains current, not retained.
    pub retained: bool,
    /// Actual known attempt completion, if the source supplies it.
    pub attempted_at: Option<DateTime<Utc>>,
    /// Actual vendor cooldown, never a guessed delay.
    pub retry_at: Option<DateTime<Utc>>,
}
/// Keep every pending/failure state separate from the values it may retain.
#[must_use]
pub fn source_view<'a>(state: &'a SourceReportState, words: &'static Translator) -> SourceView<'a> {
    let (observed, status, attempted_at, retry_at, current) = match state {
        SourceReportState::NotRequested => (
            None,
            Some(words.usage().not_requested().to_owned()),
            None,
            None,
            false,
        ),
        SourceReportState::Refreshing {
            retained, origin, ..
        } => (
            retained.as_ref(),
            Some(words.usage().loading().to_owned()),
            None,
            None,
            *origin == vessel_proto::usage_reports::UsageRefreshOrigin::Available,
        ),
        SourceReportState::Available { observed } => (Some(observed), None, None, None, true),
        SourceReportState::Unavailable {
            attempted_at,
            problem,
            retry_at,
            retained,
            ..
        } => (
            retained.as_ref(),
            Some(source_problem(problem, words).to_owned()),
            *attempted_at,
            *retry_at,
            false,
        ),
        SourceReportState::Failed {
            attempted_at,
            reason,
            retry_at,
            retained,
            ..
        } => (
            retained.as_ref(),
            Some(source_failure(*reason, words).to_owned()),
            *attempted_at,
            *retry_at,
            false,
        ),
    };
    SourceView {
        observed,
        status,
        attempted_at,
        retry_at,
        retained: observed.is_some() && !current,
    }
}
/// Only closed safe categories become display copy.
#[must_use]
pub fn source_problem(problem: &UsageProblem, words: &'static Translator) -> &'static str {
    use vessel_proto::usage_reports::NoQuotaReason;
    match problem {
        UsageProblem::Login { reason } => limit_failure(*reason, words),
        UsageProblem::NoQuota { reason } => match reason {
            NoQuotaReason::NoActivePlan => words.usage().no_active_plan(),
            NoQuotaReason::NoAllowance => words.usage().no_allowance(),
            NoQuotaReason::Inactive => words.usage().inactive_plan(),
        },
    }
}
/// Runtime failures never reveal arbitrary helper stderr or provider bodies.
#[must_use]
pub fn source_failure(failure: UsageFetchFailure, words: &'static Translator) -> &'static str {
    match failure {
        UsageFetchFailure::TimedOut => words.usage().timeout(),
        UsageFetchFailure::Unreachable => words.usage().source_unreachable(),
        UsageFetchFailure::ChangedResponse => words.usage().failed(),
        UsageFetchFailure::SourceRemoved | UsageFetchFailure::HelperUnavailable => {
            words.usage().unsupported()
        }
        UsageFetchFailure::Busy => words.usage().busy(),
    }
}
/// Unsupported and stale consent are never described as an enabled quota lookup.
#[must_use]
pub fn source_availability(
    state: UsageSourceAvailability,
    words: &'static Translator,
) -> &'static str {
    match state {
        UsageSourceAvailability::Unsupported => words.usage().source_unsupported(),
        UsageSourceAvailability::GlobalOff => words.usage().lookup_off(),
        UsageSourceAvailability::SourceOff => words.usage().source_off(),
        UsageSourceAvailability::DisclosureNeedsConsent => words.usage().source_needs_consent(),
        UsageSourceAvailability::Enabled => words.usage().enabled(),
    }
}
/// Known candidate kinds and fixed vendors are translated without inspecting a path or label.
#[must_use]
pub fn source_disclosure(disclosure: &UsageSourceDisclosure, words: &'static Translator) -> String {
    let text = words.usage();
    let login = match disclosure.store {
        UsageCredentialStore::ClaudeCode => text.store_claude(),
        UsageCredentialStore::Codex => text.store_codex(),
        UsageCredentialStore::OpenCode => text.store_opencode(),
        UsageCredentialStore::GitHubCopilot => text.store_copilot(),
        UsageCredentialStore::Cursor => text.store_cursor(),
        UsageCredentialStore::Grok => text.store_grok(),
        UsageCredentialStore::Kimi => text.store_kimi(),
        UsageCredentialStore::MiniMax => text.store_minimax(),
        UsageCredentialStore::OpenCodeGo => text.store_opencode_go(),
        UsageCredentialStore::Zai => text.store_zai(),
    };
    let recipient = match disclosure.recipient {
        UsageRecipient::Anthropic => text.recipient_anthropic(),
        UsageRecipient::OpenAi => text.recipient_openai(),
        UsageRecipient::GitHub => text.recipient_github(),
        UsageRecipient::Cursor => text.recipient_cursor(),
        UsageRecipient::Xai => text.recipient_xai(),
        UsageRecipient::Kimi => text.recipient_kimi(),
        UsageRecipient::MiniMaxGlobalOrChina => text.recipient_minimax(),
        UsageRecipient::OpenCode => text.recipient_opencode(),
        UsageRecipient::Zai => text.recipient_zai(),
    };
    text.source_disclosure(login, recipient)
        .unwrap_or_else(|_| text.account_disclosure().to_owned())
}
/// Paseo balance-bar.tsx `resolveBalance`, as one line beside the label:
/// "used / allowance" when a positive allowance is reported, else "amount
/// left", else the used amount; the label names the unit, as in Paseo, and
/// each amount is written by `amount_text` (Paseo `formatAmount`). Hidden
/// values stay hidden (visibility wins over a mistakenly populated balance),
/// and an unknown used amount under an allowance shows Paseo's dash.
#[must_use]
#[expect(
    clippy::float_arithmetic,
    reason = "Allowance minus remaining is the reported used amount, shown, never accounted"
)]
pub fn balance_amount(
    balance: &UsageBalance,
    visibility: CostVisibility,
    words: &'static Translator,
) -> String {
    /// Paseo's placeholder for an amount the source did not report.
    const UNKNOWN: &str = "\u{2014}";
    let text = words.usage();
    if visibility == CostVisibility::Withheld {
        return text.withheld().to_owned();
    }
    let amount = |value: f64| super::amount_text(value, balance.unit, words);
    let limit = balance
        .limit
        .map(vessel_proto::conversation::FiniteNumber::get)
        .filter(|limit| *limit > 0.0);
    if let Some(limit) = limit {
        let used = balance
            .used
            .map(vessel_proto::conversation::FiniteNumber::get)
            .or_else(|| balance.remaining.map(|remaining| limit - remaining.get()))
            .map_or_else(|| UNKNOWN.to_owned(), amount);
        return format!("{used} / {}", amount(limit));
    }
    if let Some(remaining) = balance.remaining {
        let left = amount(remaining.get());
        return text.amount_left(&left).unwrap_or(left);
    }
    balance
        .used
        .map_or_else(|| UNKNOWN.to_owned(), |used| amount(used.get()))
}
/// Source tone wins; otherwise explicit remaining can establish used share for the threshold fallback.
#[must_use]
#[expect(
    clippy::float_arithmetic,
    reason = "The source reports a percentage; its complement is presentation, never a quota estimate"
)]
pub fn source_window_tone(window: &vessel_proto::usage_reports::UsageWindow) -> super::Tone {
    use vessel_proto::{conversation::FiniteNumber, usage_reports::UsageTone};
    let explicit = window.tone.map(|tone| match tone {
        UsageTone::Default => super::Tone::Default,
        UsageTone::Ok => super::Tone::Ok,
        UsageTone::Warning => super::Tone::Warning,
        UsageTone::Danger => super::Tone::Danger,
    });
    let used = window
        .used
        .and_then(|used| FiniteNumber::new(used.get()))
        .or_else(|| {
            window
                .remaining
                .and_then(|remaining| FiniteNumber::new(100.0 - remaining.get()))
        });
    super::window_tone(used, explicit)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::ENGLISH;
    use vessel_proto::{
        conversation::FiniteNumber,
        usage_reports::{AvailableUsageReport, UsageBalanceUnit},
    };
    #[test]
    fn retained_observation_time_is_never_replaced_by_attempt_or_query_time() {
        let original = SuccessfulUsageReport {
            at: DateTime::UNIX_EPOCH,
            report: AvailableUsageReport::default(),
        };
        let attempted = DateTime::from_timestamp(100, 0).unwrap();
        let state = SourceReportState::Failed {
            attempted_at: Some(attempted),
            reason: UsageFetchFailure::Unreachable,
            retry_at: None,
            retained: Some(original),
            logins: vessel_proto::usage_reports::ReportLoginFailures::default(),
        };
        let view = source_view(&state, Translator::get(ENGLISH).unwrap());
        assert!(view.retained);
        assert_eq!(view.observed.unwrap().at, DateTime::UNIX_EPOCH);
        assert_eq!(view.attempted_at, Some(attempted));
    }
    #[test]
    fn pending_current_success_is_not_a_retained_failure() {
        use vessel_proto::usage_reports::UsageRefreshOrigin;
        let words = Translator::get(ENGLISH).unwrap();
        for (origin, retained) in [
            (UsageRefreshOrigin::Available, false),
            (UsageRefreshOrigin::Unavailable, true),
        ] {
            let state = SourceReportState::Refreshing {
                origin,
                started_at: Some(DateTime::from_timestamp(100, 0).unwrap()),
                retained: Some(SuccessfulUsageReport {
                    at: DateTime::UNIX_EPOCH,
                    report: AvailableUsageReport::default(),
                }),
            };
            let view = source_view(&state, words);
            assert_eq!(view.retained, retained);
            assert_eq!(view.observed.unwrap().at, DateTime::UNIX_EPOCH);
            assert!(view.attempted_at.is_none());
        }
    }
    #[test]
    fn withheld_balance_cannot_leak_even_when_a_peer_populates_amounts() {
        let words = Translator::get(ENGLISH).unwrap();
        let balance = UsageBalance {
            id: "credits".parse().unwrap(),
            label: "Credits".parse().unwrap(),
            used: FiniteNumber::new(0.0),
            remaining: FiniteNumber::new(123.45),
            limit: None,
            unit: UsageBalanceUnit::Credits,
            resets_at: None,
            tone: None,
        };
        assert_eq!(
            balance_amount(&balance, CostVisibility::Withheld, words),
            words.usage().withheld()
        );
        assert_eq!(
            balance_amount(&balance, CostVisibility::Permitted, words),
            "\u{2068}123.45\u{2069} left"
        );
        let allowance = UsageBalance {
            used: None,
            remaining: FiniteNumber::new(7.5),
            limit: FiniteNumber::new(10.0),
            ..balance
        };
        assert_eq!(
            balance_amount(&allowance, CostVisibility::Permitted, words),
            "2.5 / 10"
        );
    }
    #[test]
    fn remaining_only_threshold_fallback_and_explicit_neutral_tone_stay_independent() {
        let mut window = vessel_proto::usage_reports::UsageWindow {
            id: "window".parse().unwrap(),
            label: "Window".parse().unwrap(),
            short_label: None,
            summary: vessel_proto::usage_reports::UsageSummary::default(),
            used: None,
            remaining: Some(vessel_proto::limits::LimitPercent::reported(
                FiniteNumber::new(5.0).unwrap(),
            )),
            resets_at: None,
            runs_out_at: None,
            shortfall: None,
            tone: None,
        };
        assert_eq!(source_window_tone(&window), super::super::Tone::Danger);
        window.tone = Some(vessel_proto::usage_reports::UsageTone::Default);
        assert_eq!(source_window_tone(&window), super::super::Tone::Default);
    }
}
