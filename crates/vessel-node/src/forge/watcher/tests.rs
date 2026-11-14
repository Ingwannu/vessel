//! A refresh before a session's first folder read (the module's second
//! Decision Log), driven step by step: the test answers the read and the
//! look itself, so the refresh always comes before the read, the race the
//! e2e `a_forge_that_is_down_holds_nothing_and_an_end_clears_it` lost on
//! a loaded node.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use tempfile::TempDir;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_forge::fake::{self, FakeForge};
use vessel_harness::Lookup;
use vessel_harness::fakes::{Role, install};
use vessel_proto::forge::{ForgeAccess, ForgeKind, PullNumber, PullRequestView, RepoRef};
use vessel_proto::{HarnessKind, ProjectId, SessionId, SessionMode, SessionSpec, TerminalSize};
use vessel_session::{
    DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_RING_CAPACITY, RuntimeConfig,
    SessionHandle, SessionRuntime,
};
use vessel_store::{DataDir, Store, StoreConfig};
use vessel_testkit::Fake;

use super::{Job, MAX_WAITING, Reply, Watcher};
use crate::forge::folder::Spot;
use crate::forge::hosts::Hosts;
use crate::forge::poll::Polled;
use crate::forge::schedule::{Done, FolderRead};
use crate::forge::{Command, ForgeSetup, Started};
use crate::workspace::{Opening, Workspaces};

struct Fixture {
    _data: TempDir,
    _bin: TempDir,
    runtime: Arc<SessionRuntime>,
    handle: SessionHandle,
    watcher: Watcher,
    /// Kept so the watcher's command channel stays open.
    _commands: mpsc::Sender<Command>,
}

impl Fixture {
    /// A watcher holding one running session a person started, whose
    /// folder has not been read yet.
    async fn new() -> Self {
        let data = tempfile::tempdir().unwrap();
        let bin = tempfile::tempdir().unwrap();
        let tui = Fake::Tui.path().unwrap();
        drop(install(bin.path(), HarnessKind::ClaudeCode, Role::Terminal, &tui).unwrap());
        let runtime = Arc::new(SessionRuntime::new(
            RuntimeConfig {
                harness_lookup: Lookup::new([bin.path().to_path_buf()]),
                harness_env: vessel_harness::HarnessEnv::new(data.path().to_owned()),
                ring_capacity: DEFAULT_RING_CAPACITY,
                durable_capacity: DEFAULT_DURABLE_CAPACITY,
                agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
                park_after: None,
            },
            CancellationToken::new(),
        ));
        let spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        )
        .with_cwd(data.path().to_path_buf());
        let handle = runtime.create(spec).await.unwrap().handle;
        std::fs::create_dir(data.path().join("node")).unwrap();
        let node = DataDir::new(data.path().join("node"));
        let store = Store::open(&node, StoreConfig::default())
            .await
            .unwrap()
            .store;
        let workspaces = Workspaces::start(
            Opening {
                data: node.root().to_path_buf(),
                workspace: node.prepare_workspace_dir().unwrap(),
                home: data.path().join("home"),
                git: None,
                git_env: Vec::new(),
                removed: crate::TerminalsConfig::default().removed.into(),
            },
            store.clone(),
            TaskTracker::new(),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        let hosts = Arc::new(Hosts::new(
            ForgeSetup {
                custom: Some(FakeForge::new()),
                ..ForgeSetup::default()
            },
            data.path().to_path_buf(),
        ));
        let (commands, received) = mpsc::channel(4);
        let mut watcher = Watcher::new(
            hosts,
            store,
            workspaces,
            received,
            Arc::new(AtomicU32::new(0)),
        );
        watcher.command(Command::Started(Box::new(Started {
            handle: handle.clone(),
            folder: data.path().to_path_buf(),
            project: ProjectId::random(),
            person: true,
        })));
        Self {
            _data: data,
            _bin: bin,
            runtime,
            handle,
            watcher,
            _commands: commands,
        }
    }

    fn session(&self) -> SessionId {
        self.handle.id()
    }

    /// A person's refresh, its answer still to come.
    fn refresh(&mut self) -> oneshot::Receiver<Option<PullRequestView>> {
        let (reply, answer): (Reply, _) = oneshot::channel();
        self.watcher.refresh(self.session(), true, Some(reply));
        answer
    }

