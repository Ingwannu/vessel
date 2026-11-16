//! Admission identity tests; fake actors prove rejected handles stay owned until ending.
//! None of these observations certifies descendants or physical-root quiescence.
use super::*;
#[cfg(unix)]
use std::{future::Future, time::Duration};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

#[cfg(unix)]
const SAFETY_NET: Duration = Duration::from_secs(60);
#[cfg(unix)]
const OBSERVATION_WAIT: Duration = Duration::from_secs(10);
#[cfg(unix)]
const TRACKER_DRAIN_WAIT: Duration = Duration::from_secs(15);

struct Fixture {
    root: tempfile::TempDir,
    terminals: Arc<Terminals>,
    session: SessionId,
}
impl Fixture {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
            terminals: Terminals::new(Runner {
                config: Arc::new(super::super::TerminalsConfig::default()),
                shutdown: CancellationToken::new(),
                tasks: TaskTracker::new(),
            }),
            session: SessionId::random(),
        }
    }
    fn reserve(&self, id: TerminalId) -> Result<Reservation, TerminalsError> {
        self.terminals.reserve(Start {
            owner: Owner {
                session: Some(self.session),
                project: None,
                agent: None,
            },
            id,
            folder: self.root.path().to_owned(),
            run: None,
        })
    }

    #[cfg(unix)]
    async fn spawn(&self, id: TerminalId) -> TerminalHandle {
        use vessel_session::terminals::{Scrollback, ShellStart, start_shell};
        let program = vessel_testkit::Fake::Tui.path().unwrap();
        let handle = start_shell(ShellStart {
            info: TerminalInfo::new(
                id,
                vessel_proto::terminals::TerminalPlace::Session {
                    session: self.session,
                },
                self.root.path().to_owned(),
                vessel_proto::terminals::TerminalKind::Shell {
                    program: program.clone(),
                },
                chrono::Utc::now(),
            ),
            spec: vessel_proc::CommandSpec::new(program)
                .env_clear()
                .env("HOME", self.root.path().as_os_str())
                .env("TERM", "xterm-256color")
                .cwd(self.root.path()),
            size: vessel_proto::TerminalSize::default(),
            scrollback: Scrollback::DEFAULT,
            shutdown: self.terminals.runner.shutdown.clone(),
            tasks: self.terminals.runner.tasks.clone(),
        })
        .await
        .unwrap();
        until_text(&handle, "ready").await;
        handle
    }

    #[cfg(unix)]
    async fn stop(&self) {
        self.terminals.runner.shutdown.cancel();
        assert!(self.terminals.runner.tasks.close());
        phase(
            "tracked terminal task drain",
            TRACKER_DRAIN_WAIT,
            self.terminals.runner.tasks.wait(),
        )
        .await;
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.terminals.runner.shutdown.cancel();
    }
}

#[test]
fn duplicate_start_id_is_refused_without_replacing_the_original_guard() {
    let fixture = Fixture::new();
    let id = TerminalId::random();
    let reservation = fixture.reserve(id).unwrap();
    assert_eq!(reservation.id(), id);
    assert!(matches!(
        fixture.reserve(id),
        Err(TerminalsError::Conflict(_))
    ));
    assert_eq!(fixture.terminals.state().starting.len(), 1);
    drop(reservation);
    assert!(fixture.terminals.state().starting.is_empty());
    drop(fixture.reserve(id).unwrap());
}

