//! Builders for sessions and records, shared by the store's tests
//! (`#[path]`). Included as a `cfg(test)` module so the lints' test
//! allowances (`unwrap` and friends) apply to it.

use std::path::PathBuf;
use std::sync::Arc;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use tempfile::TempDir;
use vessel_proto::{
    EventRecord, EventSeq, HarnessKind, SessionEvent, SessionId, SessionInfo, SessionMode,
    SessionSpec, SessionStatus, TerminalBytes, TerminalOutput, TerminalSize,
};
use vessel_store::{DataDir, Store, StoreConfig};

pub(crate) fn at(second: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000_i64.saturating_add(second), 123_456_789).unwrap()
}

pub(crate) fn data_dir() -> (TempDir, DataDir) {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    (temp, dir)
}

pub(crate) fn new_session(created: i64) -> SessionInfo {
    SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        )
        .with_cwd(PathBuf::from("/work")),
        at(created),
    )
}

pub(crate) fn record(seq: u64, event: SessionEvent) -> Arc<EventRecord> {
    Arc::new(EventRecord {
        seq: EventSeq::new(seq),
        at: at(i64::try_from(seq).unwrap()),
        event,
    })
}

pub(crate) fn status(seq: u64, status: SessionStatus) -> Arc<EventRecord> {
    record(seq, SessionEvent::Status { status })
}

/// A terminal chunk of `len` bytes (stored as about `4/3 * len` + 40).
pub(crate) fn output(seq: u64, len: usize) -> Arc<EventRecord> {
    let data = TerminalBytes::new(Bytes::from(vec![b'x'; len]));
    record(seq, SessionEvent::Terminal(TerminalOutput { data }))
}

pub(crate) fn plain(records: &[Arc<EventRecord>]) -> Vec<EventRecord> {
    records.iter().map(|record| (**record).clone()).collect()
}

pub(crate) async fn open(dir: &DataDir, config: StoreConfig) -> Store {
    Store::open(dir, config).await.unwrap().store
}

#[path = "registry_rollback.rs"]
mod registry_rollback;

/// Remove the later append before replaying a legacy migration fixture.
pub(crate) fn undo_registry_schema(connection: &rusqlite::Connection) {
    registry_rollback::undo_registry_schema(connection);
}
