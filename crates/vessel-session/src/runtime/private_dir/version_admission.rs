//! A kept waiter must consult fresh admission after real lookup. An already
//! admitted version operation keeps its original capacity even after caller drop.

use std::future::Future;
use std::num::NonZeroUsize;
use std::task::Poll;

use tokio::io::AsyncWriteExt as _;
use tokio::sync::{Semaphore, watch};
use vessel_harness::fakes::{LookupBarrier, hold_lookup};
use vessel_proto::settings::{AgentDriver, AgentDrivers};

use super::*;
use crate::{
    PreparationAdmission, PreparationCancellation, PreparationEffect, PreparationGate,
    PreparationRefusal,
};

const STEP: Duration = Duration::from_secs(20);

fn native_fixture(wait: Option<&str>) -> (Fixture, Profile, PathBuf) {
    let fixture = Fixture::new();
    drop(
        install(
            &fixture.dir.path().join("bin"),
            HarnessKind::Codex,
            Role::Terminal,
            &Fake::CodexAppServer.path().unwrap(),
        )
        .unwrap(),
    );
    fixture.runtime.set_drivers(AgentDrivers {
        codex: AgentDriver::Direct,
        ..AgentDrivers::default()
    });
    let audit = fixture.dir.path().join("version-audit");
    let hold = wait.map_or(String::new(), |value| {
        format!(", FAKE_CODEX_VERSION_WAIT_STDIN = '{value}'")
    });
    let profiles = parse_profiles(&format!("[profiles.version-fixture]\nharness = 'codex'\nenv = {{ FAKE_CODEX_VERSION_AUDIT = '{audit}'{hold} }}", audit = audit.display()), Some(fixture.dir.path())).unwrap();
    let profile = profiles
        .get(&"version-fixture".parse().unwrap())
        .unwrap()
        .clone();
    (fixture, profile, audit)
}

fn spec(fixture: &Fixture, profile: &Profile) -> SessionSpec {
    SessionSpec::new(
        HarnessKind::Codex,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    )
    .with_cwd(fixture.work())
    .with_profile(profile.name().clone())
}

fn hold(fixture: &mut Fixture) -> LookupBarrier {
    let (lookup, barrier) = hold_lookup(
        fixture.runtime.config.harness_lookup.clone(),
        HarnessKind::Codex,
        Role::Terminal,
        NonZeroUsize::MIN,
    );
    fixture.runtime.config.harness_lookup = lookup;
    barrier
}

