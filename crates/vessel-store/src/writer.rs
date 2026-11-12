//! The writer thread: the only code that writes the database after
//! startup.
//!
//! It takes every operation already queued (up to [`MAX_BATCH`]), applies
//! each in its own savepoint inside one transaction, runs retention for the
//! sessions it touched, commits once, and only then answers each caller.
//! A caller's `append` therefore returns when its records are committed,
//! which is what lets the node say "everything before seq N is durable".
//!
//! It is a plain thread rather than a task because every `SQLite` call blocks;
//! the queue is a bounded tokio channel so async callers wait on it without
//! blocking. Removing a removed session's attachment folder blocks too, and
//! happens here, after the commit that removed its row (`attachments`).

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Transaction};
use tokio::sync::{mpsc, oneshot};
use vessel_proto::{
    DeviceId, DeviceInfo, EventRecord, EventSeq, KeptMarks, Mark, SessionId, SessionInfo,
};

use crate::attachments::KeptAttachment;
use crate::codec::time_to_sql;
use crate::config::Retention;
use crate::devices::{self, CredentialHash};
use crate::error::StoreError;
use crate::log::{self, Appended};
use crate::{again, attachments, marks};

/// Most operations committed in one transaction. Bounds how long one
/// commit takes (and so the latency of the first operation in it).
const MAX_BATCH: usize = 256;

/// Answers one write operation once its batch is committed.
pub(crate) type Done<T = ()> = oneshot::Sender<Result<T, StoreError>>;

/// One item on the writer's queue.
#[derive(Debug)]
pub(crate) enum WriteOp {
    /// Change the database.
    Write(Write),
    /// Commit what came before, checkpoint the WAL into the main file, and
    /// stop. Writes queued after it are dropped, which answers their callers
    /// with `Closed`.
    Close { done: oneshot::Sender<()> },
}

/// A change to the database.
#[derive(Debug)]
pub(crate) enum Write {
    /// Store a new session. Boxed: operations wait in a bounded queue, and
    /// an inline `SessionInfo` would size every slot for this rare one.
    Create { info: Box<SessionInfo>, done: Done },
    /// Append records to a session's log. Shared with the session runtime's
    /// ring and subscribers, so persisting them copies nothing.
    Append {
        session: SessionId,
        records: Vec<Arc<EventRecord>>,
        done: Done,
    },
    /// Store a newly paired device.
    AddDevice {
        device: DeviceInfo,
        credential: CredentialHash,
        done: Done,
    },
    /// Note when a device last authenticated.
    DeviceSeen {
        device: DeviceId,
        at: DateTime<Utc>,
        done: Done,
    },
    /// Revoke a device (it stays as a tombstone).
    RevokeDevice {
        device: DeviceId,
        at: DateTime<Utc>,
        done: Done,
    },
    /// Set a mark on a session (protocol 10); answers with its marks after.
    Mark {
        session: SessionId,
        mark: Mark,
        at: DateTime<Utc>,
        done: Done<KeptMarks>,
    },
    /// Delete an ended session whole (protocol 10).
    Delete { session: SessionId, done: Done },
    /// Reopen an ended session's row for a run starting at record `next`
    /// (protocol 10's `session.resume`); answers with its marks after.
    Reopen {
        session: SessionId,
        next: EventSeq,
        done: Done<KeptMarks>,
    },
    /// Record an attachment whose file is in place and checked (protocol
    /// 10), kept at `at`. Boxed for the queue's slots, as `Create` is.
    Attach {
        kept: Box<KeptAttachment>,
        at: DateTime<Utc>,
        done: Done,
    },
    /// Change the record of projects and worktrees (protocol 10).
    Workspace {
        write: crate::workspace::WorkspaceWrite,
        done: Done,
    },
}

/// What the writer needs besides its connection and its queue.
#[derive(Debug)]
pub(crate) struct Setup {
    /// What retention keeps.
    pub(crate) retention: Retention,
    /// The data directory's `attachments/`, whose session folders go with
    /// their sessions.
    pub(crate) attachments: PathBuf,
}

/// One caller's answer, held until its batch is committed.
enum Answer {
    Done(Done, Result<(), StoreError>),
    Marked(Done<KeptMarks>, Result<KeptMarks, StoreError>),
}

impl Answer {
    /// Send it, as a failure if the commit failed.
    fn send(self, failed: Option<&Arc<rusqlite::Error>>) {
        let sent = match self {
            Self::Done(done, result) => done.send(after(failed, result)).is_ok(),
            Self::Marked(done, result) => done.send(after(failed, result)).is_ok(),
        };
        if !sent {
            tracing::trace!("store caller went away before its write committed");
        }
    }
}

/// `result`, unless the commit it waited for failed.
fn after<T>(
    failed: Option<&Arc<rusqlite::Error>>,
    result: Result<T, StoreError>,
) -> Result<T, StoreError> {
    match (failed, result) {
        (Some(error), Ok(_)) => Err(StoreError::Commit(Arc::clone(error))),
        (_, result) => result,
    }
}

/// What one batch changed beyond its own rows.
#[derive(Default)]
struct Effects {
    /// Sessions whose logs grew, for the per-session byte cap.
    touched: HashSet<SessionId>,
    /// A session ended, so retention may prune ended sessions.
    ended: bool,
    /// Sessions removed (deleted or pruned), whose attachment folders go
    /// once the batch is committed.
    removed: Vec<SessionId>,
}

/// Start the writer thread on `connection`.
pub(crate) fn spawn(
    connection: Connection,
    ops: mpsc::Receiver<WriteOp>,
    setup: Setup,
) -> Result<(), StoreError> {
    let _detached = std::thread::Builder::new()
        .name("vessel-store-writer".to_owned())
        .spawn(move || run(connection, ops, &setup))
        .map_err(StoreError::Thread)?;
    Ok(())
}

