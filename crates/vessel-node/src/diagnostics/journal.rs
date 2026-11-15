//! Diagnostic source used by the bounded node collector (ADR 0074).
//! Source owners omit private paths, arbitrary text and credentials.
//! [Decision Log]
//! The guard's error must reach its caller without waiting for diagnostics.
//! A bounded actor therefore receives code/time only by `try_send`; overflow
//! increments a visible loss counter. Snapshot commands share that queue so
//! a report includes every earlier accepted observation without polling.
use chrono::{DateTime, Utc};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{mpsc, oneshot},
    time::Instant,
};
use tokio_util::{
    sync::{CancellationToken, DropGuard},
    task::TaskTracker,
};
use vessel_proto::{
    WireError,
    diagnostics::{
        DiagnosticCount, DiagnosticErrorCoverage, DiagnosticFailure, DiagnosticFailureWindow,
        DiagnosticUnavailable, InvalidDiagnostic, MAX_DIAGNOSTIC_COUNT, MAX_DIAGNOSTIC_FAILURES,
    },
};

/// Pending observations; overflow never delays an RPC.
const OBSERVATION_QUEUE: usize = 64;
/// Monotonic retention prevents wall-clock changes extending history.
const FAILURE_WINDOW: Duration = Duration::from_secs(900);

#[derive(Debug, Clone, Copy)]
struct Observation {
    error: DiagnosticFailure,
    instant: Instant,
}
#[derive(Debug)]
enum Command {
    Observe(Observation),
    Snapshot(oneshot::Sender<Result<DiagnosticFailureWindow, InvalidDiagnostic>>),
}
#[derive(Default)]
struct Omitted {
    value: AtomicU64,
    saturated: AtomicBool,
}
impl Omitted {
    fn add(&self) {
        if self
            .value
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value
                    .checked_add(1)
                    .filter(|next| *next <= MAX_DIAGNOSTIC_COUNT)
            })
            .is_err()
        {
            self.saturated.store(true, Ordering::Relaxed);
        }
    }
    fn count(&self) -> DiagnosticCount {
        let value = self.value.load(Ordering::Relaxed);
        DiagnosticCount::new(if self.saturated.load(Ordering::Relaxed) {
            MAX_DIAGNOSTIC_COUNT.saturating_add(1)
        } else {
            value
        })
    }
}
#[derive(Default)]
struct History {
    entries: VecDeque<Observation>,
}
impl History {
    fn keep(&mut self, observed: Observation, omitted: &Omitted) {
        self.entries
            .retain(|entry| fresh(entry.instant, observed.instant));
        if self.entries.len() == MAX_DIAGNOSTIC_FAILURES {
            let _evicted = self.entries.pop_back();
            omitted.add();
        }
        self.entries.push_front(observed);
    }
    fn snapshot(
        &self,
        now: Instant,
        since: DateTime<Utc>,
        omitted: &Omitted,
    ) -> Result<DiagnosticFailureWindow, InvalidDiagnostic> {
        let entries = self
            .entries
            .iter()
            .filter(|entry| fresh(entry.instant, now))
            .map(|entry| entry.error)
            .collect::<Vec<_>>()
            .try_into()?;
        Ok(DiagnosticFailureWindow {
            coverage: DiagnosticErrorCoverage::LocalRpcCompletions,
            observed_since: since,
            entries,
            omitted: omitted.count(),
        })
    }
}
fn fresh(at: Instant, now: Instant) -> bool {
    now.saturating_duration_since(at) < FAILURE_WINDOW
}

