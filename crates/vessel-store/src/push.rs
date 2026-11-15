//! Paired-device push leases. The writer rechecks revocation/scopes in the
//! same transaction as registration, so an admitted call cannot resurrect
//! a subscription after device.revoke commits.
use crate::StoreError;
use crate::codec::{encode, time_from_sql, time_to_sql};
use chrono::{DateTime, TimeDelta, Utc};
/// Account-free push leases last 48 hours; activity renews only after half-life.
pub const PUSH_LEASE: TimeDelta = TimeDelta::hours(48);
use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::push::PushSubscription;
use vessel_proto::{DeviceId, Scope};

// Shared by inbox+lease admission in one SQL snapshot and terminal lease
// admission before the synchronous runtime/presence check. Keep aliases fixed.
pub(crate) const LEASE_CURRENT: &str = "p.device=:device AND p.subscription=:subscription AND p.expires_at=:expiry AND p.expires_at>:now AND d.revoked_at IS NULL AND EXISTS(SELECT 1 FROM json_each(d.scopes) WHERE value='read')";

pub(crate) fn lease_admitted(
    connection: &Connection,
    stored: &StoredPush,
    now: DateTime<Utc>,
) -> Result<bool, StoreError> {
    connection.query_row(&format!("SELECT EXISTS(SELECT 1 FROM push_subscriptions p JOIN devices d ON d.id=p.device WHERE {LEASE_CURRENT})"), rusqlite::named_params! {
        ":device": stored.device.to_string(), ":subscription": encode(&stored.subscription)?, ":expiry": time_to_sql(stored.expires_at)?, ":now": time_to_sql(now)?
    }, |row| row.get(0)).map_err(StoreError::from)
}

/// Exact unread source selected for one generic notification opportunity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushAttention {
    /// Current source, rechecked at send admission.
    pub key: vessel_proto::inbox::InboxKey,
    /// Stable position for scanning bounded candidate pages.
    pub cursor: vessel_proto::inbox::InboxCursor,
}

/// A private subscription, held only by its node and owning device.
#[derive(Debug, Clone)]
pub struct StoredPush {
    /// Device that registered it, assigned from the authenticated principal.
    pub device: DeviceId,
    /// Private delivery capability and encryption material.
    pub subscription: PushSubscription,
    /// Lease expiry, fixed by the node rather than supplied by the caller.
    pub expires_at: DateTime<Utc>,
}
#[derive(Debug)]
pub(crate) enum PushWrite {
    Subscribe {
        stored: Box<StoredPush>,
        now: DateTime<Utc>,
    },
    Remove {
        device: DeviceId,
    },
    Expire(Box<StoredPush>),
    Renew {
        device: DeviceId,
        now: DateTime<Utc>,
    },
}
pub(crate) fn apply(connection: &Connection, write: &PushWrite) -> Result<(), StoreError> {
    match write {
        PushWrite::Subscribe { stored, now } => subscribe(connection, stored, *now),
        PushWrite::Renew { device, now } => {
            let next = now
                .checked_add_signed(PUSH_LEASE)
                .ok_or(StoreError::OutOfRange(crate::Quantity::Time))?;
            let half = PUSH_LEASE
                .checked_div(2)
                .and_then(|half| now.checked_add_signed(half))
                .ok_or(StoreError::OutOfRange(crate::Quantity::Time))?;
            let _renewed = connection.execute("UPDATE push_subscriptions SET expires_at=?1 WHERE device=?2 AND expires_at>?3 AND expires_at<?4 AND EXISTS(SELECT 1 FROM devices d WHERE d.id=?2 AND d.revoked_at IS NULL AND EXISTS(SELECT 1 FROM json_each(d.scopes) WHERE value='read'))",params![time_to_sql(next)?,device.to_string(),time_to_sql(*now)?,time_to_sql(half)?])?;
            Ok(())
        }
        PushWrite::Remove { device } => {
            let _rows = connection.execute(
                "DELETE FROM push_subscriptions WHERE device=?1",
                [device.to_string()],
            )?;
            Ok(())
        }
        PushWrite::Expire(stored) => {
            // The browser may renew the same capability during HTTP. A late
            // 404/410 belongs only to the attempted lease, not its renewal.
            let _rows = connection.execute(
                "DELETE FROM push_subscriptions WHERE device=?1 AND subscription=?2 AND expires_at=?3",
                params![stored.device.to_string(), encode(&stored.subscription)?, time_to_sql(stored.expires_at)?],
            )?;
            Ok(())
        }
    }
}
fn subscribe(
    connection: &Connection,
    stored: &StoredPush,
    now: DateTime<Utc>,
) -> Result<(), StoreError> {
    let row: Option<(String, Option<i64>)> = connection
        .query_row(
            "SELECT scopes,revoked_at FROM devices WHERE id=?1",
            [stored.device.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((scopes, None)) = row else {
        return Err(StoreError::PushDenied);
    };
    let scopes: Vec<Scope> =
        serde_json::from_str(&scopes).map_err(|source| StoreError::CorruptPush {
            device: stored.device,
            source,
        })?;
    if !scopes.contains(&Scope::Read) {
        return Err(StoreError::PushDenied);
    }
    let _expired = connection.execute(
        "DELETE FROM push_subscriptions WHERE expires_at<=?1",
        [time_to_sql(now)?],
    )?;
    let full: bool = connection.query_row("SELECT COUNT(*)>=64 AND NOT EXISTS(SELECT 1 FROM push_subscriptions WHERE device=?1) FROM push_subscriptions", [stored.device.to_string()], |row| row.get(0))?;
    if full {
        return Err(StoreError::PushFull);
    }
    let _rows = connection.execute("INSERT INTO push_subscriptions(device,subscription,expires_at) VALUES(?1,?2,?3) ON CONFLICT(device) DO UPDATE SET subscription=excluded.subscription,expires_at=excluded.expires_at", params![stored.device.to_string(), encode(&stored.subscription)?, time_to_sql(stored.expires_at)?])?;
    Ok(())
}
pub(crate) fn list(
    connection: &Connection,
    now: DateTime<Utc>,
) -> Result<Vec<StoredPush>, StoreError> {
    let mut query = connection.prepare_cached("SELECT p.device,p.subscription,p.expires_at FROM push_subscriptions p JOIN devices d ON d.id=p.device WHERE p.expires_at>?1 AND d.revoked_at IS NULL ORDER BY p.device LIMIT 64")?;
    let mut rows = query.query([time_to_sql(now)?])?;
    let mut subscriptions = Vec::new();
    while let Some(row) = rows.next()? {
        let raw: String = row.get(0)?;
        let device = raw
            .parse()
            .map_err(|source| StoreError::CorruptId { text: raw, source })?;
        let text: String = row.get(1)?;
        // This is a stored typed capability, never parsed into an untyped tree.
        let subscription = serde_json::from_str(&text)
            .map_err(|source| StoreError::CorruptPush { device, source })?;
        subscriptions.push(StoredPush {
            device,
            subscription,
            expires_at: time_from_sql(row.get(2)?),
        });
    }
    Ok(subscriptions)
}

#[cfg(test)]
mod tests;
