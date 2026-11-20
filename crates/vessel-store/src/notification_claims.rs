//! Atomic local OS-notification admission, bounded by inbox row lifetime.
//! The node elects a connection; this layer rechecks source/read/device and
//! persists the once-only admission in the writer's existing transaction.
use crate::{PushAttention, StoreError};
use rusqlite::{Connection, params};
use vessel_proto::DeviceId;

/// Admissions cannot outlive attention. No title, path, device or content is
/// retained in this once-only cursor index.
pub(crate) const SCHEMA: &str = "CREATE TABLE notification_claims (
    cursor INTEGER PRIMARY KEY REFERENCES inbox(cursor) ON DELETE CASCADE
) STRICT;";

/// Person authority already established by the node's authenticated context.
/// Agent and delegated origins have no representable admission authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationAuthority {
    /// Existing node owner credential.
    Owner,
    /// Recheck this paired device's current Read grant and revocation state.
    Device(DeviceId),
}

/// Store admission deliberately has no recipient-selection outcomes: only the
/// node's connection policy can decide who should request an admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalAdmission {
    /// First committed admission of this exact inbox cursor.
    Admitted,
    /// That source already had its one local delivery opportunity.
    AlreadyClaimed,
    /// No longer an eligible unread source.
    NoAttention,
}

pub(crate) fn claim(
    connection: &Connection,
    authority: NotificationAuthority,
    source: PushAttention,
) -> Result<LocalAdmission, StoreError> {
    if !authorized(connection, authority)? {
        return Err(StoreError::NotificationDenied);
    }
    if !current(connection, source)? {
        return Ok(LocalAdmission::NoAttention);
    }
    let inserted = connection.execute(
        "INSERT INTO notification_claims(cursor) VALUES(?1) ON CONFLICT(cursor) DO NOTHING",
        [cursor(source)?],
    )?;
    Ok(if inserted == 0 {
        LocalAdmission::AlreadyClaimed
    } else {
        LocalAdmission::Admitted
    })
}

pub(crate) fn admissible(
    connection: &Connection,
    authority: NotificationAuthority,
    source: PushAttention,
) -> Result<bool, StoreError> {
    Ok(authorized(connection, authority)? && current(connection, source)?)
}

fn authorized(
    connection: &Connection,
    authority: NotificationAuthority,
) -> Result<bool, StoreError> {
    match authority {
        NotificationAuthority::Owner => Ok(true),
        NotificationAuthority::Device(device) => connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM devices WHERE id=?1 AND revoked_at IS NULL AND EXISTS(SELECT 1 FROM json_each(scopes) WHERE value='read'))",
            [device.to_string()], |row| row.get(0)).map_err(StoreError::from),
    }
}

fn current(connection: &Connection, source: PushAttention) -> Result<bool, StoreError> {
    connection.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM inbox i JOIN sessions s ON s.id=i.session WHERE i.cursor=?1 AND i.session=?2 AND i.seq=?3 AND i.seen=0 AND (i.waiting=1 OR json_extract(i.item,'$.kind') IN ('finished','limit','failed')) AND {})",crate::notification_policy::SESSION),
        params![cursor(source)?, source.key.session.to_string(), crate::codec::seq_to_sql(source.key.seq)?],
        |row| row.get(0)).map_err(StoreError::from)
}
fn cursor(source: PushAttention) -> Result<i64, StoreError> {
    i64::try_from(source.cursor.get())
        .map_err(|_overflow| StoreError::OutOfRange(crate::Quantity::Seq))
}

#[cfg(test)]
mod tests;
