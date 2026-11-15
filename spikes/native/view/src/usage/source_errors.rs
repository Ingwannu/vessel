//! Login failures are closed categories and public labels, never source diagnostics.
use crate::{agent_name, localization::Translator};
use vessel_proto::usage_reports::{SourceReportState, UsageAttemptFailure};

/// A current attempt's individual login failures take precedence over its summary.
#[must_use]
pub fn source_login_errors(state: &SourceReportState, words: &'static Translator) -> Vec<String> {
    let failures = match state {
        SourceReportState::Unavailable { logins, .. }
        | SourceReportState::Failed { logins, .. } => logins.as_slice(),
        SourceReportState::NotRequested
        | SourceReportState::Refreshing { .. }
        | SourceReportState::Available { .. } => return Vec::new(),
    };
    failures
        .iter()
        .map(|login| {
            let label = login
                .harness
                .as_ref()
                .map_or(login.label.as_str(), agent_name);
            let reason = match &login.failure {
                UsageAttemptFailure::Unavailable { problem } => {
                    super::source_problem(problem, words)
                }
                UsageAttemptFailure::Error { reason } => super::source_failure(*reason, words),
            };
            format!("{label}: {reason}")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::{ENGLISH, Locale};
    use vessel_proto::{
        HarnessKind,
        usage_reports::{NoQuotaReason, UsageFetchFailure, UsageLoginFailure, UsageProblem},
    };
    #[test]
    fn current_login_errors_preserve_each_identity_and_translate_closed_failures() {
        for locale in [ENGLISH, Locale::from_tag("ko").unwrap()] {
            let words = Translator::get(locale).unwrap();
            let problem = UsageProblem::NoQuota {
                reason: NoQuotaReason::NoActivePlan,
            };
            let logins = vec![
                UsageLoginFailure {
                    harness: Some(HarnessKind::Codex.into()),
                    label: "unused display label".parse().unwrap(),
                    failure: UsageAttemptFailure::Error {
                        reason: UsageFetchFailure::Unreachable,
                    },
                },
                UsageLoginFailure {
                    harness: None,
                    label: "Second account".parse().unwrap(),
                    failure: UsageAttemptFailure::Unavailable {
                        problem: problem.clone(),
                    },
                },
            ];
            let state = SourceReportState::Unavailable {
                attempted_at: None,
                problem,
                retry_at: None,
                retained: None,
                logins: logins.try_into().unwrap(),
            };
            assert_eq!(
                source_login_errors(&state, words),
                vec![
                    format!("Codex: {}", words.usage().source_unreachable()),
                    format!("Second account: {}", words.usage().no_active_plan()),
                ]
            );
            assert!(source_login_errors(&SourceReportState::NotRequested, words).is_empty());
        }
    }
}
