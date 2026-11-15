//! The node's workspace calls against temporary repositories, with the
//! machine's real git isolated from its configuration.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::workspace::{
    CreateWorktree, CreateWorktreeOutcome, IfLost, KeptReason, ProjectCommands, ProjectInfo,
    ProjectState, RefName, RemoveWorktreeOutcome, SessionPlace, SetupChoice, SetupState,
    WorkspaceRoot, WorktreeBranch, WorktreeInfo, WorktreeRefusal,
};
use vessel_proto::{Creator, NodeId, ProjectId, SessionId};
use vessel_store::{DataDir, Store, StoreConfig};

use super::{Occupant, Opening, TidyScope, WorkspaceError, Workspaces};

struct Fixture {
    temp: tempfile::TempDir,
    env: Vec<(String, OsString)>,
    data: DataDir,
    store: Store,
    tasks: TaskTracker,
    stop: CancellationToken,
    workspaces: Arc<Workspaces>,
}

impl Fixture {
    async fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let config = temp.path().join("gitconfig");
        std::fs::write(
            &config,
            "[user]\n\tname = Test\n\temail = test@example.invalid\n\
             [init]\n\tdefaultBranch = main\n\
             [commit]\n\tgpgsign = false\n\
             [core]\n\tautocrlf = false\n",
        )
        .unwrap();
        let env = vec![
            ("GIT_CONFIG_GLOBAL".to_owned(), config.into_os_string()),
            ("GIT_CONFIG_NOSYSTEM".to_owned(), OsString::from("1")),
            ("NO_PROXY".to_owned(), OsString::from("*")),
            ("no_proxy".to_owned(), OsString::from("*")),
        ];
        std::fs::create_dir(temp.path().join("home")).unwrap();
        let data = DataDir::new(temp.path().join("home").join(".vessel"));
        data.prepare().unwrap();
        let store = Store::open(&data, StoreConfig::default())
            .await
            .unwrap()
            .store;
        let tasks = TaskTracker::new();
        let stop = CancellationToken::new();
        let workspaces = start(&temp, &data, &env, &store, &tasks, &stop).await;
        Self {
            temp,
            env,
            data,
            store,
            tasks,
            stop,
            workspaces,
        }
    }

    /// The node starting again on the same store.
    async fn restart(&self) -> Arc<Workspaces> {
        start(
            &self.temp,
            &self.data,
            &self.env,
            &self.store,
            &self.tasks,
            &self.stop,
        )
        .await
    }

    fn path(&self) -> PathBuf {
        std::fs::canonicalize(self.temp.path()).unwrap()
    }

    fn git(&self, dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .envs(self.env.iter().map(|(key, value)| (key, value)))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    /// A repository with one commit of `README.md`, and optionally a
    /// `vessel.toml`.
    fn repo(&self, name: &str, file: Option<&str>) -> PathBuf {
        let dir = self.path().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let _out = self.git(&dir, &["init", "-q"]);
        std::fs::write(dir.join("README.md"), "hello\n").unwrap();
        if let Some(file) = file {
            std::fs::write(dir.join("vessel.toml"), file).unwrap();
        }
        let _out = self.git(&dir, &["add", "-A"]);
        let _out = self.git(&dir, &["commit", "-q", "-m", "first"]);
        dir
    }

    async fn project(&self, name: &str, file: Option<&str>) -> ProjectInfo {
        let root = self.repo(name, file);
        self.workspaces.add_project(root, None).await.unwrap()
    }

    async fn create(
        &self,
        project: ProjectId,
        creator: Creator,
        setup: SetupChoice,
    ) -> CreateWorktreeOutcome {
        self.workspaces
            .create_worktree(
                creator,
                CreateWorktree {
                    project,
                    branch: WorktreeBranch::New {
                        name: None,
                        base: None,
                    },
                    setup,
                },
                None,
            )
            .await
            .unwrap()
    }

    async fn created(&self, project: ProjectId) -> WorktreeInfo {
        match self.create(project, Creator::Owner, SetupChoice::Run).await {
            CreateWorktreeOutcome::Created { worktree } => worktree,
            other @ (CreateWorktreeOutcome::NeedsApproval { .. }
            | CreateWorktreeOutcome::Refused { .. }) => panic!("not created: {other:?}"),
        }
    }

    /// Follow the setup log until setup ends; the whole log.
    async fn setup_end(&self, worktree: vessel_proto::WorktreeId) -> (SetupState, String) {
        let mut from = 0;
        let mut text = String::new();
        for _round in 0..60 {
            let page = self
                .workspaces
                .read_output(worktree, from, Some(5))
                .await
                .unwrap();
            text.push_str(&String::from_utf8_lossy(page.bytes.bytes()));
            from = page.next;
            if !matches!(page.setup, SetupState::Running { .. }) {
                let rest = self
                    .workspaces
                    .read_output(worktree, from, None)
                    .await
                    .unwrap();
                text.push_str(&String::from_utf8_lossy(rest.bytes.bytes()));
                return (page.setup, text);
            }
        }
        panic!("setup did not end: {text}");
    }
}

