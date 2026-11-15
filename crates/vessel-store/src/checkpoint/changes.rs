//! Bounded pagination over the exact effective target frozen by the preview.
use super::records::CheckpointStoreError as Error;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use vessel_proto::{
    DeviceId,
    checkpoint::{
        CHECKPOINT_PAGE_ENTRIES, ChangeCursor, ChangePage, CheckpointError, PathCount,
        RestorePlanId,
    },
};

pub(super) fn page(
    connection: &Connection,
    plan: RestorePlanId,
    device: DeviceId,
    after: Option<ChangeCursor>,
    at: DateTime<Utc>,
) -> Result<ChangePage, Error> {
    let (stored, consumed) = super::plans::owned(connection, plan, device)?;
    if stored.expires <= at || consumed.is_some() {
        return Err(CheckpointError::StalePreview.into());
    }
    let changes = stored.desired.changes(&stored.current);
    let offset = after.map_or(0, |cursor| cursor.0.get());
    if offset > changes.len()
        || (offset != 0 && offset == changes.len())
        || !offset.is_multiple_of(CHECKPOINT_PAGE_ENTRIES)
    {
        return Err(CheckpointError::InvalidCursor.into());
    }
    let end = offset
        .saturating_add(CHECKPOINT_PAGE_ENTRIES)
        .min(changes.len());
    let next = if end < changes.len() {
        Some(ChangeCursor(PathCount::new(end)?))
    } else {
        None
    };
    Ok(ChangePage {
        plan,
        expires: stored.expires,
        changes: changes
            .into_iter()
            .skip(offset)
            .take(CHECKPOINT_PAGE_ENTRIES)
            .collect(),
        next,
    })
}
