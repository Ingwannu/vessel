//! A fresh private realm and loader fence precede publication of host refs.
use super::{DeliveryState, Page, PageError, requests};
use crate::snapshot::{self, Config, Snapshot, SnapshotError};

impl Page {
    /// This is not runtime qualification. Actual profile/workspace/tool admission is above it.
    pub(crate) async fn snapshot(&mut self) -> Result<Snapshot, PageError> {
        self.active()?;
        let namespace = match self.references.begin_snapshot() {
            Ok(namespace) => namespace,
            Err(error) => {
                self.delivery = DeliveryState::Uncertain;
                return Err(SnapshotError::from(error).into());
            }
        };
        let frame = self.checked_document().await?;
        let world = self
            .execute(requests::IsolatedWorld::new(frame.id.clone()))
            .await?;
        let result = self
            .execute(requests::SnapshotTraversal::new(
                world.execution_context_id,
                Config::new(namespace),
            ))
            .await?;
        if self.checked_document().await? != frame {
            let _revision = self.invalidate()?;
            return Err(PageError::Navigation);
        }
        snapshot::finish_evaluation(&result, namespace, &mut self.references)
            .map_err(PageError::Snapshot)
    }
}
