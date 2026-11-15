//! Where protocol 11's two node tracks meet (wave 2's first join): a
//! session with a pull request watched (track B2, ADR 0029) and a service
//! and a shell beside it (track C, ADR 0031), and what ends when the
//! session is archived or deleted, or its worktree removed (the race of
//! a terminal's start with a removal is `removal_race`).
//!
//! Unix only, as the terminals' tests are: the shell is `/bin/sh`.

use std::sync::Arc;

use vessel_client::NodeClient;
use vessel_forge::fake::FakeForge;
use vessel_node::{ForgeSetup, Node, Shell};
use vessel_proto::conversation::{Mark, MarkSession};
use vessel_proto::forge::{GetForgeStatus, GetPullRequest, ListPullRequests};
use vessel_proto::scripts::{RunScript, ScriptPlace, ScriptRunOutcome};
use vessel_proto::terminals::{
    CreateTerminal, GetTerminal, ServiceState, TerminalInfo, TerminalPlace,
};
use vessel_proto::workspace::{
    ApproveCommands, CommitId, GetProjectCommands, IfLost, KeptReason, ListProjects,
    ProjectCommands, RemoveWorktree, RemoveWorktreeOutcome, WorktreeInfo,
};
use vessel_proto::{ErrorCode, ListSessions, ProjectId, SessionId, WorktreeId};

use crate::agents::code;
use crate::forge::{git, info_until, project, pull, running_in};
use crate::support::{Fixture, step, within};
use crate::workspace::{lookup, new_worktree};

/// A node that finds git, reaches only `forge`, and opens `/bin/sh`.
pub(crate) async fn start(fixture: &Fixture, forge: &Arc<FakeForge>) -> Node {
    let forge = Arc::clone(forge);
    fixture
        .start_with(move |config| {
            config.harness_lookup = lookup(fixture);
            config.forge = ForgeSetup {
                custom: Some(forge),
                ..ForgeSetup::default()
            };
            config.terminals.shell = Some(Shell::plain("/bin/sh"));
        })
        .await
}

/// The project's file, committed (so a worktree has it) and approved.
pub(crate) async fn with_a_service(owner: &NodeClient, root: &std::path::Path, project: ProjectId) {
    std::fs::write(
        root.join("vessel.toml"),
        "[services.web]\ncommand = 'echo web-up; exec sleep 300'\n",
    )
    .unwrap();
    let _said = git(root, &["add", "vessel.toml"]);
    let _said = git(root, &["commit", "-q", "-m", "a service"]);
    let ProjectCommands::Listed { digest, .. } = owner
        .call(GetProjectCommands {
            project,
            base: None,
        })
        .await
        .unwrap()
    else {
        panic!("the file names a service");
    };
    let _approved = owner
        .call(ApproveCommands {
            project,
            digest,
            approved: true,
        })
        .await
        .unwrap();
}

/// A watched session in a new worktree whose pull request the node has
/// read, with the service running beside it and a shell.
struct Watched {
    worktree: WorktreeInfo,
    session: SessionId,
    service: TerminalInfo,
    shell: TerminalInfo,
}

async fn watched(owner: &NodeClient, fixture: &Fixture, forge: &FakeForge) -> Watched {
    let worktree = new_worktree(owner, project_of(owner).await).await;
    let head: CommitId = git(&worktree.path, &["rev-parse", "HEAD"]).parse().unwrap();
    let session = running_in(owner, fixture, &worktree).await;
    let _first = info_until(owner, session, |info| info.pull_request.is_some()).await;
    forge.put(
        worktree.branch.clone().unwrap(),
        pull(7, &worktree, &head, false),
    );
    let _read = owner
        .call(GetPullRequest {
            session,
            refresh: true,
        })
        .await
        .unwrap();
    let _shown = info_until(owner, session, |info| {
        info.pull_request
            .as_ref()
            .is_some_and(|pull| pull.summary.is_some())
    })
    .await;
    let service = match owner
        .call(RunScript {
            place: ScriptPlace::Session { session },
            script: "web".parse().unwrap(),
        })
        .await
        .unwrap()
    {
        ScriptRunOutcome::Started { terminal } => *terminal,
        other @ (ScriptRunOutcome::NeedsApproval { .. } | ScriptRunOutcome::Refused { .. }) => {
            panic!("the service did not start: {other:?}")
        }
    };
    assert_eq!(service.folder, worktree.path);
    let shell = owner
        .call(CreateTerminal {
            place: TerminalPlace::Session { session },
            size: None,
            title: None,
        })
        .await
        .unwrap();
    Watched {
        worktree,
        session,
        service,
        shell,
    }
}

/// The one project the node has.
async fn project_of(owner: &NodeClient) -> ProjectId {
    let listed = owner.call(ListProjects {}).await.unwrap();
    listed.projects.first().map(|project| project.id).unwrap()
}

pub(crate) async fn terminal(owner: &NodeClient, info: &TerminalInfo) -> TerminalInfo {
    owner
        .call(GetTerminal {
            terminal: info.id,
            changed_from: None,
        })
        .await
        .unwrap()
}

