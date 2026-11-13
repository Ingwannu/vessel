//! GitHub's values into the wire's facts: agent-orchestrator's mapping
//! (`provider.go`, `observer_provider.go`), taken whole, since the board's
//! reducer is its port (ADR 0019). Pure functions, tested by their table.

use std::collections::HashMap;

use vessel_proto::forge::{CheckRun, CheckState, Checks, ChecksState, Mergeable, ReviewDecision};

/// One check as GitHub's rollup lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Context {
    /// Its name (a check run's `name`, a status's `context`).
    pub(crate) name: String,
    /// Its result.
    pub(crate) state: CheckState,
    /// When it started (for the newest run of each name).
    pub(crate) at: Option<String>,
    /// Its page.
    pub(crate) url: Option<String>,
}

/// A check run's `status` and `conclusion`.
pub(crate) fn check_run(status: &str, conclusion: Option<&str>) -> CheckState {
    if status != "COMPLETED" {
        return CheckState::Pending;
    }
    match conclusion.unwrap_or_default() {
        "SUCCESS" | "NEUTRAL" => CheckState::Passed,
        "FAILURE" | "TIMED_OUT" | "ACTION_REQUIRED" | "STARTUP_FAILURE" => CheckState::Failed,
        "CANCELLED" => CheckState::Cancelled,
        "SKIPPED" | "STALE" => CheckState::Skipped,
        _ => CheckState::Unknown,
    }
}

/// A status context's `state`.
pub(crate) fn status_context(state: &str) -> CheckState {
    match state {
        "SUCCESS" => CheckState::Passed,
        "FAILURE" | "ERROR" => CheckState::Failed,
        "PENDING" | "EXPECTED" => CheckState::Pending,
        _ => CheckState::Unknown,
    }
}

/// The head's checks, from its rollup's contexts: only the newest run of
/// each name counts (a superseded run does not fail the head), then any
/// failed is failing, else any pending pending, else a cancelled one
/// `cancelled`, else none `none`, else passing.
pub(crate) fn checks(contexts: &[Context], truncated: bool) -> Checks {
    let mut newest: HashMap<&str, &Context> = HashMap::new();
    for context in contexts {
        let keep = newest
            .get(context.name.as_str())
            .is_none_or(|kept| context.at > kept.at);
        if keep {
            let _replaced = newest.insert(&context.name, context);
        }
    }
    let mut names: Vec<&&Context> = newest.values().collect();
    names.sort_by(|a, b| a.name.cmp(&b.name));
    let count = |wanted: &[CheckState]| {
        u32::try_from(
            names
                .iter()
                .filter(|context| wanted.contains(&context.state))
                .count(),
        )
        .unwrap_or(u32::MAX)
    };
    let failed = count(&[CheckState::Failed]);
    let pending = count(&[CheckState::Pending]);
    let passed = count(&[CheckState::Passed, CheckState::Skipped]);
    let cancelled = count(&[CheckState::Cancelled]);
    let state = if truncated {
        ChecksState::Unknown
    } else if failed > 0 {
        ChecksState::Failing
    } else if pending > 0 {
        ChecksState::Pending
    } else if cancelled > 0 {
        ChecksState::Cancelled
    } else if names.is_empty() {
        ChecksState::None
    } else {
        ChecksState::Passing
    };
    let failing = names
        .iter()
        .filter(|context| context.state == CheckState::Failed)
        .take(Checks::MAX_FAILING)
        .map(|context| CheckRun {
            name: vessel_proto::forge::CheckName::lossy(&context.name),
            state: context.state,
            url: context
                .url
                .as_deref()
                .and_then(vessel_proto::forge::ForgeUrl::lossy),
        })
        .collect();
    Checks {
        state,
        passed,
        failed,
        pending,
        truncated,
        failing,
    }
}

/// `reviewDecision`.
pub(crate) fn review(decision: Option<&str>) -> ReviewDecision {
    match decision {
        Some("APPROVED") => ReviewDecision::Approved,
        Some("CHANGES_REQUESTED") => ReviewDecision::ChangesRequested,
        Some("REVIEW_REQUIRED") => ReviewDecision::ReviewRequired,
        None => ReviewDecision::None,
        Some(_) => ReviewDecision::Unknown,
    }
}

/// What decides mergeability, as GitHub reports it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MergeFacts<'a> {
    pub(crate) mergeable: Option<&'a str>,
    pub(crate) merge_state: Option<&'a str>,
    pub(crate) draft: bool,
    pub(crate) checks: ChecksState,
    pub(crate) review: ReviewDecision,
}

