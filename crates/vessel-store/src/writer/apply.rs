//! Applying one queued write inside the writer's batch transaction.
//!
//! Separate from the thread, batching and commit in `writer` so each file
//! keeps one responsibility within the file budget (docs/QUALITY.md):
//! this file maps each [`Write`] to its store operation, in its own
//! savepoint (`isolated`), and records what the batch must do after it.

mod workspace_labels;

use std::sync::Arc;

use rusqlite::{Connection, Transaction};
use vessel_proto::{EventRecord, SessionId};

use super::{Answer, Done, Effects, Write};
use crate::codec::time_to_sql;
use crate::devices;
use crate::error::StoreError;
use crate::log::{self, Appended};
use crate::search::idle::Indexer;
use crate::search::index;
use crate::{SessionCommit, again, attachments, marks};

/// Apply one write in its own savepoint, so a failing write leaves no
/// partial rows and does not affect the rest of the batch.
pub(super) fn apply(
    transaction: &mut Transaction<'_>,
    write: Write,
    effects: &mut Effects,
    indexer: &mut Indexer,
) -> Answer {
    match write {
        Write::WorkspaceLabels(write) => workspace_labels::apply(transaction, *write, effects),
        Write::NotificationClaim {
            authority,
            source,
            done,
        } => notification_claim(transaction, authority, source, done),
        Write::Restart(operation) => {
            super::restarting::apply(transaction, operation, effects, indexer)
        }
        Write::Create {
            info,
            eligibility,
            done,
        } => session_answer(transaction, info.id, done, |connection| {
            super::restarting::create(connection, &info, eligibility)
        }),
        Write::Append {
            session,
            records,
            done,
        } => Answer::Session(
            done,
            append(transaction, session, &records, effects, indexer),
        ),
        Write::AddDevice {
            device,
            credential,
            done,
        } => write_answer(transaction, done, |connection| {
            devices::insert(connection, &device, &credential)
        }),
        Write::DeviceSeen { device, at, done } => write_answer(transaction, done, |connection| {
            devices::seen(connection, device, at)
        }),
        Write::RevokeDevice { device, at, done } => write_answer(transaction, done, |connection| {
            devices::revoke(connection, device, at)
        }),
        Write::Mark {
            session,
            mark,
            at,
            residence,
            done,
        } => marks_answer(transaction, effects, session, done, |connection| {
            marks::mark(connection, session, mark, at, residence)
        }),
        Write::ReadAttention {
            session,
            through,
            at,
            residence,
            done,
        } => marks_answer(transaction, effects, session, done, |connection| {
            read_attention(connection, session, through, at, residence)
        }),
        Write::Detach { session, at, done } => {
            let applied = isolated(transaction, |connection| {
                crate::subagents::detach(connection, session, at)
            });
            Answer::Detached(done, applied)
        }
        Write::Delete { session, done } => delete_session(transaction, session, done, effects),
        Write::Reopen {
            session,
            next,
            mode,
            generation,
            done,
        } => marks_answer(transaction, effects, session, done, |connection| {
            again::reopen(connection, session, next, mode, generation)
        }),
        Write::Attach { kept, at, done } => write_answer(transaction, done, |connection| {
            attachments::insert(connection, &kept, time_to_sql(at)?)
        }),
        Write::Inbox {
            mark,
            residence,
            done,
        } => mark_inbox(transaction, &mark, residence, done, effects),
        Write::Push { write, done } => write_answer(transaction, done, |connection| {
            crate::push::apply(connection, &write)
        }),
        Write::Registry { mutation, done } => registry(transaction, &mutation, done, effects),
        Write::Workspace { write, done } => workspace(transaction, &write, done, effects),
        Write::Placement { register, done } => placement(transaction, &register, done, effects),
        Write::Forge { write, done } => write_answer(transaction, done, |connection| {
            crate::forge::apply(connection, &write)
        }),
        Write::Automation { writes, done } => write_answer(transaction, done, |connection| {
            writes
                .iter()
                .try_for_each(|write| crate::automation::apply(connection, write))
        }),
    }
}

