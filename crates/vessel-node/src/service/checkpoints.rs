//! Registration seam for ADR 0077. No incomplete filesystem mechanism is exposed.
//!
//! Quality B permits pending methods to be registered while `FILE_CHECKPOINTS`
//! remains `None`. Admission refuses them before dispatch. These handlers also
//! refuse explicitly, so an accidental internal call cannot claim an image or
//! mutate files before the durable coordinator has been connected and reviewed.

use super::{Connection, NodeService};
use std::future::{Ready, ready};
use vessel_proto::{
    ErrorCode, ErrorDetail, WireError,
    checkpoint::{
        ChangePage, CheckpointError, CheckpointPage, CheckpointRestoreStatus, ListCheckpoints,
        PreviewCheckpoint, ReadCheckpointChanges, RestoreCheckpoint, RestoreOutcome,
        RestorePreview,
    },
};

fn unavailable() -> WireError {
    let reason = CheckpointError::FeatureUnavailable;
    WireError {
        code: ErrorCode::Unavailable,
        message: reason.to_string(),
        detail: Some(ErrorDetail::Checkpoint { reason }),
    }
}

fn unavailable_for<T>(connection: &Connection) -> Ready<Result<T, WireError>> {
    ready(
        connection
            .principal()
            .checkpoint_owner()
            .and(Err(unavailable())),
    )
}

impl NodeService {
    pub(crate) fn list_checkpoints(
        &self,
        _connection: &Connection,
        _request: ListCheckpoints,
    ) -> Ready<Result<CheckpointPage, WireError>> {
        ready(Err(unavailable()))
    }
    pub(crate) fn preview_checkpoint(
        &self,
        connection: &Connection,
        _request: PreviewCheckpoint,
    ) -> Ready<Result<RestorePreview, WireError>> {
        unavailable_for(connection)
    }
    pub(crate) fn read_checkpoint_changes(
        &self,
        connection: &Connection,
        _request: ReadCheckpointChanges,
    ) -> Ready<Result<ChangePage, WireError>> {
        unavailable_for(connection)
    }
    pub(crate) fn restore_checkpoint(
        &self,
        connection: &Connection,
        _request: RestoreCheckpoint,
    ) -> Ready<Result<RestoreOutcome, WireError>> {
        unavailable_for(connection)
    }
    pub(crate) fn checkpoint_restore_status(
        &self,
        connection: &Connection,
        _request: CheckpointRestoreStatus,
    ) -> Ready<Result<RestoreOutcome, WireError>> {
        unavailable_for(connection)
    }
}