async fn start(
    temp: &tempfile::TempDir,
    data: &DataDir,
    env: &[(String, OsString)],
    store: &Store,
    tasks: &TaskTracker,
    stop: &CancellationToken,
) -> Arc<Workspaces> {
    Workspaces::start(
        Opening {
            data: data.root().to_path_buf(),
            workspace: data.prepare_workspace_dir().unwrap(),
            home: temp.path().join("home"),
            git: Some(which::which("git").expect("the tests need git on PATH")),
            git_env: env.to_vec(),
            removed: crate::TerminalsConfig::default().removed.into(),
        },
        store.clone(),
        tasks.clone(),
        stop.clone(),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn a_folder_inside_a_repository_adds_its_top_level_once() {
    let fixture = Fixture::new().await;
    let root = fixture.repo("app", None);
    std::fs::create_dir(root.join("src")).unwrap();
    let added = fixture
        .workspaces
        .add_project(root.join("src"), None)
        .await
        .unwrap();
    assert_eq!(added.root, std::fs::canonicalize(&root).unwrap());
    assert!(added.git);
    assert_eq!(added.name.as_str(), "app");
    assert_eq!(
        added.default_branch.as_ref().map(RefName::as_str),
        Some("main")
    );
    let again = fixture.workspaces.add_project(root, None).await.unwrap();
    assert_eq!(again.id, added.id);
    assert_eq!(fixture.workspaces.list_projects().await.projects.len(), 1);
}

#[tokio::test]
async fn roots_that_reach_the_home_or_the_data_folder_are_refused() {
    let fixture = Fixture::new().await;
    let home = fixture.path().join("home");
    for refused in [
        home.clone(),
        home.join(".vessel"),
        PathBuf::from("relative"),
    ] {
        let error = fixture
            .workspaces
            .add_project(refused.clone(), None)
            .await
            .unwrap_err();
        assert!(
            matches!(error, WorkspaceError::Invalid(_)),
            "{refused:?}: {error:?}"
        );
    }
}

#[tokio::test]
async fn init_starts_a_repository_in_an_empty_folder_only() {
    let fixture = Fixture::new().await;
    let fresh = fixture.path().join("fresh");
    let made = fixture
        .workspaces
        .init_project(fresh.clone(), None)
        .await
        .unwrap();
    assert!(made.git);
    assert!(fresh.join(".git").is_dir());
    let full = fixture.path().join("full");
    std::fs::create_dir(&full).unwrap();
    std::fs::write(full.join("a.txt"), "a").unwrap();
    let error = fixture
        .workspaces
        .init_project(full, None)
        .await
        .unwrap_err();
    assert!(matches!(error, WorkspaceError::Conflict(_)), "{error:?}");
}

#[tokio::test]
async fn init_makes_no_folder_where_a_project_is_refused() {
    let fixture = Fixture::new().await;
    let inside = fixture
        .path()
        .join("home")
        .join(".vessel")
        .join("new")
        .join("app");
    let error = fixture
        .workspaces
        .init_project(inside.clone(), None)
        .await
        .unwrap_err();
    assert!(matches!(error, WorkspaceError::Invalid(_)), "{error:?}");
    assert!(!fixture.path().join("home/.vessel/new").exists());
    let climbing = fixture.path().join("fresh").join("..").join("home");
    let error = fixture
        .workspaces
        .init_project(climbing, None)
        .await
        .unwrap_err();
    assert!(matches!(error, WorkspaceError::Invalid(_)), "{error:?}");
    assert!(!fixture.path().join("fresh").exists());
}

#[tokio::test]
async fn a_clean_worktree_is_made_placed_and_removed_with_its_branch() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let worktree = fixture.created(project.id).await;
    assert!(
        worktree
            .path
            .starts_with(fixture.data.root().canonicalize().unwrap())
    );
    assert!(worktree.path.join("README.md").is_file());
    let branch = worktree.branch.clone().unwrap();
    assert_eq!(worktree.created_branch.as_ref(), Some(&branch));
    assert!(branch.as_str().starts_with("vessel/"));
    assert_eq!(worktree.setup, SetupState::None);
    let listed = fixture.workspaces.list_worktrees(Some(project.id), &[]);
    assert_eq!(listed.worktrees.len(), 1);

    let placed = fixture
        .workspaces
        .place(&SessionPlace::Worktree {
            worktree: worktree.id,
        })
        .await
        .unwrap();
    assert_eq!(placed.cwd, worktree.path);
    assert!(
        placed
            .notices
            .iter()
            .any(|line| line.contains(branch.as_str())),
        "{:?}",
        placed.notices
    );
    // The session is live now (ADR 0030: until then, `placed` keeps any
    // removal out).
    drop(placed);

    // A live session in it keeps it.
    let session = SessionId::random();
    let live = [(Occupant::Session(session), worktree.path.join("src"))];
    let kept = fixture
        .workspaces
        .remove_worktree(worktree.id, IfLost::Keep, || live.to_vec())
        .await
        .unwrap();
    assert_eq!(
        kept,
        RemoveWorktreeOutcome::Kept {
            reasons: vec![KeptReason::InUse]
        }
    );

    let removed = fixture
        .workspaces
        .remove_worktree(worktree.id, IfLost::Keep, Vec::new)
        .await
        .unwrap();
    assert_eq!(removed, RemoveWorktreeOutcome::Removed { snapshot: None });
    assert!(!worktree.path.exists());
    let branches = fixture.git(&project.root, &["branch", "--list", branch.as_str()]);
    assert!(branches.is_empty(), "{branches}");
    assert!(
        fixture
            .workspaces
            .list_worktrees(None, &[])
            .worktrees
            .is_empty()
    );
}

#[tokio::test]
async fn work_is_kept_or_snapshotted_and_a_snapshot_comes_back() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let worktree = fixture.created(project.id).await;
    std::fs::write(worktree.path.join("notes.txt"), "unsaved work\n").unwrap();

    let kept = fixture
        .workspaces
        .remove_worktree(worktree.id, IfLost::Keep, Vec::new)
        .await
        .unwrap();
    let RemoveWorktreeOutcome::Kept { reasons } = kept else {
        panic!("removed with work in it");
    };
    assert!(
        reasons.contains(&KeptReason::Uncommitted { files: 1 }),
        "{reasons:?}"
    );
    assert!(worktree.path.join("notes.txt").is_file());

    let removed = fixture
        .workspaces
        .remove_worktree(worktree.id, IfLost::Snapshot, Vec::new)
        .await
        .unwrap();
    let RemoveWorktreeOutcome::Removed {
        snapshot: Some(snapshot),
    } = removed
    else {
        panic!("no snapshot: {removed:?}");
    };
    assert_eq!(snapshot.worktree, worktree.id);
    assert!(!worktree.path.exists());
    let listed = fixture.workspaces.list_snapshots(project.id).await.unwrap();
    assert_eq!(listed.snapshots.len(), 1);

    let back = fixture
        .workspaces
        .create_worktree(
            Creator::Owner,
            CreateWorktree {
                project: project.id,
                branch: WorktreeBranch::Snapshot {
                    worktree: worktree.id,
                    name: None,
                },
                setup: SetupChoice::Skip,
            },
            None,
        )
        .await
        .unwrap();
    let CreateWorktreeOutcome::Created { worktree: back } = back else {
        panic!("not restored: {back:?}");
    };
    assert_eq!(
        std::fs::read_to_string(back.path.join("notes.txt")).unwrap(),
        "unsaved work\n"
    );
    fixture
        .workspaces
        .drop_snapshot(project.id, worktree.id)
        .await
        .unwrap();
    let error = fixture
        .workspaces
        .drop_snapshot(project.id, worktree.id)
        .await
        .unwrap_err();
    assert!(
        matches!(error, WorkspaceError::UnknownSnapshot(_)),
        "{error:?}"
    );
}

