//! The existing explicit-stop method, separate from registry/persistence size.
//! Authorization remains the service's; no new stop helper or capability exists.
use super::{Found, Sessions};
use crate::error::ServiceError;
use vessel_proto::{SessionId, timeline::ExecutionInvalidation};

impl Sessions {
    /// Stop a session. Killing one that already ended is a no-op.
    pub(crate) async fn kill(&self, id: SessionId) -> Result<(), ServiceError> {
        match self.find(id).await? {
            Found::Live { handle, .. } => {
                self.agents
                    .invalidate_execution(id, ExecutionInvalidation::PersonStop);
                handle.kill();
            }
            Found::Stored(_) => {}
        }
        Ok(())
    }
}
