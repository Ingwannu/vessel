//! Isolated real git/registry with no session, real agent or global env edit.
mod git;
use super::super::{Hosts, Mode, Outcome, ProjectPulls};
use super::deadline::step;
use crate::workspace::{Opening, Workspaces};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_forge::fake::FakeForge;
use vessel_proto::{ProjectId, workspace::ProjectInfo};
use vessel_store::{DataDir, Store, StoreConfig};

#[derive(Clone, Copy)]
pub(super) enum Initial {
    Committed,
    Unborn,
}

pub(super) struct Fixture {
    pub(super) temp: tempfile::TempDir,
    env: Vec<(String, OsString)>,
    pub(super) fake: Arc<FakeForge>,
    pub(super) hosts: Arc<Hosts>,
    pub(super) client: ProjectPulls,
    pub(super) tasks: TaskTracker,
    pub(super) stop: CancellationToken,
    pub(super) workspaces: Arc<Workspaces>,
}
impl Fixture {
    pub(super) async fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let config = temp.path().join("gitconfig");
        std::fs::write(&config, "[user]\nname = Test\nemail = test@example.invalid\n[init]\ndefaultBranch = main\n[commit]\ngpgsign = false\n[core]\nautocrlf = false\n").unwrap();
        let env = vec![
            ("GIT_CONFIG_GLOBAL".to_owned(), config.into_os_string()),
            ("GIT_CONFIG_NOSYSTEM".to_owned(), OsString::from("1")),
        ];
        let home = temp.path().join("home");
        std::fs::create_dir(&home).unwrap();
        let data = DataDir::new(home.join(".vessel"));
        data.prepare().unwrap();
        let store = step(
            "open fixture journal",
            Store::open(&data, StoreConfig::default()),
        )
        .await
        .unwrap()
        .store;
        let tasks = TaskTracker::new();
        let stop = CancellationToken::new();
        let workspaces = step(
            "start fixture workspace owner",
            Workspaces::start(
                Opening {
                    data: data.root().to_path_buf(),
                    workspace: data.prepare_workspace_dir().unwrap(),
                    home,
                    git: Some(which::which("git").unwrap()),
                    git_env: env.clone(),
                    removed: crate::TerminalsConfig::default().removed.into(),
                },
                store,
                tasks.clone(),
                stop.clone(),
            ),
        )
        .await
        .unwrap();
        let fake = FakeForge::new();
        let provider = Arc::clone(&fake);
        let hosts = Arc::new(Hosts::new(
            crate::forge::ForgeSetup {
                custom: Some(provider),
                ..Default::default()
            },
            temp.path().to_path_buf(),
        ));
        let client =
            ProjectPulls::start(Arc::clone(&workspaces), Arc::clone(&hosts), &tasks, &stop);
        Self {
            temp,
            env,
            fake,
            hosts,
            client,
            tasks,
            stop,
            workspaces,
        }
    }
    pub(super) async fn git(&self, dir: &Path, args: &[&str]) {
        git::run(dir, args, &self.env).await;
    }
    pub(super) async fn project(&self, name: &str, initial: Initial) -> ProjectInfo {
        let root: PathBuf = self.temp.path().join(name);
        std::fs::create_dir(&root).unwrap();
        self.git(&root, &["init", "-q"]).await;
        if matches!(initial, Initial::Committed) {
            self.git(&root, &["commit", "--allow-empty", "-q", "-m", "initial"])
                .await;
        }
        self.git(
            &root,
            &[
                "remote",
                "add",
                "origin",
                &format!("https://github.com/fixture/{name}.git"),
            ],
        )
        .await;
        step(
            "register fixture project",
            self.workspaces.add_project(root, None),
        )
        .await
        .unwrap()
    }
    pub(super) async fn read(&self, project: ProjectId, mode: Mode) -> Outcome {
        step(
            "project listing reply",
            self.client.read(project, None, mode),
        )
        .await
    }
    pub(super) async fn finish(self) {
        self.stop.cancel();
        let _closed = self.tasks.close();
        step("drain fixture actors", self.tasks.wait()).await;
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // A failed phase also closes owned async work; normal finish still
        // waits for it explicitly rather than treating cancellation as exit.
        self.stop.cancel();
    }
}