const SETUP: &str = "[worktree]\nsetup = [\"echo prepared-by-setup\"]\n";

#[tokio::test]
async fn setup_runs_only_once_a_person_approved_that_exact_set() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", Some(SETUP)).await;
    let outcome = fixture
        .create(project.id, Creator::Owner, SetupChoice::Run)
        .await;
    let CreateWorktreeOutcome::NeedsApproval { commands } = outcome else {
        panic!("created without approval: {outcome:?}");
    };
    let ProjectCommands::Listed {
        digest,
        approved: false,
        ..
    } = commands
    else {
        panic!("{commands:?}");
    };
    assert!(
        fixture
            .workspaces
            .list_worktrees(None, &[])
            .worktrees
            .is_empty()
    );

    // Skipping makes it without running anything.
    let skipped = fixture
        .create(project.id, Creator::Owner, SetupChoice::Skip)
        .await;
    let CreateWorktreeOutcome::Created { worktree: skipped } = skipped else {
        panic!("{skipped:?}");
    };
    assert_eq!(skipped.setup, SetupState::Skipped);
    let error = fixture.workspaces.run_setup(skipped.id).await.unwrap_err();
    assert!(matches!(error, WorkspaceError::Conflict(_)), "{error:?}");

    let approved = fixture
        .workspaces
        .approve(project.id, digest.clone(), true)
        .await
        .unwrap();
    assert!(matches!(
        approved,
        ProjectCommands::Listed { approved: true, .. }
    ));
    let worktree = fixture.created(project.id).await;
    assert!(matches!(worktree.setup, SetupState::Running { .. }));
    let (state, log) = fixture.setup_end(worktree.id).await;
    assert!(
        matches!(state, SetupState::Succeeded { .. }),
        "{state:?}: {log}"
    );
    assert!(log.contains("prepared-by-setup"), "{log}");

    // A changed set is another set: asked again.
    std::fs::write(
        project.root.join("vessel.toml"),
        "[worktree]\nsetup = [\"echo changed\"]\n",
    )
    .unwrap();
    let _out = fixture.git(&project.root, &["commit", "-q", "-am", "change setup"]);
    let again = fixture
        .create(project.id, Creator::Owner, SetupChoice::Run)
        .await;
    assert!(
        matches!(again, CreateWorktreeOutcome::NeedsApproval { .. }),
        "{again:?}"
    );
}

