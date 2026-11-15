//! Complete storage-only result projection before retention or process admission.
use crate::StoreError;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use vessel_proto::checkpoint::RestoreOutcome;

/// The existing startup interrupt pass already ended previous live runs. Every
/// append and restore-row marker shares one FULL-sync transaction; a retry after
/// crash cannot append a second `FilesRestored` or alter status, end timestamp or queue.
pub(crate) fn replay_metadata(
    connection: &mut Connection,
    at: DateTime<Utc>,
) -> Result<(), StoreError> {
    let original: i64 = connection.pragma_query_value(None, "synchronous", |row| row.get(0))?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    let result: Result<(), StoreError> = (|| {
        let transaction = connection.transaction()?;
        let mut after = None;
        loop {
            let page =
                super::query::operation_page(&transaction, after, super::query::Scan::Unpublished)?;
            for operation in page.operations {
                if matches!(operation.outcome, RestoreOutcome::Restored { .. }) {
                    super::metadata::append_ended(&transaction, &operation, at)?;
                }
            }
            match page.next {
                Some(next) => after = Some(next),
                None => break,
            }
        }
        transaction.commit()?;
        Ok(())
    })();
    let reset = connection.pragma_update(None, "synchronous", original);
    match (result, reset) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
    }
}
