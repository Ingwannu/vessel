//! Admission, resource claims and transitions share one serialized transaction.

use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::WorkspaceOperationId;
use vessel_proto::creation::{
    CreationError, Kind, MAX_RECEIPT_BYTES, MAX_RECEIPTS, MAX_RETAINED_BYTES,
    RESERVED_BYTES_PER_RECEIPT, Receipt,
};

use super::{Admission, Identity, Owner, Write, claims, owner::MAX_OWNER_BYTES};
use crate::{StoreError, codec};

#[derive(Clone, Copy)]
struct Budget {
    count: usize,
    retained: usize,
    row: usize,
}

const BUDGET: Budget = Budget {
    count: MAX_RECEIPTS,
    retained: MAX_RETAINED_BYTES,
    row: MAX_RECEIPT_BYTES,
};

#[cfg(test)]
mod capacity_tests;

struct Stored {
    raw: String,
    reserved: i64,
    owner: String,
    operation: String,
}
fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Stored> {
    Ok(Stored {
        raw: row.get(0)?,
        reserved: row.get(1)?,
        owner: row.get(2)?,
        operation: row.get(3)?,
    })
}

pub(crate) fn lookup(
    connection: &Connection,
    identity: &Identity,
) -> Result<Option<Receipt>, StoreError> {
    let stored = connection.query_row("SELECT receipt,reserved_bytes,owner,operation FROM creation_receipts WHERE key_hash=?1", [identity.digest().to_string()], row).optional()?;
    stored
        .map(|stored| decoded(connection, &stored, Some(identity)))
        .transpose()
}

pub(crate) fn operation(
    connection: &Connection,
    operation: WorkspaceOperationId,
) -> Result<Option<Receipt>, StoreError> {
    let stored = connection.query_row("SELECT receipt,reserved_bytes,owner,operation FROM creation_receipts WHERE operation=?1", [operation.to_string()], row).optional()?;
    stored
        .map(|stored| decoded(connection, &stored, None))
        .transpose()
}

fn decoded(
    connection: &Connection,
    stored: &Stored,
    identity: Option<&Identity>,
) -> Result<Receipt, StoreError> {
    if stored.reserved != reservation_bytes()? {
        return Err(CreationError::CapacityCorrupt.into());
    }
    let owner = Owner::decode(&stored.owner)?;
    if identity.is_some_and(|identity| identity.owner != owner) {
        return Err(CreationError::NamespaceCorrupt.into());
    }
    let receipt = decode(&stored.raw)?;
    if receipt.snapshot().reserved.operation.to_string() != stored.operation
        || identity.is_some_and(|identity| identity.kind != receipt.snapshot().kind)
        || (matches!(owner, Owner::Agent { .. }) && receipt.snapshot().kind != Kind::Agent)
    {
        return Err(CreationError::NamespaceCorrupt.into());
    }
    claims::validate(connection, &receipt)?;
    Ok(receipt)
}

fn decode(raw: &str) -> Result<Receipt, StoreError> {
    if raw.len() > MAX_RECEIPT_BYTES {
        return Err(CreationError::ReceiptTooLarge.into());
    }
    serde_json::from_str(raw).map_err(StoreError::CorruptCreation)
}

fn reservation_bytes() -> Result<i64, StoreError> {
    i64::try_from(RESERVED_BYTES_PER_RECEIPT)
        .map_err(|_overflow| CreationError::CapacityCorrupt.into())
}

fn encode(receipt: &Receipt, row_limit: usize) -> Result<String, StoreError> {
    let encoded = codec::encode(receipt)?;
    if encoded.len() > row_limit {
        return Err(CreationError::ReceiptTooLarge.into());
    }
    Ok(encoded)
}

