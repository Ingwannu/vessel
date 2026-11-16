//! Public async inbox operations; SQL remains in the store's projection.
use super::Store;
use crate::{Residence, SessionCommit, StoreError, writer::Write};
use vessel_proto::inbox::{InboxPage, ListInbox, MarkInbox};
impl Store {
    /// A bounded inbox page, newest first, read without scanning session logs.
    ///
    /// # Errors
    /// The database cannot be read or contains a corrupt projection.
    pub async fn inbox(&self, page: ListInbox) -> Result<InboxPage, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| {
                let transaction = connection.unchecked_transaction()?;
                let result = crate::inbox::list(&transaction, page)?;
                transaction.commit()?;
                Ok(result)
            })
        })
        .await
    }
    /// Retain this exact entry mark's acknowledged mutation clock for live
    /// publication, without changing how an entry is selected or authorized.
    /// `residence` decides whether a changed entry advances that clock.
    /// # Errors
    /// Gone entry, invalid/exhausted session clock, or the existing write failure.
    pub async fn mark_inbox_committed(
        &self,
        mark: MarkInbox,
        residence: Residence,
    ) -> Result<SessionCommit<()>, StoreError> {
        self.write(|done| Write::Inbox {
            mark,
            residence,
            done,
        })
        .await
    }

    /// Mark exactly one current entry of a session served from the store
    /// read or unread.
    ///
    /// # Errors
    /// [`StoreError::InboxGone`] once answered/removed, or a database failure.
    pub async fn mark_inbox(&self, mark: MarkInbox) -> Result<(), StoreError> {
        self.mark_inbox_committed(mark, Residence::Stored)
            .await
            .map(SessionCommit::into_value)
    }
}

impl Store {
    /// Coalesced wakeups after committed attention events, not raw output.
    /// Subscribers take their initial snapshot before generating notices.
    #[must_use]
    pub fn inbox_changes(&self) -> tokio::sync::watch::Receiver<()> {
        self.inbox.clone()
    }

    /// Highest cursor ever assigned, even after its entry has been removed.
    /// Used as the initial delivery watermark to avoid replay after restart.
    ///
    /// # Errors
    /// The database cannot be read or its cursor is corrupt.
    pub async fn inbox_head(&self) -> Result<vessel_proto::inbox::InboxCursor, StoreError> {
        self.read(|reader| {
            reader.with(|connection| {
                let raw: i64 = connection.query_row(
                    "SELECT COALESCE((SELECT seq FROM sqlite_sequence WHERE name='inbox'),0)",
                    [],
                    |row| row.get(0),
                )?;
                let cursor = u64::try_from(raw)
                    .map_err(|_negative| StoreError::OutOfRange(crate::Quantity::Seq))?;
                Ok(vessel_proto::inbox::InboxCursor::new(cursor))
            })
        })
        .await
    }
    /// At most 64 unread push candidates, ordered by stable cursor. The
    /// caller advances the cursor to consider later background sessions even
    /// when an earlier candidate is currently visible on another device.
    ///
    /// # Errors
    /// The database cannot be read or a source is corrupt.
    pub async fn pending_push(
        &self,
        after: vessel_proto::inbox::InboxCursor,
        through: vessel_proto::inbox::InboxCursor,
    ) -> Result<Vec<crate::PushAttention>, StoreError> {
        self.read(move |reader| reader.with(|connection| {
            let lower = i64::try_from(after.get()).unwrap_or(i64::MAX);
            let upper = i64::try_from(through.get()).unwrap_or(i64::MAX);
            let mut query = connection.prepare_cached("SELECT cursor,session,seq FROM inbox WHERE cursor>?1 AND cursor<=?2 AND seen=0 AND (waiting=1 OR json_extract(item,'$.kind') IN ('finished','limit')) ORDER BY cursor LIMIT 64")?;
            let mut rows = query.query([lower, upper])?;
            let mut candidates = Vec::new();
            while let Some(row) = rows.next()? {
                let raw: i64 = row.get(0)?;
                let cursor = u64::try_from(raw).map_err(|_negative| StoreError::OutOfRange(crate::Quantity::Seq))?;
                candidates.push(crate::PushAttention {
                    key: vessel_proto::inbox::InboxKey { session: crate::codec::session_id_from_sql(&row.get::<_, String>(1)?)?, seq: crate::codec::seq_from_sql(row.get(2)?)? },
                    cursor: vessel_proto::inbox::InboxCursor::new(cursor),
                });
            }
            Ok(candidates)
        })).await
    }
}

impl Store {
    /// Recheck exactly one current item; absent means it moved on.
    ///
    /// # Errors
    /// Database read or typed projection decoding failed.
    pub async fn inbox_entry(
        &self,
        key: vessel_proto::inbox::InboxKey,
    ) -> Result<Option<vessel_proto::inbox::InboxEntry>, StoreError> {
        self.read(move |reader| reader.with(|connection| crate::inbox::get(connection, key)))
            .await
    }
}