/// Mergeability, the first rule that holds (agent-orchestrator's table):
/// a conflict; blocked; unstable (before the draft, checks and review
/// rules); a draft, failing checks, changes requested or a review required
/// blocks; mergeable and clean (or with hooks, or nothing said) merges;
/// else unknown, and behind alone is `behind`.
pub(crate) fn mergeable(facts: MergeFacts<'_>) -> Mergeable {
    let state = facts.merge_state.unwrap_or_default();
    if state == "DIRTY" || facts.mergeable == Some("CONFLICTING") {
        return Mergeable::Conflicting;
    }
    if state == "BLOCKED" {
        return Mergeable::Blocked;
    }
    if state == "UNSTABLE" {
        return Mergeable::Unstable;
    }
    let held = facts.draft
        || facts.checks == ChecksState::Failing
        || matches!(
            facts.review,
            ReviewDecision::ChangesRequested | ReviewDecision::ReviewRequired
        );
    if held {
        return Mergeable::Blocked;
    }
    if facts.mergeable == Some("MERGEABLE") && matches!(state, "CLEAN" | "HAS_HOOKS" | "") {
        return Mergeable::Mergeable;
    }
    if state == "BEHIND" {
        return Mergeable::Behind;
    }
    Mergeable::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(name: &str, state: CheckState, at: &str) -> Context {
        Context {
            name: name.to_owned(),
            state,
            at: Some(at.to_owned()),
            url: None,
        }
    }

    #[test]
    fn check_results_follow_agent_orchestrator_s_table() {
        for (status, conclusion, state) in [
            ("COMPLETED", Some("SUCCESS"), CheckState::Passed),
            ("COMPLETED", Some("NEUTRAL"), CheckState::Passed),
            ("COMPLETED", Some("FAILURE"), CheckState::Failed),
            ("COMPLETED", Some("TIMED_OUT"), CheckState::Failed),
            ("COMPLETED", Some("ACTION_REQUIRED"), CheckState::Failed),
            ("COMPLETED", Some("STARTUP_FAILURE"), CheckState::Failed),
            ("COMPLETED", Some("CANCELLED"), CheckState::Cancelled),
            ("COMPLETED", Some("SKIPPED"), CheckState::Skipped),
            ("COMPLETED", Some("STALE"), CheckState::Skipped),
            ("QUEUED", None, CheckState::Pending),
            ("IN_PROGRESS", None, CheckState::Pending),
            ("WAITING", None, CheckState::Pending),
        ] {
            assert_eq!(
                check_run(status, conclusion),
                state,
                "{status} {conclusion:?}"
            );
        }
        assert_eq!(status_context("ERROR"), CheckState::Failed);
        assert_eq!(status_context("EXPECTED"), CheckState::Pending);
    }

    #[test]
    fn a_superseded_run_does_not_fail_the_head() {
        let checks = checks(
            &[
                context("test", CheckState::Failed, "2026-01-01T00:00:00Z"),
                context("test", CheckState::Passed, "2026-01-01T00:10:00Z"),
                context("lint", CheckState::Passed, "2026-01-01T00:00:00Z"),
            ],
            false,
        );
        assert_eq!(checks.state, ChecksState::Passing);
        assert_eq!((checks.passed, checks.failed), (2, 0));
    }

    #[test]
    fn checks_together_rank_failed_pending_cancelled() {
        let at = "2026-01-01T00:00:00Z";
        let state = |contexts: &[Context]| checks(contexts, false).state;
        assert_eq!(state(&[]), ChecksState::None);
        assert_eq!(
            state(&[
                context("a", CheckState::Pending, at),
                context("b", CheckState::Failed, at)
            ]),
            ChecksState::Failing
        );
        assert_eq!(
            state(&[
                context("a", CheckState::Pending, at),
                context("b", CheckState::Cancelled, at)
            ]),
            ChecksState::Pending
        );
        assert_eq!(
            state(&[
                context("a", CheckState::Passed, at),
                context("b", CheckState::Cancelled, at)
            ]),
            ChecksState::Cancelled
        );
        assert_eq!(
            checks(&[context("a", CheckState::Passed, at)], true).state,
            ChecksState::Unknown,
            "more than one read takes"
        );
    }

    #[test]
    fn mergeability_takes_the_first_rule_that_holds() {
        let facts = |mergeable, merge_state, draft, checks, review| MergeFacts {
            mergeable,
            merge_state,
            draft,
            checks,
            review,
        };
        let open = ChecksState::Passing;
        let none = ReviewDecision::None;
        for (case, expected) in [
            (
                facts(Some("CONFLICTING"), Some("BLOCKED"), false, open, none),
                Mergeable::Conflicting,
            ),
            (
                facts(Some("MERGEABLE"), Some("DIRTY"), false, open, none),
                Mergeable::Conflicting,
            ),
            (
                facts(Some("MERGEABLE"), Some("BLOCKED"), false, open, none),
                Mergeable::Blocked,
            ),
            (
                facts(
                    Some("MERGEABLE"),
                    Some("UNSTABLE"),
                    true,
                    ChecksState::Failing,
                    none,
                ),
                Mergeable::Unstable,
            ),
            (
                facts(Some("MERGEABLE"), Some("CLEAN"), true, open, none),
                Mergeable::Blocked,
            ),
            (
                facts(
                    Some("MERGEABLE"),
                    Some("CLEAN"),
                    false,
                    open,
                    ReviewDecision::ReviewRequired,
                ),
                Mergeable::Blocked,
            ),
            (
                facts(Some("MERGEABLE"), Some("CLEAN"), false, open, none),
                Mergeable::Mergeable,
            ),
            (
                facts(Some("MERGEABLE"), Some("HAS_HOOKS"), false, open, none),
                Mergeable::Mergeable,
            ),
            (
                facts(Some("MERGEABLE"), Some("BEHIND"), false, open, none),
                Mergeable::Behind,
            ),
            (
                facts(Some("UNKNOWN"), Some("UNKNOWN"), false, open, none),
                Mergeable::Unknown,
            ),
        ] {
            assert_eq!(mergeable(case), expected, "{case:?}");
        }
    }
}