#[tokio::test]
async fn a_failing_setup_is_told_and_the_session_still_starts() {
    let fixture = Fixture::new().await;
    let project = fixture
        .project(
            "app",
            Some("[worktree]\nsetup = \"echo before-failing && exit 3\"\n"),
        )
        .await;
    let ProjectCommands::Listed { digest, .. } =
        fixture.workspaces.commands(project.id, None).await.unwrap()
    else {
        panic!("no commands");
    };
    let _approved = fixture
        .workspaces
        .approve(project.id, digest, true)
        .await
        .unwrap();
    let worktree = fixture.created(project.id).await;
    // Setup runs: a session cannot start in it yet.
    let early = fixture
        .workspaces
        .place(&SessionPlace::Worktree {
            worktree: worktree.id,
        })
        .await;
    let (state, _log) = fixture.setup_end(worktree.id).await;
    if let Err(error) = early {
        assert!(matches!(error, WorkspaceError::Conflict(_)), "{error:?}");
    }
    assert!(
        matches!(
            state,
            SetupState::Failed {
                failure: vessel_proto::workspace::SetupFailure::Exit {
                    command: 0,
                    code: Some(3)
                },
                ..
            }
        ),
        "{state:?}"
    );
    let placed = fixture
        .workspaces
        .place(&SessionPlace::Worktree {
            worktree: worktree.id,
        })
        .await
        .unwrap();
    let notices = placed.notices.join("\n");
    assert!(notices.contains("setup failed"), "{notices}");
    assert!(notices.contains("before-failing"), "{notices}");
}

