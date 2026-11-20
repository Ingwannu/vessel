//! Async push storage, with all blocking SQL on the existing store threads.
use super::Store;
use crate::{
    StoreError,
    push::{PushWrite, StoredPush},
    writer::Write,
};
use chrono::{DateTime, Utc};
use vessel_proto::DeviceId;
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
    /// Explicitly unsubscribe this device, regardless of its current lease.
    ///
    /// # Errors
    /// The database write failed.
    pub async fn remove_push(&self, device: DeviceId) -> Result<(), StoreError> {
        self.write(|done| Write::Push {
            write: PushWrite::Remove { device },
            done,
        })
        .await
    }
    /// Renew this paired caller's existing active lease after its half-life.
    /// Revoked, read-less, missing and expired leases are never resurrected.
    /// # Errors
    /// The committed writer update or node-assigned timestamp cannot be stored.
    pub async fn renew_push(&self, device: DeviceId, now: DateTime<Utc>) -> Result<(), StoreError> {
        self.write(|done| Write::Push {
            write: PushWrite::Renew { device, now },
            done,
        })
        .await
    }
    /// Remove only the exact lease whose HTTP attempt returned 404/410.
    /// Renewing the same capability with a later expiry keeps the new lease.
    ///
    /// # Errors
    /// The database write failed.
    pub async fn expire_push(&self, stored: StoredPush) -> Result<(), StoreError> {
        self.write(|done| Write::Push {
            write: PushWrite::Expire(Box::new(stored)),
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
            connection.query_row(&format!("SELECT EXISTS(SELECT 1 FROM push_subscriptions p JOIN devices d ON d.id=p.device JOIN inbox i ON i.session=:session AND i.seq=:seq JOIN sessions s ON s.id=i.session WHERE {} AND i.cursor=:cursor AND i.seen=0 AND (i.waiting=1 OR json_extract(i.item,'$.kind') IN ('finished','limit')) AND {})",crate::push::LEASE_CURRENT,crate::notification_policy::SESSION), rusqlite::named_params! {
                ":device": stored.device.to_string(), ":subscription": crate::codec::encode(&stored.subscription)?, ":expiry": crate::codec::time_to_sql(stored.expires_at)?, ":now": crate::codec::time_to_sql(now)?,
                ":session": attention.key.session.to_string(), ":seq": crate::codec::seq_to_sql(attention.key.seq)?, ":cursor": cursor
            }, |row| row.get(0)).map_err(StoreError::from)
        })).await
    }
    /// Recheck an exact current lease for a runtime terminal source. The caller
    /// must synchronously check terminal revision/presence after this await.
    /// # Errors
    /// The database or stored subscription could not be read.
    pub async fn push_lease_admitted(
        &self,
        stored: &StoredPush,
        now: DateTime<Utc>,
    ) -> Result<bool, StoreError> {
        let stored = stored.clone();
        self.read(move |reader| {
            reader.with(|connection| crate::push::lease_admitted(connection, &stored, now))
        })
        .await
    }
}
