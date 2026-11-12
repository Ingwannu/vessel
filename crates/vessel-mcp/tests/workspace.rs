//! The project and worktree tools (protocol 10) end to end, with a
//! project present, through a node in this process: an agent lists the
//! projects, makes a worktree (following its approved setup to the end),
//! starts a session in it, and is refused what it may not do: another's
//! worktree, more than eight of its own, and every workspace call but
//! orchestration's.

use serde_json::json;
use vessel_harness::Lookup;
use vessel_mcp::VesselTools;

use fixtures::{owners_worktree, project_with_setup, refusals};
use support::{Fixture, agent_env, owner, serve, within};

#[cfg(test)]
#[path = "support/mcp.rs"]
#[expect(
    dead_code,
    reason = "the shared MCP fixture, not every helper of it: this test needs the agent's \
              environment itself, for a client of its own beside the tools"
)]
mod support;

/// Builders and checks. A `cfg(test)` module so the lints' test allowances
/// (`unwrap` and friends) apply to it.
#[cfg(test)]
mod fixtures {
    use std::path::Path;
    use std::process::Command;

    use serde_json::{Value, json};
    use tempfile::TempDir;
    use vessel_client::{CallError, NodeClient};
    use vessel_proto::ErrorCode;
    use vessel_proto::workspace::{
        AddProject, ApproveCommands, CreateWorktree, CreateWorktreeOutcome, GetProjectCommands,
        GetStatus, IfLost, ProjectCommands, ProjectInfo, ReadFile, RemoveWorktree, SetupChoice,
        WorkspaceRoot, WorktreeBranch, WorktreeInfo,
    };

    use super::support::McpClient;

