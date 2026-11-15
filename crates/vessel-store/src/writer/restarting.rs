//! Commit restart state/barriers through the writer's normal savepoint path.
use super::{Answer, Done, Effects, Indexer, isolated};
use crate::{
    log,
    restart::{self, RestartWrite},
    search::index,
};
use rusqlite::{Connection, Transaction};
use std::sync::Arc;
use vessel_proto::{EventRecord, SessionId, SessionInfo, restart::RestartCheckpoint};

#[derive(Debug)]
pub(crate) enum RestartOp {
    Change {
        write: RestartWrite,
        done: Done<bool>,
    },
    Checkpoint {
        session: SessionId,
        records: Vec<Arc<EventRecord>>,
        change: RestartCheckpoint,
        done: Done<bool>,
    },
}
pub(super) fn apply(
    transaction: &mut Transaction<'_>,
    operation: RestartOp,
    effects: &mut Effects,
    indexer: &mut Indexer,
) -> Answer {
    match operation {
        RestartOp::Change { write, done } => {
            let may_release = matches!(
                &write,
                RestartWrite::Policy { enabled: false } | RestartWrite::Finish { .. }
            );
            let applied = isolated(transaction, |connection| restart::write(connection, write));
            if applied.is_ok() && may_release {
                effects.ended = true;
            }
            Answer::Checked(done, applied)
        }
        RestartOp::Checkpoint {
            session,
            records,
            change,
            done,
        } => {
            let cached = indexer.take(session);
            let applied = isolated(transaction, |connection| {
                let log::Appended { ended } =
                    log::append(connection, session, records.iter().map(Arc::as_ref))?;
                let changed =
                    restart::write(connection, RestartWrite::Checkpoint { session, change })?;
                Ok((
                    changed,
                    ended,
                    index::on_append(connection, session, cached, &records),
                ))
            });
            let applied = applied.map(|(changed, ended, advanced)| {
                indexer.keep(session, advanced);
                effects.appended(session, &records, ended);
                changed
            });
            Answer::Checked(done, applied)
        }
    }
}

/// The caller's savepoint owns both inserts, so failure leaves no partial row.
pub(super) fn create(
    connection: &Connection,
    info: &SessionInfo,
    eligibility: crate::RestartEligibility,
) -> Result<(), crate::StoreError> {
    log::create_session(connection, info)?;
    if !restart::write(
        connection,
        RestartWrite::Configure {
            session: info.id,
            eligibility,
        },
    )? {
        return Err(crate::StoreError::UnknownSession(info.id));
    }
    Ok(())
}
