//! Admission, resource claims and transitions share one serialized transaction.

use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::creation::{
    CreationError, MAX_RECEIPT_BYTES, MAX_RECEIPTS, MAX_RETAINED_BYTES, RESERVED_BYTES_PER_RECEIPT,
    Receipt, Resource, WorkspaceClaim,
};
use vessel_proto::{SessionId, WorkspaceOperationId};

use super::{Admission, Identity, Write};
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

pub(crate) fn lookup(
    connection: &Connection,
    identity: &Identity,
) -> Result<Option<Receipt>, StoreError> {
    let raw: Option<(String, i64)> = connection
        .query_row(
            "SELECT receipt,reserved_bytes FROM creation_receipts WHERE key_hash=?1",
            [identity.digest().to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    raw.map(|(raw, reserved)| decode_reserved(raw, reserved))
        .transpose()
}

pub(crate) fn operation(
    connection: &Connection,
    operation: WorkspaceOperationId,
) -> Result<Option<Receipt>, StoreError> {
    let raw: Option<(String, i64)> = connection
        .query_row(
            "SELECT receipt,reserved_bytes FROM creation_receipts WHERE operation=?1",
            [operation.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    raw.map(|(raw, reserved)| decode_reserved(raw, reserved))
        .transpose()
}

fn decode_reserved(raw: String, reserved: i64) -> Result<Receipt, StoreError> {
    if reserved != reservation_bytes()? {
        return Err(CreationError::CapacityCorrupt.into());
    }
    decode(raw)
}

fn decode(raw: String) -> Result<Receipt, StoreError> {
    if raw.len() > MAX_RECEIPT_BYTES {
        return Err(CreationError::ReceiptTooLarge.into());
    }
    serde_json::from_str(&raw).map_err(StoreError::CorruptCreation)
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
        "SELECT COUNT(*),COALESCE(SUM(reserved_bytes),0),COALESCE(SUM(reserved_bytes != ?1),0) FROM creation_receipts",
        [reservation_bytes()?], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
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

pub(crate) fn apply(connection: &Connection, write: Write) -> Result<(), StoreError> {
    apply_with(connection, write, BUDGET)
}

fn apply_with(connection: &Connection, write: Write, budget: Budget) -> Result<(), StoreError> {
    match write {
        Write::Admit(admission) => admit(connection, admission, budget),
        Write::Advance {
            operation: id,
            expected,
            update,
        } => {
            let mut receipt = operation(connection, id)?.ok_or(CreationError::NotFound)?;
            receipt.advance(expected, update)?;
            save(connection, &receipt, budget.row)
        }
    }
}

fn admit(connection: &Connection, admission: Admission, budget: Budget) -> Result<(), StoreError> {
    let Admission { identity, receipt } = admission;
    if identity.kind != receipt.snapshot().kind {
        return Err(CreationError::InvalidTransition.into());
    }
    if let Some(existing) = lookup(connection, &identity)? {
        return if existing.fingerprint() == receipt.fingerprint() {
            Ok(())
        } else {
            Err(CreationError::IntentConflict.into())
        };
    }
    reserve_capacity(connection, budget)?;
    let encoded = encode(&receipt, budget.row)?;
    let reserved = receipt.snapshot().reserved;
    if operation(connection, reserved.operation)?.is_some() {
        return Err(CreationError::ResourceClaimed.into());
    }
    if reserved.workspace_claim == WorkspaceClaim::Create {
        claim(
            connection,
            "workspace",
            &reserved.workspace.to_string(),
            reserved.operation,
        )?;
    }
    if let Some(resource) = reserved.resource {
        match resource {
            Resource::Agent { session } => {
                refuse_existing_session(connection, session)?;
                claim(
                    connection,
                    "agent",
                    &session.to_string(),
                    reserved.operation,
                )?;
            }
            Resource::Terminal { terminal } => claim(
                connection,
                "terminal",
                &terminal.to_string(),
                reserved.operation,
            )?,
        }
    }
    connection.execute(
        "INSERT INTO creation_receipts(key_hash,operation,reserved_bytes,receipt) VALUES (?1,?2,?3,?4)",
        params![
            identity.digest().to_string(),
            reserved.operation.to_string(),
            reservation_bytes()?,
            encoded
        ],
    )?;
    Ok(())
}

fn refuse_existing_session(connection: &Connection, session: SessionId) -> Result<(), StoreError> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM sessions WHERE id=?1",
            [session.to_string()],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if exists {
        return Err(CreationError::ResourceClaimed.into());
    }
    Ok(())
}

fn claim(
    connection: &Connection,
    kind: &str,
    resource: &str,
    operation: WorkspaceOperationId,
) -> Result<(), StoreError> {
    let owner: Option<String> = connection
        .query_row(
            "SELECT operation FROM creation_claims WHERE kind=?1 AND resource=?2",
            params![kind, resource],
            |row| row.get(0),
        )
        .optional()?;
    if owner.is_some() {
        return Err(CreationError::ResourceClaimed.into());
    }
    connection.execute(
        "INSERT INTO creation_claims(kind,resource,operation) VALUES (?1,?2,?3)",
        params![kind, resource, operation.to_string()],
    )?;
    Ok(())
}

fn save(connection: &Connection, receipt: &Receipt, row_limit: usize) -> Result<(), StoreError> {
    let encoded = encode(receipt, row_limit)?;
    connection.execute(
        "UPDATE creation_receipts SET receipt=?1 WHERE operation=?2",
        params![encoded, receipt.snapshot().reserved.operation.to_string()],
    )?;
    Ok(())
}

/// Called once during store open before the node admits new operations.
pub(crate) fn recover(connection: &mut Connection) -> Result<(), StoreError> {
    let transaction = connection.transaction()?;
    let receipts: Vec<(String, i64)> = {
        let mut query = transaction.prepare("SELECT receipt,reserved_bytes FROM creation_receipts WHERE json_extract(receipt,'$.in_flight') IS NOT NULL")?;
        query
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?
    };
    for (raw, reserved) in receipts {
        let mut receipt = decode_reserved(raw, reserved)?;
        receipt.advance(
            receipt.snapshot().revision,
            vessel_proto::creation::Advance::Recover,
        )?;
        save(&transaction, &receipt, BUDGET.row)?;
    }
    transaction.commit()?;
    Ok(())
}
