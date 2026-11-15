//! Applying one queued write inside the writer's batch transaction.
//!
//! Separate from the thread, batching and commit in `writer` so each file
//! keeps one responsibility within the file budget (docs/QUALITY.md):
//! this file maps each [`Write`] to its store operation, in its own
//! savepoint (`isolated`), and records what the batch must do after it.

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
use crate::{again, attachments, marks};

/// Apply one write in its own savepoint, so a failing write leaves no
/// partial rows and does not affect the rest of the batch.
pub(super) fn apply(
    transaction: &mut Transaction<'_>,
    write: Write,
    effects: &mut Effects,
    indexer: &mut Indexer,
) -> Answer {
    match write {
        Write::NotificationClaim {
            authority,
            source,
            done,
        } => notification_claim(transaction, authority, source, done),
        Write::Create { info, done } => write_answer(transaction, done, |connection| {
            log::create_session(connection, &info)
        }),
        Write::Append {
            session,
            records,
            done,
        } => {
            let applied = append(transaction, session, &records, effects, indexer);
            Answer::Done(done, applied)
        }
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
            done,
        } => marked_answer(transaction, effects, done, |connection| {
            marks::mark(connection, session, mark, at)
        }),
        Write::ReadAttention {
            session,
            through,
            at,
            done,
        } => marked_answer(transaction, effects, done, |connection| {
            read_attention(connection, session, through, at)
        }),
        Write::Detach { session, at, done } => {
            let applied = isolated(transaction, |connection| {
                crate::subagents::detach(connection, session, at)
            });
            Answer::Detached(done, applied)
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
        } => marked_answer(transaction, effects, done, |connection| {
            again::reopen(connection, session, next)
        }),
        Write::Attach { kept, at, done } => {
            let applied = isolated(transaction, |connection| {
                attachments::insert(connection, &kept, time_to_sql(at)?)
            });
            Answer::Done(done, applied)
        }
        Write::Inbox { mark, done } => {
            let applied = isolated(transaction, |connection| {
                crate::inbox::mark(connection, &mark)
            });
            effects.attention |= applied.is_ok();
            Answer::Done(done, applied)
        }
        Write::Push { write, done } => write_answer(transaction, done, |connection| {
            crate::push::apply(connection, &write)
        }),
        Write::Registry { mutation, done } => registry(transaction, &mutation, done, effects),
        Write::Workspace { write, done } => write_answer(transaction, done, |connection| {
            crate::workspace::apply(connection, &write)
        }),
        Write::Forge { write, done } => write_answer(transaction, done, |connection| {
            crate::forge::apply(connection, &write)
        }),
        Write::Automation { writes, done } => {
            let applied = isolated(transaction, |connection| {
                writes
                    .iter()
                    .try_for_each(|write| crate::automation::apply(connection, write))
            });
            Answer::Done(done, applied)
        }
    }
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

// Both paths keep their existing savepoint, answer and post-commit wakeup.
fn marked_answer(
    transaction: &mut Transaction<'_>,
    effects: &mut Effects,
    done: Done<vessel_proto::KeptMarks>,
    operation: impl FnOnce(&Connection) -> Result<vessel_proto::KeptMarks, StoreError>,
) -> Answer {
    let applied = isolated(transaction, operation);
    effects.attention |= applied.is_ok();
    Answer::Marked(done, applied)
}
fn read_attention(
    connection: &Connection,
    session: SessionId,
    through: vessel_proto::EventSeq,
    at: chrono::DateTime<chrono::Utc>,
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
) -> Result<(), StoreError> {
    let cached = indexer.take(session);
    let (now_ended, advanced) = isolated(transaction, |connection| {
        let Appended { ended } = log::append(connection, session, records.iter().map(Arc::as_ref))?;
        Ok((
            ended,
            index::on_append(connection, session, cached, records),
        ))
    })?;
    indexer.keep(session, advanced);
    let _first = effects.touched.insert(session);
    effects.ended |= now_ended;
    effects.attention |= records.iter().any(|record| {
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
    Ok(())
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
