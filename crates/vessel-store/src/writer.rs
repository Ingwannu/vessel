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
//! What a session leaves outside the data directory's own folders (a
//! worktree, protocol 10) is the node's to tidy: after a commit in which
//! retention removed sessions, the writer says so ([`Setup::pruned`]).
//!
//! It also builds the search index (protocol 12, `search::index`): each
//! append's records are indexed in the append's own savepoint, and when
//! nothing is queued and some session's index is behind its log (sessions
//! stored before the index existed), it indexes one bounded step of it in
//! a transaction of its own before looking at the queue again.

mod apply;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Transaction};
use tokio::sync::{mpsc, oneshot, watch};
use vessel_proto::{
    DeviceId, DeviceInfo, EventRecord, EventSeq, KeptMarks, Mark, SessionId, SessionInfo,
};

use self::apply::{apply, isolated};
use crate::SessionCommit;
use crate::attachments::{self, KeptAttachment};
use crate::config::Retention;
use crate::devices::CredentialHash;
use crate::error::StoreError;
use crate::log;
use crate::search::idle::Indexer;

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
    /// Accepted label writes belong to the serial publication actor.
    WorkspaceLabels(Box<crate::workspace_labels::Write>),
    /// Store a new session. Boxed: operations wait in a bounded queue, and
    /// an inline `SessionInfo` would size every slot for this rare one.
    Create {
        info: Box<SessionInfo>,
        done: Done<SessionCommit<()>>,
    },
    /// Append records to a session's log. Shared with the session runtime's
    /// ring and subscribers, so persisting them copies nothing.
    Append {
        session: SessionId,
        records: Vec<Arc<EventRecord>>,
        done: Done<SessionCommit<()>>,
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
        residence: crate::Residence,
        done: Done<SessionCommit<KeptMarks>>,
    },
    /// Atomic first/repeated detach, replied only after commit.
    Detach {
        session: SessionId,
        at: DateTime<Utc>,
        done: Done<vessel_proto::subagents::DetachedSession>,
    },
    /// Delete an ended session whole (protocol 10).
    Delete { session: SessionId, done: Done },
    /// Reopen an ended session's row for a run starting at record `next`
    /// (protocol 10's `session.resume`); answers with its marks after.
    Reopen {
        session: SessionId,
        next: EventSeq,
        done: Done<SessionCommit<KeptMarks>>,
    },
    /// Record an attachment whose file is in place and checked (protocol
    /// 10), kept at `at`. Boxed for the queue's slots, as `Create` is.
    Attach {
        kept: Box<KeptAttachment>,
        at: DateTime<Utc>,
        done: Done,
    },
    /// Mark one exact inbox entry; a read never answers a request.
    Inbox {
        mark: vessel_proto::inbox::MarkInbox,
        residence: crate::Residence,
        done: Done<SessionCommit<()>>,
    },
    /// Device-scoped subscription changes; lease and scope checked atomically.
    Push {
        write: crate::push::PushWrite,
        done: Done,
    },
    /// Change independent workspace lifecycle/ownership in one savepoint.
    Registry {
        mutation: crate::registry::RegistryWrite,
        done: Done,
    },
    /// Find or create a project root's registry workspace (ADR 0106).
    Placement {
        register: Box<crate::registry::Register>,
        done: Done<Option<vessel_proto::WorkspaceId>>,
    },
    /// Change the record of projects and worktrees (protocol 10).
    Workspace {
        write: crate::workspace::WorkspaceWrite,
        done: Done,
    },
    /// Change a session's pull request feedback (protocol 11).
    Forge {
        write: crate::forge::ForgeWrite,
        done: Done,
    },
    /// Change the record of schedules, saved prompts and hooks (protocol
    /// 11): every change in `writes` or none.
    Automation {
        writes: Vec<crate::automation::AutomationWrite>,
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
    /// How many sessions retention has removed since the store opened,
    /// raised after each commit that removed some (protocol 10): the node
    /// then tidies the worktrees those sessions had. A count, not the ids:
    /// a watch keeps only the latest value and never fills, and the node
    /// looks for every worktree whose session is gone anyway.
    pub(crate) pruned: watch::Sender<u64>,
    /// Coalesced wake-up after a committed attention event, never raw output.
    pub(crate) inbox: watch::Sender<()>,
    /// Registry invalidation carries no data; consumers read a complete committed snapshot.
    pub(crate) registry: watch::Sender<()>,
    /// Whether some session's index is behind its log (protocol 12): true
    /// while the writer catches up, false once it waits on its queue with
    /// nothing behind.
    pub(crate) indexing: watch::Sender<bool>,
}

/// One caller's answer, held until its batch is committed.
enum Answer {
    LabelAssigned(
        Done<crate::WorkspaceLabelCommit<vessel_proto::workspace_labels::WorkspaceLabelAssigned>>,
        Result<
            crate::WorkspaceLabelCommit<vessel_proto::workspace_labels::WorkspaceLabelAssigned>,
            StoreError,
        >,
    ),
    LabelUpdated(
        Done<crate::WorkspaceLabelCommit<vessel_proto::workspace_labels::WorkspaceLabelUpdated>>,
        Result<
            crate::WorkspaceLabelCommit<vessel_proto::workspace_labels::WorkspaceLabelUpdated>,
            StoreError,
        >,
    ),
    LabelDeleted(
        Done<crate::WorkspaceLabelCommit<vessel_proto::workspace_labels::WorkspaceLabelDeleted>>,
        Result<
            crate::WorkspaceLabelCommit<vessel_proto::workspace_labels::WorkspaceLabelDeleted>,
            StoreError,
        >,
    ),
    Done(Done, Result<(), StoreError>),
    Session(
        Done<SessionCommit<()>>,
        Result<SessionCommit<()>, StoreError>,
    ),
    Marked(
        Done<SessionCommit<KeptMarks>>,
        Result<SessionCommit<KeptMarks>, StoreError>,
    ),
    Detached(
        Done<vessel_proto::subagents::DetachedSession>,
        Result<vessel_proto::subagents::DetachedSession, StoreError>,
    ),
    Placed(
        Done<Option<vessel_proto::WorkspaceId>>,
        Result<Option<vessel_proto::WorkspaceId>, StoreError>,
    ),
}

