//! Where protocol 10's two halves meet (milestone 11, the plan's
//! "Integration (node)"): a session placed in a worktree says so in every
//! answer naming it; deleting it, or retention dropping it, tidies the
//! worktree made for it by the rule that keeps work (ADR 0027); ending or
//! archiving it does not, so `session.resume` finds its folder, and a
//! session whose folder is gone anyway is refused, not started elsewhere.

use std::path::PathBuf;
use std::time::Duration;

use bytes::Bytes;
use vessel_client::NodeClient;
use vessel_node::Node;
use vessel_proto::conversation::TidiedWorktree;
use vessel_proto::workspace::{
    AddProject, ApproveCommands, IfLost, KeptReason, ListWorktrees, ProjectInfo, RemoveWorktree,
    RemoveWorktreeOutcome, RunSetup, SessionPlace, SetupState,
};
use vessel_proto::{ErrorCode, EventSeq, HarnessKind, Mark, SessionId, SessionStatus, WorktreeId};
use vessel_store::Retention;

use crate::agents::code;
use crate::attachments::{answer, attached, mime, name, text};
use crate::support::{Fixture, read_until, status_of, step, structured_spec, within};
use crate::workspace::{in_place, lookup, new_worktree, repository};

/// A node that finds git, started on `fixture`.
async fn start(fixture: &Fixture) -> Node {
    fixture
        .start_with(|config| config.harness_lookup = lookup(fixture))
        .await
}

/// A repository with one commit as a project of `client`'s node; its
/// folder lives as long as the returned guard.
pub(crate) async fn project(client: &NodeClient) -> (tempfile::TempDir, ProjectInfo) {
    let folder = tempfile::tempdir().unwrap();
    let root: PathBuf = std::fs::canonicalize(folder.path()).unwrap();
    repository(&root);
    let project = client
        .call(AddProject {
            path: root,
            name: None,
        })
        .await
        .unwrap();
    (folder, project)
}

/// The worktrees the node has now.
async fn worktrees(client: &NodeClient) -> Vec<WorktreeId> {
    let list = client.call(ListWorktrees { project: None }).await.unwrap();
    list.worktrees.iter().map(|worktree| worktree.id).collect()
}

/// A structured session placed in `worktree`, once it runs.
async fn running_in(client: &NodeClient, fixture: &Fixture, worktree: WorktreeId) -> SessionId {
    let mut spec = structured_spec(fixture, None).with_place(SessionPlace::Worktree { worktree });
    spec.cwd = None;
    let id = client.create_session(spec).await.unwrap().id;
    let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
    read_until(&mut stream, &mut Vec::new(), |record| {
        status_of(record) == Some(&SessionStatus::Running)
    })
    .await;
    id
}

/// `SessionInfo.workspace` is in every answer that carries a session's
/// info: the creation, a read, the list, a subscription, a mark and a
/// start again; ending and archiving leave the worktree where it is.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_placed_session_says_where_it_runs_in_every_answer() {
    within(async {
        let fixture = Fixture::new();
        let node = start(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;
        let worktree = new_worktree(&owner, project.id).await;
        let id = running_in(&owner, &fixture, worktree.id).await;
        let expected = owner.session(id).await.unwrap().workspace;
        let workspace = expected.as_deref().unwrap();
        assert_eq!(
            (workspace.project, workspace.worktree),
            (project.id, Some(worktree.id))
        );

        let listed = owner.sessions().await.unwrap();
        let listed = listed.iter().find(|info| info.id == id).unwrap();
        assert_eq!(listed.workspace, expected, "session.list");
        let subscription = owner.subscribe(id, EventSeq::ZERO).await.unwrap();
        assert_eq!(subscription.info().workspace, expected, "session.subscribe");
        drop(subscription);
        let pinned = owner.mark(id, Mark::Pin).await.unwrap();
        assert_eq!(pinned.workspace, expected, "session.mark");

        // Archiving ends it; its worktree stays for a start again.
        let archived = owner.mark(id, Mark::Archive).await.unwrap();
        assert_eq!(archived.workspace, expected, "session.mark archive");
        assert_eq!(worktrees(&owner).await, [worktree.id]);
        node.stop().await.unwrap();

        let node = start(&fixture).await;
        let owner = fixture.connect(&node).await;
        assert_eq!(worktrees(&owner).await, [worktree.id], "a restart keeps it");
        let resumed = step("start again", owner.resume_session(id)).await.unwrap();
        assert_eq!(resumed.status, SessionStatus::Running);
        assert_eq!(resumed.spec.cwd.as_ref(), Some(&worktree.path));
        assert_eq!(resumed.workspace, expected, "session.resume");
        owner.kill(id).await.unwrap();
        node.stop().await.unwrap();
    })
    .await;
}

