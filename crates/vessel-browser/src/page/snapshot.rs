//! A fresh private realm and loader fence precede publication of host refs.
use super::{Page, PageError, requests};
use crate::snapshot::{self, Config, Snapshot, SnapshotError};

impl Page {
    /// This is not runtime qualification. Actual profile/workspace/tool admission is above it.
    pub(crate) async fn snapshot(&mut self) -> Result<Snapshot, PageError> {
        let namespace = self
            .references
            .begin_snapshot()
            .map_err(SnapshotError::from)?;
        let frame = self.execute(requests::FrameTree {}).await?.frame_tree.frame;
        let world = self
            .execute(requests::IsolatedWorld::new(frame.id.clone()))
            .await?;
        let result = self
            .execute(requests::SnapshotTraversal::new(
                world.execution_context_id,
                Config::new(namespace),
            ))
            .await?;
        let after = self.execute(requests::FrameTree {}).await?.frame_tree.frame;
        if frame.id != after.id || frame.loader_id != after.loader_id {
            let _revision = self.invalidate()?;
            return Err(PageError::Navigation);
        }
        snapshot::finish_evaluation(&result, namespace, &mut self.references)
            .map_err(PageError::Snapshot)
    }
}
