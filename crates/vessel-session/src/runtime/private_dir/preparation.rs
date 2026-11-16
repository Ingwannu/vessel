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
/// How far past a deadline the paused clock jumps. The time driver rounds a
/// deadline up to its next millisecond tick, and a held blocking lookup
/// inhibits auto-advance, so a jump of exactly the budget can leave the
/// deadline one tick ahead with nothing left to move the clock.
const PAST_DEADLINE: Duration = Duration::from_secs(1);
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

/// One named, bounded step: a stall fails with the name of the phase that
/// stalled instead of the whole test hanging.
async fn phase<F: Future>(name: &str, future: F) -> F::Output {
    tokio::time::timeout(STEP, future)
        .await
        .unwrap_or_else(|_elapsed| panic!("phase `{name}` did not finish within {STEP:?}"))
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

// A paused clock needs the current-thread runtime, so this test cannot take
// the multi-thread flavour its neighbours use.
#[tokio::test]
async fn the_handshake_deadline_includes_held_preparation_without_releasing_its_owners() {
    // The outer bound is clock-aware and the test jumps the clock by
    // `PROBE_BUDGET + PAST_DEADLINE`, so the bound adds that jump: the test
    // still gets `SAFETY_NET` of real time, as its neighbours do.
    vessel_testkit::within(
        PROBE_BUDGET + PAST_DEADLINE + SAFETY_NET,
        Box::pin(async {
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
            // The handshake deadline is armed before the lookup, so it is
            // registered by the time the lookup arrives.
            let held = phase("the probe reaches the held lookup", barrier.arrived())
                .await
                .unwrap();
            // Jump past the deadline while the lookup is held, then resume at
            // once: no await runs on the paused clock, where the held blocking
            // lookup would keep auto-advance from moving it.
            tokio::time::pause();
            tokio::time::advance(PROBE_BUDGET + PAST_DEADLINE).await;
            tokio::time::resume();
            assert!(matches!(
                phase("the deadline expires while the lookup is held", probing)
                    .await
                    .unwrap(),
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
            drop(held);
            phase("the runtime drains once the lookup is released", drain).await;
            assert_eq!(slots.available_permits(), NonZeroUsize::MIN.get());
            assert!(runtime.pids().is_empty());
        }),
    )
    .await
    .expect("test timed out");
}