fn delete_session(
    transaction: &mut Transaction<'_>,
    session: SessionId,
    done: Done,
    effects: &mut Effects,
) -> Answer {
    let applied = isolated(transaction, |connection| marks::delete(connection, session));
    if applied.is_ok() {
        effects.removed.push(session);
    }
    Answer::Done(done, applied)
}

/// An inbox change acknowledges its clock and wakes attention after commit (ADR 0089).
fn mark_inbox(
    transaction: &mut Transaction<'_>,
    mark: &vessel_proto::inbox::MarkInbox,
    residence: crate::Residence,
    done: Done<SessionCommit<()>>,
    effects: &mut Effects,
) -> Answer {
    let applied = isolated(transaction, |connection| {
        with_clock(connection, mark.key.session, |connection| {
            crate::inbox::mark(connection, mark, residence)
        })
    });
    effects.attention |= applied.is_ok();
    Answer::Session(done, applied)
}

/// Projects and worktrees; legacy project removal archives the project's
/// `Local` workspaces in the same savepoint (ADR 0106), so observers wake.
fn workspace(
    transaction: &mut Transaction<'_>,
    write: &crate::workspace::WorkspaceWrite,
    done: Done,
    effects: &mut Effects,
) -> Answer {
    let applied = isolated(transaction, |connection| {
        crate::workspace::apply(connection, write)
    });
    if matches!(applied, Ok(crate::registry::RegistryChange::Changed)) {
        effects.registry = crate::registry::RegistryChange::Changed;
    }
    Answer::Done(done, applied.map(drop))
}

/// Find or create a project root's workspace (ADR 0106); observers wake when
/// it is new.
fn placement(
    transaction: &mut Transaction<'_>,
    register: &crate::registry::Register,
    done: Done<Option<vessel_proto::WorkspaceId>>,
    effects: &mut Effects,
) -> Answer {
    let applied = isolated(transaction, |connection| {
        crate::registry::register(connection, register)
    });
    if matches!(
        applied,
        Ok(Some((_, crate::registry::RegistryChange::Changed)))
    ) {
        effects.registry = crate::registry::RegistryChange::Changed;
    }
    Answer::Placed(
        done,
        applied.map(|placed| placed.map(|(workspace, _)| workspace)),
    )
}

// Keep the claim's existing savepoint and post-commit answer unchanged while
// leaving the canonical writer dispatch below its per-function size budget.
fn notification_claim(
    transaction: &mut Transaction<'_>,
    authority: crate::NotificationAuthority,
    source: crate::PushAttention,
    done: Done<crate::LocalAdmission>,
) -> Answer {
    Answer::NotificationClaim(
        done,
        isolated(transaction, |connection| {
            crate::notification_claims::claim(connection, authority, source)
        }),
    )
}

/// `inbox.read` (ADR 0089): a passive Read mark, refused while a request
/// waits, through the same marks write and session mutation clock as
/// `session.mark`'s Read, so `residence` decides whether the clock ticks.
fn read_attention(
    connection: &Connection,
    session: SessionId,
    through: vessel_proto::EventSeq,
    at: chrono::DateTime<chrono::Utc>,
    residence: crate::Residence,
) -> Result<vessel_proto::KeptMarks, StoreError> {
    let waiting: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM inbox WHERE session=?1 AND waiting=1)",
        [session.to_string()],
        |row| row.get(0),
    )?;
    if waiting {
        return Err(StoreError::AttentionWaiting);
    }
    marks::mark(
        connection,
        session,
        vessel_proto::Mark::Read { through },
        at,
        residence,
    )
}

// State-only writes keep the same savepoint and acknowledgement boundary,
// apart from the session writes that also drive retention.
fn write_answer(
    transaction: &mut Transaction<'_>,
    done: Done,
    operation: impl FnOnce(&Connection) -> Result<(), StoreError>,
) -> Answer {
    Answer::Done(done, isolated(transaction, operation))
}