#[test]
fn simultaneous_attempts_for_one_id_admit_only_one_guard() {
    let fixture = Fixture::new();
    let id = TerminalId::random();
    let start = Start {
        owner: Owner {
            session: None,
            project: None,
            agent: None,
        },
        id,
        folder: fixture.root.path().to_owned(),
        run: None,
    };
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let terminals = Arc::clone(&fixture.terminals);
        let barrier = Arc::clone(&barrier);
        let start = start.clone();
        workers.push(std::thread::spawn(move || {
            let _released = barrier.wait();
            terminals.reserve(start)
        }));
    }
    let _released = barrier.wait();
    let outcomes = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(fixture.terminals.state().starting.len(), 1);
    drop(outcomes);
    assert!(fixture.terminals.state().starting.is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn an_existing_id_is_refused_and_the_original_actor_is_untouched() {
    vessel_testkit::within(SAFETY_NET, async {
        let fixture = Fixture::new();
        let id = TerminalId::random();
        let reservation = fixture.reserve(id).unwrap();
        let handle = fixture.spawn(id).await;
        fixture
            .terminals
            .register(reservation, handle.clone(), None)
            .unwrap();
        assert!(matches!(
            fixture.reserve(id),
            Err(TerminalsError::Conflict(_))
        ));
        handle
            .send(vessel_proto::TerminalInput::Data {
                data: vessel_proto::TerminalBytes::new(bytes::Bytes::from_static(b"still here\r")),
            })
            .await
            .unwrap();
        until_text(&handle, "echo: still here").await;
        assert_eq!(fixture.terminals.get(id).unwrap().handle.id(), id);
        assert!(!handle.is_ended());
        fixture.stop().await;
    })
    .await
    .unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn wrong_id_is_refused_and_the_rejected_actor_is_cleaned_up() {
    vessel_testkit::within(SAFETY_NET, async {
        let fixture = Fixture::new();
        let expected = TerminalId::random();
        let actual = TerminalId::random();
        let reservation = fixture.reserve(expected).unwrap();
        let handle = fixture.spawn(actual).await;
        let observed_process = handle.process_lifetime().unwrap();
        let rejected = fixture
            .terminals
            .register(reservation, handle.clone(), None)
            .unwrap_err();
        let refusal = rejected.into_wire().await;
        assert_eq!(refusal.code, ErrorCode::Conflict);
        phase("rejected actor end", OBSERVATION_WAIT, handle.ended()).await;
        phase(
            "rejected process waiter",
            OBSERVATION_WAIT,
            observed_process.ended(),
        )
        .await;
        assert!(!observed_process.is_live());
        assert!(fixture.terminals.state().entries.is_empty());
        assert!(fixture.terminals.state().starting.is_empty());
        drop(fixture.reserve(expected).unwrap());
        fixture.stop().await;
    })
    .await
    .unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn a_foreign_registry_cannot_consume_another_incarnations_guard() {
    vessel_testkit::within(SAFETY_NET, async {
        let fixture = Fixture::new();
        let other = Terminals::new(fixture.terminals.runner.clone());
        let id = TerminalId::random();
        let reservation = fixture.reserve(id).unwrap();
        let handle = fixture.spawn(id).await;
        let rejected = other
            .register(reservation, handle.clone(), None)
            .unwrap_err();
        assert_eq!(rejected.into_wire().await.code, ErrorCode::Conflict);
        phase("rejected actor end", OBSERVATION_WAIT, handle.ended()).await;
        assert!(other.state().entries.is_empty());
        assert!(fixture.terminals.state().entries.is_empty());
        assert!(fixture.terminals.state().starting.is_empty());
        fixture.stop().await;
    })
    .await
    .unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn dropping_the_refusal_observer_does_not_drop_its_cleanup_owner() {
    vessel_testkit::within(SAFETY_NET, async {
        let fixture = Fixture::new();
        let expected = TerminalId::random();
        let reservation = fixture.reserve(expected).unwrap();
        let handle = fixture.spawn(TerminalId::random()).await;
        let rejected = fixture
            .terminals
            .register(reservation, handle.clone(), None)
            .unwrap_err();
        drop(rejected);
        phase("rejected actor end", OBSERVATION_WAIT, handle.ended()).await;
        fixture.stop().await;
        assert!(fixture.terminals.state().starting.is_empty());
    })
    .await
    .unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn a_misbound_incumbent_is_retained_instead_of_being_killed() {
    vessel_testkit::within(SAFETY_NET, async {
        let fixture = Fixture::new();
        let id = TerminalId::random();
        let reservation = fixture.reserve(id).unwrap();
        let handle = fixture.spawn(id).await;
        fixture
            .terminals
            .register(reservation, handle.clone(), None)
            .unwrap();
        let unrelated = fixture.reserve(TerminalId::random()).unwrap();
        let rejected = fixture
            .terminals
            .register(unrelated, handle.clone(), None)
            .unwrap_err();
        drop(rejected);
        handle
            .send(vessel_proto::TerminalInput::Data {
                data: vessel_proto::TerminalBytes::new(bytes::Bytes::from_static(b"kept alive\r")),
            })
            .await
            .unwrap();
        until_text(&handle, "echo: kept alive").await;
        assert_eq!(fixture.terminals.state().starting.len(), 1);
        assert!(!handle.is_ended());
        fixture.stop().await;
        assert!(fixture.terminals.state().starting.is_empty());
    })
    .await
    .unwrap();
}

#[cfg(unix)]
async fn until_text(handle: &TerminalHandle, expected: &str) {
    phase(expected, OBSERVATION_WAIT, observe_text(handle, expected)).await;
}

#[cfg(unix)]
async fn phase<F: Future>(name: &str, budget: Duration, future: F) -> F::Output {
    tokio::time::timeout(budget, future)
        .await
        .unwrap_or_else(|elapsed| panic!("{name} phase timed out: {elapsed}"))
}

#[cfg(unix)]
async fn observe_text(handle: &TerminalHandle, expected: &str) {
    let mut stream = handle.follow(vessel_proto::EventSeq::ZERO).unwrap();
    let mut text = String::new();
    for record in stream.cached {
        if let vessel_proto::SessionEvent::Terminal(output) = &record.event {
            text.push_str(&String::from_utf8_lossy(output.data.bytes()));
        }
    }
    while !text.contains(expected) {
        let record = stream.live.recv().await.unwrap();
        if let vessel_proto::SessionEvent::Terminal(output) = &record.event {
            text.push_str(&String::from_utf8_lossy(output.data.bytes()));
            assert!(
                text.len() < 8192,
                "fixture output exceeded its observation budget"
            );
        }
    }
}
