//! The public atomic detach write shares the store's existing commit boundary.

use chrono::{DateTime, Utc};
use tokio::sync::oneshot;
use vessel_proto::{SessionId, subagents::DetachedSession};

use super::Store;
use crate::{StoreError, writer::Write};

impl Store {
    /// Commit an explicit human detach, or return the original durable receipt.
    /// Authorization belongs to the node; this operation only changes parentage.
    /// # Errors
    /// Unknown session, never-child, corrupt parentage or failed commit.
    pub async fn detach(
        &self,
        session: SessionId,
        at: DateTime<Utc>,
    ) -> Result<DetachedSession, StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(Write::Detach { session, at, done }).await?;
        committed.await.map_err(|_dropped| StoreError::Closed)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DataDir, StoreConfig};
    use vessel_proto::{
        Creator, HarnessKind, Mark, NodeId, SessionInfo, SessionMode, SessionSpec,
        subagents::DetachDisposition,
    };

    #[tokio::test]
    async fn a_lost_commit_receiver_does_not_roll_back_the_receipt() {
        let temp = tempfile::tempdir().unwrap();
        let dir = DataDir::new(temp.path());
        dir.prepare().unwrap();
        let store = Store::open(&dir, StoreConfig::default())
            .await
            .unwrap()
            .store;
        let info = SessionInfo::new(
            SessionId::random(),
            SessionSpec::new(
                HarnessKind::Codex,
                SessionMode::Structured {
                    model: None,
                    resume: None,
                    options: vec![],
                },
            ),
            Utc::now(),
        )
        .with_creator(Creator::Agent {
            node: NodeId::random(),
            session: SessionId::random(),
        });
        store.create_session(info.clone()).await.unwrap();
        let (done, receiver) = oneshot::channel();
        let at = Utc::now();
        store
            .send(Write::Detach {
                session: info.id,
                at,
                done,
            })
            .await
            .unwrap();
        drop(receiver);
        // The following write is a deterministic barrier after the abandoned
        // receiver, even if the writer grouped both into the same transaction.
        let _marks = store.mark(info.id, Mark::Pin, at).await.unwrap();
        let repeated = store.detach(info.id, Utc::now()).await.unwrap();
        assert_eq!(repeated.disposition, DetachDisposition::AlreadyDetached);
        assert_eq!(repeated.receipt.detached_at(), at);
        assert_eq!(
            store
                .session(info.id)
                .await
                .unwrap()
                .unwrap()
                .detach_receipt,
            Some(repeated.receipt)
        );
    }
}