    /// The session's first folder read comes back with `spot`.
    fn read(&mut self, spot: Result<Spot, ForgeAccess>) {
        let access = spot.is_ok().then_some(ForgeAccess::Ready { login: None });
        self.watcher.done(Done::Folder(Box::new(FolderRead {
            session: self.session(),
            spot,
            remembered: None,
            access,
            feedback: None,
        })));
    }

    async fn stop(self) {
        self.handle.kill();
        let _ended = self.handle.ended().await;
        self.runtime.shutdown().await;
    }
}

fn repo() -> RepoRef {
    RepoRef {
        kind: ForgeKind::Github,
        host: "github.com".parse().unwrap(),
        path: "owner/name".parse().unwrap(),
    }
}

fn spot() -> Spot {
    Spot {
        repo: repo(),
        branch: "feature".parse().unwrap(),
        head: "a".repeat(40).parse().unwrap(),
        remote: "origin".parse().unwrap(),
    }
}

/// A look at the watch for `spot()` that found pull request 5.
fn looked() -> Done {
    let spot = spot();
    let number = PullNumber::new(5).unwrap();
    let read = fake::pull(&spot.repo, number, &spot.branch, &spot.head).unwrap();
    Done::Polled(Box::new((
        spot.key(),
        Polled {
            listed: Vec::new(),
            found: true,
            number: Some(number),
            read: Some(read),
            changed: true,
            error: None,
        },
    )))
}

fn waiting(answer: &mut oneshot::Receiver<Option<PullRequestView>>) -> bool {
    matches!(answer.try_recv(), Err(oneshot::error::TryRecvError::Empty))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refresh_before_the_first_read_waits_for_it_and_gets_the_pull_request() {
    let mut fixture = Fixture::new().await;
    let session = fixture.session();
    let mut answer = fixture.refresh();
    assert!(waiting(&mut answer), "answered before the folder was read");
    let held = &fixture.watcher.sessions[&session];
    assert_eq!(held.refreshed, None, "no look happened, so none is stamped");
    assert_eq!(held.waiting.len(), 1);

    // The read puts the session on a watch: the refresh joins its next
    // look, made now and as a person's.
    fixture.read(Ok(spot()));
    assert!(waiting(&mut answer), "answered before the look");
    let watch = &fixture.watcher.watches[&spot().key()];
    assert_eq!(watch.replies.len(), 1);
    assert!(watch.urgent);
    assert!(watch.due <= tokio::time::Instant::now());
    assert!(fixture.watcher.sessions[&session].refreshed.is_some());

    fixture.watcher.done(looked());
    let view = answer.try_recv().unwrap().unwrap();
    assert_eq!(
        view.pull_request.map(|pull| pull.summary.number.get()),
        Some(5)
    );

    // Within 10 s of that look the next refresh is held to it, as before.
    let mut again = fixture.refresh();
    let held = again.try_recv().unwrap().unwrap();
    assert!(held.pull_request.is_some());
    fixture.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_first_read_that_finds_no_branch_answers_why() {
    let mut fixture = Fixture::new().await;
    let mut answer = fixture.refresh();
    fixture.read(Err(ForgeAccess::NoBranch));
    let view = answer.try_recv().unwrap().unwrap();
    assert_eq!(view.access, ForgeAccess::NoBranch);
    assert_eq!(view.pull_request, None);
    fixture.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_read_or_a_session_that_leaves_answers_every_waiting_refresh() {
    let mut fixture = Fixture::new().await;
    let session = fixture.session();
    // The read job panicked: the refresh gets the view as it is.
    let mut answer = fixture.refresh();
    fixture.watcher.failed(Job::Folder(session));
    let view = answer.try_recv().unwrap().unwrap();
    assert_eq!(view.access, ForgeAccess::Unknown);
    assert!(!fixture.watcher.sessions[&session].busy);

    // Archived before any read: not held.
    let mut gone = fixture.refresh();
    fixture.watcher.forget(session);
    assert_eq!(gone.try_recv().unwrap(), None);
    fixture.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn waiting_refreshes_are_bounded() {
    let mut fixture = Fixture::new().await;
    let mut kept: Vec<_> = (0..MAX_WAITING).map(|_| fixture.refresh()).collect();
    let mut over = fixture.refresh();
    let view = over.try_recv().unwrap().unwrap();
    assert_eq!(view.pull_request, None, "past the bound: the view as it is");
    assert!(kept.iter_mut().all(waiting));
    fixture.read(Ok(spot()));
    fixture.watcher.done(looked());
    for mut answer in kept {
        assert!(answer.try_recv().unwrap().unwrap().pull_request.is_some());
    }
    fixture.stop().await;
}