#[tokio::test]
async fn an_agent_has_at_most_eight_worktrees() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let agent = Creator::Agent {
        node: NodeId::random(),
        session: SessionId::random(),
    };
    for _made in 0..super::MAX_PER_AGENT {
        let outcome = fixture
            .create(project.id, agent.clone(), SetupChoice::Run)
            .await;
        assert!(
            matches!(outcome, CreateWorktreeOutcome::Created { .. }),
            "{outcome:?}"
        );
    }
    let outcome = fixture.create(project.id, agent, SetupChoice::Run).await;
    assert_eq!(
        outcome,
        CreateWorktreeOutcome::Refused {
            reason: WorktreeRefusal::Limit
        }
    );
    // The owner is held only to the node's bound.
    let outcome = fixture
        .create(project.id, Creator::Owner, SetupChoice::Run)
        .await;
    assert!(
        matches!(outcome, CreateWorktreeOutcome::Created { .. }),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn creations_at_once_never_pass_an_agents_limit() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let agent = Creator::Agent {
        node: NodeId::random(),
        session: SessionId::random(),
    };
    let tries = super::MAX_PER_AGENT.saturating_add(4);
    let outcomes = futures_util::future::join_all(
        (0..tries).map(|_try| fixture.create(project.id, agent.clone(), SetupChoice::Run)),
    )
    .await;
    let created = outcomes
        .iter()
        .filter(|outcome| matches!(outcome, CreateWorktreeOutcome::Created { .. }))
        .count();
    let refused = outcomes
        .iter()
        .filter(|outcome| {
            **outcome
                == CreateWorktreeOutcome::Refused {
                    reason: WorktreeRefusal::Limit,
                }
        })
        .count();
    assert_eq!(
        (created, refused),
        (super::MAX_PER_AGENT, 4),
        "{outcomes:?}"
    );
    // Every one given back: the count is the recorded worktrees again.
    assert!(fixture.workspaces.state().making.is_empty());
}

