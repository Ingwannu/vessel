//! Pending wire admission remains closed until central checkpoint version assignment.
use super::{Connection, NodeService};
use vessel_proto::{
    ErrorDetail, WireError,
    checkpoint::{
        ChangePage, CheckpointError, CheckpointOwner, CheckpointPage, CheckpointRestoreStatus,
        FILE_CHECKPOINTS, ListCheckpoints, PreviewCheckpoint, ReadCheckpointChanges,
        RestoreCheckpoint, RestoreOutcome, RestorePreview,
    },
};
fn wire(reason: CheckpointError) -> WireError {
    WireError {
        code: vessel_store::CheckpointStoreError::from(reason).code(),
        message: reason.to_string(),
        detail: Some(ErrorDetail::Checkpoint { reason }),
    }
}
fn active() -> Result<(), WireError> {
    FILE_CHECKPOINTS
        .map(|_version| ())
        .ok_or_else(|| wire(CheckpointError::FeatureUnavailable))
}
fn human(connection: &Connection) -> Result<CheckpointOwner, WireError> {
    connection.principal().checkpoint_owner()
}
impl NodeService {
    pub(crate) async fn list_checkpoints(
        &self,
        connection: &Connection,
        request: ListCheckpoints,
    ) -> Result<CheckpointPage, WireError> {
        let _principal = human(connection)?;
        active()?;
        #[cfg(target_os = "linux")]
        {
            self.checkpoints.list(request).await.map_err(wire)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _scope = (self, request);
            Err(wire(CheckpointError::UnsupportedPlatform))
        }
    }
    pub(crate) async fn preview_checkpoint(
        &self,
        connection: &Connection,
        request: PreviewCheckpoint,
    ) -> Result<RestorePreview, WireError> {
        let owner = human(connection)?;
        active()?;
        #[cfg(target_os = "linux")]
        {
            self.checkpoints.preview(owner, request).await.map_err(wire)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _scope = (self, owner, request);
            Err(wire(CheckpointError::UnsupportedPlatform))
        }
    }
    pub(crate) async fn read_checkpoint_changes(
        &self,
        connection: &Connection,
        request: ReadCheckpointChanges,
    ) -> Result<ChangePage, WireError> {
        let owner = human(connection)?;
        active()?;
        #[cfg(target_os = "linux")]
        {
            self.checkpoints.changes(owner, request).await.map_err(wire)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _scope = (self, owner, request);
            Err(wire(CheckpointError::UnsupportedPlatform))
        }
    }
    pub(crate) async fn restore_checkpoint(
        &self,
        connection: &Connection,
        request: RestoreCheckpoint,
    ) -> Result<RestoreOutcome, WireError> {
        let owner = human(connection)?;
        active()?;
        #[cfg(target_os = "linux")]
        {
            self.checkpoints.restore(owner, request).await.map_err(wire)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _scope = (self, owner, request);
            Err(wire(CheckpointError::UnsupportedPlatform))
        }
    }
    pub(crate) async fn checkpoint_restore_status(
        &self,
        connection: &Connection,
        request: CheckpointRestoreStatus,
    ) -> Result<RestoreOutcome, WireError> {
        let owner = human(connection)?;
        active()?;
        #[cfg(target_os = "linux")]
        {
            self.checkpoints.status(owner, request).await.map_err(wire)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _scope = (self, owner, request);
            Err(wire(CheckpointError::UnsupportedPlatform))
        }
    }
}
