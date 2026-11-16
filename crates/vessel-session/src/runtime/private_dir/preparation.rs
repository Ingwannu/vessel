//! Real launch lookup outlives a dropped waiter; its original owners must too.

use std::future::Future;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::task::Poll;

use tokio::sync::Semaphore;
use vessel_harness::fakes::{LookupArrival, LookupBarrier, hold_lookup};
use vessel_proto::settings::{AgentDriver, AgentDrivers};
use vessel_proto::{HarnessSessionId, SessionId, SessionInfo};

use super::*;

const STEP: Duration = Duration::from_secs(20);
const ADMISSION: usize = 2;
const PROBE_BUDGET: Duration = Duration::from_secs(3600);

fn held_fixture(kind: HarnessKind, role: Role, capacity: NonZeroUsize) -> (Fixture, LookupBarrier) {
    let mut fixture = Fixture::new();
    if kind == HarnessKind::Codex {
        drop(
            install(
                &fixture.dir.path().join("bin"),
                kind,
                Role::Terminal,
                &Fake::CodexAppServer.path().unwrap(),
            )
            .unwrap(),
        );
        fixture.runtime.set_drivers(AgentDrivers {
            codex: AgentDriver::Direct,
            ..AgentDrivers::default()
        });
    }
    let (lookup, barrier) = hold_lookup(
        fixture.runtime.config.harness_lookup.clone(),
        kind,
        role,
        capacity,
    );
    fixture.runtime.config.harness_lookup = lookup;
    (fixture, barrier)
}

async fn arrived(barrier: &mut LookupBarrier) -> LookupArrival {
    tokio::time::timeout(STEP, barrier.arrived())
        .await
        .unwrap()
        .unwrap()
}