// Unexpected channel loss remains distinct from ordinary node shutdown.
#[derive(Debug, Clone, Copy)]
enum Closure {
    Stopping,
    Lost,
}
#[derive(Debug, thiserror::Error)]
enum SnapshotCause {
    #[error("the node is stopping")]
    Stopping,
    #[error("the diagnostic observation queue closed")]
    Request {
        when: Closure,
        #[source]
        source: mpsc::error::SendError<Command>,
    },
    #[error("the diagnostic observation reply closed")]
    Reply {
        when: Closure,
        #[source]
        source: oneshot::error::RecvError,
    },
    #[error("the diagnostic failure window could not be constructed: {0}")]
    Invalid(#[source] InvalidDiagnostic),
}
impl SnapshotCause {
    // The collector projects one closed section state at this privacy edge;
    // internal callers still have the bounded constructor/channel cause.
    fn unavailable(&self) -> DiagnosticUnavailable {
        match self {
            Self::Stopping
            | Self::Request {
                when: Closure::Stopping,
                ..
            }
            | Self::Reply {
                when: Closure::Stopping,
                ..
            } => DiagnosticUnavailable::Stopping,
            Self::Request {
                when: Closure::Lost,
                ..
            }
            | Self::Reply {
                when: Closure::Lost,
                ..
            }
            | Self::Invalid(_) => DiagnosticUnavailable::ReadFailed,
        }
    }
}

// Keep actor command/state types private while preserving the full cause.
// A collector needs only the closed unavailable projection.
#[derive(Debug, thiserror::Error)]
#[error("diagnostic failure snapshot failed")]
pub(super) struct SnapshotError(#[source] SnapshotCause);
impl SnapshotError {
    pub(super) fn unavailable(&self) -> DiagnosticUnavailable {
        self.0.unavailable()
    }
}

/// Nonblocking observer; tracked child task owns the history.
pub(super) struct Journal {
    send: mpsc::Sender<Command>,
    omitted: Arc<Omitted>,
    stop: CancellationToken,
    _cancel: DropGuard,
}
impl std::fmt::Debug for Journal {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("Journal")
            .field("omitted", &self.omitted.count())
            .finish_non_exhaustive()
    }
}
impl Journal {
    pub(super) async fn snapshot(&self) -> Result<DiagnosticFailureWindow, SnapshotError> {
        self.read_snapshot().await.map_err(SnapshotError)
    }
    pub(super) fn new(parent: &CancellationToken, tasks: &TaskTracker) -> Self {
        let stop = parent.child_token();
        let cancel = stop.clone().drop_guard();
        let (send, receive) = mpsc::channel(OBSERVATION_QUEUE);
        let omitted = Arc::new(Omitted::default());
        drop(tasks.spawn(run(receive, Arc::clone(&omitted), stop.clone(), Utc::now())));
        Self {
            send,
            omitted,
            stop,
            _cancel: cancel,
        }
    }
    pub(super) fn observe(&self, error: &WireError) {
        // Never clone error.message/detail, a method name or caller id.
        let event = Command::Observe(Observation {
            error: DiagnosticFailure {
                code: error.code,
                at: Utc::now(),
            },
            instant: Instant::now(),
        });
        match self.send.try_send(event) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_event)) => self.omitted.add(),
            Err(mpsc::error::TrySendError::Closed(_event)) => {
                // Best-effort observation never changes the original RPC.
                // A later snapshot reports the closed actor explicitly.
                self.omitted.add();
            }
        }
    }
    fn closure(&self) -> Closure {
        if self.stop.is_cancelled() {
            Closure::Stopping
        } else {
            Closure::Lost
        }
    }
    async fn read_snapshot(&self) -> Result<DiagnosticFailureWindow, SnapshotCause> {
        if self.stop.is_cancelled() {
            return Err(SnapshotCause::Stopping);
        }
        let (reply, result) = oneshot::channel();
        self.send
            .send(Command::Snapshot(reply))
            .await
            .map_err(|source| SnapshotCause::Request {
                when: self.closure(),
                source,
            })?;
        let snapshot = result.await.map_err(|source| SnapshotCause::Reply {
            when: self.closure(),
            source,
        })?;
        snapshot.map_err(SnapshotCause::Invalid)
    }
}
async fn run(
    mut receive: mpsc::Receiver<Command>,
    omitted: Arc<Omitted>,
    stop: CancellationToken,
    since: DateTime<Utc>,
) {
    let mut history = History::default();
    loop {
        let event = tokio::select! {
            biased;
            () = stop.cancelled() => break,
            event = receive.recv() => event,
        };
        match event {
            Some(Command::Observe(event)) => history.keep(event, &omitted),
            Some(Command::Snapshot(reply)) => {
                // Preserve a constructor failure internally; no raw RPC
                // message was ever retained in this actor.
                let snapshot = history.snapshot(Instant::now(), since, &omitted);
                if reply.send(snapshot).is_err() {
                    // Closing the report drops only its reply, not history.
                    tracing::debug!("diagnostic snapshot receiver closed");
                }
            }
            None => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::deadline::{self, phase};
    use super::*;
    use vessel_proto::ErrorCode;
    #[tokio::test]
    async fn overload_is_bounded_and_never_copies_the_message() {
        deadline::run(async {
            let stop = CancellationToken::new();
            let tasks = TaskTracker::new();
            let journal = Journal::new(&stop, &tasks);
            let secret = "NOT-FOR-A-DIAGNOSTIC-REPORT";
            // Current-thread runtime: no await permits the receiver to drain.
            for _ in 0..OBSERVATION_QUEUE.saturating_add(1) {
                journal.observe(&WireError::new(ErrorCode::Internal, secret));
            }
            let snapshot = phase("journal snapshot", journal.snapshot()).await.unwrap();
            assert_eq!(snapshot.entries.len(), MAX_DIAGNOSTIC_FAILURES);
            assert_eq!(
                snapshot.omitted.get(),
                u64::try_from(
                    OBSERVATION_QUEUE
                        .saturating_sub(MAX_DIAGNOSTIC_FAILURES)
                        .saturating_add(1)
                )
                .unwrap()
            );
            assert!(!serde_json::to_string(&snapshot).unwrap().contains(secret));
            drop(journal);
            assert!(tasks.close());
            phase("journal task drains", tasks.wait()).await;
        })
        .await;
    }
    #[test]
    fn retention_is_monotonic_without_sleeping() {
        let base = Instant::now();
        let later = base.checked_add(FAILURE_WINDOW).unwrap();
        let mut history = History::default();
        let omitted = Omitted::default();
        history.keep(
            Observation {
                error: DiagnosticFailure {
                    code: ErrorCode::Forbidden,
                    at: DateTime::UNIX_EPOCH,
                },
                instant: base,
            },
            &omitted,
        );
        assert!(
            history
                .snapshot(later, DateTime::UNIX_EPOCH, &omitted)
                .unwrap()
                .entries
                .is_empty()
        );
    }
    #[tokio::test]
    async fn restart_and_shutdown_are_explicit() {
        deadline::run(async {
            let stop = CancellationToken::new();
            let tasks = TaskTracker::new();
            let journal = Journal::new(&stop, &tasks);
            journal.observe(&WireError::new(ErrorCode::Forbidden, "private"));
            assert_eq!(
                phase("journal snapshot", journal.snapshot())
                    .await
                    .unwrap()
                    .entries
                    .len(),
                1
            );
            drop(journal);
            let next = Journal::new(&stop, &tasks);
            assert!(
                phase("restarted journal snapshot", next.snapshot())
                    .await
                    .unwrap()
                    .entries
                    .is_empty()
            );
            stop.cancel();
            assert!(tasks.close());
            phase("journal task drains", tasks.wait()).await;
            assert_eq!(
                phase("restarted journal snapshot", next.snapshot())
                    .await
                    .unwrap_err()
                    .unavailable(),
                DiagnosticUnavailable::Stopping,
            );
        })
        .await;
    }
    #[tokio::test]
    async fn unexpected_actor_loss_keeps_its_cause_and_is_not_empty_success() {
        deadline::run(async {
            let stop = CancellationToken::new();
            let (send, receive) = mpsc::channel(OBSERVATION_QUEUE);
            drop(receive);
            let journal = Journal {
                send,
                omitted: Arc::new(Omitted::default()),
                stop: stop.clone(),
                _cancel: stop.drop_guard(),
            };
            let error = phase("journal snapshot", journal.snapshot())
                .await
                .unwrap_err();
            assert_eq!(error.unavailable(), DiagnosticUnavailable::ReadFailed);
            assert!(std::error::Error::source(&error).is_some());
        })
        .await;
    }
    #[tokio::test]
    async fn cancelled_reply_does_not_remove_accepted_history() {
        deadline::run(async {
            let stop = CancellationToken::new();
            let tasks = TaskTracker::new();
            let journal = Journal::new(&stop, &tasks);
            journal.observe(&WireError::new(ErrorCode::Forbidden, "not retained"));
            let (reply, receiver) = oneshot::channel();
            drop(receiver);
            phase(
                "cancelled snapshot is queued",
                journal.send.send(Command::Snapshot(reply)),
            )
            .await
            .unwrap();
            let snapshot = phase("journal snapshot", journal.snapshot()).await.unwrap();
            assert_eq!(snapshot.entries.len(), 1);
            assert_eq!(
                snapshot.entries.as_slice().first().unwrap().code,
                ErrorCode::Forbidden
            );
            drop(journal);
            assert!(tasks.close());
            phase("journal task drains", tasks.wait()).await;
        })
        .await;
    }
    #[test]
    fn omission_count_marks_overflow_instead_of_wrapping_or_rounding() {
        let omitted = Omitted::default();
        omitted
            .value
            .store(MAX_DIAGNOSTIC_COUNT.saturating_sub(1), Ordering::Relaxed);
        omitted.add();
        assert_eq!(omitted.count().get(), MAX_DIAGNOSTIC_COUNT);
        assert!(!omitted.count().is_saturated());
        omitted.add();
        assert_eq!(omitted.count().get(), MAX_DIAGNOSTIC_COUNT);
        assert!(omitted.count().is_saturated());
    }
}