async fn remove(owner: &NodeClient, worktree: WorktreeId) -> RemoveWorktreeOutcome {
    owner
        .call(RemoveWorktree {
            worktree,
            if_lost: IfLost::Discard,
        })
        .await
        .unwrap()
}

/// The watcher has let go of every session.
pub(crate) async fn nothing_watched(owner: &NodeClient) {
    step("the watcher lets the session go", async {
        loop {
            let list = owner.call(ListPullRequests {}).await.unwrap();
            let status = owner.call(GetForgeStatus {}).await.unwrap();
            if list.entries.is_empty() && status.watched == 0 {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await;
}

/// Archiving a watched session with a service and a shell beside it ends
/// all of it: the session, its watch, its pull request on its info, the
/// service (not restarted) and the shell, both kept with their output;
/// until then its worktree is kept, and then it can go. Every reply that
/// carries the session carries its pull request while it is watched.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn archiving_ends_the_watch_the_service_and_the_shell() {
    within(async {
        let forge = FakeForge::new();
        let fixture = Fixture::new();
        let node = start(&fixture, &forge).await;
        let owner = fixture.connect(&node).await;
        let (_folder, made) = project(&owner).await;
        with_a_service(&owner, &made.root, made.id).await;
        let Watched {
            worktree,
            session,
            service,
            shell,
        } = watched(&owner, &fixture, &forge).await;

        let listed = owner.call(ListSessions::default()).await.unwrap();
        let shown = listed.iter().find(|info| info.id == session).unwrap();
        assert!(shown.pull_request.is_some(), "the list carries it");
        let pinned = owner
            .call(MarkSession {
                session,
                mark: Mark::Pin,
            })
            .await
            .unwrap();
        assert!(pinned.pull_request.is_some(), "a mark's reply carries it");

        // What runs in the worktree keeps it.
        let kept = remove(&owner, worktree.id).await;
        assert_eq!(
            kept,
            RemoveWorktreeOutcome::Kept {
                reasons: vec![KeptReason::InUse]
            }
        );

        let archived = owner
            .call(MarkSession {
                session,
                mark: Mark::Archive,
            })
            .await
            .unwrap();
        assert!(archived.pull_request.is_none(), "{archived:?}");
        nothing_watched(&owner).await;
        let service = terminal(&owner, &service).await;
        assert!(!service.is_live(), "{service:?}");
        assert!(
            matches!(service.service, Some(ServiceState::Stopped)),
            "not restarted: {service:?}"
        );
        let shell = terminal(&owner, &shell).await;
        assert!(!shell.is_live(), "{shell:?}");
        // Put away, nothing new starts beside it until it is brought back.
        let beside = CreateTerminal {
            place: TerminalPlace::Session { session },
            size: None,
            title: None,
        };
        assert_eq!(code(owner.call(beside).await), ErrorCode::Conflict);
        let run = RunScript {
            place: ScriptPlace::Session { session },
            script: "web".parse().unwrap(),
        };
        assert_eq!(code(owner.call(run).await), ErrorCode::Conflict);

        // Nothing runs there any more.
        assert!(matches!(
            remove(&owner, worktree.id).await,
            RemoveWorktreeOutcome::Removed { .. }
        ));
        node.stop().await.unwrap();
    })
    .await;
}

/// A session that only ended keeps its service and its shell (it can be
/// started again in the same folder, ADR 0026), so its worktree is kept;
/// its watch and its pull request end with it. Deleting it ends and
/// forgets them first, then tidies its worktree.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_ends_and_forgets_the_runs_before_the_worktree_is_tidied() {
    within(async {
        let forge = FakeForge::new();
        let fixture = Fixture::new();
        let node = start(&fixture, &forge).await;
        let owner = fixture.connect(&node).await;
        let (_folder, made) = project(&owner).await;
        with_a_service(&owner, &made.root, made.id).await;
        let Watched {
            worktree,
            session,
            service,
            shell,
        } = watched(&owner, &fixture, &forge).await;

        owner.kill(session).await.unwrap();
        let ended = info_until(&owner, session, |info| !info.status.is_live()).await;
        assert!(ended.pull_request.is_none(), "{ended:?}");
        nothing_watched(&owner).await;
        assert!(terminal(&owner, &service).await.is_live());
        assert!(terminal(&owner, &shell).await.is_live());
        assert_eq!(
            remove(&owner, worktree.id).await,
            RemoveWorktreeOutcome::Kept {
                reasons: vec![KeptReason::InUse]
            }
        );

        let deleted = owner.delete_session(session).await.unwrap();
        let tidied = deleted.worktree.unwrap();
        assert_eq!(tidied.worktree, worktree.id);
        assert!(
            matches!(tidied.outcome, RemoveWorktreeOutcome::Removed { .. }),
            "{tidied:?}"
        );
        for gone in [&service, &shell] {
            assert_eq!(
                code(
                    owner
                        .call(GetTerminal {
                            terminal: gone.id,
                            changed_from: None,
                        })
                        .await
                ),
                ErrorCode::NotFound
            );
        }
        node.stop().await.unwrap();
    })
    .await;
}
