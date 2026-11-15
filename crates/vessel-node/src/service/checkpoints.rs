//! Registration seam for ADR 0077. No incomplete filesystem mechanism is exposed.
//!
//! Quality B permits pending methods to be registered while `FILE_CHECKPOINTS`
//! remains `None`. Admission refuses them before dispatch. These handlers also
//! refuse explicitly, so an accidental internal call cannot claim an image or
//! mutate files before the durable coordinator has been connected and reviewed.

use super::{Connection, NodeService};
use vessel_proto::{ErrorCode, ErrorDetail, WireError, checkpoint::*};

fn unavailable() -> WireError {
    let reason = CheckpointError::FeatureUnavailable;
    WireError {
        code: ErrorCode::Unavailable,
        message: reason.to_string(),
        detail: Some(ErrorDetail::Checkpoint { reason }),
    }
}

impl NodeService {
    pub(crate) async fn list_checkpoints(
        &self,
        _connection: &Connection,
        _request: ListCheckpoints,
    ) -> Result<CheckpointPage, WireError> {
        Err(unavailable())
    }
    pub(crate) async fn preview_checkpoint(
        &self,
        _connection: &Connection,
        _request: PreviewCheckpoint,
    ) -> Result<RestorePreview, WireError> {
        Err(unavailable())
    }
    pub(crate) async fn read_checkpoint_changes(
        &self,
        _connection: &Connection,
        _request: ReadCheckpointChanges,
    ) -> Result<ChangePage, WireError> {
        Err(unavailable())
    }
    pub(crate) async fn restore_checkpoint(
        &self,
        _connection: &Connection,
        _request: RestoreCheckpoint,
    ) -> Result<RestoreOutcome, WireError> {
        Err(unavailable())
    }
    pub(crate) async fn checkpoint_restore_status(
        &self,
        _connection: &Connection,
        _request: CheckpointRestoreStatus,
    ) -> Result<RestoreOutcome, WireError> {
        Err(unavailable())
    }
}
