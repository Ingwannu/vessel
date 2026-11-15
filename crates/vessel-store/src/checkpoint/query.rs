//! Typed reads use the store's existing blocking read pool, never another writer.
use super::{CheckpointStoreError, Operation, Plan, garbage, journal, paths};
use crate::{Store, StoreError};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::{
    EventSeq, SessionId,
    checkpoint::{
        CaptureKey, ChangeCursor, ChangePage, CheckpointCursor, CheckpointError, CheckpointId,
        CheckpointOwner, CheckpointPage, ObservedSnapshot, RestoreId, RestorePlanId,
    },
};

/// Immutable selected evidence returned to the node's preview/capture controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selected {
    /// Full original boundary identity, never inferred from a current turn.
    pub key: CaptureKey,
    /// Published checkpoint identity.
    pub checkpoint: CheckpointId,
    /// Validated whole image and stable object evidence.
    pub snapshot: ObservedSnapshot,
}

/// Keyset progress for internal startup reconciliation; not an RPC response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationPage {
    /// Authoritative operation snapshots, bounded by the checkpoint page limit.
    pub operations: Vec<Operation>,
    /// Continue even when a caller has no work for the outcomes on this page.
    pub next: Option<RestoreId>,
}

#[derive(Clone, Copy)]
enum Scan {
    Recovery,
    Unpublished,
}

fn operation_page(
    connection: &Connection,
    after: Option<RestoreId>,
    scan: Scan,
) -> Result<OperationPage, CheckpointStoreError> {
    let filter = match scan {
        Scan::Recovery => "phase='recovery'",
        Scan::Unpublished => "phase='complete' AND event_seq IS NULL",
    };
    let mut statement = connection.prepare(&format!(
        "SELECT id,session,owner FROM checkpoint_restores WHERE {filter} AND (?1 IS NULL OR id>?1) ORDER BY id LIMIT ?2",
    ))?;
    let limit = i64::try_from(vessel_proto::checkpoint::CHECKPOINT_PAGE_ENTRIES.saturating_add(1))
        .map_err(|_large| CheckpointError::TooManyPaths)?;
    let mut rows = statement
        .query_map(params![after.map(|id| id.to_string()), limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let more = rows.len() > vessel_proto::checkpoint::CHECKPOINT_PAGE_ENTRIES;
    rows.truncate(vessel_proto::checkpoint::CHECKPOINT_PAGE_ENTRIES);
    let next = if more {
        rows.last().map(|(id, _, _)| text(id)).transpose()?
    } else {
        None
    };
    let operations = rows
        .into_iter()
        .map(|(id, session, owner)| {
            journal::status(
                connection,
                text(&session)?,
                text(&id)?,
                serde_json::from_str(&owner)?,
            )
        })
        .collect::<Result<_, _>>()?;
    Ok(OperationPage { operations, next })
}

pub(super) fn text<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T, CheckpointStoreError> {
    Ok(T::deserialize(serde::de::value::StrDeserializer::<
        serde_json::Error,
    >::new(raw))?)
}

fn selected(
    connection: &Connection,
    session: SessionId,
    checkpoint: CheckpointId,
) -> Result<Selected, CheckpointStoreError> {
    let size: Option<i64> = connection.query_row(
        "SELECT length(CAST(image AS BLOB))+length(CAST(evidence AS BLOB)) FROM checkpoint_captures WHERE session=?1 AND checkpoint=?2 AND state='ready'",
        params![session.to_string(), checkpoint.to_string()], |row| row.get(0),
    ).optional()?;
    let size = u64::try_from(size.ok_or(CheckpointError::NotFound)?)
        .map_err(|_negative| CheckpointError::StorageFull)?;
    if size > super::quota::MANIFEST_BYTES {
        return Err(CheckpointError::StorageFull.into());
    }
    let (capture, worktree, generation, boundary, image, evidence): (String, String, String, String, String, String) = connection.query_row(
        "SELECT id,worktree,CAST(generation AS TEXT),boundary,image,evidence FROM checkpoint_captures WHERE session=?1 AND checkpoint=?2 AND state='ready'",
        params![session.to_string(), checkpoint.to_string()], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?)),
    ).optional()?.ok_or(CheckpointError::NotFound)?;
    Ok(Selected {
        key: CaptureKey {
            capture: text(&capture)?,
            session,
            worktree: text(&worktree)?,
            generation: serde_json::from_str(&generation)?,
            boundary: serde_json::from_str(&boundary)?,
        },
        checkpoint,
        snapshot: ObservedSnapshot::new(
            serde_json::from_str(&image)?,
            serde_json::from_str(&evidence)?,
        )?,
    })
}

