//! Diagnostic source used by the bounded node collector (ADR 0074).
//! Source owners omit private paths, arbitrary text and credentials.
//! [Decision Log]
//! Existing session enumeration clones private labels and state per row.
//! A single readonly transaction instead projects only schema and counts.
//! The caller's guard stays in the real blocking reader through cancellation.
//! A caller-owned cancellation check avoids a new product dependency or a
//! second cancellation engine; it must be nonblocking and do no source I/O.
//! SQL validates JSON syntax and recognized state tags/count consistency, not
//! each known status payload: this aggregate is not an integrity scan.
use crate::{Store, StoreError};
use rusqlite::{Connection, params};
use serde::Serialize;
use std::num::TryFromIntError;
use vessel_proto::{
    SessionStatus,
    diagnostics::{
        DiagnosticCount, DiagnosticSchemaVersion, DiagnosticStore, DiagnosticStoredSessions,
        InvalidDiagnostic, MAX_DIAGNOSTIC_COUNT, MAX_DIAGNOSTIC_SCHEMA,
    },
};

#[derive(Debug, thiserror::Error)]
enum Cause {
    #[error("diagnostic store reader failed")]
    Reader(#[from] StoreError),
    #[error("diagnostic aggregate query failed")]
    Query(#[from] rusqlite::Error),
    #[error("diagnostic status classification could not be encoded")]
    Encode(#[from] serde_json::Error),
    #[error("diagnostic count must be unsigned; saturation ceiling {max}", max = MAX_DIAGNOSTIC_COUNT)]
    Count(#[source] TryFromIntError),
    #[error("diagnostic schema must be unsigned and at most {max}", max = MAX_DIAGNOSTIC_SCHEMA)]
    Schema(#[source] TryFromIntError),
    #[error("diagnostic stored facts violate their bounded contract: {0}")]
    Invalid(#[from] InvalidDiagnostic),
    #[error("diagnostic supported schema could not be projected")]
    Version(#[source] crate::schema::DiagnosticSchemaError),
    #[error("the node is stopping")]
    Stopping,
}
// This is queued-work cancellation, never a permission grant or wire scope.
/// Caller cancellation policy checked before queued source I/O starts.
#[derive(Debug)]
pub enum ReadAdmission {
    /// The caller still permits queued work to start.
    Continue,
    /// Skip source I/O because shutdown has started.
    Stopping,
}

async fn guarded_snapshot<G, C>(
    store: &Store,
    guard: G,
    admission: C,
) -> Result<(DiagnosticStore, G), Cause>
where
    G: Send + 'static,
    C: FnOnce() -> ReadAdmission + Send + 'static,
{
    let (facts, guard) = store
        .read(move |reader| {
            if matches!(admission(), ReadAdmission::Stopping) {
                return Ok((Err(Cause::Stopping), guard));
            }
            // Keep the nested error so both the existing reader's failure
            // and the projection's own cause survive without new product
            // StoreError variants. The guard leaves this closure last.
            let facts = reader.with(|connection| Ok(snapshot(connection)))?;
            Ok((facts, guard))
        })
        .await?;
    Ok((facts?, guard))
}

#[derive(Serialize)]
struct StatusGroups {
    live: Vec<SessionStatus>,
    ended: Vec<SessionStatus>,
}
fn groups() -> Result<String, Cause> {
    // Serialize real variants instead of duplicating serde's status names.
    // The exhaustive match forces a new variant to be classified here.
    let representatives = [
        SessionStatus::Starting,
        SessionStatus::Running,
        SessionStatus::Exited { code: None },
        SessionStatus::failed(""),
        SessionStatus::Interrupted,
    ];
    let (live, ended) = representatives
        .into_iter()
        .partition(|status| match status {
            SessionStatus::Starting | SessionStatus::Running => true,
            SessionStatus::Exited { .. }
            | SessionStatus::Failed { .. }
            | SessionStatus::Interrupted => false,
        });
    Ok(serde_json::to_string(&StatusGroups { live, ended })?)
}
fn count(raw: i64) -> Result<DiagnosticCount, Cause> {
    let value = u64::try_from(raw).map_err(Cause::Count)?;
    Ok(DiagnosticCount::new(value))
}
fn snapshot(connection: &Connection) -> Result<DiagnosticStore, Cause> {
    let transaction = connection.unchecked_transaction()?;
    let observed: i64 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let classification = groups()?;
    let (total, live, ended): (i64, i64, i64) = transaction.query_row(
        "SELECT COUNT(*),
            COUNT(*) FILTER (WHERE json_extract(status, '$.state') IN
                (SELECT json_extract(value, '$.state') FROM json_each(?1, '$.live'))),
            COUNT(*) FILTER (WHERE json_extract(status, '$.state') IN
                (SELECT json_extract(value, '$.state') FROM json_each(?1, '$.ended')))
         FROM sessions",
        params![classification],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    if live.checked_add(ended) != Some(total) {
        return Err(InvalidDiagnostic::CountsDisagree.into());
    }
    // Getter is staged inside schema's owner too; no migration runs here.
    // It returns MIGRATIONS.len(), not another independently written number.
    let expected = crate::schema::diagnostic_version().map_err(Cause::Version)?;
    let observed = u32::try_from(observed).map_err(Cause::Schema)?;
    let facts = DiagnosticStore {
        expected_schema: expected,
        observed_schema: DiagnosticSchemaVersion::try_from(observed)?,
        sessions: DiagnosticStoredSessions {
            total: count(total)?,
            live: count(live)?,
            ended: count(ended)?,
        },
    };
    transaction.commit()?;
    Ok(facts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DataDir, StoreConfig};
    use chrono::Utc;
    use futures_util::FutureExt as _;
    use std::{
        future::Future,
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };
    use tempfile::TempDir;
    use tokio::sync::{Notify, oneshot};
    use vessel_proto::{
        EventRecord, EventSeq, HarnessKind, SessionEvent, SessionId, SessionInfo, SessionMode,
        SessionSpec, TerminalSize,
    };

    const SAFETY_NET: Duration = Duration::from_secs(60);
    const PHASE_DEADLINE: Duration = Duration::from_secs(20);

    fn run(test: impl Future<Output = ()>) -> impl Future<Output = ()> {
        // Box before constructing the timeout future, like the existing e2e step
        // helper, so nested Store futures do not inflate the test task's stack.
        let test = Box::pin(test);
        async move {
            vessel_testkit::within(SAFETY_NET, test)
                .await
                .unwrap_or_else(|error| {
                    panic!("diagnostic fixture exceeded {SAFETY_NET:?}: {error}")
                });
        }
    }
    fn phase<F: Future>(name: &str, work: F) -> impl Future<Output = F::Output> {
        let work = Box::pin(work);
        async move {
            tokio::time::timeout(PHASE_DEADLINE, work)
                .await
                .unwrap_or_else(|error| {
                    panic!("diagnostic phase {name} exceeded {PHASE_DEADLINE:?}: {error}")
                })
        }
    }
    const PRIVATE_WORDS: &str = "PRIVATE-DIAGNOSTIC-SOURCE-CANARY";
    async fn open() -> (TempDir, DataDir, Store) {
        let temp = tempfile::tempdir().unwrap();
        let dir = DataDir::new(temp.path());
        dir.prepare().unwrap();
        let store = Store::open(&dir, StoreConfig::default())
            .await
            .unwrap()
            .store;
        (temp, dir, store)
    }
    async fn insert(store: &Store, status: SessionStatus) -> SessionId {
        let id = SessionId::random();
        let spec = SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        )
        .with_cwd(PathBuf::from(PRIVATE_WORDS));
        store
            .create_session(SessionInfo::new(id, spec, Utc::now()))
            .await
            .unwrap();
        store
            .append(
                id,
                vec![Arc::new(EventRecord {
                    seq: EventSeq::new(0),
                    at: Utc::now(),
                    event: SessionEvent::Status { status },
                })],
            )
            .await
            .unwrap();
        id
    }
    #[tokio::test]
    async fn actual_store_counts_and_schema_do_not_return_rows_or_messages() {
        run(async {
            let (_temp, _dir, store) = phase("store opens", open()).await;
            let statuses = [
                SessionStatus::Starting,
                SessionStatus::Running,
                SessionStatus::Exited { code: Some(0) },
                SessionStatus::failed(PRIVATE_WORDS),
                SessionStatus::Interrupted,
            ];
            let total = u64::try_from(statuses.len()).unwrap();
            let live =
                u64::try_from(statuses.iter().filter(|status| status.is_live()).count()).unwrap();
            let mut ids = Vec::new();
            for status in statuses {
                ids.push(phase("session is inserted", insert(&store, status)).await);
            }
            let (facts, ()) = phase(
                "store snapshot",
                guarded_snapshot(&store, (), || ReadAdmission::Continue),
            )
            .await
            .unwrap();
            assert_eq!(facts.sessions.total.get(), total);
            assert_eq!(facts.sessions.live.get(), live);
            assert_eq!(facts.sessions.ended.get(), total.checked_sub(live).unwrap());
            assert_eq!(
                facts.expected_schema.get(),
                crate::schema::diagnostic_version().unwrap().get()
            );
            assert_eq!(facts.observed_schema, facts.expected_schema);
            assert!(
                !serde_json::to_string(&facts)
                    .unwrap()
                    .contains(PRIVATE_WORDS)
            );
            let encoded = serde_json::to_string(&facts).unwrap();
            assert!(ids.iter().all(|id| !encoded.contains(&id.to_string())));
            assert!(!format!("{facts:?}").contains(PRIVATE_WORDS));
            phase("store closes", store.close()).await;
        })
        .await;
    }
    #[tokio::test]
    async fn unknown_status_is_unavailable_instead_of_counted_as_ended() {
        run(async {
            let (_temp, dir, store) = phase("store opens", open()).await;
            let id = phase(
                "running session is inserted",
                insert(&store, SessionStatus::Running),
            )
            .await;
            let connection = Connection::open(dir.database_path()).unwrap();
            assert_eq!(
                connection
                    .execute(
                        "UPDATE sessions SET status = ?1 WHERE id = ?2",
                        params![r#"{"state":"future"}"#, id.to_string()],
                    )
                    .unwrap(),
                1,
            );
            let error = phase(
                "store snapshot",
                guarded_snapshot(&store, (), || ReadAdmission::Continue),
            )
            .await
            .unwrap_err();
            assert!(matches!(
                error,
                Cause::Invalid(InvalidDiagnostic::CountsDisagree)
            ));
            assert_eq!(
                connection
                    .execute(
                        "UPDATE sessions SET status = ?1 WHERE id = ?2",
                        params![
                            serde_json::to_string(&SessionStatus::Running).unwrap(),
                            id.to_string()
                        ],
                    )
                    .unwrap(),
                1,
            );
            phase("store closes", store.close()).await;
        })
        .await;
    }
    #[tokio::test]
    async fn schema_is_observed_without_migrating_or_claiming_integrity() {
        run(async {
            let (_temp, dir, store) = phase("store opens", open()).await;
            let before = crate::schema::diagnostic_version().unwrap();
            let changed = before.get().checked_add(1).unwrap();
            let connection = Connection::open(dir.database_path()).unwrap();
            connection
                .pragma_update(None, "user_version", changed)
                .unwrap();
            let (facts, ()) = phase(
                "store snapshot",
                guarded_snapshot(&store, (), || ReadAdmission::Continue),
            )
            .await
            .unwrap();
            assert_eq!(facts.expected_schema.get(), before.get());
            assert_eq!(facts.observed_schema.get(), changed);
            let after: u32 = connection
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .unwrap();
            assert_eq!(after, changed);
            phase("store closes", store.close()).await;
        })
        .await;
    }
    #[tokio::test]
    async fn failed_read_and_stopping_remain_distinct_with_internal_sources() {
        run(async {
            let (_temp, dir, store) = phase("store opens", open()).await;
            let connection = Connection::open(dir.database_path()).unwrap();
            let id = phase(
                "running session is inserted",
                insert(&store, SessionStatus::Running),
            )
            .await;
            assert_eq!(
                connection
                    .execute(
                        "UPDATE sessions SET status = ?1 WHERE id = ?2",
                        params!["not-json", id.to_string()],
                    )
                    .unwrap(),
                1,
            );
            let stopped = phase(
                "stopping store snapshot",
                guarded_snapshot(&store, (), || ReadAdmission::Stopping),
            )
            .await
            .unwrap_err();
            assert!(matches!(stopped, Cause::Stopping));
            let failed = phase(
                "store snapshot",
                guarded_snapshot(&store, (), || ReadAdmission::Continue),
            )
            .await
            .unwrap_err();
            assert!(matches!(failed, Cause::Query(_)));
            assert!(std::error::Error::source(&failed).is_some());
            assert_eq!(
                connection
                    .execute(
                        "UPDATE sessions SET status = ?1 WHERE id = ?2",
                        params![
                            serde_json::to_string(&SessionStatus::Running).unwrap(),
                            id.to_string()
                        ],
                    )
                    .unwrap(),
                1,
            );
            phase("store closes", store.close()).await;
        })
        .await;
    }
    struct Held {
        released: Arc<AtomicBool>,
        wake: Arc<Notify>,
    }
    impl Drop for Held {
        fn drop(&mut self) {
            self.released.store(true, Ordering::SeqCst);
            self.wake.notify_one();
        }
    }
    #[test]
    fn cancelling_a_queued_read_retains_guard_until_the_blocking_job_finishes() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .max_blocking_threads(1)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(run(async {
            let (_temp, _dir, store) = phase("store opens", open()).await;
            let (entered, ready) = oneshot::channel();
            let (release, finish) = std::sync::mpsc::channel();
            let blocker = tokio::task::spawn_blocking(move || {
                entered.send(()).unwrap();
                // Dropping the timed future closes its release sender before
                // custom runtime shutdown waits for this blocking operation.
                let _released = finish.recv();
            });
            phase("pool blocker enters", ready).await.unwrap();
            let released = Arc::new(AtomicBool::new(false));
            let wake = Arc::new(Notify::new());
            let held = Held {
                released: Arc::clone(&released),
                wake: Arc::clone(&wake),
            };
            let stop = Arc::new(AtomicBool::new(false));
            let job_stop = Arc::clone(&stop);
            // One poll reaches the real Store.read spawn_blocking and then
            // drops only the awaiting future; the pool is deterministically full.
            assert!(
                guarded_snapshot(&store, held, move || {
                    if job_stop.load(Ordering::SeqCst) {
                        ReadAdmission::Stopping
                    } else {
                        ReadAdmission::Continue
                    }
                })
                .now_or_never()
                .is_none()
            );
            assert!(!released.load(Ordering::SeqCst));
            stop.store(true, Ordering::SeqCst);
            release.send(()).unwrap();
            phase("pool blocker joins", blocker).await.unwrap();
            phase("cancelled read releases its guard", wake.notified()).await;
            assert!(released.load(Ordering::SeqCst));
            phase("store closes", store.close()).await;
        }));
    }
}

/// Store diagnostic failure. Display is content-free; its source is internal
/// diagnostic evidence and must never be rendered into a shareable report.
#[derive(Debug, thiserror::Error)]
#[error("diagnostic store snapshot failed")]
pub struct DiagnosticSnapshotError(#[source] Cause);

impl Store {
    /// Read schema and durable counts through the ordinary blocking reader.
    /// The guard stays in queued/running work even if the caller drops it.
    ///
    /// # Errors
    /// Source/read failures, stopping, invalid JSON/state tags or inconsistent
    /// counts. This is not a full store integrity scan.
    pub async fn diagnostic_snapshot<G, C>(
        &self,
        guard: G,
        admission: C,
    ) -> Result<(DiagnosticStore, G), DiagnosticSnapshotError>
    where
        G: Send + 'static,
        C: FnOnce() -> ReadAdmission + Send + 'static,
    {
        guarded_snapshot(self, guard, admission)
            .await
            .map_err(DiagnosticSnapshotError)
    }
}
