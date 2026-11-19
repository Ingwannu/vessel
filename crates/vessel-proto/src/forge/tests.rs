//! The forge's wire: shapes on the wire, and what an older reader makes
//! of what a newer node sends.

use chrono::{DateTime, Utc};

use super::*;
use crate::workspace::GitRefusal;

fn summary() -> PullSummary {
    PullSummary {
        repo: RepoRef {
            kind: ForgeKind::Github,
            host: "github.com".parse().unwrap(),
            path: "owner/name".parse().unwrap(),
        },
        number: PullNumber::new(12).unwrap(),
        url: "https://github.com/owner/name/pull/12".parse().unwrap(),
        title: PullTitle::lossy("Fix the tests"),
        state: PullState::Open,
        draft: false,
        head_commit: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
        checks: ChecksState::Failing,
        review: ReviewDecision::ChangesRequested,
        mergeable: Mergeable::Blocked,
        updated_at: DateTime::<Utc>::UNIX_EPOCH,
        tally: None,
    }
}

#[test]
fn a_session_s_pull_request_crosses_the_wire_and_back() {
    let carried = SessionPullRequest {
        access: ForgeAccess::Ready {
            login: ForgeLogin::lossy("someone"),
        },
        summary: Some(summary()),
        feedback: true,
    };
    let json = serde_json::to_string(&carried).unwrap();
    assert!(
        json.contains(r#""access":{"access":"ready","login":"someone"}"#),
        "{json}"
    );
    assert!(json.contains(r#""checks":"failing""#), "{json}");
    assert!(json.contains(r#""review":"changes_requested""#), "{json}");
    assert_eq!(
        serde_json::from_str::<SessionPullRequest>(&json).unwrap(),
        carried
    );
}

#[test]
fn what_a_newer_node_adds_reads_as_unknown() {
    let access: ForgeAccess = serde_json::from_str(r#"{"access":"queued_elsewhere"}"#).unwrap();
    assert_eq!(access, ForgeAccess::Unknown);
    let state: ChecksState = serde_json::from_str(r#""skipping""#).unwrap();
    assert_eq!(state, ChecksState::Unknown);
    let outcome: MergeOutcome = serde_json::from_str(r#"{"outcome":"scheduled"}"#).unwrap();
    assert_eq!(outcome, MergeOutcome::Unknown);
    let refusal: MergeRefusal = serde_json::from_str(r#"{"kind":"frozen"}"#).unwrap();
    assert_eq!(refusal, MergeRefusal::Unknown);
    let opened: OpenOutcome = serde_json::from_str(r#"{"outcome":"forked"}"#).unwrap();
    assert_eq!(opened, OpenOutcome::Unknown);
}

/// A protocol 10 reader knows `GitRefusal` without `repository_config`
/// (ADR 0030): it reads it as `unknown`, its key and reason ignored.
#[test]
fn an_older_reader_takes_a_configuration_refusal_as_unknown() {
    let refusal = GitRefusal::RepositoryConfig {
        key: "core.sshcommand".parse().ok(),
        reason: crate::workspace::RepositoryConfigReason::Program,
    };
    let json = serde_json::to_string(&refusal).unwrap();
    assert_eq!(
        json,
        r#"{"kind":"repository_config","key":"core.sshcommand","reason":"program"}"#
    );
    assert_eq!(serde_json::from_str::<GitRefusal>(&json).unwrap(), refusal);
    let older: GitRefusal =
        serde_json::from_str(r#"{"kind":"something_newer","key":"x"}"#).unwrap();
    assert_eq!(older, GitRefusal::Unknown);
}

#[test]
fn a_merge_names_its_pull_request_and_head() {
    let merge = MergePullRequest {
        session: crate::SessionId::random(),
        number: PullNumber::new(3).unwrap(),
        method: MergeMethod::Squash,
        head: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
    };
    let json = serde_json::to_string(&merge).unwrap();
    assert!(json.contains(r#""method":"squash""#), "{json}");
    assert!(json.contains(r#""number":3"#), "{json}");
    assert_eq!(
        serde_json::from_str::<MergePullRequest>(&json).unwrap(),
        merge
    );
}

#[test]
fn a_row_s_tally_is_its_checks_counts_and_optional_on_the_wire() {
    let mut checks = Checks::unknown();
    assert_eq!(CheckTally::of(&checks), None, "nothing counted");
    checks.passed = 7;
    checks.pending = 2;
    assert_eq!(
        CheckTally::of(&checks),
        Some(CheckTally {
            passed: 7,
            failed: 0,
            pending: 2,
        })
    );
    let older = serde_json::to_value(summary()).unwrap();
    assert!(older.get("tally").is_none(), "none is left out");
    let mut newer = summary();
    newer.tally = CheckTally::of(&checks);
    let sent = serde_json::to_value(&newer).unwrap();
    assert_eq!(
        sent["tally"],
        serde_json::json!({"passed": 7, "failed": 0, "pending": 2})
    );
    assert_eq!(serde_json::from_value::<PullSummary>(sent).unwrap(), newer);
    assert_eq!(
        serde_json::from_value::<PullSummary>(older).unwrap().tally,
        None,
        "an older node's summary reads without one"
    );
}