async fn pending<F: Future>(future: Pin<&mut F>) {
    let mut future = future;
    assert!(std::future::poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx).is_pending())).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_probes_keep_existing_admission_and_hard_drain_through_lookup() {
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async {
            let (fixture, mut barrier) = held_fixture(
                HarnessKind::ClaudeCode,
                Role::Structured,
                NonZeroUsize::new(ADMISSION).unwrap(),
            );
            let work = fixture.work();
            let runtime = Arc::new(fixture.runtime);
            let slots = Arc::new(Semaphore::new(ADMISSION));
            let mut held = Vec::new();
            for _ in 0..ADMISSION {
                let runtime = Arc::clone(&runtime);
                let work = work.clone();
                let permit = Arc::clone(&slots).try_acquire_owned().unwrap();
                let probing = tokio::spawn(async move {
                    runtime
                        .probe_with_choices_owned(
                            HarnessKind::ClaudeCode.into(),
                            None,
                            None,
                            work,
                            super::super::agent::ProbeChoices::default(),
                            permit,
                        )
                        .await
                });
                held.push(arrived(&mut barrier).await);
                probing.abort();
                assert!(probing.await.unwrap_err().is_cancelled());
            }
            assert!(Arc::clone(&slots).try_acquire_owned().is_err());
            assert_eq!(barrier.entered().get(), ADMISSION);
            let mut drain = Box::pin(runtime.shutdown());
            pending(drain.as_mut()).await;
            drop(held);
            tokio::time::timeout(STEP, drain).await.unwrap();
            assert_eq!(slots.available_permits(), ADMISSION);
            assert!(runtime.pids().is_empty());
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_restart_keeps_the_same_conversation_claim_until_lookup_settles() {
    vessel_testkit::within(SAFETY_NET, Box::pin(async {
        let (fixture, mut barrier) = held_fixture(HarnessKind::ClaudeCode, Role::Structured, NonZeroUsize::MIN);
        let mut info = SessionInfo::new(SessionId::random(), fixture.spec(), chrono::Utc::now());
        info.harness_session = Some(HarnessSessionId::new("retained-preparation"));
        info.status = SessionStatus::Interrupted;
        let ended = Ended { info, choices: Vec::new(), queue: PromptQueue::default(), usage: None,
            attention: None, moved_through: BTreeSet::new() };
        let runtime = Arc::new(fixture.runtime);
        let starting = { let runtime = Arc::clone(&runtime); let ended = ended.clone();
            tokio::spawn(async move { runtime.start_again(ended, Relaunch::default()).await }) };
        let held = arrived(&mut barrier).await;
        starting.abort();
        assert!(starting.await.unwrap_err().is_cancelled());
        let original = ended.info.id;
        let mut other = ended;
        other.info.id = SessionId::random();
        let outcome = tokio::select! {
            result = runtime.start_again(other, Relaunch::default()) => result,
            _unexpected = barrier.arrived() => panic!("still-claimed conversation entered lookup"),
        };
        assert!(matches!(outcome, Err(crate::SessionError::ConversationTaken(id)) if id == original));
        let mut drain = Box::pin(runtime.shutdown());
        pending(drain.as_mut()).await;
        drop(held);
        tokio::time::timeout(STEP, drain).await.unwrap();
        assert!(runtime.pids().is_empty());
    })).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_native_version_lookup_retains_the_original_owner_and_runtime_drain() {
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async {
            let (fixture, mut barrier) =
                held_fixture(HarnessKind::Codex, Role::Terminal, NonZeroUsize::MIN);
            let work = fixture.work();
            let runtime = Arc::new(fixture.runtime);
            let slots = Arc::new(Semaphore::new(NonZeroUsize::MIN.get()));
            let permit = Arc::clone(&slots).try_acquire_owned().unwrap();
            let probing = {
                let runtime = Arc::clone(&runtime);
                tokio::spawn(async move {
                    runtime
                        .probe_with_choices_owned(
                            HarnessKind::Codex.into(),
                            None,
                            None,
                            work,
                            super::super::agent::ProbeChoices::default(),
                            permit,
                        )
                        .await
                })
            };
            let held = arrived(&mut barrier).await;
            probing.abort();
            assert!(probing.await.unwrap_err().is_cancelled());
            assert_eq!(slots.available_permits(), 0);
            let mut drain = Box::pin(runtime.shutdown());
            pending(drain.as_mut()).await;
            drop(held);
            tokio::time::timeout(STEP, drain).await.unwrap();
            assert_eq!(slots.available_permits(), NonZeroUsize::MIN.get());
            assert!(runtime.pids().is_empty());
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn the_handshake_deadline_includes_held_preparation_without_releasing_its_owners() {
    vessel_testkit::prepare().unwrap();
    let (mut fixture, mut barrier) =
        held_fixture(HarnessKind::ClaudeCode, Role::Structured, NonZeroUsize::MIN);
    fixture.runtime.config.agent_handshake_timeout = PROBE_BUDGET;
    let work = fixture.work();
    let runtime = Arc::new(fixture.runtime);
    let slots = Arc::new(Semaphore::new(NonZeroUsize::MIN.get()));
    let permit = Arc::clone(&slots).try_acquire_owned().unwrap();
    let probing = {
        let runtime = Arc::clone(&runtime);
        tokio::spawn(async move {
            runtime
                .probe_with_choices_owned(
                    HarnessKind::ClaudeCode.into(),
                    None,
                    None,
                    work,
                    super::super::agent::ProbeChoices::default(),
                    permit,
                )
                .await
        })
    };
    let held = arrived(&mut barrier).await;
    tokio::time::pause();
    tokio::time::advance(PROBE_BUDGET).await;
    assert!(matches!(
        probing.await.unwrap(),
        Err(crate::SessionError::Probe(
            vessel_agent::ProbeError::TimedOut {
                limit: PROBE_BUDGET,
                ..
            }
        ))
    ));
    assert_eq!(slots.available_permits(), 0);
    let mut drain = Box::pin(runtime.shutdown());
    pending(drain.as_mut()).await;
    tokio::time::resume();
    drop(held);
    tokio::time::timeout(STEP, drain).await.unwrap();
    assert_eq!(slots.available_permits(), NonZeroUsize::MIN.get());
    assert!(runtime.pids().is_empty());
}