impl Answer {
    /// Send it, as a failure if the commit failed.
    fn send(self, failed: Option<&Arc<rusqlite::Error>>) {
        let sent = match self {
            Self::LabelAssigned(done, result) => done.send(after(failed, result)).is_ok(),
            Self::LabelUpdated(done, result) => done.send(after(failed, result)).is_ok(),
            Self::LabelDeleted(done, result) => done.send(after(failed, result)).is_ok(),
            Self::Detached(done, result) => done.send(after(failed, result)).is_ok(),
            Self::Done(done, result) => done.send(after(failed, result)).is_ok(),
            Self::Session(done, result) => done.send(after(failed, result)).is_ok(),
            Self::Marked(done, result) => done.send(after(failed, result)).is_ok(),
            Self::Placed(done, result) => done.send(after(failed, result)).is_ok(),
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
    registry: crate::registry::RegistryChange,
    /// Sessions whose logs grew, for the per-session byte cap.
    touched: HashSet<SessionId>,
    /// A committed batch introduced attention.
    attention: bool,
    /// A session ended, so retention may prune ended sessions.
    ended: bool,
    /// Sessions removed (deleted or pruned), whose attachment folders go
    /// once the batch is committed.
    removed: Vec<SessionId>,
    /// How many of them retention pruned.
    pruned: usize,
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
    let mut indexer = Indexer::new();
    while next_batch(&mut ops, &mut batch, &mut indexer, &mut connection, setup) {
        for op in batch.drain(..) {
            match op {
                WriteOp::Write(write) if closes.is_empty() => writes.push(write),
                WriteOp::Write(_after_close) => {}
                WriteOp::Close { done } => closes.push(done),
            }
        }
        commit(&mut connection, writes.drain(..), setup, &mut indexer);
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

/// Wait for the next operations, up to [`MAX_BATCH`]; `false` once every
/// `Store` handle is gone. While the search index is behind, an empty
/// queue is spent on one catch-up step at a time instead of waiting, so a
/// write that arrives waits at most one step; once nothing is behind, it
/// waits on the queue as before.
fn next_batch(
    ops: &mut mpsc::Receiver<WriteOp>,
    batch: &mut Vec<WriteOp>,
    indexer: &mut Indexer,
    connection: &mut Connection,
    setup: &Setup,
) -> bool {
    while indexer.maybe() {
        match ops.try_recv() {
            Ok(op) => {
                batch.push(op);
                while batch.len() < MAX_BATCH {
                    let Ok(op) = ops.try_recv() else { break };
                    batch.push(op);
                }
                return true;
            }
            Err(mpsc::error::TryRecvError::Empty) => {
                indexer.step(connection);
                publish(indexer, setup);
            }
            Err(mpsc::error::TryRecvError::Disconnected) => return false,
        }
    }
    // Zero means every `Store` handle is gone.
    ops.blocking_recv_many(batch, MAX_BATCH) > 0
}

/// Say whether idle work may be left ([`Setup::indexing`]).
fn publish(indexer: &Indexer, setup: &Setup) {
    let _changed = setup.indexing.send_if_modified(|indexing| {
        std::mem::replace(indexing, indexer.maybe()) != indexer.maybe()
    });
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
fn commit(
    connection: &mut Connection,
    writes: impl Iterator<Item = Write>,
    setup: &Setup,
    indexer: &mut Indexer,
) {
    let mut answers = Vec::new();
    let mut effects = Effects::default();
    let committed = connection.transaction().and_then(|mut transaction| {
        for write in writes {
            answers.push(apply(&mut transaction, write, &mut effects, indexer));
        }
        retain(&mut transaction, &mut effects, &setup.retention);
        transaction.commit()
    });
    let failed = committed.err().map(Arc::new);
    if let Some(error) = &failed {
        tracing::error!(%error, "committing to the event log failed");
        // The runs it kept were for rows that are now rolled back.
        indexer.forget_all();
    } else {
        // Said before any caller hears its write is in, so a caller that
        // then waits for the index waits for this write's messages too.
        indexer.committed();
        publish(indexer, setup);
        // Only now are the rows gone for good; a failed commit kept them,
        // and with them their files.
        for &session in &effects.removed {
            attachments::remove_folder(&setup.attachments, session);
        }
        if effects.registry == crate::registry::RegistryChange::Changed {
            setup.registry.send_replace(());
        }
        if effects.attention {
            setup.inbox.send_replace(());
        }
        if effects.pruned > 0 {
            // A count of events, so wrapping is harmless: readers only look
            // for a change.
            let pruned = u64::try_from(effects.pruned).unwrap_or(u64::MAX);
            setup
                .pruned
                .send_modify(|count| *count = count.wrapping_add(pruned));
        }
    }
    for answer in answers {
        answer.send(failed.as_ref());
    }
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
            Ok(pruned) => {
                effects.pruned = effects.pruned.saturating_add(pruned.len());
                effects.removed.extend(pruned);
            }
            Err(error) => tracing::warn!(%error, "could not prune ended sessions"),
        }
    }
}