#[tokio::test]
async fn a_sessions_worktree_is_tidied_unless_another_session_is_in_it() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let made_for = || CreateWorktree {
        project: project.id,
        branch: WorktreeBranch::New {
            name: None,
            base: None,
        },
        setup: SetupChoice::Run,
    };
    let (first, second) = (SessionId::random(), SessionId::random());
    let mut made = Vec::new();
    for session in [first, second] {
        let outcome = fixture
            .workspaces
            .create_worktree(Creator::Owner, made_for(), Some(session))
            .await
            .unwrap();
        let CreateWorktreeOutcome::Created { worktree } = outcome else {
            panic!("{outcome:?}");
        };
        made.push(worktree);
    }
    assert_eq!(
        fixture
            .workspaces
            .retire(SessionId::random(), Vec::new)
            .await,
        None
    );
    // A worktree made for no session is claimed by the first one started
    // in it, and only by that one.
    let unclaimed = fixture.created(project.id).await;
    let (claimer, later) = (SessionId::random(), SessionId::random());
    fixture.workspaces.claim(unclaimed.id, claimer).await;
    fixture.workspaces.claim(unclaimed.id, later).await;
    assert_eq!(
        fixture
            .workspaces
            .worktree(unclaimed.id)
            .unwrap()
            .created_for,
        Some(claimer)
    );
    let removed = Some(RemoveWorktreeOutcome::Removed { snapshot: None });
    let tidied = fixture.workspaces.retire(claimer, Vec::new).await.unwrap();
    assert_eq!(
        (tidied.worktree, Some(tidied.outcome)),
        (unclaimed.id, removed.clone())
    );
    let tidied = fixture
        .workspaces
        .retire(first, || {
            vec![(Occupant::Session(first), made[0].path.clone())]
        })
        .await;
    assert_eq!(tidied.map(|tidied| tidied.outcome), removed);
    // Every worktree made for a session is a candidate for the sweep; once
    // kept because another session is in it, it is one for the narrow
    // sweep after a deletion too.
    assert_eq!(
        fixture.workspaces.tidy_candidates(TidyScope::Every),
        [second]
    );
    assert!(
        fixture
            .workspaces
            .tidy_candidates(TidyScope::Kept)
            .is_empty()
    );
    let other = [(Occupant::Session(SessionId::random()), made[1].path.clone())];
    let kept = fixture.workspaces.retire(second, || other.to_vec()).await;
    assert_eq!(
        kept.map(|tidied| tidied.outcome),
        Some(RemoveWorktreeOutcome::Kept {
            reasons: vec![KeptReason::InUse]
        })
    );
    assert_eq!(
        fixture.workspaces.worktree(made[1].id).unwrap().kept,
        Some(KeptReason::InUse)
    );
    assert_eq!(
        fixture.workspaces.tidy_candidates(TidyScope::Kept),
        [second]
    );
}

/// A worktree tidying kept for the work in it waits for a person: no sweep
/// looks at it again.
#[tokio::test]
async fn a_worktree_kept_for_its_work_is_left_to_a_person() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let session = SessionId::random();
    let outcome = fixture
        .workspaces
        .create_worktree(
            Creator::Owner,
            CreateWorktree {
                project: project.id,
                branch: WorktreeBranch::New {
                    name: None,
                    base: None,
                },
                setup: SetupChoice::Run,
            },
            Some(session),
        )
        .await
        .unwrap();
    let CreateWorktreeOutcome::Created { worktree } = outcome else {
        panic!("{outcome:?}");
    };
    std::fs::write(worktree.path.join("notes.txt"), "not committed\n").unwrap();
    let kept = fixture.workspaces.retire(session, Vec::new).await.unwrap();
    assert_eq!(kept.worktree, worktree.id);
    assert_eq!(
        kept.outcome,
        RemoveWorktreeOutcome::Kept {
            reasons: vec![KeptReason::Uncommitted { files: 1 }]
        }
    );
    assert!(worktree.path.join("notes.txt").exists(), "nothing was lost");
    for scope in [TidyScope::Every, TidyScope::Kept] {
        assert!(fixture.workspaces.tidy_candidates(scope).is_empty());
    }
}

#[tokio::test]
async fn a_setup_the_node_stopped_is_failed_after_a_restart() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let mut worktree = fixture.created(project.id).await;
    worktree.setup = SetupState::Running {
        started_at: chrono::Utc::now(),
    };
    fixture.store.put_worktree(worktree.clone()).await.unwrap();
    let restarted = fixture.restart().await;
    let after = restarted.worktree(worktree.id).unwrap();
    assert!(
        matches!(after.setup, SetupState::Failed { .. }),
        "{:?}",
        after.setup
    );
    let page = restarted.read_output(worktree.id, 0, None).await.unwrap();
    assert!(String::from_utf8_lossy(page.bytes.bytes()).contains("stopped during setup"));
    assert_eq!(restarted.list_projects().await.projects.len(), 1);
}

