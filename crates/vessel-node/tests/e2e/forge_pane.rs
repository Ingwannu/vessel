//! The pull request pane's reads end to end (ADR 0088): `pr.timeline` and
//! `pr.check` for the pull request the watcher holds for a session; a
//! check its last read did not name is not read; an agent reads only its
//! own session's (and its subagents').

use std::sync::Arc;

use chrono::Utc;
use vessel_forge::Timeline;
use vessel_forge::fake::FakeForge;
use vessel_node::ForgeSetup;
use vessel_proto::ErrorCode;
use vessel_proto::forge::{
    CheckName, CheckReport, CheckRun, CheckState, ForgeAccess, GetCheckDetails, GetPullRequest,
    GetPullTimeline, PullBody, ReviewState, RunId, TimelineItem,
};
use vessel_proto::workspace::CommitId;

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::forge::{git, project, pull, running_in, start};
use crate::support::{Fixture, within};
use crate::workspace::{lookup, new_worktree};

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_reads_no_other_session_s_pull_request() {
    within(async {
        let forge = FakeForge::new();
        let fixture = Fixture::new();
        // The forge's node, with the `vessel` server its agents are given.
        let shared = Arc::clone(&forge);
        let node = fixture
            .start_with(|config| {
                config.harness_lookup = lookup(&fixture);
                config.forge = ForgeSetup {
                    custom: Some(shared),
                    ..ForgeSetup::default()
                };
                config.vessel_mcp = Some(vessel_mcp(&fixture));
            })
            .await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;
        let worktree = new_worktree(&owner, project.id).await;
        let theirs = running_in(&owner, &fixture, &worktree).await;
        let mut own = structured(&owner, &fixture).await;
        let (_program, socket, auth) = credential(&owner, &mut own).await;
        let agent = as_agent(&socket, &auth).await;
        // The shared owner check (`check_owner`) refuses both reads.
        assert_eq!(
            code(agent.call(GetPullTimeline { session: theirs }).await),
            ErrorCode::Forbidden
        );
        assert_eq!(
            code(
                agent
                    .call(GetCheckDetails {
                        session: theirs,
                        run: RunId::new(1).unwrap(),
                    })
                    .await
            ),
            ErrorCode::Forbidden
        );
    })
    .await;
}
