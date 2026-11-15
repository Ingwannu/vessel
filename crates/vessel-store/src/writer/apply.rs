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
        Write::Timeline(write) => Answer::Timeline(super::timeline::apply(transaction, write)),
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
        Write::Inbox { mark, done } => write_answer(transaction, done, |connection| {
            crate::inbox::mark(connection, &mark)
        }),
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
    effects.attention |= records
        .iter()
        .any(|record| record.event.asks_for_attention());
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