#[tokio::test]
async fn a_clone_that_cannot_connect_ends_failed() {
    let fixture = Fixture::new().await;
    let url = "https://127.0.0.1:1/nothing.git".parse().unwrap();
    let cloning = fixture
        .workspaces
        .clone_project(url, fixture.path(), None, None)
        .await
        .unwrap();
    assert!(
        matches!(cloning.state, ProjectState::Cloning { .. }),
        "{:?}",
        cloning.state
    );
    assert_eq!(cloning.root, fixture.path().join("nothing"));
    let mut info = cloning;
    for _round in 0..30 {
        if !matches!(info.state, ProjectState::Cloning { .. }) {
            break;
        }
        info = fixture
            .workspaces
            .get_project(info.id, Some(info.revision), true)
            .await
            .unwrap();
    }
    assert!(
        matches!(info.state, ProjectState::CloneFailed { .. }),
        "{:?}",
        info.state
    );
    assert!(!fixture.path().join("nothing").exists());
    let leftovers: Vec<_> = std::fs::read_dir(fixture.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains("vessel-clone"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[cfg(unix)]
#[tokio::test]
async fn a_link_out_of_the_root_reads_and_writes_as_not_there() {
    use vessel_proto::workspace::WriteExpect;

    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let outside = fixture.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), "secret").unwrap();
    std::os::unix::fs::symlink(&outside, project.root.join("out")).unwrap();
    let root = || WorkspaceRoot::Project {
        project: project.id,
    };
    let path = "out/secret.txt".parse().unwrap();
    let read = fixture
        .workspaces
        .read_file(root(), &path, 0, None)
        .await
        .unwrap_err();
    assert!(matches!(read, WorkspaceError::NoSuchPath(_)), "{read:?}");
    let written = fixture
        .workspaces
        .write_file(
            root(),
            &"out/new.txt".parse().unwrap(),
            "x".to_owned(),
            WriteExpect::Any,
        )
        .await
        .unwrap_err();
    assert!(
        matches!(written, WorkspaceError::NoSuchPath(_)),
        "{written:?}"
    );
    assert!(!outside.join("new.txt").exists());
    let listed = fixture
        .workspaces
        .list_dir(root(), &"out".parse().unwrap(), false)
        .await
        .unwrap_err();
    assert!(
        matches!(listed, WorkspaceError::NoSuchPath(_)),
        "{listed:?}"
    );
}

#[tokio::test]
async fn git_calls_answer_in_a_worktree_and_follow_its_branch() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let worktree = fixture.created(project.id).await;
    let root = || WorkspaceRoot::Worktree {
        worktree: worktree.id,
    };
    std::fs::write(worktree.path.join("README.md"), "changed\n").unwrap();
    let status = fixture.workspaces.git_status(root()).await.unwrap();
    assert_eq!(status.files.len(), 1);
    assert_eq!(
        status.base.as_ref().map(|base| base.name.as_str()),
        Some("main")
    );
    let to = "feature/renamed".parse().unwrap();
    let renamed = fixture
        .workspaces
        .git_rename_branch(root(), None, &to)
        .await
        .unwrap();
    assert!(
        matches!(
            renamed,
            vessel_proto::workspace::RenameOutcome::Renamed { .. }
        ),
        "{renamed:?}"
    );
    let after = fixture.workspaces.worktree(worktree.id).unwrap();
    assert_eq!(after.branch.as_ref(), Some(&to));
    assert_eq!(after.created_branch.as_ref(), Some(&to));
    let found = fixture
        .workspaces
        .find_files(
            crate::transfers::Holder::Owner,
            root(),
            &"readme".parse().unwrap(),
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        found.matches.first().map(|file| file.path.as_str()),
        Some("README.md")
    );
    let at = vessel_proto::workspace::WorktreeBase::Ref {
        name: "main".parse().unwrap(),
    };
    let shown = fixture
        .workspaces
        .git_show(root(), &at, &"README.md".parse().unwrap(), 0, None)
        .await
        .unwrap();
    assert!(
        matches!(shown.content, vessel_proto::workspace::FileContent::Text { ref text, .. } if text == "hello\n")
    );
    // A project without git refuses git calls.
    let plain = fixture.path().join("plain");
    std::fs::create_dir(&plain).unwrap();
    let plain = fixture.workspaces.add_project(plain, None).await.unwrap();
    let error = fixture
        .workspaces
        .git_status(WorkspaceRoot::Project { project: plain.id })
        .await
        .unwrap_err();
    assert!(matches!(error, WorkspaceError::NotGit), "{error:?}");
}

