//! Durable private-work reservations precede preview/restore filesystem allocation.
//! The caller retains the actual worker; SQL cannot certify that kernel I/O ended.
use super::{quota, records::CheckpointStoreError as Error};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension as _, params};
use serde::{Deserialize, Serialize};
use vessel_proto::checkpoint::{
    CHECKPOINT_IMAGE_BYTES, CaptureKey, CheckpointError, CheckpointOwner, RestoreId, RestorePlanId,
};

/// Private current-image, metadata and one in-flight candidate fit this envelope.
const PREVIEW_BYTES: u64 = CHECKPOINT_IMAGE_BYTES.saturating_mul(2);
/// Desired stage, before-image, retained original and journal fit this envelope.
const RESTORE_BYTES: u64 = CHECKPOINT_IMAGE_BYTES.saturating_mul(4);
/// Retain accounting for the immutable cleaned-id tombstone, never recycle it.
const TOMBSTONE_BYTES: u64 = 64 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
/// Non-reusable identity of a private filesystem allocation and its fixed escrow.
pub enum Allocation {
    /// Current-image and preview staging.
    Preview {
        /// Owning preview.
        plan: RestorePlanId,
    },
    /// Before-image, desired stage, displaced originals and restore journal.
    Restore {
        /// Owning restore.
        restore: RestoreId,
    },
}
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Reserved,
    Attached,
    Cleaned,
}
impl Phase {
    const fn label(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Attached => "attached",
            Self::Cleaned => "cleaned",
        }
    }
}
impl Allocation {
    const fn bytes(self) -> u64 {
        match self {
            Self::Preview { .. } => PREVIEW_BYTES,
            Self::Restore { .. } => RESTORE_BYTES,
        }
    }
}
fn encode<T: Serialize>(value: &T) -> Result<String, Error> {
    Ok(serde_json::to_string(value)?)
}
fn current(connection: &Connection, key: CaptureKey) -> Result<(), Error> {
    let ready: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM checkpoint_captures WHERE id=?1 AND session=?2 AND worktree=?3 AND generation=?4 AND boundary=?5 AND state='ready')",
        params![key.capture.to_string(), key.session.to_string(), key.worktree.to_string(), encode(&key.generation)?, encode(&key.boundary)?], |row| row.get(0),
    )?;
    if ready {
        Ok(())
    } else {
        Err(CheckpointError::StaleCapture.into())
    }
}
pub(crate) fn reserve(
    connection: &Connection,
    key: CaptureKey,
    allocation: Allocation,
    owner: CheckpointOwner,
    at: DateTime<Utc>,
) -> Result<(), Error> {
    current(connection, key)?;
    quota::available(connection, crate::ByteCount::new(allocation.bytes()))?;
    let prior: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM checkpoint_allocations WHERE object=?1)",
        [encode(&allocation)?],
        |row| row.get(0),
    )?;
    if prior {
        return Err(CheckpointError::RestoreIdConflict.into());
    }
    let bytes = i64::try_from(allocation.bytes()).map_err(|_large| CheckpointError::StorageFull)?;
    let _inserted = connection.execute("INSERT INTO checkpoint_allocations(object,capture,owner,reserved_bytes,phase,created_at) VALUES (?1,?2,?3,?4,'reserved',?5)", params![encode(&allocation)?,key.capture.to_string(),encode(&owner)?,bytes,at.timestamp_millis()])?;
    Ok(())
}
fn owned(
    connection: &Connection,
    key: CaptureKey,
    allocation: Allocation,
    owner: CheckpointOwner,
) -> Result<Phase, Error> {
    current(connection, key)?;
    let row: Option<(String, String, String)> = connection
        .query_row(
            "SELECT capture,owner,phase FROM checkpoint_allocations WHERE object=?1",
            [encode(&allocation)?],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (capture, recorded_owner, phase) = row.ok_or(CheckpointError::NotFound)?;
    if capture != key.capture.to_string()
        || serde_json::from_str::<CheckpointOwner>(&recorded_owner)? != owner
    {
        return Err(CheckpointError::Forbidden.into());
    }
    Ok(serde_json::from_str(&encode(&phase)?)?)
}
/// Only after the worker has actually finished private I/O/cleanup or retention.
/// A terminal restore journal or a ready plan remains a separate pin.
pub(crate) fn attach(
    connection: &Connection,
    key: CaptureKey,
    allocation: Allocation,
    owner: CheckpointOwner,
) -> Result<(), Error> {
    let phase = owned(connection, key, allocation, owner)?;
    let referenced: bool = match allocation {
        Allocation::Preview { plan } => connection.query_row("SELECT EXISTS(SELECT 1 FROM checkpoint_plans WHERE id=?1 AND capture=?2 AND owner=?3)", params![plan.to_string(),key.capture.to_string(),encode(&owner)?], |row| row.get(0))?,
        Allocation::Restore { restore } => connection.query_row("SELECT EXISTS(SELECT 1 FROM checkpoint_restores r JOIN checkpoint_plans p ON p.id=r.plan WHERE r.id=?1 AND p.capture=?2 AND r.owner=?3 AND r.phase IN ('complete','recovery'))",params![restore.to_string(),key.capture.to_string(),encode(&owner)?], |row| row.get(0))?,
    };
    if !referenced {
        return Err(CheckpointError::RecoveryPending.into());
    }
    let changed = connection.execute("UPDATE checkpoint_allocations SET phase='attached' WHERE object=?1 AND phase=?2 AND phase IN ('reserved','attached')",params![encode(&allocation)?,phase.label()])?;
    if changed != 1 {
        return Err(CheckpointError::RestoreIdConflict.into());
    }
    Ok(())
}
/// Acknowledge physical cleanup of an unpublished allocation only. Keep its id
/// tombstone so a delayed callback cannot release a replacement's reservation.
pub(crate) fn cleaned(
    connection: &Connection,
    key: CaptureKey,
    allocation: Allocation,
    owner: CheckpointOwner,
) -> Result<(), Error> {
    let phase = owned(connection, key, allocation, owner)?;
    let referenced: bool = match allocation {
        Allocation::Preview { plan } => connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM checkpoint_plans WHERE id=?1)",
            [plan.to_string()],
            |row| row.get(0),
        )?,
        Allocation::Restore { restore } => connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM checkpoint_restores WHERE id=?1)",
            [restore.to_string()],
            |row| row.get(0),
        )?,
    };
    if referenced {
        return Err(CheckpointError::RecoveryPending.into());
    }
    let bytes = i64::try_from(TOMBSTONE_BYTES).map_err(|_large| CheckpointError::StorageFull)?;
    let changed = connection.execute("UPDATE checkpoint_allocations SET phase='cleaned',reserved_bytes=?1 WHERE object=?2 AND phase=?3 AND phase IN ('reserved','cleaned')",params![bytes,encode(&allocation)?,phase.label()])?;
    if changed != 1 {
        return Err(CheckpointError::RestoreIdConflict.into());
    }
    Ok(())
}

/// Publishing cannot allocate its first escrow after private file I/O already ran.
pub(super) fn preview_reserved(connection: &Connection, plan: &super::Plan) -> Result<(), Error> {
    match owned(
        connection,
        plan.key,
        Allocation::Preview { plan: plan.id },
        plan.owner,
    )? {
        Phase::Reserved => Ok(()),
        Phase::Attached | Phase::Cleaned => Err(CheckpointError::RestoreIdConflict.into()),
    }
}
