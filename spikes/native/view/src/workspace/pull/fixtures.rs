//! A pull request as the node sends one, for the tests.

use chrono::{TimeZone as _, Utc};
use vessel_proto::forge::{
    Checks, ChecksState, ForgeKind, MergeMethod, Mergeable, PullNumber, PullRequest, PullState,
    PullSummary, PullTitle, RepoRef, ReviewDecision, Reviews,
};

/// The head every fixture's pull request is at.
pub(crate) const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";

/// Number 42 whole, open and mergeable, allowing squash and merge.
pub(crate) fn whole() -> PullRequest {
    let at = Utc.with_ymd_and_hms(2026, 11, 14, 9, 0, 0).unwrap();
    PullRequest {
        summary: summary(PullState::Open, false),
        author: None,
        head_branch: "feature".parse().unwrap(),
        base: "main".parse().unwrap(),
        checks: Checks {
            state: ChecksState::Passing,
            passed: 3,
            failed: 0,
            pending: 0,
            truncated: false,
            failing: vec![],
        },
        reviews: Reviews {
            approved: true,
            changes_requested: false,
            commented: false,
            unresolved: 0,
        },
        created_at: at,
        merged_at: None,
        closed_at: None,
        merge_methods: vec![MergeMethod::Merge, MergeMethod::Squash],
        read_at: at,
    }
}

/// Number 42 of `acme/widgets`, in `state`.
pub(crate) fn summary(state: PullState, draft: bool) -> PullSummary {
    PullSummary {
        repo: RepoRef {
            kind: ForgeKind::Github,
            host: "github.com".parse().unwrap(),
            path: "acme/widgets".parse().unwrap(),
        },
        number: PullNumber::new(42).unwrap(),
        url: "https://github.com/acme/widgets/pull/42".parse().unwrap(),
        title: PullTitle::lossy("Add the widget"),
        state,
        draft,
        head_commit: HEAD.parse().unwrap(),
        checks: ChecksState::Passing,
        review: ReviewDecision::None,
        mergeable: Mergeable::Mergeable,
        updated_at: Utc.with_ymd_and_hms(2026, 11, 14, 9, 0, 0).unwrap(),
    }
}
