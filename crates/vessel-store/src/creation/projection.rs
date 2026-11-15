//! Admission, resource claims and transitions share one serialized transaction.

use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::creation::{CreationError, MAX_RECEIPTS, Receipt, Resource, WorkspaceClaim};
use vessel_proto::{SessionId, WorkspaceOperationId};

use super::{Admission, Identity, Write};
use crate::{StoreError, codec};

pub(crate) fn lookup(
    connection: &Connection,
    identity: &Identity,
) -> Result<Option<Receipt>, StoreError> {
    let raw: Option<String> = connection
        .query_row(
            "SELECT receipt FROM creation_receipts WHERE key_hash=?1",
            [identity.digest().to_string()],
            |row| row.get(0),
        )
        .optional()?;
    raw.map(decode).transpose()
}

pub(crate) fn operation(
    connection: &Connection,
    operation: WorkspaceOperationId,
) -> Result<Option<Receipt>, StoreError> {
    let raw: Option<String> = connection
        .query_row(
            "SELECT receipt FROM creation_receipts WHERE operation=?1",
            [operation.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    raw.map(decode).transpose()
}

fn decode(raw: String) -> Result<Receipt, StoreError> {
    serde_json::from_str(&raw).map_err(StoreError::CorruptCreation)
}

pub(crate) fn apply(connection: &Connection, write: Write) -> Result<(), StoreError> {
    match write {
        Write::Admit(admission) => admit(connection, admission),
        Write::Advance {
            operation: id,
            expected,
            update,
        } => {
            let mut receipt = operation(connection, id)?.ok_or(CreationError::NotFound)?;
            receipt.advance(expected, update)?;
            save(connection, &receipt)
        }
    }
}

fn admit(connection: &Connection, admission: Admission) -> Result<(), StoreError> {
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
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM creation_receipts", [], |row| {
        row.get(0)
    })?;
    if usize::try_from(count).map_or(true, |count| count >= MAX_RECEIPTS) {
        return Err(CreationError::Full.into());
    }
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
        "INSERT INTO creation_receipts(key_hash,operation,receipt) VALUES (?1,?2,?3)",
        params![
            identity.digest().to_string(),
            reserved.operation.to_string(),
            codec::encode(&receipt)?
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

fn save(connection: &Connection, receipt: &Receipt) -> Result<(), StoreError> {
    connection.execute(
        "UPDATE creation_receipts SET receipt=?1 WHERE operation=?2",
        params![
            codec::encode(receipt)?,
            receipt.snapshot().reserved.operation.to_string()
        ],
    )?;
    Ok(())
}

/// Called once during store open before the node admits new operations.
pub(crate) fn recover(connection: &mut Connection) -> Result<(), StoreError> {
    let transaction = connection.transaction()?;
    let receipts: Vec<String> = {
        let mut query = transaction.prepare("SELECT receipt FROM creation_receipts WHERE json_extract(receipt,'$.in_flight') IS NOT NULL")?;
        query
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?
    };
    for raw in receipts {
        let mut receipt = decode(raw)?;
        receipt.advance(
            receipt.snapshot().revision,
            vessel_proto::creation::Advance::Recover,
        )?;
        save(&transaction, &receipt)?;
    }
    transaction.commit()?;
    Ok(())
}
