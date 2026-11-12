//! Pull requests end to end (protocol 11, ADR 0029): a node watching a
//! placed session's branch on an in-memory forge, its facts on the
//! session's card and board, opening (an agent's never pushes) and an
//! exact merge, and feedback through the queue. No request leaves the
//! machine: the forge is the test double, and a push goes to a bare
//! repository on disk (the remote's push address), while its fetch
//! address names the forge.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use vessel_client::NodeClient;
use vessel_forge::fake::{self, FakeForge};
use vessel_forge::{FailedCheck, Read};
use vessel_node::{ForgeSetup, Node};
use vessel_proto::forge::{
    CheckName, ChecksState, ForgeAccess, ForgeKind, GetForgeStatus, GetPullRequest,
    ListPullRequests, MergeMethod, MergeOutcome, MergePullRequest, MergeRefusal, OpenOutcome,
    OpenPullRequest, PullNumber, PullTitle, RepoRef, SetFeedback,
};
use vessel_proto::workspace::{AddProject, CommitId, ProjectInfo, SessionPlace, WorktreeInfo};
use vessel_proto::{
    ErrorCode, EventSeq, GetSession, KanbanColumn, SessionId, SessionInfo, SessionStatus,
};

use crate::support::{Fixture, read_until, status_of, step, structured_spec, within};
use crate::workspace::{lookup, new_worktree, repository};

/// A node that finds git and reaches only `forge`.
pub(crate) async fn start(fixture: &Fixture, forge: &Arc<FakeForge>) -> Node {
    let forge = Arc::clone(forge);
    fixture
        .start_with(move |config| {
            config.harness_lookup = lookup(fixture);
            config.forge = ForgeSetup {
                custom: Some(forge),
                ..ForgeSetup::default()
            };
        })
        .await
}

pub(crate) fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn repo() -> RepoRef {
    RepoRef {
        kind: ForgeKind::Github,
        host: "github.com".parse().unwrap(),
        path: "owner/name".parse().unwrap(),
    }
}

/// A project whose `origin` names the forge for fetching and a bare
/// repository on disk for pushing; the folders live as long as the guard.
pub(crate) async fn project(client: &NodeClient) -> (tempfile::TempDir, ProjectInfo) {
    let folder = tempfile::tempdir().unwrap();
    let root: PathBuf = std::fs::canonicalize(folder.path()).unwrap().join("repo");
    std::fs::create_dir(&root).unwrap();
    repository(&root);
    let bare = root.with_file_name("pushed.git");
    let _said = git(
        root.parent().unwrap(),
        &["init", "-q", "--bare", "pushed.git"],
    );
    let _said = git(
        &root,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/owner/name.git",
        ],
    );
    let _said = git(
        &root,
        &[
            "remote",
            "set-url",
            "--push",
            "origin",
            bare.to_str().unwrap(),
        ],
    );
    let project = client
        .call(AddProject {
            path: root,
            name: None,
        })
        .await
        .unwrap();
    (folder, project)
}

/// A structured session placed in `worktree`, once it runs.
pub(crate) async fn running_in(
    client: &NodeClient,
    fixture: &Fixture,
    worktree: &WorktreeInfo,
) -> SessionId {
    let mut spec = structured_spec(fixture, None).with_place(SessionPlace::Worktree {
        worktree: worktree.id,
    });
    spec.cwd = None;
    let id = client.create_session(spec).await.unwrap().id;
    let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
    read_until(&mut stream, &mut Vec::new(), |record| {
        status_of(record) == Some(&SessionStatus::Running)
    })
    .await;
    id
}

