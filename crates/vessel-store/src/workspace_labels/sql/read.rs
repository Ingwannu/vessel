//! Decode label rows once, refusing corrupt identities instead of skipping them.

use rusqlite::{Connection, params};
use vessel_proto::{
    WorkspaceId,
    workspace_labels::{Label, LabelName},
};

use crate::{LabelRow, StoreError};

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
    rows.enumerate()
        .map(|(index, row)| {
            let at = LabelRow::catalog(index);
            let (key, raw_name, raw_color) = row?;
            let name = checked_name(&key, &raw_name, at)?;
            // The decoder's error can repeat the stored value; only the row is kept.
            let color = serde_json::from_str(&raw_color)
                .map_err(|_value_echoing| StoreError::WorkspaceLabelColor { row: at })?;
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
    rows.enumerate()
        .map(|(index, row)| {
            let at = LabelRow::assignment(workspace, index);
            let (key, name) = row?;
            let raw = name.ok_or(StoreError::WorkspaceLabelIdentity { row: at })?;
            checked_name(&key, &raw, at)
        })
        .collect()
}

fn checked_name(key: &str, raw: &str, row: LabelRow) -> Result<LabelName, StoreError> {
    let name: LabelName = raw
        .parse()
        .map_err(|source| StoreError::WorkspaceLabelName { row, source })?;
    if name.key().as_str() != key {
        return Err(StoreError::WorkspaceLabelIdentity { row });
    }
    Ok(name)
}
