//! Visibility of sessions while they are created, and when the store
//! cannot record one; what a session looks like once handed over.

mod handover;

use std::sync::Arc;

use tempfile::TempDir;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_harness::Lookup;
use vessel_harness::fakes::{Role, install};
use vessel_proto::{
    HarnessKind, SessionBoard, SessionId, SessionInfo, SessionInput, SessionMode, SessionSpec,
    SessionStatus, TerminalSize,
};
use vessel_session::{
    DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_RING_CAPACITY, LaunchOptions,
    Relaunch, RuntimeConfig, SessionRuntime,
};
use vessel_store::{DataDir, Store, StoreConfig};
use vessel_testkit::Fake;

use super::again::{StoredEnd, ready};
use super::{Found, Sessions};
use crate::error::ServiceError;

struct Fixture {
    data: TempDir,
    _bin: TempDir,
    runtime: Arc<SessionRuntime>,
    store: Store,
    sessions: Arc<Sessions>,
    /// Session creations, persistence and hand-overs.
    tasks: TaskTracker,
}

impl Fixture {
    async fn new() -> Self {
        let data = tempfile::tempdir().unwrap();
        let bin = tempfile::tempdir().unwrap();
        let tui = Fake::Tui.path().unwrap();
        drop(install(bin.path(), HarnessKind::ClaudeCode, Role::Terminal, &tui).unwrap());
        let acp = Fake::Acp.path().unwrap();
        drop(install(bin.path(), HarnessKind::ClaudeCode, Role::Structured, &acp).unwrap());
        let runtime = Arc::new(SessionRuntime::new(
            RuntimeConfig {
                harness_lookup: Lookup::new([bin.path().to_path_buf()]),
                harness_env: vessel_harness::HarnessEnv::new(data.path().to_owned()),
                ring_capacity: DEFAULT_RING_CAPACITY,
                durable_capacity: DEFAULT_DURABLE_CAPACITY,
                agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
                park_after: None,
                private_dir: None,
            },
            CancellationToken::new(),
        ));
        let store = Store::open(&DataDir::new(data.path()), StoreConfig::default())
            .await
            .unwrap()
            .store;
        let tasks = TaskTracker::new();
        let sessions = Arc::new(Sessions::new(
            Arc::clone(&runtime),
            store.clone(),
            tasks.clone(),
            Arc::new(crate::agents::Agents::new(vessel_proto::NodeId::random())),
            Arc::new(crate::models::Models::for_tests(&DataDir::new(data.path()))),
        ));
        Self {
            data,
            _bin: bin,
            runtime,
            store,
            sessions,
            tasks,
        }
    }

    fn spec(&self) -> SessionSpec {
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        )
        .with_cwd(self.data.path().to_path_buf())
    }

    fn structured(&self) -> SessionSpec {
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        )
        .with_cwd(self.data.path().to_path_buf())
    }

    fn listed(&self) -> Vec<SessionId> {
        self.runtime
            .list()
            .into_iter()
            .map(|info| info.id)
            .collect()
    }

    /// Wait until every session task (creation, persistence, hand-over)
    /// has finished.
    async fn settle(&self) {
        let _first_close = self.tasks.close();
        self.tasks.wait().await;
    }

    async fn stop(self) {
        self.runtime.shutdown().await;
        self.store.close().await;
    }
}

/// The runtime knows a session before the node has its entry (the window
/// between `runtime.create` and the durability table). Such a session is
/// neither listed nor found, so nothing can see it half created.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_runtime_session_without_an_entry_is_neither_listed_nor_found() {
    let fixture = Fixture::new().await;
    let created = fixture.runtime.create(fixture.spec()).await.unwrap();
    let id = created.handle.id();
    assert_eq!(fixture.listed(), [id]);

    let listed = fixture.sessions.list().await.unwrap();
    assert!(listed.iter().all(|info| info.id != id), "{listed:?}");
    assert!(matches!(
        fixture.sessions.find(id).await,
        Err(ServiceError::NotFound(missing)) if missing == id
    ));
    // Let it end without a persist task.
    created.handle.kill();
    drop(created.durable);
    fixture.stop().await;
}

/// A session the store cannot record is killed and disappears: it is not
/// listed, and once it has ended the runtime no longer has it either.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_the_store_cannot_record_is_killed_and_forgotten() {
    let fixture = Fixture::new().await;
    // Writes fail from here on; reads keep working.
    fixture.store.close().await;
    let refused = fixture
        .sessions
        .create(fixture.spec(), LaunchOptions::default(), None)
        .await;
    assert!(
        matches!(refused, Err(ServiceError::Store(_))),
        "{refused:?}"
    );
    assert!(fixture.sessions.list().await.unwrap().is_empty());

    // It ends (it was killed) and is removed from the runtime.
    fixture.settle().await;
    assert!(fixture.listed().is_empty());
    assert!(fixture.sessions.list().await.unwrap().is_empty());
    fixture.stop().await;
}