/// `session.delete` removes the worktree made for the session when
/// nothing would be lost, and says so; one with work in it is kept, with
/// the reason, in the answer and in the worktree's record, and its work is
/// still there.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_a_session_tidies_its_worktree_and_never_loses_work() {
    within(async {
        let fixture = Fixture::new();
        let node = start(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;
        let clean = new_worktree(&owner, project.id).await;
        let worked = new_worktree(&owner, project.id).await;
        let tidy = running_in(&owner, &fixture, clean.id).await;
        let keep = running_in(&owner, &fixture, worked.id).await;
        std::fs::write(worked.path.join("notes.txt"), "not committed\n").unwrap();
        // The restart ends both, and the store serves them from then on.
        node.stop().await.unwrap();
        let node = start(&fixture).await;
        let owner = fixture.connect(&node).await;

        let deleted = owner.delete_session(tidy).await.unwrap();
        assert_eq!(
            deleted.worktree,
            Some(TidiedWorktree {
                worktree: clean.id,
                outcome: RemoveWorktreeOutcome::Removed { snapshot: None },
            })
        );
        assert!(!clean.path.exists(), "its folder went");

        let deleted = owner.delete_session(keep).await.unwrap();
        let kept = RemoveWorktreeOutcome::Kept {
            reasons: vec![KeptReason::Uncommitted { files: 1 }],
        };
        assert_eq!(
            deleted.worktree,
            Some(TidiedWorktree {
                worktree: worked.id,
                outcome: kept,
            })
        );
        assert_eq!(
            std::fs::read_to_string(worked.path.join("notes.txt")).unwrap(),
            "not committed\n"
        );
        let list = owner.call(ListWorktrees { project: None }).await.unwrap();
        let [listed] = list.worktrees.as_slice() else {
            panic!("only the kept one: {list:?}");
        };
        assert_eq!(listed.id, worked.id);
        assert_eq!(listed.kept, Some(KeptReason::Uncommitted { files: 1 }));
        assert_eq!(code(owner.session(keep).await), ErrorCode::NotFound);

        // A person removes it now, knowing why it was kept.
        let removed = owner
            .call(RemoveWorktree {
                worktree: worked.id,
                if_lost: IfLost::Discard,
            })
            .await
            .unwrap();
        assert_eq!(removed, RemoveWorktreeOutcome::Removed { snapshot: None });
        node.stop().await.unwrap();
    })
    .await;
}