    /// A repository with one commit whose `vessel.toml` names one setup
    /// command, made with the machine's git and a fixed identity.
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
        std::fs::write(
            dir.join("vessel.toml"),
            "[worktree]\nsetup = \"echo setting up\"\n",
        )
        .unwrap();
        git(&["add", "README.md", "vessel.toml"]);
        git(&["commit", "-q", "-m", "first"]);
    }

    /// That repository as a project of `owner`'s node, its setup approved
    /// by the owner (an agent never can); its folder lives as long as the
    /// returned guard.
    pub(super) async fn project_with_setup(owner: &NodeClient) -> (TempDir, ProjectInfo) {
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
        let ProjectCommands::Listed { digest, .. } = owner
            .call(GetProjectCommands {
                project: project.id,
                base: None,
            })
            .await
            .unwrap()
        else {
            panic!("the project's file names its setup");
        };
        let approved = owner
            .call(ApproveCommands {
                project: project.id,
                digest,
                approved: true,
            })
            .await
            .unwrap();
        assert!(matches!(
            approved,
            ProjectCommands::Listed { approved: true, .. }
        ));
        (folder, project)
    }

    /// A worktree the owner makes for itself.
    pub(super) async fn owners_worktree(owner: &NodeClient, project: &ProjectInfo) -> WorktreeInfo {
        let outcome = owner
            .call(CreateWorktree {
                project: project.id,
                branch: WorktreeBranch::New {
                    name: None,
                    base: None,
                },
                setup: SetupChoice::Skip,
            })
            .await
            .unwrap();
        let CreateWorktreeOutcome::Created { worktree } = outcome else {
            panic!("{outcome:?}");
        };
        worktree
    }

    /// A tool's refusal: its text starts with the node's error code.
    fn refused(failed: bool, refusal: &Value, code: &str) {
        assert!(failed, "{refusal}");
        let text = refusal.as_str().unwrap_or_default();
        assert!(text.starts_with(&format!("\"{code}\"")), "{text}");
    }

    fn code<T: std::fmt::Debug>(result: Result<T, CallError>) -> ErrorCode {
        match result {
            Err(CallError::Remote(error)) => error.code,
            other => panic!("{other:?}"),
        }
    }

    /// What the agent may not do: start in another's worktree (through its
    /// tool), and every workspace call but orchestration's (through its
    /// own connection, whatever its scopes).
    pub(super) async fn refusals(
        mcp: &mut McpClient,
        agent: &NodeClient,
        project: &ProjectInfo,
        owners: &WorktreeInfo,
    ) {
        let (failed, refusal) = mcp
            .call(
                "create_session",
                json!({ "harness": "claude", "worktree": owners.id }),
            )
            .await;
        refused(failed, &refusal, "forbidden");
        let root = WorkspaceRoot::Worktree {
            worktree: owners.id,
        };
        assert_eq!(
            code(agent.call(GetStatus { root }).await),
            ErrorCode::Forbidden
        );
        let read = ReadFile {
            root,
            path: "README.md".parse().unwrap(),
            offset: 0,
            limit: None,
        };
        assert_eq!(code(agent.call(read).await), ErrorCode::Forbidden);
        let remove = RemoveWorktree {
            worktree: owners.id,
            if_lost: IfLost::Discard,
        };
        assert_eq!(code(agent.call(remove).await), ErrorCode::Forbidden);
        let approve = ApproveCommands {
            project: project.id,
            digest: "0".repeat(64).parse().unwrap(),
            approved: true,
        };
        assert_eq!(code(agent.call(approve).await), ErrorCode::Forbidden);

        // Eight of its own at most (it has one): the ninth is refused, not
        // made.
        for _ in 1..8 {
            let (failed, made) = mcp
                .call(
                    "create_worktree",
                    json!({ "project": project.id, "setup": "skip" }),
                )
                .await;
            assert!(!failed && made["worktree"].is_object(), "{made}");
        }
        let (failed, ninth) = mcp
            .call(
                "create_worktree",
                json!({ "project": project.id, "setup": "skip" }),
            )
            .await;
        assert!(!failed, "{ninth}");
        assert_eq!(ninth["refused"]["kind"], "limit", "{ninth}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_starts_a_session_in_a_worktree_of_its_own() {
    within(async {
        let git = which::which("git").expect("the tests need git on PATH");
        let git_folder = git.parent().unwrap().to_path_buf();
        let fixture = Fixture::start_with(|config, _| {
            let dirs = config.harness_lookup.dirs().to_vec();
            config.harness_lookup = Lookup::new(dirs.into_iter().chain([git_folder]));
        })
        .await;
        let owner = owner(&fixture).await;
        let (_folder, project) = project_with_setup(&owner).await;
        let owners = owners_worktree(&owner, &project).await;

        let (_own_session, env) = agent_env(&owner).await;
        let tools = VesselTools::new(vessel_mcp::connect(&env).await.unwrap());
        let (mut mcp, server) = serve(tools);
        let _names = mcp.initialize().await;
        let agent = vessel_mcp::connect(&env).await.unwrap();

        let (failed, listed) = mcp.call("list_projects", json!({})).await;
        assert!(!failed, "{listed}");
        assert_eq!(listed["projects"][0]["id"], json!(project.id), "{listed}");
        assert_eq!(listed["git"]["state"], "found", "{listed}");

        // Its own worktree, its approved setup followed to the end.
        let (failed, made) = mcp
            .call("create_worktree", json!({ "project": project.id }))
            .await;
        assert!(!failed, "{made}");
        assert_eq!(made["setup"]["state"], "succeeded", "{made}");
        let output = made["output"].as_str().unwrap();
        assert!(output.contains("setting up"), "{made}");
        let worktree = made["worktree"]["id"].clone();
        let path = made["worktree"]["path"].clone();
        let (failed, listed) = mcp
            .call("list_worktrees", json!({ "project": project.id }))
            .await;
        assert!(!failed, "{listed}");
        assert_eq!(listed["worktrees"].as_array().unwrap().len(), 2, "{listed}");

        // A session in it, with a first prompt.
        let (failed, created) = mcp
            .call(
                "create_session",
                json!({ "harness": "claude", "worktree": worktree, "prompt": "plain" }),
            )
            .await;
        assert!(!failed, "{created}");
        let session = &created["session"];
        assert_eq!(session["spec"]["cwd"], path, "{created}");
        assert_eq!(session["workspace"]["worktree"], worktree, "{created}");
        assert_eq!(session["workspace"]["project"], json!(project.id));

        refusals(&mut mcp, &agent, &project, &owners).await;
        drop(mcp);
        server.await.unwrap().unwrap();
        fixture.node.stop().await.unwrap();
    })
    .await;
}
