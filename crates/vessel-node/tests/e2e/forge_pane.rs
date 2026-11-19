//! The pull request pane's reads end to end (ADR 0088): `pr.timeline` and
//! `pr.check` for the pull request the watcher holds for a session; a
//! check its last read did not name is not read.

use chrono::Utc;
use vessel_forge::Timeline;
use vessel_forge::fake::FakeForge;
use vessel_proto::forge::{
    CheckName, CheckReport, CheckRun, CheckState, ForgeAccess, GetCheckDetails, GetPullRequest,
    GetPullTimeline, PullBody, ReviewState, RunId, TimelineItem,
};
use vessel_proto::workspace::CommitId;

use crate::forge::{git, project, pull, running_in, start};
use crate::support::{Fixture, within};
use crate::workspace::new_worktree;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_pane_reads_a_session_s_pull_request_activity_and_checks() {
    within(async {
        let forge = FakeForge::new();
        let fixture = Fixture::new();
        let node = start(&fixture, &forge).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;
        let worktree = new_worktree(&owner, project.id).await;
        let head: CommitId = git(&worktree.path, &["rev-parse", "HEAD"]).parse().unwrap();
        let session = running_in(&owner, &fixture, &worktree).await;
        let mut read = pull(7, &worktree, &head, false);
        let run = RunId::new(11).unwrap();
        let mut check = CheckRun::named(CheckName::lossy("tests"), CheckState::Failed, None);
        check.run = Some(run);
        read.pull.checks.runs = vec![check];
        let repo = read.pull.summary.repo.clone();
        let number = read.pull.summary.number;
        forge.put(worktree.branch.clone().unwrap(), read);
        forge.put_timeline(
            repo.clone(),
            number,
            Timeline {
                items: vec![TimelineItem::Review {
                    id: "PRR_1".parse().unwrap(),
                    author: None,
                    body: PullBody::lossy("Fix it"),
                    created_at: Utc::now(),
                    url: None,
                    state: ReviewState::ChangesRequested,
                }],
                truncated: false,
            },
        );
        forge.put_check(
            repo,
            CheckReport {
                run,
                name: CheckName::lossy("tests"),
                state: CheckState::Failed,
                url: None,
                details_url: None,
                output: None,
                annotations: Vec::new(),
                failed_jobs: Vec::new(),
                truncated: false,
            },
        );
        // The watcher reads the pull request now.
        let view = owner
            .call(GetPullRequest {
                session,
                refresh: true,
            })
            .await
            .unwrap();
        assert!(view.pull_request.is_some(), "{view:?}");
        let timeline = owner.call(GetPullTimeline { session }).await.unwrap();
        assert!(
            matches!(timeline.access, ForgeAccess::Ready { .. }),
            "{timeline:?}"
        );
        assert_eq!(timeline.number, Some(number));
        assert_eq!(timeline.items.len(), 1);
        let details = owner.call(GetCheckDetails { session, run }).await.unwrap();
        assert_eq!(details.report.map(|report| report.run), Some(run));
        let unknown = owner
            .call(GetCheckDetails {
                session,
                run: RunId::new(12).unwrap(),
            })
            .await
            .unwrap();
        assert!(
            unknown.report.is_none(),
            "a check the last read did not name is not read"
        );
    })
    .await;
}
