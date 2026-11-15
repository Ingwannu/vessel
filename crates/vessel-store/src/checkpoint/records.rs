//! Decoded durable checkpoint rows; SQL/JSON stay inside this crate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vessel_proto::{
    DeviceId,
    checkpoint::{
        CaptureKey, EffectiveDesiredImage, FileImage, RestoreId, RestoreOutcome, RestorePlanId,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Plan {
    pub(super) id: RestorePlanId,
    pub(super) key: CaptureKey,
    pub(super) device: DeviceId,
    pub(super) current: FileImage,
    pub(super) desired: EffectiveDesiredImage,
    pub(super) expires: DateTime<Utc>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Operation {
    pub(super) session: vessel_proto::SessionId,
    pub(super) id: RestoreId,
    pub(super) plan: RestorePlanId,
    pub(super) device: DeviceId,
    pub(super) outcome: RestoreOutcome,
    pub(super) before: FileImage,
}
#[derive(Debug, thiserror::Error)]
pub(super) enum CheckpointStoreError {
    #[error("checkpoint preview lifetime could not be represented")]
    Duration(#[from] chrono::OutOfRangeError),
    #[error("checkpoint record contains an invalid operation identity")]
    Id(#[source] <RestoreId as std::str::FromStr>::Err),
    #[error("checkpoint storage operation failed")]
    Sql(#[from] rusqlite::Error),
    #[error("checkpoint record could not be encoded or decoded")]
    Json(#[from] serde_json::Error),
    #[error("checkpoint operation refused: {0}")]
    Refused(#[from] vessel_proto::checkpoint::CheckpointError),
}
