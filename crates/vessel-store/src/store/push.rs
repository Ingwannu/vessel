//! Async push storage, with all blocking SQL on the existing store threads.
use super::Store;
use crate::{
    StoreError,
    push::{PushWrite, StoredPush},
    writer::Write,
};
use chrono::{DateTime, Utc};
use vessel_proto::{DeviceId, push::PushSubscription};
impl Store {
    /// Register/renew a paired device's lease; at most 64 subscriptions.
    ///
    /// # Errors
    /// Device revoked/missing/read scope gone, node full, or store failure.
    pub async fn subscribe_push(
        &self,
        stored: StoredPush,
        now: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        self.write(|done| Write::Push {
            write: PushWrite::Subscribe {
                stored: Box::new(stored),
                now,
            },
            done,
        })
        .await
    }
    /// Remove a subscription, optionally only if it is still the one sent.
    ///
    /// # Errors
    /// The database write failed.
    pub async fn remove_push(
        &self,
        device: DeviceId,
        expected: Option<PushSubscription>,
    ) -> Result<(), StoreError> {
        self.write(|done| Write::Push {
            write: PushWrite::Remove { device, expected },
            done,
        })
        .await
    }
    /// Current unexpired, non-revoked subscriptions (at most 64); callers
    /// recheck each device's current read scope immediately before delivery.
    ///
    /// # Errors
    /// The database read or a stored typed subscription failed.
    pub async fn push_subscriptions(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<StoredPush>, StoreError> {
        self.read(move |reader| reader.with(|connection| crate::push::list(connection, now)))
            .await
    }
}

impl Store {
    /// Atomic send-admission snapshot, after encryption and DNS preparation.
    /// Revocation after this read cannot recall an already admitted packet.
    ///
    /// # Errors
    /// The database or typed subscription could not be read.
    pub async fn push_admitted(
        &self,
        stored: &StoredPush,
        attention: crate::PushAttention,
        now: DateTime<Utc>,
    ) -> Result<bool, StoreError> {
        let stored = stored.clone();
        self.read(move |reader| reader.with(|connection| {
            let cursor = i64::try_from(attention.cursor.get()).unwrap_or(i64::MAX);
            connection.query_row("SELECT EXISTS(SELECT 1 FROM push_subscriptions p JOIN devices d ON d.id=p.device JOIN inbox i ON i.session=?4 AND i.seq=?5 WHERE p.device=?1 AND p.subscription=?2 AND p.expires_at=?3 AND p.expires_at>?6 AND d.revoked_at IS NULL AND EXISTS(SELECT 1 FROM json_each(d.scopes) WHERE value='read') AND i.cursor=?7 AND i.seen=0 AND (i.waiting=1 OR json_extract(i.item,'$.kind') IN ('finished','limit')))", rusqlite::params![stored.device.to_string(), crate::codec::encode(&stored.subscription)?, crate::codec::time_to_sql(stored.expires_at)?, attention.key.session.to_string(), crate::codec::seq_to_sql(attention.key.seq)?, crate::codec::time_to_sql(now)?, cursor], |row| row.get(0)).map_err(StoreError::from)
        })).await
    }
}