async fn pending<F: Future>(mut future: std::pin::Pin<&mut F>) {
    assert!(std::future::poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx).is_pending())).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kept_lookup_then_stop_refuses_without_starting_a_version_process() {
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async {
            let (mut fixture, profile, audit) = native_fixture(None);
            let mut barrier = hold(&mut fixture);
            let spec = spec(&fixture, &profile);
            let runtime = Arc::new(fixture.runtime);
            let cancel = CancellationToken::new();
            let gate = Arc::new(PreparationGate::new(
                cancel.clone(),
                |_effect, live: PreparationCancellation<'_>| match live.check() {
                    Ok(()) => PreparationAdmission::Admitted,
                    Err(reason) => PreparationAdmission::Refused(reason),
                },
            ));
            let slots = Arc::new(Semaphore::new(NonZeroUsize::MIN.get()));
            let permit = Arc::clone(&slots).try_acquire_owned().unwrap();
            let mut preparing = {
                let runtime = Arc::clone(&runtime);
                tokio::spawn(async move {
                    runtime
                        .agent_command_owned(
                            spec,
                            profile,
                            None,
                            super::super::prepared::RetainedPreparation {
                                owner: Arc::new(permit),
                                gate,
                                admission: runtime.launch_admission(),
                            },
                        )
                        .await
                        .map(|_command| ())
                })
            };
            let held = tokio::time::timeout(STEP, barrier.arrived())
                .await
                .unwrap()
                .unwrap();
            cancel.cancel();
            assert_eq!(slots.available_permits(), 0);
            // Caller cancellation is tested without node shutdown masking it.
            pending(std::pin::Pin::new(&mut preparing)).await;
            drop(held);
            let result = tokio::time::timeout(STEP, preparing)
                .await
                .unwrap()
                .unwrap();
            assert!(matches!(
                result,
                Err(crate::SessionError::Preparation(
                    PreparationRefusal::Cancelled
                ))
            ));
            tokio::time::timeout(STEP, runtime.shutdown())
                .await
                .unwrap();
            assert_eq!(slots.available_permits(), NonZeroUsize::MIN.get());
            assert!(!runtime.version_control.has_started());
            assert!(
                !audit.exists(),
                "kept preparation must not execute --version after stop"
            );
            assert_eq!(barrier.entered().get(), NonZeroUsize::MIN.get());
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fresh_fence_change_after_lookup_is_terminal_instead_of_an_acp_fallback() {
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async {
            for expected in [PreparationRefusal::Changed, PreparationRefusal::Expired] {
                let (mut fixture, profile, audit) = native_fixture(None);
                let mut barrier = hold(&mut fixture);
                let spec = spec(&fixture, &profile);
                let runtime = Arc::new(fixture.runtime);
                let changed = Arc::new(std::sync::atomic::AtomicBool::new(false));
                let observed = Arc::clone(&changed);
                let gate = Arc::new(PreparationGate::new(
                    CancellationToken::new(),
                    move |effect, live: PreparationCancellation<'_>| {
                        if let Err(reason) = live.check() {
                            return PreparationAdmission::Refused(reason);
                        }
                        if effect == PreparationEffect::Version
                            && observed.load(std::sync::atomic::Ordering::Acquire)
                        {
                            PreparationAdmission::Refused(expected)
                        } else {
                            PreparationAdmission::Admitted
                        }
                    },
                ));
                let preparing = {
                    let runtime = Arc::clone(&runtime);
                    tokio::spawn(async move {
                        runtime
                            .agent_command_owned(
                                spec,
                                profile,
                                None,
                                super::super::prepared::RetainedPreparation {
                                    owner: Arc::new(()),
                                    gate,
                                    admission: runtime.launch_admission(),
                                },
                            )
                            .await
                            .map(|_command| ())
                    })
                };
                let held = tokio::time::timeout(STEP, barrier.arrived())
                    .await
                    .unwrap()
                    .unwrap();
                changed.store(true, std::sync::atomic::Ordering::Release);
                drop(held);
                let result = tokio::time::timeout(STEP, preparing)
                    .await
                    .unwrap()
                    .unwrap();
                assert!(matches!(
                    result,
                    Err(crate::SessionError::Preparation(reason)) if reason == expected
                ));
                assert!(!runtime.version_control.has_started());
                assert!(!audit.exists());
                assert_eq!(barrier.entered().get(), NonZeroUsize::MIN.get());
                tokio::time::timeout(STEP, runtime.shutdown())
                    .await
                    .unwrap();
            }
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_host_pause_can_restore_without_permanent_cancellation() {
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async {
            let (mut fixture, profile, audit) = native_fixture(None);
            let mut barrier = hold(&mut fixture);
            let spec = spec(&fixture, &profile);
            let runtime = Arc::new(fixture.runtime);
            let (policy, _receiver) = watch::channel(());
            let watched = policy.clone();
            let paused = Arc::new(std::sync::atomic::AtomicBool::new(true));
            let observed = Arc::clone(&paused);
            let waiting = Arc::new(tokio::sync::Notify::new());
            let noticed = Arc::clone(&waiting);
            let gate = Arc::new(PreparationGate::new(
                CancellationToken::new(),
                move |effect, live: PreparationCancellation<'_>| {
                    let changed = watched.subscribe();
                    if let Err(reason) = live.check() {
                        return PreparationAdmission::Refused(reason);
                    }
                    if effect == PreparationEffect::Version
                        && observed.load(std::sync::atomic::Ordering::Acquire)
                    {
                        noticed.notify_one();
                        PreparationAdmission::Wait(changed)
                    } else {
                        PreparationAdmission::Admitted
                    }
                },
            ));
            let slots = Arc::new(Semaphore::new(NonZeroUsize::MIN.get()));
            let permit = Arc::clone(&slots).try_acquire_owned().unwrap();
            let preparing = {
                let runtime = Arc::clone(&runtime);
                tokio::spawn(async move {
                    runtime
                        .agent_command_owned(
                            spec,
                            profile,
                            None,
                            super::super::prepared::RetainedPreparation {
                                owner: Arc::new(permit),
                                gate,
                                admission: runtime.launch_admission(),
                            },
                        )
                        .await
                        .map(|command| command.driver)
                })
            };
            let held = tokio::time::timeout(STEP, barrier.arrived())
                .await
                .unwrap()
                .unwrap();
            drop(held);
            drop(barrier); // No further lookup hold after the forced physical preparation.
            tokio::time::timeout(STEP, waiting.notified())
                .await
                .unwrap();
            assert_eq!(slots.available_permits(), 0);
            assert!(!runtime.version_control.has_started());
            assert!(!audit.exists());
            paused.store(false, std::sync::atomic::Ordering::Release);
            policy.send_replace(());
            let driver = tokio::time::timeout(STEP, preparing)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(driver, vessel_agent::Driver::CodexAppServer);
            assert_eq!(std::fs::read_to_string(audit).unwrap(), "version\n");
            assert_eq!(slots.available_permits(), NonZeroUsize::MIN.get());
            tokio::time::timeout(STEP, runtime.shutdown())
                .await
                .unwrap();
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admitted_version_keeps_original_capacity_and_root_drain_after_caller_drop() {
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async {
            let (fixture, profile, audit) = native_fixture(Some("1"));
            let spec = spec(&fixture, &profile);
            let runtime = Arc::new(fixture.runtime);
            let started = runtime.version_control.arm();
            let slots = Arc::new(Semaphore::new(NonZeroUsize::MIN.get()));
            let permit = Arc::clone(&slots).try_acquire_owned().unwrap();
            let cancel = CancellationToken::new();
            let gate = Arc::new(PreparationGate::new(
                cancel.clone(),
                |_effect, live: PreparationCancellation<'_>| match live.check() {
                    Ok(()) => PreparationAdmission::Admitted,
                    Err(reason) => PreparationAdmission::Refused(reason),
                },
            ));
            let preparing = {
                let runtime = Arc::clone(&runtime);
                tokio::spawn(async move {
                    runtime
                        .agent_command_owned(
                            spec,
                            profile,
                            None,
                            super::super::prepared::RetainedPreparation {
                                owner: Arc::new(permit),
                                gate,
                                admission: runtime.launch_admission(),
                            },
                        )
                        .await
                        .map(|_command| ())
                })
            };
            let mut stdin = tokio::time::timeout(STEP, started).await.unwrap().unwrap();
            assert!(runtime.version_control.has_started());
            cancel.cancel();
            preparing.abort();
            assert!(preparing.await.unwrap_err().is_cancelled());
            assert_eq!(slots.available_permits(), 0);
            let mut draining = Box::pin(runtime.shutdown());
            pending(draining.as_mut()).await;
            stdin.write_all(b"release\n").await.unwrap();
            drop(stdin);
            tokio::time::timeout(STEP, draining).await.unwrap();
            assert_eq!(slots.available_permits(), NonZeroUsize::MIN.get());
            assert_eq!(std::fs::read_to_string(audit).unwrap(), "version\n");
            assert!(runtime.pids().is_empty());
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_stop_after_held_lookup_blocks_version_and_drains_original_capacity() {
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async {
            let (mut fixture, profile, audit) = native_fixture(None);
            let mut barrier = hold(&mut fixture);
            let spec = spec(&fixture, &profile);
            let runtime = Arc::new(fixture.runtime);
            let slots = Arc::new(Semaphore::new(NonZeroUsize::MIN.get()));
            let permit = Arc::clone(&slots).try_acquire_owned().unwrap();
            let preparing = {
                let runtime = Arc::clone(&runtime);
                tokio::spawn(async move {
                    let gate = runtime.preparation_gate();
                    runtime
                        .agent_command_owned(
                            spec,
                            profile,
                            None,
                            super::super::prepared::RetainedPreparation {
                                owner: Arc::new(permit),
                                gate,
                                admission: runtime.launch_admission(),
                            },
                        )
                        .await
                        .map(|_command| ())
                })
            };
            let held = tokio::time::timeout(STEP, barrier.arrived())
                .await
                .unwrap()
                .unwrap();
            let mut draining = Box::pin(runtime.shutdown());
            pending(draining.as_mut()).await;
            assert_eq!(slots.available_permits(), 0);
            drop(held);
            let result = tokio::time::timeout(STEP, preparing)
                .await
                .unwrap()
                .unwrap();
            assert!(matches!(
                result,
                Err(crate::SessionError::Preparation(
                    PreparationRefusal::Cancelled
                ))
            ));
            tokio::time::timeout(STEP, draining).await.unwrap();
            assert!(!runtime.version_control.has_started());
            assert!(!audit.exists());
            assert_eq!(slots.available_permits(), NonZeroUsize::MIN.get());
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_paused_gate_wakes_on_the_same_caller_cancel_without_policy_notification() {
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async {
            let (fixture, profile, audit) = native_fixture(None);
            let spec = spec(&fixture, &profile);
            let runtime = Arc::new(fixture.runtime);
            let cancel = CancellationToken::new();
            let (policy, _receiver) = watch::channel(());
            let gate = Arc::new(PreparationGate::new(
                cancel.clone(),
                move |_effect, live: PreparationCancellation<'_>| {
                    let changed = policy.subscribe();
                    match live.check() {
                        Err(reason) => PreparationAdmission::Refused(reason),
                        Ok(()) => PreparationAdmission::Wait(changed),
                    }
                },
            ));
            let mut preparing = Box::pin(runtime.agent_command_owned(
                spec,
                profile,
                None,
                super::super::prepared::RetainedPreparation {
                    owner: Arc::new(()),
                    gate,
                    admission: runtime.launch_admission(),
                },
            ));
            pending(preparing.as_mut()).await;
            cancel.cancel();
            let result = tokio::time::timeout(STEP, preparing).await.unwrap();
            assert!(matches!(
                result,
                Err(crate::SessionError::Preparation(
                    PreparationRefusal::Cancelled
                ))
            ));
            assert!(!runtime.version_control.has_started());
            assert!(!audit.exists());
            tokio::time::timeout(STEP, runtime.shutdown())
                .await
                .unwrap();
        }),
    )
    .await
    .unwrap();
}

mod view;
