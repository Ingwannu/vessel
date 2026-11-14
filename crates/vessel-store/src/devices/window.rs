//! Bounded device reads in primary-key order. Unlike the legacy complete
//! list, a status query must not allocate every device before finding a cap.
use super::{DEVICE_COLUMNS, device_from_row};
use crate::StoreError;
use rusqlite::{Connection, params};
use vessel_proto::{DeviceId, DeviceInfo};

/// A bounded window of paired devices, revoked records included.
#[derive(Debug)]
pub struct DeviceWindow {
    /// At most `MAX_ITEMS` records in stable id order, strictly after the cursor.
    pub devices: Vec<DeviceInfo>,
    /// Another record exists. A caller must not present this as a complete list.
    pub more: bool,
}

impl DeviceWindow {
    /// Maximum records allocated by one read.
    pub const MAX_ITEMS: u16 = 128;
}

pub(crate) fn read(
    connection: &Connection,
    after: Option<DeviceId>,
) -> Result<DeviceWindow, StoreError> {
    // The id range uses the primary-key index; no full-table ordering or
    // count query. Pairing limits labels to 64 Unicode characters and scopes
    // to the five distinct known variants. Check their byte representation
    // before decoding too, so corrupt rows cannot evade that memory bound.
    let mut statement = connection.prepare_cached(&format!(
        "SELECT {DEVICE_COLUMNS},
         length(CAST(id AS BLOB)) = 36 AND length(secret_hash) = 32 AND
         length(CAST(label AS BLOB)) <= 256 AND length(CAST(scopes AS BLOB)) <= 128
         FROM devices WHERE id > ?1 ORDER BY id LIMIT ?2"
    ))?;
    let cursor = after.map_or_else(String::new, |id| id.to_string());
    // One extra row is only a presence check, never decoded into a record.
    let limit = i64::from(DeviceWindow::MAX_ITEMS).saturating_add(1);
    let mut rows = statement.query(params![cursor, limit])?;
    let mut devices = Vec::new();
    let mut more = false;
    while let Some(row) = rows.next()? {
        if devices.len() == usize::from(DeviceWindow::MAX_ITEMS) {
            more = true;
            break;
        }
        if !row.get::<_, bool>(7)? {
            return Err(StoreError::CorruptDevice {
                id: "<oversize metadata>".to_owned(),
                what: "device metadata exceed the bounded read budget",
            });
        }
        devices.push(device_from_row(row)?.info);
    }
    Ok(DeviceWindow { devices, more })
}