fn run(mut connection: Connection, mut ops: mpsc::Receiver<WriteOp>, setup: &Setup) {
    let mut batch = Vec::with_capacity(MAX_BATCH);
    let mut writes = Vec::with_capacity(MAX_BATCH);
    let mut closes = Vec::new();
    // Zero means every `Store` handle is gone.
    while ops.blocking_recv_many(&mut batch, MAX_BATCH) > 0 {
        for op in batch.drain(..) {
            match op {
                WriteOp::Write(write) if closes.is_empty() => writes.push(write),
                WriteOp::Write(_after_close) => {}
                WriteOp::Close { done } => closes.push(done),
            }
        }
        commit(&mut connection, writes.drain(..), setup);
        if !closes.is_empty() {
            break;
        }
    }
    checkpoint(&connection);
    for done in closes {
        if done.send(()).is_err() {
            tracing::trace!("close caller went away");
        }
    }
    tracing::debug!("event log writer stopped");
}

/// Fold the WAL back into the database file so a stopped node leaves one
/// self-contained file behind.
fn checkpoint(connection: &Connection) {
    if let Err(error) = connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)") {
        tracing::warn!(%error, "could not checkpoint the event log");
    }
}

/// Apply and commit one batch, remove what it removed, then answer every
/// caller in it.
fn commit(connection: &mut Connection, writes: impl Iterator<Item = Write>, setup: &Setup) {
    let mut answers = Vec::new();
    let mut effects = Effects::default();
    let committed = connection.transaction().and_then(|mut transaction| {
        for write in writes {
            answers.push(apply(&mut transaction, write, &mut effects));
        }
        retain(&mut transaction, &mut effects, &setup.retention);
        transaction.commit()
    });
    let failed = committed.err().map(Arc::new);
    match &failed {
        Some(error) => tracing::error!(%error, "committing to the event log failed"),
        // Only now are the rows gone for good; a failed commit kept them,
        // and with them their files.
        None => {
            for &session in &effects.removed {
                attachments::remove_folder(&setup.attachments, session);
            }
        }
    }
    for answer in answers {
        answer.send(failed.as_ref());
    }
}

/// Apply one write in its own savepoint, so a failing write leaves no
/// partial rows and does not affect the rest of the batch.
fn apply(transaction: &mut Transaction<'_>, write: Write, effects: &mut Effects) -> Answer {
    match write {
        Write::Create { info, done } => {
            let applied = isolated(transaction, |connection| {
                log::create_session(connection, &info)
            });
            Answer::Done(done, applied)
        }
        Write::Append {
            session,
            records,
            done,
        } => {
            let applied = isolated(transaction, |connection| {
                let Appended { ended: now_ended } =
                    log::append(connection, session, records.iter().map(Arc::as_ref))?;
                let _first = effects.touched.insert(session);
                effects.ended |= now_ended;
                Ok(())
            });
            Answer::Done(done, applied)
        }
        Write::AddDevice {
            device,
            credential,
            done,
        } => {
            let applied = isolated(transaction, |connection| {
                devices::insert(connection, &device, &credential)
            });
            Answer::Done(done, applied)
        }
        Write::DeviceSeen { device, at, done } => {
            let applied = isolated(transaction, |connection| {
                devices::seen(connection, device, at)
            });
            Answer::Done(done, applied)
        }
        Write::RevokeDevice { device, at, done } => {
            let applied = isolated(transaction, |connection| {
                devices::revoke(connection, device, at)
            });
            Answer::Done(done, applied)
        }
        Write::Mark {
            session,
            mark,
            at,
            done,
        } => {
            let applied = isolated(transaction, |connection| {
                marks::mark(connection, session, mark, at)
            });
            Answer::Marked(done, applied)
        }
        Write::Delete { session, done } => {
            let applied = isolated(transaction, |connection| marks::delete(connection, session));
            if applied.is_ok() {
                effects.removed.push(session);
            }
            Answer::Done(done, applied)
        }
        Write::Reopen {
            session,
            next,
            done,
        } => {
            let applied = isolated(transaction, |connection| {
                again::reopen(connection, session, next)
            });
            Answer::Marked(done, applied)
        }
        Write::Attach { kept, at, done } => {
            let applied = isolated(transaction, |connection| {
                attachments::insert(connection, &kept, time_to_sql(at)?)
            });
            Answer::Done(done, applied)
        }
        Write::Workspace { write, done } => {
            let applied = isolated(transaction, |connection| {
                crate::workspace::apply(connection, &write)
            });
            Answer::Done(done, applied)
        }
    }
}

fn isolated<T>(
    transaction: &mut Transaction<'_>,
    operation: impl FnOnce(&Connection) -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    let savepoint = transaction.savepoint()?;
    let done = operation(&savepoint)?;
    savepoint.commit()?;
    Ok(done)
}

/// Retention for this batch. Failures are logged, not returned: keeping a
/// bit more history than configured must never fail the appends.
fn retain(transaction: &mut Transaction<'_>, effects: &mut Effects, retention: &Retention) {
    for &session in &effects.touched {
        let pruned = isolated(transaction, |connection| {
            log::prune_session(connection, session, retention)
        });
        if let Err(error) = pruned {
            tracing::warn!(%session, %error, "could not prune session log");
        }
    }
    if effects.ended {
        let pruned = isolated(transaction, |connection| {
            log::prune_ended(connection, retention)
        });
        match pruned {
            Ok(pruned) => effects.removed.extend(pruned),
            Err(error) => tracing::warn!(%error, "could not prune ended sessions"),
        }
    }
}
