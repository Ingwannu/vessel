//! A pull request's summary as the node sends one, for the tests.

use chrono::{TimeZone as _, Utc};
use vessel_proto::forge::{
    ChecksState, ForgeKind, Mergeable, PullNumber, PullState, PullSummary, PullTitle, RepoRef,
    ReviewDecision,
};

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
        head_commit: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
        checks: ChecksState::Passing,
        review: ReviewDecision::None,
        mergeable: Mergeable::Mergeable,
        updated_at: Utc.with_ymd_and_hms(2026, 11, 14, 9, 0, 0).unwrap(),
    }
}
