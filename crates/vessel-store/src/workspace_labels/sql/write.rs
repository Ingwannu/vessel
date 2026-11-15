//! Apply a prevalidated mutation inside the existing writer's savepoint.
//! Never publish a returned delta until the outer writer batch has committed.

use rusqlite::{Connection, params};
use vessel_proto::{
    WorkspaceId,
    workspace_labels::{
        AssignWorkspaceLabel, Assignment, CatalogDelta, DeleteWorkspaceLabel, LabelCount,
        LabelName, UpdateWorkspaceLabel, WorkspaceLabelAssigned, WorkspaceLabelDeleted,
        WorkspaceLabelUpdated,
    },
};

use super::{Mutation, Registry, read};
use crate::{StoreError, codec::encode, workspace_labels::plans};

pub(in crate::workspace_labels) fn assign(
    connection: &Connection,
    registry: &impl Registry,
    request: &AssignWorkspaceLabel,
) -> Result<Mutation<WorkspaceLabelAssigned>, StoreError> {
    registry.require_active(connection, request.workspace)?;
    let catalog = read::catalog(connection)?;
    let names = read::names(connection, request.workspace)?;
    let plan = plans::assign(&catalog, &names, &request.label, request.assignment);
    if let Some(CatalogDelta::Upsert { label, .. }) = &plan.catalog {
        let _inserted = connection.execute(
            "INSERT INTO workspace_label_catalog(key,name,color) VALUES (?1,?2,?3)",
            params![
                label.name.key().as_str(),
                label.name.as_str(),
                encode(&label.color)?
            ],
        )?;
    }
    if plan.assignment == plans::AssignmentChange::Changed {
        match request.assignment {
            Assignment::Assigned => {
                let position = next_position(connection, request.workspace)?;
                let _inserted = connection.execute("INSERT INTO workspace_label_assignments(workspace,key,position) VALUES (?1,?2,?3)",
                    params![request.workspace.to_string(), plan.definition.name.key().as_str(), position])?;
            }
            Assignment::Unassigned => {
                let _removed = connection.execute(
                    "DELETE FROM workspace_label_assignments WHERE workspace=?1 AND key=?2",
                    params![
                        request.workspace.to_string(),
                        plan.definition.name.key().as_str()
                    ],
                )?;
            }
        }
        registry.changed(connection)?;
    }
    Ok(Mutation {
        result: WorkspaceLabelAssigned {
            label: plan.definition,
            labels: plan.names,
        },
        catalog: plan.catalog,
        assignments: plan.assignment,
    })
}

fn next_position(connection: &Connection, workspace: WorkspaceId) -> Result<i64, StoreError> {
    let highest: Option<i64> = connection.query_row(
        "SELECT MAX(position) FROM workspace_label_assignments WHERE workspace=?1",
        [workspace.to_string()],
        |row| row.get(0),
    )?;
    match highest {
        None => Ok(0),
        Some(highest) => highest
            .checked_add(1)
            .ok_or(StoreError::WorkspaceLabelOrder),
    }
}

pub(in crate::workspace_labels) fn update(
    connection: &Connection,
    registry: &impl Registry,
    request: &UpdateWorkspaceLabel,
) -> Result<Mutation<WorkspaceLabelUpdated>, StoreError> {
    let catalog = read::catalog(connection)?;
    let plan = plans::update(&catalog, request).map_err(StoreError::WorkspaceLabelRefused)?;
    let affected = if let Some(CatalogDelta::Upsert {
        previous: Some(_), ..
    }) = &plan.catalog
    {
        affected(connection, &request.name)?
    } else {
        LabelCount::default()
    };
    if plan.catalog.is_some() {
        let label = &plan.definition;
        let _changed = connection.execute(
            "UPDATE workspace_label_catalog SET key=?1,name=?2,color=?3 WHERE key=?4",
            params![
                label.name.key().as_str(),
                label.name.as_str(),
                encode(&label.color)?,
                request.name.key().as_str()
            ],
        )?;
        if affected.get() != 0 {
            registry.changed(connection)?;
        }
    }
    Ok(Mutation {
        result: WorkspaceLabelUpdated {
            label: plan.definition,
            affected,
        },
        catalog: plan.catalog,
        assignments: assignment_change(affected),
    })
}

pub(in crate::workspace_labels) fn inspect(
    connection: &Connection,
    name: &LabelName,
) -> Result<WorkspaceLabelDeleted, StoreError> {
    Ok(WorkspaceLabelDeleted {
        affected: affected(connection, name)?,
    })
}

pub(in crate::workspace_labels) fn delete(
    connection: &Connection,
    registry: &impl Registry,
    request: &DeleteWorkspaceLabel,
) -> Result<Mutation<WorkspaceLabelDeleted>, StoreError> {
    let catalog = read::catalog(connection)?;
    let definition = catalog
        .iter()
        .find(|label| label.name.key() == request.name.key());
    let Some(definition) = definition else {
        return Ok(Mutation {
            result: WorkspaceLabelDeleted {
                affected: LabelCount::default(),
            },
            catalog: None,
            assignments: plans::AssignmentChange::Unchanged,
        });
    };
    let affected = affected(connection, &definition.name)?;
    let _removed = connection.execute(
        "DELETE FROM workspace_label_catalog WHERE key=?1",
        [definition.name.key().as_str()],
    )?;
    if affected.get() != 0 {
        registry.changed(connection)?;
    }
    Ok(Mutation {
        result: WorkspaceLabelDeleted { affected },
        catalog: Some(CatalogDelta::Remove {
            name: definition.name.clone(),
        }),
        assignments: assignment_change(affected),
    })
}

fn affected(connection: &Connection, name: &LabelName) -> Result<LabelCount, StoreError> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM workspace_label_assignments WHERE key=?1",
        [name.key().as_str()],
        |row| row.get(0),
    )?;
    u32::try_from(count)
        .map(LabelCount::new)
        .map_err(StoreError::WorkspaceLabelCount)
}

fn assignment_change(affected: LabelCount) -> plans::AssignmentChange {
    if affected.get() == 0 {
        plans::AssignmentChange::Unchanged
    } else {
        plans::AssignmentChange::Changed
    }
}
