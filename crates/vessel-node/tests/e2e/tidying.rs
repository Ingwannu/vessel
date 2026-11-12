//! Where protocol 10's two halves meet (milestone 11, the plan's
//! "Integration (node)"): a session placed in a worktree says so in every
//! answer naming it, and ending or archiving it leaves the worktree, so
//! `session.resume` finds its folder.

use std::path::PathBuf;

use vessel_client::NodeClient;
use vessel_node::Node;
use vessel_proto::workspace::{AddProject, ListWorktrees, ProjectInfo, SessionPlace};
use vessel_proto::{EventSeq, Mark, SessionId, SessionStatus, WorktreeId};

use crate::support::{Fixture, read_until, status_of, step, structured_spec, within};
use crate::workspace::{lookup, new_worktree, repository};

/// A node that finds git, started on `fixture`.
async fn start(fixture: &Fixture) -> Node {
    fixture
        .start_with(|config| config.harness_lookup = lookup(fixture))
        .await
}

/// A repository with one commit as a project of `client`'s node; its
/// folder lives as long as the returned guard.
async fn project(client: &NodeClient) -> (tempfile::TempDir, ProjectInfo) {
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
