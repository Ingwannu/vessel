//! Authoritative plan decoding joins its capture instead of trusting a caller's key.
use super::records::{CheckpointStoreError as Error, Plan};
use rusqlite::{Connection, OptionalExtension as _};
use vessel_proto::{
    DeviceId,
    checkpoint::{CaptureKey, CheckpointError, RestoreId, RestorePlanId},
};

struct Stored {
    capture: String,
    session: String,
    worktree: String,
    generation: i64,
    boundary: String,
    device: String,
    current: String,
    desired: String,
    expires: i64,
    consumed: Option<String>,
}
fn id<T: serde::de::DeserializeOwned>(source: &str) -> Result<T, Error> {
    Ok(serde_json::from_str(&serde_json::to_string(source)?)?)
}

/// Ownership is checked before image decoding or returning a session association.
pub(super) fn owned(
    connection: &Connection,
    plan: RestorePlanId,
    device: DeviceId,
) -> Result<(Plan, Option<RestoreId>), Error> {
    let raw = connection.query_row(
        "SELECT c.id,c.session,c.worktree,c.generation,c.boundary,p.device,p.current_image,p.desired_image,p.expires_at,p.consumed_by
         FROM checkpoint_plans p JOIN checkpoint_captures c ON c.id=p.capture WHERE p.id=?1 AND c.state='ready'",
        [plan.to_string()],
        |row| Ok(Stored {capture:row.get(0)?,session:row.get(1)?,worktree:row.get(2)?,generation:row.get(3)?,boundary:row.get(4)?,
            device:row.get(5)?,current:row.get(6)?,desired:row.get(7)?,expires:row.get(8)?,consumed:row.get(9)?}),
    ).optional()?.ok_or(CheckpointError::NotFound)?;
    if raw.device != device.to_string() {
        return Err(CheckpointError::Forbidden.into());
    }
    let expires = chrono::DateTime::from_timestamp_millis(raw.expires)
        .ok_or(CheckpointError::StalePreview)?;
    Ok((
        Plan {
            id: plan,
            device,
            key: CaptureKey {
                capture: id(&raw.capture)?,
                session: id(&raw.session)?,
                worktree: id(&raw.worktree)?,
                generation: vessel_proto::checkpoint::CaptureGeneration::try_from(
                    u64::try_from(raw.generation)
                        .map_err(|_invalid| CheckpointError::StaleCapture)?,
                )?,
                boundary: serde_json::from_str(&raw.boundary)?,
            },
            current: serde_json::from_str(&raw.current)?,
            desired: serde_json::from_str(&raw.desired)?,
            expires,
        },
        raw.consumed.as_deref().map(id).transpose()?,
    ))
}
