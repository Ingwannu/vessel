//! Diagnostic source used by the bounded node collector (ADR 0074).
//! Source owners omit private paths, arbitrary text and credentials.
use super::SessionRuntime;
use std::sync::PoisonError;
use vessel_proto::diagnostics::{DiagnosticCount, DiagnosticRuntimeSessions};

impl SessionRuntime {
    /// Borrow live and parked counts without cloning session descriptions.
    /// This is one observation per actor, not a global transaction.
    #[must_use]
    pub fn diagnostic_counts(&self) -> DiagnosticRuntimeSessions {
        let handles = self.sessions.read().unwrap_or_else(PoisonError::into_inner);
        let mut counts = DiagnosticRuntimeSessions {
            live: DiagnosticCount::default(),
            parked: DiagnosticCount::default(),
        };
        for handle in handles.values() {
            let source = handle.watch_info();
            let info = source.borrow();
            // One borrowed value keeps each actor's status/parked pair
            // coherent; this is not a global transaction across actors.
            if info.status.is_live() {
                counts.live = counts.live.saturating_add(DiagnosticCount::new(1));
                if info.parked.is_some() {
                    counts.parked = counts.parked.saturating_add(DiagnosticCount::new(1));
                }
            }
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Created, RuntimeConfig, SessionHandle};
    use crate::{DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_RING_CAPACITY};
    use std::{path::PathBuf, sync::Arc, time::Duration};
    use tokio::sync::mpsc;
    use tokio_util::sync::{CancellationToken, DropGuard};
    use vessel_harness::{
        HarnessEnv, Lookup,
        fakes::{Role, install},
    };
    use vessel_proto::{EventRecord, HarnessKind, SessionMode, SessionSpec, SessionStatus};
    use vessel_testkit::Fake;

    const SAFETY_NET: Duration = Duration::from_secs(60);
    struct Fixture {
        _cancel: DropGuard,
        temp: tempfile::TempDir,
        runtime: SessionRuntime,
    }
    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let bin = temp.path().join("bin");
            std::fs::create_dir(&bin).unwrap();
            let _adapter = install(
                &bin,
                HarnessKind::ClaudeCode,
                Role::Structured,
                &Fake::Acp.path().unwrap(),
            )
            .unwrap();
            std::fs::create_dir(temp.path().join("private")).unwrap();
            std::fs::create_dir(temp.path().join("work")).unwrap();
            let stop = CancellationToken::new();
            let runtime = SessionRuntime::new(
                RuntimeConfig {
                    harness_lookup: Lookup::new([bin]),
                    harness_env: HarnessEnv::new(temp.path().to_owned()),
                    ring_capacity: DEFAULT_RING_CAPACITY,
                    durable_capacity: DEFAULT_DURABLE_CAPACITY,
                    agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
                    park_after: None,
                    private_dir: Some(temp.path().join("private")),
                },
                stop.clone(),
            );
            Self {
                _cancel: stop.drop_guard(),
                temp,
                runtime,
            }
        }
        fn work(&self) -> PathBuf {
            self.temp.path().join("work")
        }
        async fn running(&self) -> (SessionHandle, tokio::task::JoinHandle<()>) {
            let spec = SessionSpec::new(
                HarnessKind::ClaudeCode,
                SessionMode::Structured {
                    model: None,
                    resume: None,
                    options: Vec::new(),
                },
            )
            .with_cwd(self.work());
            let Created { handle, durable } = self.runtime.create(spec).await.unwrap();
            let drain = tokio::spawn(drain(durable));
            until(&handle, Target::Running).await;
            (handle, drain)
        }
    }
    #[derive(Clone, Copy)]
    enum Target {
        Running,
        Ended,
    }
    async fn until(handle: &SessionHandle, target: Target) {
        let mut source = handle.watch_info();
        loop {
            let reached = {
                let info = source.borrow_and_update();
                match target {
                    Target::Running => info.status == SessionStatus::Running,
                    Target::Ended => !info.status.is_live(),
                }
            };
            if reached {
                return;
            }
            source.changed().await.unwrap();
        }
    }
    async fn drain(mut source: mpsc::Receiver<Arc<EventRecord>>) {
        while source.recv().await.is_some() {}
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn real_fake_sessions_count_live_and_parked_but_not_retained_ended_handles() {
        vessel_testkit::within(SAFETY_NET, async {
            let fixture = Fixture::new();
            let before = fixture.runtime.diagnostic_counts();
            assert_eq!(before.live.get(), 0);
            let (first, first_drain) = fixture.running().await;
            let (second, second_drain) = fixture.running().await;
            let both = fixture.runtime.diagnostic_counts();
            assert_eq!(both.live.get(), 2);
            assert_eq!(both.parked.get(), 0);
            let parked = first.park().await.unwrap();
            assert!(parked.parked.is_some());
            let after_park = fixture.runtime.diagnostic_counts();
            assert_eq!(after_park.live.get(), 2);
            assert_eq!(after_park.parked.get(), 1);
            second.kill();
            until(&second, Target::Ended).await;
            assert!(fixture.runtime.get(second.id()).is_some());
            let retained = fixture.runtime.diagnostic_counts();
            assert_eq!(retained.live.get(), 1);
            assert_eq!(retained.parked.get(), 1);
            fixture.runtime.shutdown().await;
            first_drain.await.unwrap();
            second_drain.await.unwrap();
            let stopped = fixture.runtime.diagnostic_counts();
            assert_eq!(stopped.live.get(), 0);
            assert_eq!(stopped.parked.get(), 0);
        })
        .await
        .unwrap_or_else(|error| {
            panic!("runtime diagnostic fixture exceeded {SAFETY_NET:?}: {error}")
        });
    }
}