/// The session's info once `wanted` holds of it.
pub(crate) async fn info_until(
    client: &NodeClient,
    session: SessionId,
    wanted: impl Fn(&SessionInfo) -> bool,
) -> SessionInfo {
    step("the session's info", async {
        loop {
            let info = client.call(GetSession { session }).await.unwrap();
            if wanted(&info) {
                return info;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
}

/// A pull request of `branch` at `head`, its checks failing when `failing`.
pub(crate) fn pull(number: u64, worktree: &WorktreeInfo, head: &CommitId, failing: bool) -> Read {
    let branch = worktree.branch.clone().unwrap();
    let mut read = fake::pull(&repo(), PullNumber::new(number).unwrap(), &branch, head).unwrap();
    read.pull.merge_methods = vec![MergeMethod::Squash];
    if failing {
        read.pull.summary.checks = ChecksState::Failing;
        read.discussion.failed = vec![FailedCheck {
            name: CheckName::lossy("tests"),
            url: None,
        }];
    }
    read
}

/// A watched session's pull request reaches its card and its board; a
/// refresh reads it now; the list and the status name it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_watched_session_shows_its_pull_request() {
    within(async {
        let forge = FakeForge::new();
        let fixture = Fixture::new();
        let node = start(&fixture, &forge).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;
        let worktree = new_worktree(&owner, project.id).await;
        let head: CommitId = git(&worktree.path, &["rev-parse", "HEAD"]).parse().unwrap();
        let session = running_in(&owner, &fixture, &worktree).await;

        // Watched, none yet: awaiting one between turns.
        let waiting = info_until(&owner, session, |info| info.pull_request.is_some()).await;
        let access = &waiting.pull_request.as_ref().unwrap().access;
        assert!(matches!(access, ForgeAccess::Ready { .. }), "{access:?}");

        forge.put(
            worktree.branch.clone().unwrap(),
            pull(7, &worktree, &head, false),
        );
        let view = owner
            .call(GetPullRequest {
                session,
                refresh: true,
            })
            .await
            .unwrap();
        let pull_request = view.pull_request.unwrap();
        assert_eq!(pull_request.summary.number.get(), 7);
        let shown = info_until(&owner, session, |info| {
            info.pull_request
                .as_ref()
                .is_some_and(|pull| pull.summary.is_some())
        })
        .await;
        assert_eq!(
            shown.board.map(|board| board.column),
            Some(KanbanColumn::NeedsReview)
        );
        let list = owner.call(ListPullRequests {}).await.unwrap();
        assert_eq!(list.entries.len(), 1);
        let status = owner.call(GetForgeStatus {}).await.unwrap();
        assert_eq!(status.watched, 1);
        assert!(
            status
                .hosts
                .iter()
                .any(|host| host.host.as_str() == "github.com")
        );
    })
    .await;
}

/// A merge names its pull request and head exactly; a moved head is
/// refused and nothing is merged; the right one merges once.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_merge_is_exact() {
    within(async {
        let forge = FakeForge::new();
        let fixture = Fixture::new();
        let node = start(&fixture, &forge).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;
        let worktree = new_worktree(&owner, project.id).await;
        let head: CommitId = git(&worktree.path, &["rev-parse", "HEAD"]).parse().unwrap();
        let session = running_in(&owner, &fixture, &worktree).await;
        forge.put(
            worktree.branch.clone().unwrap(),
            pull(9, &worktree, &head, false),
        );
        let merge = |number: u64, head: CommitId| MergePullRequest {
            session,
            number: PullNumber::new(number).unwrap(),
            method: MergeMethod::Squash,
            head,
        };

        let short = owner
            .call(merge(9, head.as_str().get(..7).unwrap().parse().unwrap()))
            .await
            .unwrap_err();
        assert_eq!(short.code(), Some(ErrorCode::InvalidRequest));
        let other: CommitId = "f".repeat(40).parse().unwrap();
        assert_eq!(
            owner.call(merge(9, other)).await.unwrap(),
            MergeOutcome::Refused {
                refusal: MergeRefusal::HeadMoved { head: head.clone() }
            }
        );
        assert!(matches!(
            owner.call(merge(3, head.clone())).await.unwrap(),
            MergeOutcome::Refused {
                refusal: MergeRefusal::OtherPullRequest { .. }
            }
        ));
        assert!(forge.merges().is_empty());
        assert!(matches!(
            owner.call(merge(9, head.clone())).await.unwrap(),
            MergeOutcome::Merged { .. }
        ));
        assert_eq!(forge.merges().len(), 1);
    })
    .await;
}

/// A person's open pushes the branch (to the push address, on disk) and
/// makes one; a second finds it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_person_s_open_pushes_and_opens_once() {
    within(async {
        let forge = FakeForge::new();
        let fixture = Fixture::new();
        let node = start(&fixture, &forge).await;
        let owner = fixture.connect(&node).await;
        let (folder, project) = project(&owner).await;
        let worktree = new_worktree(&owner, project.id).await;
        let head: CommitId = git(&worktree.path, &["rev-parse", "HEAD"]).parse().unwrap();
        // What the forge has the branch at once the node pushed it.
        forge.pushed(worktree.branch.clone().unwrap(), head.clone());
        let session = running_in(&owner, &fixture, &worktree).await;
        let open = || OpenPullRequest {
            session,
            title: PullTitle::lossy("A change"),
            body: None,
            base: None,
            draft: false,
        };
        let opened = owner.call(open()).await.unwrap();
        let OpenOutcome::Opened { pull_request } = opened else {
            panic!("not opened: {opened:?}");
        };
        assert_eq!(pull_request.base.as_str(), "main");
        let branch = worktree.branch.clone().unwrap();
        let pushed = git(
            &folder.path().join("pushed.git"),
            &["rev-parse", &format!("refs/heads/{branch}")],
        );
        assert_eq!(pushed, head.as_str(), "the push reached the push address");
        assert_eq!(pull_request.summary.head_commit, head);
        assert!(matches!(
            owner.call(open()).await.unwrap(),
            OpenOutcome::Exists { .. }
        ));
        assert_eq!(forge.calls("open"), 1);
    })
    .await;
}

/// Feedback is off until a person turns it on; then the failing check is
/// sent to the session once, as a prompt the node marks as its own.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn feedback_sends_a_failing_check_once() {
    within(async {
        let forge = FakeForge::new();
        let fixture = Fixture::new();
        let node = start(&fixture, &forge).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;
        let worktree = new_worktree(&owner, project.id).await;
        let head: CommitId = git(&worktree.path, &["rev-parse", "HEAD"]).parse().unwrap();
        let session = running_in(&owner, &fixture, &worktree).await;
        forge.put(
            worktree.branch.clone().unwrap(),
            pull(4, &worktree, &head, true),
        );
        let _view = owner
            .call(GetPullRequest {
                session,
                refresh: true,
            })
            .await
            .unwrap();
        let switched = owner
            .call(SetFeedback {
                session,
                enabled: true,
            })
            .await
            .unwrap();
        assert!(switched.feedback);
        let mut stream = owner.follow(session, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            format!("{:?}", record.event).contains("Vessel: pull request feedback")
        })
        .await;
        let _again = owner
            .call(GetPullRequest {
                session,
                refresh: true,
            })
            .await
            .unwrap();
        let info = owner.call(GetSession { session }).await.unwrap();
        assert!(info.pull_request.unwrap().feedback);
    })
    .await;
}
