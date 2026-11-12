//! A live session given its pull request by the node (protocol 11, ADR
//! 0029): the summary reaches its info, the facts its board, and both are
//! gone once it ends.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use vessel_proto::forge::{
    ChecksState, ForgeAccess, ForgeKind, Mergeable, PullNumber, PullState, PullSummary, PullTitle,
    RepoRef, ReviewDecision, SessionPullRequest,
};
use vessel_proto::{DisplayStatus, KanbanColumn, SessionEvent, SessionInfo};
use vessel_session::ForgeView;
use vessel_session::board::{CiState, Policy, PullRequestFacts, PullRequestUrl};

use fixtures::{failing, info_until};
use support::{Fixture, Log, structured, within};

#[cfg(test)]
#[path = "support/runtime.rs"]
#[expect(
    dead_code,
    reason = "the shared runtime fixture, not every helper of it"
)]
mod support;

/// Builders: a `cfg(test)` module so the lints' test allowances apply.
#[cfg(test)]
mod fixtures {
    use super::*;

    pub(crate) fn failing() -> ForgeView {
        let at = DateTime::<Utc>::UNIX_EPOCH;
        let url = "https://github.com/owner/name/pull/7";
        ForgeView {
            info: SessionPullRequest {
                access: ForgeAccess::Ready { login: None },
                summary: Some(PullSummary {
                    repo: RepoRef {
                        kind: ForgeKind::Github,
                        host: "github.com".parse().unwrap(),
                        path: "owner/name".parse().unwrap(),
                    },
                    number: PullNumber::new(7).unwrap(),
                    url: url.parse().unwrap(),
                    title: PullTitle::lossy("Fix it"),
                    state: PullState::Open,
                    draft: false,
                    head_commit: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
                    checks: ChecksState::Failing,
                    review: ReviewDecision::None,
                    mergeable: Mergeable::Unstable,
                    updated_at: at,
                }),
                feedback: false,
            },
            seen: Some(Arc::from([PullRequestFacts {
                ci: CiState::Failing,
                ..PullRequestFacts::new(PullRequestUrl::new(url), at)
            }])),
            policy: Policy::default(),
        }
    }

    /// Wait until the info satisfies `wanted`.
    pub(crate) async fn info_until(
        watch: &mut tokio::sync::watch::Receiver<SessionInfo>,
        wanted: impl Fn(&SessionInfo) -> bool,
    ) -> SessionInfo {
        watch.wait_for(|info| wanted(info)).await.unwrap().clone()
    }
}

#[tokio::test]
async fn a_pull_request_reaches_the_info_and_the_board_until_the_end() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        log.until(|event| matches!(event, SessionEvent::Agent { .. }))
            .await;
        let mut info = handle.watch_info();
        let _idle = info_until(&mut info, |info| {
            info.board.as_ref().map(|board| board.display_status) == Some(DisplayStatus::Idle)
        })
        .await;

        handle.set_pull_request(Some(failing()));
        let placed = info_until(&mut info, |info| info.pull_request.is_some()).await;
        assert_eq!(placed.pull_request.as_deref(), Some(&failing().info));
        let board = placed.board.unwrap();
        assert_eq!(
            (board.column, board.display_status),
            (KanbanColumn::NeedsReview, DisplayStatus::CiFailing)
        );
        // Nothing of a forge is a record.
        assert!(log.records.iter().all(|record| !matches!(
            record.event,
            SessionEvent::Renamed { .. } | SessionEvent::ModelCall(_)
        )));

        handle.set_pull_request(None);
        let dropped = info_until(&mut info, |info| info.pull_request.is_none()).await;
        assert_eq!(
            dropped.board.map(|board| board.column),
            Some(KanbanColumn::Building)
        );

        handle.set_pull_request(Some(failing()));
        let _again = info_until(&mut info, |info| info.pull_request.is_some()).await;
        handle.kill();
        let end = handle.ended().await;
        assert!(end.pull_request.is_none(), "an ended session has none");
        assert_eq!(
            end.board.map(|board| board.column),
            Some(KanbanColumn::Building)
        );
        // Given after the end, it is ignored.
        handle.set_pull_request(Some(failing()));
        assert!(handle.info().pull_request.is_none());
    }))
    .await;
}
