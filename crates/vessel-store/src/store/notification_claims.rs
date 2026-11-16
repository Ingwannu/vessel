//! Local notification admission uses the same committed writer replies as
//! inbox marks; no caller receives a grant before its transaction commits.
use super::Store;
use crate::writer::Write;
use crate::{LocalAdmission, NotificationAuthority, PushAttention, StoreError};
use tokio::sync::oneshot;

impl Store {
    /// Admit an eligible exact source at most once, after the node elects its
    /// authenticated connection as the current local recipient.
    /// # Errors
    /// Current device Read authority was lost, the source is corrupt, or the
    /// database write/commit failed. An uncertain result is never a retry grant.
    pub async fn claim_notification(
        &self,
        authority: NotificationAuthority,
        source: PushAttention,
    ) -> Result<LocalAdmission, StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(Write::NotificationClaim {
            authority,
            source,
            done,
        })
        .await?;
        committed.await.map_err(|_closed| StoreError::Closed)?
    }

    /// Final source/authority recheck after asynchronous preparation. Reading
    /// or revoking during preparation suppresses best-effort delivery.
    /// # Errors
    /// The database or current source could not be read.
    pub async fn notification_current(
        &self,
        authority: NotificationAuthority,
        source: PushAttention,
    ) -> Result<bool, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| {
                // Authority and source must describe one database snapshot.
                // A revoke/read toggle between separate auto-commit SELECTs
                // must not combine two states that were never jointly eligible.
                let transaction = connection.unchecked_transaction()?;
                let admitted =
                    crate::notification_claims::admissible(&transaction, authority, source)?;
                transaction.commit()?;
                Ok(admitted)
            })
        })
        .await
    }
}
