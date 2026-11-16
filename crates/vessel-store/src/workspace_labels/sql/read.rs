//! Decode label rows once, refusing corrupt identities instead of skipping them.

use rusqlite::{Connection, params};
use vessel_proto::{
    WorkspaceId,
    workspace_labels::{Label, LabelName},
};

use crate::StoreError;

pub(in crate::workspace_labels) fn catalog(
    connection: &Connection,
) -> Result<Vec<Label>, StoreError> {
    let mut statement =
        connection.prepare("SELECT key,name,color FROM workspace_label_catalog ORDER BY key")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    rows.map(|row| {
        let (key, raw_name, raw_color) = row?;
        let name = checked_name(&key, &raw_name)?;
        let color = serde_json::from_str(&raw_color).map_err(StoreError::WorkspaceLabelColor)?;
        Ok(Label { name, color })
    })
    .collect()
}

pub(in crate::workspace_labels) fn names(
    connection: &Connection,
    workspace: WorkspaceId,
) -> Result<Vec<LabelName>, StoreError> {
    let mut statement = connection.prepare("SELECT a.key,c.name FROM workspace_label_assignments a LEFT JOIN workspace_label_catalog c ON c.key=a.key WHERE a.workspace=?1 ORDER BY a.position")?;
    let rows = statement.query_map(params![workspace.to_string()], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    rows.map(|row| {
        let (key, name) = row?;
        let raw = name.ok_or(StoreError::WorkspaceLabelIdentity)?;
        checked_name(&key, &raw)
    })
    .collect()
}

fn checked_name(key: &str, raw: &str) -> Result<LabelName, StoreError> {
    let name: LabelName = raw.parse().map_err(StoreError::WorkspaceLabelName)?;
    if name.key().as_str() != key {
        return Err(StoreError::WorkspaceLabelIdentity);
    }
    Ok(name)
}