impl Store {
    /// Read one bounded page of recovery-pinned operations before admitting writers.
    /// # Errors
    /// Malformed identities/evidence or read failure.
    pub async fn checkpoint_recovery_page(
        &self,
        after: Option<RestoreId>,
    ) -> Result<OperationPage, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| {
                operation_page(connection, after, Scan::Recovery).map_err(StoreError::from)
            })
        })
        .await
    }
    /// Scan terminal operations lacking a result-event marker after restart.
    /// Callers publish only Restored outcomes; other terminal outcomes need none.
    /// # Errors
    /// Malformed identities/evidence or read failure.
    pub async fn checkpoint_unpublished_page(
        &self,
        after: Option<RestoreId>,
    ) -> Result<OperationPage, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| {
                operation_page(connection, after, Scan::Unpublished).map_err(StoreError::from)
            })
        })
        .await
    }
    /// Read one keyset page of a session's checkpoint marks.
    /// # Errors
    /// Foreign/invalid cursor, malformed storage or read failure.
    pub async fn checkpoint_page(
        &self,
        session: SessionId,
        after: Option<CheckpointCursor>,
    ) -> Result<CheckpointPage, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| {
                super::list::page(connection, session, after).map_err(StoreError::from)
            })
        })
        .await
    }
    /// Read the immutable published image selected in an authorized session.
    /// # Errors
    /// Unknown/non-ready checkpoint or invalid evidence/storage.
    pub async fn checkpoint_selected(
        &self,
        session: SessionId,
        checkpoint: CheckpointId,
    ) -> Result<Selected, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| {
                selected(connection, session, checkpoint).map_err(StoreError::from)
            })
        })
        .await
    }
    /// Reload a principal-bound preview and its optional consuming operation.
    /// # Errors
    /// Foreign/unknown preview or malformed storage.
    pub async fn checkpoint_plan(
        &self,
        plan: RestorePlanId,
        owner: CheckpointOwner,
    ) -> Result<(Plan, Option<RestoreId>), StoreError> {
        self.read(move |reader| {
            reader.with(|connection| {
                super::plans::owned(connection, plan, owner).map_err(StoreError::from)
            })
        })
        .await
    }
    /// Read a page from the same frozen effective target shown for confirmation.
    /// # Errors
    /// Foreign, expired or consumed plan, invalid cursor or read failure.
    pub async fn checkpoint_changes(
        &self,
        plan: RestorePlanId,
        owner: CheckpointOwner,
        after: Option<ChangeCursor>,
        at: DateTime<Utc>,
    ) -> Result<ChangePage, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| {
                super::changes::page(connection, plan, owner, after, at).map_err(StoreError::from)
            })
        })
        .await
    }
    /// Reload authoritative status with the complete human principal binding.
    /// # Errors
    /// Foreign/unknown operation or malformed storage.
    pub async fn checkpoint_operation(
        &self,
        session: SessionId,
        restore: RestoreId,
        owner: CheckpointOwner,
    ) -> Result<Operation, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| {
                journal::status(connection, session, restore, owner).map_err(StoreError::from)
            })
        })
        .await
    }
    /// Read the bounded durable path journal after rechecking its owner.
    /// # Errors
    /// Foreign/unknown operation or malformed path journal.
    pub async fn checkpoint_paths(
        &self,
        operation: Operation,
    ) -> Result<Vec<super::PathIntent>, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| paths::list(connection, &operation).map_err(StoreError::from))
        })
        .await
    }
    /// Read the committed result-event marker; actor publication alone is insufficient.
    /// # Errors
    /// Foreign/unknown operation, mismatched plan or invalid sequence.
    pub async fn checkpoint_event_seq(
        &self,
        operation: Operation,
    ) -> Result<Option<EventSeq>, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| {
                let current =
                    journal::status(connection, operation.session, operation.id, operation.owner)?;
                if current.plan != operation.plan {
                    return Err(
                        CheckpointStoreError::from(CheckpointError::RestoreIdConflict).into(),
                    );
                }
                let seq: Option<i64> = connection.query_row(
                    "SELECT event_seq FROM checkpoint_restores WHERE id=?1",
                    [current.id.to_string()],
                    |row| row.get(0),
                )?;
                seq.map(crate::codec::seq_from_sql).transpose()
            })
        })
        .await
    }
    /// Read one bounded batch of garbage without releasing any reservation.
    /// # Errors
    /// Malformed accounting or read failure.
    pub async fn checkpoint_garbage(&self) -> Result<Vec<super::Garbage>, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| garbage::pending(connection).map_err(StoreError::from))
        })
        .await
    }
}