/// A project's `vessel.toml` linked out of its folder (to a file of the
/// node's, say) is not read: `project.commands` names none of what the
/// other file holds, and answers as for no file, so it says nothing of
/// what exists outside.
#[cfg(unix)]
#[tokio::test]
async fn a_project_file_linked_out_of_its_folder_is_not_read() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let elsewhere = fixture.path().join("elsewhere.toml");
    std::fs::write(&elsewhere, "[worktree]\nsetup = \"echo not yours\"\n").unwrap();
    std::os::unix::fs::symlink(&elsewhere, project.root.join("vessel.toml")).unwrap();
    let commands = fixture.workspaces.commands(project.id, None).await.unwrap();
    assert_eq!(
        commands,
        ProjectCommands::None,
        "as if there were none, as for a link that leads nowhere"
    );
}

/// git is not run in a worktree whose `.git` was rewritten to name a
/// folder of someone else's (its configuration could name a program), nor
/// in a recorded folder replaced by a link: the calls are `conflict`, and
/// tidying keeps the worktree instead of inspecting it.
#[cfg(unix)]
#[tokio::test]
async fn a_worktree_redirected_or_replaced_is_not_worked_in() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let session = SessionId::random();
    let outcome = fixture
        .workspaces
        .create_worktree(
            Creator::Owner,
            CreateWorktree {
                project: project.id,
                branch: WorktreeBranch::New {
                    name: None,
                    base: None,
                },
                setup: SetupChoice::Run,
            },
            Some(session),
        )
        .await
        .unwrap();
    let CreateWorktreeOutcome::Created { worktree } = outcome else {
        panic!("{outcome:?}");
    };
    let root = WorkspaceRoot::Worktree {
        worktree: worktree.id,
    };
    let _clean = fixture.workspaces.git_status(root).await.unwrap();

    let gitfile = worktree.path.join(".git");
    let original = std::fs::read_to_string(&gitfile).unwrap();
    std::fs::create_dir(worktree.path.join("fake")).unwrap();
    std::fs::write(&gitfile, "gitdir: ./fake\n").unwrap();
    let refused = fixture.workspaces.git_status(root).await.unwrap_err();
    assert!(
        matches!(refused, WorkspaceError::Conflict(_)),
        "{refused:?}"
    );
    let kept = fixture.workspaces.retire(session, Vec::new).await.unwrap();
    assert_eq!(
        kept.outcome,
        RemoveWorktreeOutcome::Kept {
            reasons: vec![KeptReason::InspectionFailed]
        }
    );
    assert!(
        fixture
            .workspaces
            .tidy_candidates(TidyScope::Every)
            .is_empty(),
        "left to a person: a retry would run its teardown again"
    );
    std::fs::write(&gitfile, original).unwrap();
    std::fs::remove_dir(worktree.path.join("fake")).unwrap();

    // The project's folder replaced by a link to another.
    let other = fixture.path().join("other");
    std::fs::create_dir(&other).unwrap();
    let moved = fixture.path().join("moved");
    std::fs::rename(&project.root, &moved).unwrap();
    std::os::unix::fs::symlink(&other, &project.root).unwrap();
    let project_root = WorkspaceRoot::Project {
        project: project.id,
    };
    let refused = fixture
        .workspaces
        .git_status(project_root)
        .await
        .unwrap_err();
    assert!(
        matches!(refused, WorkspaceError::Conflict(_)),
        "{refused:?}"
    );
}

mod approvals;
mod git_apart;

mod project_admission;