/// Apply a registry write in its savepoint; a change wakes the registry's
/// watchers once the batch commits.
fn registry(
    transaction: &mut Transaction<'_>,
    mutation: &crate::registry::RegistryWrite,
    done: Done,
    effects: &mut Effects,
) -> Answer {
    let applied = isolated(transaction, |connection| {
        crate::registry::apply(connection, mutation)
    });
    if matches!(applied, Ok(crate::registry::RegistryChange::Changed)) {
        effects.registry = crate::registry::RegistryChange::Changed;
    }
    Answer::Done(done, applied.map(drop))
}

/// Append a session's records and index them, in one savepoint. The
/// session's open run leaves the indexer's cache for the savepoint and
/// comes back only if it commits, so a rolled-back append never leaves a
/// run ahead of its row.
fn append(
    transaction: &mut Transaction<'_>,
    session: SessionId,
    records: &[Arc<EventRecord>],
    effects: &mut Effects,
    indexer: &mut Indexer,
) -> Result<SessionCommit<()>, StoreError> {
    let cached = indexer.take(session);
    let (now_ended, advanced, updated_at) = isolated(transaction, |connection| {
        let Appended { ended } = log::append(connection, session, records.iter().map(Arc::as_ref))?;
        let updated_at = crate::mutation_time::read(connection, session)?;
        Ok((
            ended,
            index::on_append(connection, session, cached, records),
            updated_at,
        ))
    })?;
    indexer.keep(session, advanced);
    effects.appended(session, records, now_ended);
    Ok(SessionCommit::new((), updated_at))
}

impl Effects {
    /// Both ordinary appends and checkpoint batches retain the head's post-commit wakes.
    pub(super) fn appended(
        &mut self,
        session: SessionId,
        records: &[Arc<EventRecord>],
        ended: bool,
    ) {
        let _first = self.touched.insert(session);
        self.ended |= ended;
        // ADR 0089 observes any event that changes outstanding attention.
        self.attention |= records.iter().any(|record| {
            record.event.asks_for_attention()
                || matches!(
                    &record.event,
                    vessel_proto::SessionEvent::Agent {
                        event: vessel_proto::AgentEvent::PermissionResolved { .. }
                            | vessel_proto::AgentEvent::QuestionResolved { .. }
                            | vessel_proto::AgentEvent::TurnStarted { .. }
                    } | vessel_proto::SessionEvent::Rewound { .. }
                )
        });
    }
}

pub(super) fn isolated<T>(
    transaction: &mut Transaction<'_>,
    operation: impl FnOnce(&Connection) -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    let savepoint = transaction.savepoint()?;
    let done = operation(&savepoint)?;
    savepoint.commit()?;
    Ok(done)
}

// Read within THIS operation's savepoint, before the writer applies another
// queued mutation; the normal Answer boundary still waits for outer commit.
fn with_clock<T>(
    connection: &Connection,
    session: SessionId,
    operation: impl FnOnce(&Connection) -> Result<T, StoreError>,
) -> Result<SessionCommit<T>, StoreError> {
    let value = operation(connection)?;
    let updated_at = crate::mutation_time::read(connection, session)?;
    Ok(SessionCommit::new(value, updated_at))
}

// These typed answer constructors share the same savepoint/clock routine;
// only their result variants differ. Neither changes commit-before-reply.
fn session_answer(
    transaction: &mut Transaction<'_>,
    session: SessionId,
    done: Done<SessionCommit<()>>,
    operation: impl FnOnce(&Connection) -> Result<(), StoreError>,
) -> Answer {
    let applied = isolated(transaction, |connection| {
        with_clock(connection, session, operation)
    });
    Answer::Session(done, applied)
}

/// A marks write with the session's mutation clock. A committed change
/// also wakes the attention observers (ADR 0089): a mark, a reopen or an
/// `inbox.read` can change a session's badges.
fn marks_answer(
    transaction: &mut Transaction<'_>,
    effects: &mut Effects,
    session: SessionId,
    done: Done<SessionCommit<vessel_proto::KeptMarks>>,
    operation: impl FnOnce(&Connection) -> Result<vessel_proto::KeptMarks, StoreError>,
) -> Answer {
    let applied = isolated(transaction, |connection| {
        with_clock(connection, session, operation)
    });
    effects.attention |= applied.is_ok();
    Answer::Marked(done, applied)
}
