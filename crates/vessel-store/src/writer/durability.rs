//! A filesystem journal cannot acknowledge a commit that WAL NORMAL may lose.
//!
//! Only a batch carrying checkpoint work pays the FULL sync cost. The writer
//! changes this connection policy outside transactions and restores it even when
//! applying a batch fails. Ordinary event batching keeps the existing policy.
use super::{Answer, Write};
use crate::StoreError;
use rusqlite::Connection;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PreviousSync(i64);

pub(super) fn begin(
    connection: &Connection,
    writes: &[Write],
) -> rusqlite::Result<Option<PreviousSync>> {
    if !writes
        .iter()
        .any(|write| matches!(write, Write::Checkpoint { .. })
            || matches!(write, Write::Append { records, .. } if records.iter().any(|record| matches!(record.event, vessel_proto::SessionEvent::Checkpoint { .. }))))
    {
        return Ok(None);
    }
    let original = connection.pragma_query_value(None, "synchronous", |row| row.get(0))?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    Ok(Some(PreviousSync(original)))
}
pub(super) fn finish(
    connection: &Connection,
    previous: Option<PreviousSync>,
) -> rusqlite::Result<()> {
    if let Some(PreviousSync(previous)) = previous {
        connection.pragma_update(None, "synchronous", previous)?;
    }
    Ok(())
}

/// A failed durability admission must answer every caller without applying work.
pub(super) fn rejected(write: Write, error: &Arc<rusqlite::Error>) -> Answer {
    let failure = Err(StoreError::Commit(Arc::clone(error)));
    match write {
        Write::Detach { done, .. } => {
            Answer::Detached(done, Err(StoreError::Commit(Arc::clone(error))))
        }
        Write::Mark { done, .. } | Write::Reopen { done, .. } => {
            Answer::Marked(done, Err(StoreError::Commit(Arc::clone(error))))
        }
        Write::Checkpoint { done, .. }
        | Write::Create { done, .. }
        | Write::Append { done, .. }
        | Write::AddDevice { done, .. }
        | Write::DeviceSeen { done, .. }
        | Write::RevokeDevice { done, .. }
        | Write::Delete { done, .. }
        | Write::Attach { done, .. }
        | Write::Inbox { done, .. }
        | Write::Push { done, .. }
        | Write::Registry { done, .. }
        | Write::Workspace { done, .. }
        | Write::Forge { done, .. }
        | Write::Automation { done, .. } => Answer::Done(done, failure),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{checkpoint::CheckpointWrite, schema};
    use tokio::sync::oneshot;
    fn mode(connection: &Connection) -> i64 {
        connection
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .unwrap()
    }
    #[test]
    fn checkpoint_batch_commits_with_full_sync_and_restores_normal_policy() {
        let temp = tempfile::tempdir().unwrap();
        let mut connection = Connection::open(temp.path().join("journal.sqlite")).unwrap();
        schema::configure_writer(&connection).unwrap();
        let initial = mode(&connection);
        assert_eq!(initial, 1);
        let (done, _reply) = oneshot::channel();
        let writes = vec![Write::Checkpoint {
            write: Box::new(CheckpointWrite::InvalidateCandidates),
            done,
        }];
        let previous = begin(&connection, &writes).unwrap();
        assert_eq!(mode(&connection), 2);
        let transaction = connection.transaction().unwrap();
        transaction
            .execute_batch("CREATE TABLE evidence(id INTEGER);")
            .unwrap();
        transaction.commit().unwrap();
        finish(&connection, previous).unwrap();
        assert_eq!(mode(&connection), initial);
        drop(connection);
        let reopened = Connection::open(temp.path().join("journal.sqlite")).unwrap();
        assert_eq!(
            reopened
                .query_row("SELECT count(*) FROM evidence", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn checkpoint_ordinary_batch_does_not_change_sync_and_rollback_also_restores_policy() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "synchronous", "NORMAL")
            .unwrap();
        assert_eq!(begin(&connection, &[]).unwrap(), None);
        let (done, _reply) = oneshot::channel();
        let writes = vec![Write::Checkpoint {
            write: Box::new(CheckpointWrite::InvalidateCandidates),
            done,
        }];
        let previous = begin(&connection, &writes).unwrap();
        let transaction = connection.transaction().unwrap();
        transaction
            .execute_batch("CREATE TABLE rolled_back(id INTEGER);")
            .unwrap();
        transaction.rollback().unwrap();
        finish(&connection, previous).unwrap();
        assert_eq!(mode(&connection), 1);
        assert!(connection.prepare("SELECT * FROM rolled_back").is_err());
    }
    #[test]
    fn checkpoint_sync_admission_failure_answers_without_starting_a_transaction() {
        let mut connection = Connection::open_in_memory().unwrap();
        let transaction = connection.transaction().unwrap();
        let (done, mut reply) = oneshot::channel();
        let (detached, mut detached_reply) = oneshot::channel();
        let (registry, mut registry_reply) = oneshot::channel();
        let mut writes = vec![
            Write::Checkpoint {
                write: Box::new(CheckpointWrite::InvalidateCandidates),
                done,
            },
            Write::Detach {
                session: vessel_proto::SessionId::random(),
                at: chrono::DateTime::UNIX_EPOCH,
                done: detached,
            },
            Write::Registry {
                mutation: crate::registry::RegistryWrite::RecoverCreating,
                done: registry,
            },
        ];
        let error = Arc::new(begin(&transaction, &writes).unwrap_err());
        for write in writes.drain(..) {
            rejected(write, &error).send(None);
        }
        assert!(
            matches!(reply.try_recv().unwrap(), Err(StoreError::Commit(source)) if Arc::ptr_eq(&source, &error))
        );
        assert!(
            matches!(detached_reply.try_recv().unwrap(), Err(StoreError::Commit(source)) if Arc::ptr_eq(&source, &error))
        );
        assert!(
            matches!(registry_reply.try_recv().unwrap(), Err(StoreError::Commit(source)) if Arc::ptr_eq(&source, &error))
        );
        transaction.rollback().unwrap();
    }
}

#[cfg(test)]
mod checkpoint_log_tests {
    use super::*;
    #[test]
    fn actor_checkpoint_records_also_require_full_sync() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "synchronous", "NORMAL")
            .unwrap();
        let (done, _reply) = tokio::sync::oneshot::channel();
        let writes = [Write::Append {
            session: vessel_proto::SessionId::random(),
            records: vec![Arc::new(vessel_proto::EventRecord {
                seq: vessel_proto::EventSeq::ZERO,
                at: chrono::DateTime::UNIX_EPOCH,
                event: vessel_proto::SessionEvent::Checkpoint {
                    event: vessel_proto::checkpoint::CheckpointEvent::PromptAccepted,
                },
            })],
            done,
        }];
        let previous = begin(&connection, &writes).unwrap();
        assert!(previous.is_some());
        let sync: i64 = connection
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .unwrap();
        assert_eq!(sync, 2);
        finish(&connection, previous).unwrap();
    }
}
