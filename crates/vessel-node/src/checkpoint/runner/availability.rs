//! Wait for accepted publication/cleanup without another I/O job or timeout budget.
use super::{CheckpointError, Client, Instant, PhysicalRoot, RootState};
impl Client {
    #[cfg(test)]
    pub(crate) async fn wait_released(&self, root: PhysicalRoot) -> Result<(), CheckpointError> {
        let mut roots = self.roots.clone();
        roots
            .wait_for(|roots| !roots.contains_key(&root))
            .await
            .map(|_released| ())
            .map_err(|_closed| CheckpointError::StaleCapture)
    }
    /// A previous accepted capture may still be committing/cleaning. Wait inside
    /// this handler's existing bounded slot, never spawn another I/O worker.
    pub(crate) async fn wait_for_root(
        &self,
        root: PhysicalRoot,
        deadline: Instant,
    ) -> Result<(), CheckpointError> {
        let mut roots = self.roots.clone();
        loop {
            if Instant::now() >= deadline {
                return Err(CheckpointError::CaptureTimedOut);
            }
            let state = roots.borrow_and_update().get(&root).copied();
            match state {
                None => return Ok(()),
                Some(RootState::Unavailable) => return Err(CheckpointError::Busy),
                Some(RootState::Reading { until }) if Instant::now() >= until => {
                    return Err(CheckpointError::Busy);
                }
                Some(RootState::Reading { .. } | RootState::Publishing) => {}
            }
            tokio::time::timeout_at(deadline, roots.changed())
                .await
                .map_err(|_deadline| CheckpointError::CaptureTimedOut)?
                .map_err(|_closed| CheckpointError::StaleCapture)?;
        }
    }
}