/// Once a session's whole log is committed it is handed over to the
/// store: the runtime frees it, and lookups, listing and input refusals
/// come from the store, with the final status.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_finished_session_is_handed_over_to_the_store() {
    let fixture = Fixture::new().await;
    let id = fixture
        .sessions
        .create(fixture.spec(), LaunchOptions::default(), None)
        .await
        .unwrap()
        .id;
    let Found::Live { handle, .. } = fixture.sessions.find(id).await.unwrap() else {
        panic!("a new session is live");
    };
    handle.kill();
    fixture.settle().await;

    assert!(fixture.runtime.get(id).is_none());
    let Found::Stored(stored) = fixture.sessions.find(id).await.unwrap() else {
        panic!("a finished session is served from the store");
    };
    assert!(
        matches!(stored.status, SessionStatus::Exited { .. }),
        "{stored:?}"
    );
    assert_eq!(stored.next_seq, handle.info().next_seq);
    the_store_places_it_as_the_session_did(&handle.info(), &stored);
    let listed = fixture.sessions.list().await.unwrap();
    assert_eq!(listed.iter().map(|info| info.id).collect::<Vec<_>>(), [id]);
    assert!(matches!(
        fixture.sessions.send(id, SessionInput::CancelTurn { turn: None }).await,
        Err(ServiceError::Ended(ended)) if ended == id
    ));
    fixture.stop().await;
}

/// Protocol 8, ADR 0019 (ADR 0026 for ended sessions): a session keeps its
/// place on the board across the hand-over: the store's and the session's
/// both come from `SessionBoard::exited` until it is archived.
fn the_store_places_it_as_the_session_did(live: &SessionInfo, stored: &SessionInfo) {
    let ended = stored.ended_at.expect("a stored ended session has its end");
    assert_eq!(stored.board, Some(SessionBoard::exited(ended)));
    assert_eq!(live.board, stored.board, "the same place as at its end");
}

/// `session.delete` takes only what the store serves: a session the
/// runtime has is live (`Live`), or has ended with its log still on its way
/// to the store, which delete waits for up to `HAND_OVER_WAIT` and then
/// refuses (`LogWriting`, a retry works). Here the log never completes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_the_runtime_has_is_not_deleted() {
    let fixture = Fixture::new().await;
    let created = fixture.runtime.create(fixture.spec()).await.unwrap();
    let id = created.handle.id();
    // The node's entry, as `start` makes it, with no persistence behind
    // it: its log never completes, so the node keeps serving it.
    let (_durability, watching) = tokio::sync::watch::channel(crate::persist::Durability::Through(
        vessel_proto::EventSeq::ZERO,
    ));
    drop(
        fixture
            .sessions
            .durability
            .write()
            .unwrap()
            .insert(id, watching),
    );
    assert!(matches!(
        fixture.sessions.delete(id).await,
        Err(ServiceError::Live(live)) if live == id
    ));
    created.handle.kill();
    drop(created.durable);
    let _final = created.handle.ended().await;
    assert!(matches!(
        fixture.sessions.delete(id).await,
        Err(ServiceError::LogWriting(writing)) if writing == id
    ));
    assert!(
        fixture.runtime.get(id).is_some(),
        "refused: nothing was removed"
    );
    fixture.stop().await;
}

/// Starting a stored session again (protocol 10): one call at a time
/// claims it; once the runtime has it again, a subscription the store
/// served ends `lagged`; once that run is handed over too, one that stopped
/// at the old end reads on, and one at the new end ends (review item 2).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stored_subscription_finds_a_session_started_again() {
    let fixture = Fixture::new().await;
    let id = fixture
        .sessions
        .create(fixture.structured(), LaunchOptions::default(), None)
        .await
        .unwrap()
        .id;
    let Found::Live { handle, .. } = fixture.sessions.find(id).await.unwrap() else {
        panic!("a new session is live");
    };
    let _running = ready(&handle).await.unwrap();
    handle.kill();
    fixture.settle().await;
    let end = fixture.sessions.get(id).await.unwrap().next_seq;
    assert_eq!(fixture.sessions.stored_end(id, end).await, StoredEnd::Ended);

    let claim = fixture.sessions.claim(id).unwrap();
    assert!(matches!(
        fixture.sessions.claim(id),
        Err(ServiceError::StartingAgain(claimed)) if claimed == id
    ));
    let ended = fixture.sessions.ended(id).await.unwrap();
    let handle = fixture
        .sessions
        .start_again(ended, Relaunch::default(), claim)
        .await
        .unwrap();
    assert_eq!(
        fixture.sessions.stored_end(id, end).await,
        StoredEnd::Reopened
    );
    assert!(matches!(
        fixture.sessions.ended(id).await,
        Err(ServiceError::AlreadyLive(live)) if live == id
    ));
    let again = ready(&handle).await.unwrap();
    assert_eq!(again.status, SessionStatus::Running);
    assert!(again.next_seq > end);

    handle.kill();
    fixture.settle().await;
    assert_eq!(
        fixture.sessions.stored_end(id, end).await,
        StoredEnd::Grew,
        "a run started and ended meanwhile"
    );
    let after = fixture.sessions.get(id).await.unwrap().next_seq;
    assert_eq!(
        fixture.sessions.stored_end(id, after).await,
        StoredEnd::Ended
    );
    fixture.stop().await;
}