fn reserve_capacity(connection: &Connection, budget: Budget) -> Result<(), StoreError> {
    let (count, bytes, invalid): (i64, i64, i64) = connection.query_row(
        "SELECT COUNT(*),COALESCE(SUM(reserved_bytes),0),COALESCE(SUM(reserved_bytes != ?1 OR length(CAST(receipt AS BLOB)) > ?2 OR length(CAST(owner AS BLOB)) > ?3),0) FROM creation_receipts",
        params![reservation_bytes()?, i64::try_from(MAX_RECEIPT_BYTES).map_err(|_overflow| CreationError::CapacityCorrupt)?, i64::try_from(MAX_OWNER_BYTES).map_err(|_overflow| CreationError::CapacityCorrupt)?], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
    )?;
    let count = usize::try_from(count).map_err(|_negative| CreationError::CapacityCorrupt)?;
    let bytes = usize::try_from(bytes).map_err(|_negative| CreationError::CapacityCorrupt)?;
    if invalid != 0 || count.checked_mul(RESERVED_BYTES_PER_RECEIPT) != Some(bytes) {
        return Err(CreationError::CapacityCorrupt.into());
    }
    if count >= budget.count
        || bytes
            .checked_add(RESERVED_BYTES_PER_RECEIPT)
            .is_none_or(|next| next > budget.retained)
    {
        return Err(CreationError::Full.into());
    }
    Ok(())
}

pub(crate) fn apply(connection: &Connection, write: Write) -> Result<Receipt, StoreError> {
    apply_with(connection, write, BUDGET)
}

fn apply_with(
    connection: &Connection,
    write: Write,
    budget: Budget,
) -> Result<Receipt, StoreError> {
    match write {
        Write::Admit(admission) => admit(connection, admission, budget),
        Write::Advance {
            operation: id,
            expected,
            update,
        } => {
            let mut receipt = operation(connection, id)?.ok_or(CreationError::NotFound)?;
            receipt.advance(expected, update)?;
            save(connection, &receipt, budget.row)?;
            Ok(receipt)
        }
    }
}

fn admit(
    connection: &Connection,
    admission: Admission,
    budget: Budget,
) -> Result<Receipt, StoreError> {
    let Admission { identity, receipt } = admission;
    if identity.kind != receipt.snapshot().kind {
        return Err(CreationError::InvalidTransition.into());
    }
    if let Some(existing) = lookup(connection, &identity)? {
        return if existing.fingerprint() == receipt.fingerprint() {
            Ok(existing)
        } else {
            Err(CreationError::IntentConflict.into())
        };
    }
    reserve_capacity(connection, budget)?;
    let encoded = encode(&receipt, budget.row)?;
    let reserved = &receipt.snapshot().reserved;
    if operation(connection, reserved.operation)?.is_some() {
        return Err(CreationError::ResourceClaimed.into());
    }
    claims::reserve(connection, reserved)?;
    let owner = identity.owner.encoded()?;
    let _changed = connection.execute(
        "INSERT INTO creation_receipts(key_hash,operation,reserved_bytes,receipt,owner) VALUES (?1,?2,?3,?4,?5)",
        params![
            identity.digest().to_string(),
            reserved.operation.to_string(),
            reservation_bytes()?,
            encoded,
            owner
        ],
    )?;
    Ok(receipt)
}

fn save(connection: &Connection, receipt: &Receipt, row_limit: usize) -> Result<(), StoreError> {
    let encoded = encode(receipt, row_limit)?;
    let _changed = connection.execute(
        "UPDATE creation_receipts SET receipt=?1 WHERE operation=?2",
        params![encoded, receipt.snapshot().reserved.operation.to_string()],
    )?;
    Ok(())
}

/// Called once during store open before the node admits new operations.
pub(crate) fn recover(connection: &mut Connection) -> Result<(), StoreError> {
    let transaction = connection.transaction()?;
    let receipts: Vec<Stored> = {
        let mut query = transaction.prepare("SELECT receipt,reserved_bytes,owner,operation FROM creation_receipts WHERE json_extract(receipt,'$.in_flight') IS NOT NULL")?;
        query.query_map([], row)?.collect::<Result<_, _>>()?
    };
    for stored in receipts {
        let mut receipt = decoded(&transaction, &stored, None)?;
        receipt.advance(
            receipt.snapshot().revision,
            vessel_proto::creation::Advance::Recover,
        )?;
        save(&transaction, &receipt, BUDGET.row)?;
    }
    transaction.commit()?;
    Ok(())
}
