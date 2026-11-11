//! Protocol 10 over the wire (milestone 11, track B, ADR 0024): a project,
//! a worktree, and a session started in it, as the owner and as an agent,
//! who gets only what orchestration needs.

use std::path::{Path, PathBuf};
use std::process::Command;

use vessel_client::NodeClient;
use vessel_harness::Lookup;
use vessel_proto::workspace::{
    AddProject, ApproveCommands, CreateWorktree, CreateWorktreeOutcome, GetStatus, ListProjects,
    ReadFile, SessionPlace, SetupChoice, WorkspaceRoot, WorktreeBranch, WorktreeInfo,
};
use vessel_proto::{ErrorCode, HarnessKind, ProjectId, SessionSpec};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, terminal_spec, within};

/// A repository with one commit, made with the machine's git and a fixed
/// identity.
fn repository(dir: &Path) {
    let git = |args: &[&str]| {
        let status = Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
            ])
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    };
    git(&["init", "-q"]);
    std::fs::write(dir.join("README.md"), "hello\n").unwrap();
    git(&["add", "README.md"]);
    git(&["commit", "-q", "-m", "first"]);
}

/// The fixture's fake harnesses first, then the folder git is in.
fn lookup(fixture: &Fixture) -> Lookup {
    let git = which::which("git").expect("the tests need git on PATH");
    let folder = git.parent().map(Path::to_path_buf).unwrap_or_default();
    Lookup::new([fixture.bin().to_path_buf(), folder])
}

async fn new_worktree(client: &NodeClient, project: ProjectId) -> WorktreeInfo {
    let outcome = client
        .call(CreateWorktree {
            project,
            branch: WorktreeBranch::New {
                name: None,
                base: None,
            },
            setup: SetupChoice::Run,
        })
        .await
        .unwrap();
    match outcome {
        CreateWorktreeOutcome::Created { worktree } => worktree,
        other @ (CreateWorktreeOutcome::NeedsApproval { .. }
        | CreateWorktreeOutcome::Refused { .. }) => panic!("no worktree: {other:?}"),
    }
}

fn in_place(fixture: &Fixture, place: SessionPlace) -> SessionSpec {
    let mut spec = terminal_spec(fixture, HarnessKind::ClaudeCode).with_place(place);
    spec.cwd = None;
    spec
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_starts_in_a_worktree_and_says_where_it_runs() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.harness_lookup = lookup(&fixture))
            .await;
        let owner = fixture.connect(&node).await;
        let folder = tempfile::tempdir().unwrap();
        let root: PathBuf = std::fs::canonicalize(folder.path()).unwrap().join("app");
        std::fs::create_dir(&root).unwrap();
        repository(&root);

        let listed = owner.call(ListProjects {}).await.unwrap();
        assert!(listed.projects.is_empty());
        let project = owner
            .call(AddProject {
                path: root.clone(),
                name: None,
            })
            .await
            .unwrap();
        assert_eq!(project.root, root);
        let worktree = new_worktree(&owner, project.id).await;

        let session = owner
            .create_session(in_place(
                &fixture,
                SessionPlace::Worktree {
                    worktree: worktree.id,
                },
            ))
            .await
            .unwrap();
        assert_eq!(session.spec.cwd.as_ref(), Some(&worktree.path));
        let workspace = session.workspace.as_deref().unwrap();
        assert_eq!(workspace.project, project.id);
        assert_eq!(workspace.worktree, Some(worktree.id));
        assert_eq!(workspace.branch, worktree.branch);
        let again = owner.session(session.id).await.unwrap();
        assert_eq!(again.workspace, session.workspace);

        // A place and a folder of the client's own do not go together.
        let mut both = in_place(
            &fixture,
            SessionPlace::Project {
                project: project.id,
                dir: None,
            },
        );
        both.cwd = Some(root.clone());
        assert_eq!(
            code(owner.create_session(both).await),
            ErrorCode::InvalidRequest
        );

        let root_of = WorkspaceRoot::Worktree {
            worktree: worktree.id,
        };
        let read = owner
            .call(ReadFile {
                root: root_of,
                path: "README.md".parse().unwrap(),
                offset: 0,
                limit: None,
            })
            .await
            .unwrap();
        assert_eq!(read.version.size, 6);
        let status = owner.call(GetStatus { root: root_of }).await.unwrap();
        assert_eq!(status.branch, worktree.branch);

        owner.kill(session.id).await.unwrap();
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_gets_projects_and_worktrees_and_nothing_else() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| {
                config.vessel_mcp = Some(vessel_mcp(&fixture));
                config.harness_lookup = lookup(&fixture);
            })
            .await;
        let owner = fixture.connect(&node).await;
        let folder = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(folder.path()).unwrap();
        repository(&root);
        let project = owner
            .call(AddProject {
                path: root,
                name: None,
            })
            .await
            .unwrap();
        let owners = new_worktree(&owner, project.id).await;

        let mut first = structured(&owner, &fixture).await;
        let (_, socket, auth) = credential(&owner, &mut first).await;
        let agent = as_agent(&socket, &auth).await;

        let listed = agent.call(ListProjects {}).await.unwrap();
        assert_eq!(listed.projects.len(), 1);
        let file = ReadFile {
            root: WorkspaceRoot::Project {
                project: project.id,
            },
            path: "README.md".parse().unwrap(),
            offset: 0,
            limit: None,
        };
        assert_eq!(code(agent.call(file).await), ErrorCode::Forbidden);
        let approve = ApproveCommands {
            project: project.id,
            digest: "0".repeat(64).parse().unwrap(),
            approved: true,
        };
        assert_eq!(code(agent.call(approve).await), ErrorCode::Forbidden);

        // Its own worktree, and a session in it; not the owner's.
        let own = new_worktree(&agent, project.id).await;
        let started = agent
            .create_session(in_place(
                &fixture,
                SessionPlace::Worktree { worktree: own.id },
            ))
            .await
            .unwrap();
        assert_eq!(started.spec.cwd.as_ref(), Some(&own.path));
        assert_eq!(
            code(
                agent
                    .create_session(in_place(
                        &fixture,
                        SessionPlace::Worktree {
                            worktree: owners.id
                        },
                    ))
                    .await
            ),
            ErrorCode::Forbidden
        );
        agent.kill(started.id).await.unwrap();
        owner.kill(first.id).await.unwrap();
        node.stop().await.unwrap();
    })
    .await;
}