/// A session in a worktree takes a prompt with a file attached: the file
/// is the session's, in the node's data folder, never written into the
/// worktree, so it leaves the worktree clean and tidying removes it when
/// the session is deleted, the attachment with it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_placed_session_takes_attachments_without_dirtying_its_worktree() {
    within(async {
        let fixture = Fixture::new();
        let node = start(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;
        let worktree = new_worktree(&owner, project.id).await;
        let mut spec = structured_spec(&fixture, None).with_place(SessionPlace::Worktree {
            worktree: worktree.id,
        });
        spec.cwd = None;
        let id = owner.create_session(spec).await.unwrap().id;
        let mut stream = owner.follow(id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        let file = owner
            .attach(
                id,
                name("notes.txt"),
                mime("text/plain"),
                Bytes::from_static(b"notes"),
            )
            .await
            .unwrap();
        let said = answer(
            &owner,
            (&mut stream, &mut seen),
            id,
            vec![text("describe"), attached(&file)],
        )
        .await;
        assert_eq!(said, "text 8; link notes.txt text/plain 5 read 5");
        assert!(!worktree.path.join("notes.txt").exists());
        let kept = fixture.data_dir().attachments_dir().join(id.to_string());
        assert!(kept.exists(), "the session's own");
        drop(stream);
        node.stop().await.unwrap();

        let node = start(&fixture).await;
        let owner = fixture.connect(&node).await;
        let deleted = owner.delete_session(id).await.unwrap();
        assert_eq!(
            deleted.worktree.map(|tidied| tidied.outcome),
            Some(RemoveWorktreeOutcome::Removed { snapshot: None }),
            "nothing of the attachment is in the worktree"
        );
        assert!(!kept.exists(), "the attachment went with its session");
        node.stop().await.unwrap();
    })
    .await;
}

/// A session whose worktree a person removed while it had ended is
/// refused when started again (`conflict`, naming the folder), not started
/// in another folder; nothing of it changes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_whose_folder_is_gone_does_not_start_again() {
    within(async {
        let fixture = Fixture::new();
        let node = start(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;
        let worktree = new_worktree(&owner, project.id).await;
        let id = running_in(&owner, &fixture, worktree.id).await;
        node.stop().await.unwrap();
        let node = start(&fixture).await;
        let owner = fixture.connect(&node).await;
        let removed = owner
            .call(RemoveWorktree {
                worktree: worktree.id,
                if_lost: IfLost::Keep,
            })
            .await
            .unwrap();
        assert_eq!(removed, RemoveWorktreeOutcome::Removed { snapshot: None });

        let refused = owner.resume_session(id).await.unwrap_err();
        let vessel_client::CallError::Remote(error) = &refused else {
            panic!("{refused:?}");
        };
        assert_eq!(error.code, ErrorCode::Conflict);
        assert!(error.message.contains("is gone"), "{}", error.message);
        let after = owner.session(id).await.unwrap();
        assert_eq!(after.status, SessionStatus::Interrupted, "it stays ended");
        assert_eq!(after.workspace.as_deref().map(|w| w.worktree), None);
        node.stop().await.unwrap();
    })
    .await;
}

/// A session is not started again in a worktree setup (or a removal)
/// holds: the removal could take the folder from under its agent.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_does_not_start_again_while_its_worktree_is_busy() {
    within(async {
        let fixture = Fixture::new();
        let node = start(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;
        let worktree = new_worktree(&owner, project.id).await;
        let id = running_in(&owner, &fixture, worktree.id).await;
        node.stop().await.unwrap();
        let node = start(&fixture).await;
        let owner = fixture.connect(&node).await;

        // A slow setup of the worktree's own file, approved, run again.
        let file = "[worktree]\nsetup = \"sleep 30\"\n";
        std::fs::write(worktree.path.join("vessel.toml"), file).unwrap();
        let digest = vessel_workspace::project_file::ProjectFile::parse(file.as_bytes())
            .unwrap()
            .digest();
        let _approved = owner
            .call(ApproveCommands {
                project: project.id,
                digest,
                approved: true,
            })
            .await
            .unwrap();
        let running = owner
            .call(RunSetup {
                worktree: worktree.id,
            })
            .await
            .unwrap();
        assert!(
            matches!(running.setup, SetupState::Running { .. }),
            "{running:?}"
        );
        let refused = owner.resume_session(id).await.unwrap_err();
        let vessel_client::CallError::Remote(error) = &refused else {
            panic!("{refused:?}");
        };
        assert_eq!(error.code, ErrorCode::Conflict, "{}", error.message);
        assert!(
            error.message.contains("being set up or removed"),
            "{}",
            error.message
        );
        node.stop().await.unwrap();
    })
    .await;
}

/// Retention dropping a session tidies the worktree made for it, as a
/// deletion does; the node does it in the background after the prune.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retention_tidies_the_worktree_of_a_session_it_drops() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| {
                config.harness_lookup = lookup(&fixture);
                config.store.retention = Retention {
                    max_ended_sessions: 0,
                    ..Retention::default()
                };
            })
            .await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;
        let worktree = new_worktree(&owner, project.id).await;
        let spec = in_place(
            &fixture,
            SessionPlace::Worktree {
                worktree: worktree.id,
            },
        );
        assert_eq!(spec.harness, HarnessKind::ClaudeCode);
        let id = owner.create_session(spec).await.unwrap().id;
        owner.kill(id).await.unwrap();
        // Nothing answers when the background tidying is done, so this
        // looks until the worktree is gone (the step's deadline bounds it).
        step("the pruned session's worktree tidied", async {
            while !worktrees(&owner).await.is_empty() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await;
        assert!(!worktree.path.exists());
        assert_eq!(code(owner.session(id).await), ErrorCode::NotFound);
        node.stop().await.unwrap();
    })
    .await;
}
