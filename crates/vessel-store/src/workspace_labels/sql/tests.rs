//! Real `SQLite` atomicity with an explicit fake for the unmerged registry seam.

use std::collections::BTreeMap;

use super::*;
use rusqlite::params;
use vessel_proto::workspace_labels::{
    AssignWorkspaceLabel, Assignment, DeleteWorkspaceLabel, LabelColor, UpdateWorkspaceLabel,
    WorkspaceLabelFailure,
};

#[derive(Clone, Copy)]
enum State {
    Active,
    Archiving,
    Archived,
}
#[derive(Clone, Copy)]
enum Revision {
    Writable,
    Closed,
}
struct FakeRegistry {
    owners: BTreeMap<WorkspaceId, State>,
    revision: Revision,
}

impl Registry for FakeRegistry {
    fn require_active(&self, _: &Connection, workspace: WorkspaceId) -> Result<(), StoreError> {
        match self.owners.get(&workspace) {
            Some(State::Active) => Ok(()),
            Some(State::Archiving | State::Archived) => Err(StoreError::WorkspaceLabelRefused(
                WorkspaceLabelFailure::WorkspaceInactive,
            )),
            None => Err(StoreError::WorkspaceLabelRefused(
                WorkspaceLabelFailure::WorkspaceNotFound,
            )),
        }
    }
    fn changed(&self, connection: &Connection) -> Result<(), StoreError> {
        match self.revision {
            Revision::Closed => Err(StoreError::Closed),
            Revision::Writable => {
                let _changed = connection.execute(
                    "UPDATE label_registry_test_revision SET revision=revision+1",
                    [],
                )?;
                Ok(())
            }
        }
    }
}

fn database(owners: &[(WorkspaceId, State)]) -> (Connection, FakeRegistry) {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    // Only the agreed parent primary key is modeled. Lifecycle/admission is
    // an explicit fake, not copied registry SQL or an inferred session/cwd.
    connection.execute_batch("CREATE TABLE workspace_registry(id TEXT PRIMARY KEY) STRICT; CREATE TABLE label_registry_test_revision(revision INTEGER NOT NULL) STRICT; INSERT INTO label_registry_test_revision VALUES (0);").unwrap();
    for (workspace, _) in owners {
        let _inserted = connection
            .execute(
                "INSERT INTO workspace_registry(id) VALUES (?1)",
                [workspace.to_string()],
            )
            .unwrap();
    }
    connection.execute_batch(SQL).unwrap();
    (
        connection,
        FakeRegistry {
            owners: owners.iter().copied().collect(),
            revision: Revision::Writable,
        },
    )
}

fn committed<T>(
    connection: &mut Connection,
    action: impl FnOnce(&Connection) -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    let transaction = connection.transaction()?;
    let result = action(&transaction)?;
    transaction.commit()?;
    Ok(result)
}

fn request(
    workspace: WorkspaceId,
    name: &str,
    color: LabelColor,
    assignment: Assignment,
) -> AssignWorkspaceLabel {
    AssignWorkspaceLabel {
        workspace,
        label: vessel_proto::workspace_labels::Label {
            name: name.parse().unwrap(),
            color,
        },
        assignment,
    }
}

fn revision(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT revision FROM label_registry_test_revision",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn assignment_requires_active_explicit_identity_before_any_definition_is_written() {
    let active = WorkspaceId::random();
    let archiving = WorkspaceId::random();
    let archived = WorkspaceId::random();
    let (mut connection, registry) = database(&[
        (active, State::Active),
        (archiving, State::Archiving),
        (archived, State::Archived),
    ]);
    for (workspace, reason) in [
        (
            WorkspaceId::random(),
            WorkspaceLabelFailure::WorkspaceNotFound,
        ),
        (archiving, WorkspaceLabelFailure::WorkspaceInactive),
        (archived, WorkspaceLabelFailure::WorkspaceInactive),
    ] {
        let result = committed(&mut connection, |db| {
            assign(
                db,
                &registry,
                &request(
                    workspace,
                    "Never created",
                    LabelColor::Red,
                    Assignment::Assigned,
                ),
            )
        });
        assert!(matches!(result, Err(StoreError::WorkspaceLabelRefused(found)) if found == reason));
        assert!(catalog(&connection).unwrap().is_empty());
    }
    let result = committed(&mut connection, |db| {
        assign(
            db,
            &registry,
            &request(active, "  Ready ", LabelColor::Violet, Assignment::Assigned),
        )
    })
    .unwrap();
    assert_eq!(result.result.labels, vec!["Ready".parse().unwrap()]);
    assert!(result.catalog.is_some());
    assert_eq!(revision(&connection), 1);
}

#[test]
fn host_color_and_spelling_win_and_unknown_unassignment_creates_nothing() {
    let workspace = WorkspaceId::random();
    let (mut connection, registry) = database(&[(workspace, State::Active)]);
    let absent = committed(&mut connection, |db| {
        assign(
            db,
            &registry,
            &request(
                workspace,
                "Unknown",
                LabelColor::Sky,
                Assignment::Unassigned,
            ),
        )
    })
    .unwrap();
    assert!(absent.catalog.is_none());
    assert_eq!(
        absent.assignments,
        crate::workspace_labels::plans::AssignmentChange::Unchanged
    );
    assert!(catalog(&connection).unwrap().is_empty());
    let _created = committed(&mut connection, |db| {
        assign(
            db,
            &registry,
            &request(workspace, "Ready", LabelColor::Violet, Assignment::Assigned),
        )
    })
    .unwrap();
    let repeated = committed(&mut connection, |db| {
        assign(
            db,
            &registry,
            &request(workspace, "ready", LabelColor::Red, Assignment::Assigned),
        )
    })
    .unwrap();
    assert_eq!(repeated.result.label.color, LabelColor::Violet);
    assert_eq!(repeated.result.label.name.as_str(), "Ready");
    assert_eq!(repeated.result.labels, vec!["Ready".parse().unwrap()]);
    assert!(repeated.catalog.is_none());
    assert_eq!(
        repeated.assignments,
        crate::workspace_labels::plans::AssignmentChange::Unchanged
    );
    assert_eq!(revision(&connection), 1);
}

#[test]
fn atomic_rename_updates_every_assignment_including_archived_and_recolor_is_catalog_only() {
    let a = WorkspaceId::random();
    let b = WorkspaceId::random();
    let (mut connection, mut registry) = database(&[(a, State::Active), (b, State::Active)]);
    for workspace in [a, b] {
        let _created = committed(&mut connection, |db| {
            assign(
                db,
                &registry,
                &request(workspace, "Ready", LabelColor::Violet, Assignment::Assigned),
            )
        })
        .unwrap();
    }
    let _archived = registry.owners.insert(b, State::Archived);
    let renamed = committed(&mut connection, |db| {
        update(
            db,
            &registry,
            &UpdateWorkspaceLabel {
                name: "ready".parse().unwrap(),
                new_name: Some("Waiting".parse().unwrap()),
                color: Some(LabelColor::Sky),
            },
        )
    })
    .unwrap();
    assert_eq!(renamed.result.affected.get(), 2);
    assert_eq!(
        renamed.assignments,
        crate::workspace_labels::plans::AssignmentChange::Changed
    );
    for workspace in [a, b] {
        assert_eq!(
            names(&connection, workspace).unwrap(),
            vec!["Waiting".parse().unwrap()]
        );
    }
    let before = revision(&connection);
    let recolored = committed(&mut connection, |db| {
        update(
            db,
            &registry,
            &UpdateWorkspaceLabel {
                name: "waiting".parse().unwrap(),
                new_name: None,
                color: Some(LabelColor::Teal),
            },
        )
    })
    .unwrap();
    assert_eq!(recolored.result.affected.get(), 0);
    assert_eq!(
        recolored.assignments,
        crate::workspace_labels::plans::AssignmentChange::Unchanged
    );
    assert_eq!(revision(&connection), before);
    assert_eq!(
        catalog(&connection).unwrap().first().unwrap().color,
        LabelColor::Teal
    );
    let case_only = committed(&mut connection, |db| {
        update(
            db,
            &registry,
            &UpdateWorkspaceLabel {
                name: "waiting".parse().unwrap(),
                new_name: Some("WAITING".parse().unwrap()),
                color: None,
            },
        )
    })
    .unwrap();
    assert_eq!(case_only.result.affected.get(), 2);
    assert_eq!(
        names(&connection, a).unwrap(),
        vec!["WAITING".parse().unwrap()]
    );
}

#[test]
fn collision_rejects_rename_and_color_and_keeps_assignment_order() {
    let workspace = WorkspaceId::random();
    let (mut connection, registry) = database(&[(workspace, State::Active)]);
    for (name, color) in [("Ready", LabelColor::Violet), ("Taken", LabelColor::Sky)] {
        let _created = committed(&mut connection, |db| {
            assign(
                db,
                &registry,
                &request(workspace, name, color, Assignment::Assigned),
            )
        })
        .unwrap();
    }
    let before = catalog(&connection).unwrap();
    let version = revision(&connection);
    let collision = committed(&mut connection, |db| {
        update(
            db,
            &registry,
            &UpdateWorkspaceLabel {
                name: "Ready".parse().unwrap(),
                new_name: Some("taken".parse().unwrap()),
                color: Some(LabelColor::Red),
            },
        )
    });
    assert!(matches!(
        collision,
        Err(StoreError::WorkspaceLabelRefused(
            WorkspaceLabelFailure::NameTaken
        ))
    ));
    assert_eq!(catalog(&connection).unwrap(), before);
    assert_eq!(revision(&connection), version);
    assert_eq!(
        names(&connection, workspace).unwrap(),
        vec!["Ready".parse().unwrap(), "Taken".parse().unwrap()]
    );
}

#[test]
fn delete_recomputes_advisory_count_removes_assignments_and_is_idempotent() {
    let a = WorkspaceId::random();
    let b = WorkspaceId::random();
    let (mut connection, registry) = database(&[(a, State::Active), (b, State::Active)]);
    let _created = committed(&mut connection, |db| {
        assign(
            db,
            &registry,
            &request(a, "Ready", LabelColor::Violet, Assignment::Assigned),
        )
    })
    .unwrap();
    assert_eq!(
        inspect(&connection, &"Ready".parse().unwrap())
            .unwrap()
            .affected
            .get(),
        1
    );
    let _created_later = committed(&mut connection, |db| {
        assign(
            db,
            &registry,
            &request(b, "Ready", LabelColor::Sky, Assignment::Assigned),
        )
    })
    .unwrap();
    let request = DeleteWorkspaceLabel {
        name: "ready".parse().unwrap(),
    };
    let removed = committed(&mut connection, |db| delete(db, &registry, &request)).unwrap();
    assert_eq!(removed.result.affected.get(), 2);
    assert_eq!(
        removed.assignments,
        crate::workspace_labels::plans::AssignmentChange::Changed
    );
    assert!(removed.catalog.is_some());
    assert!(names(&connection, a).unwrap().is_empty());
    assert!(names(&connection, b).unwrap().is_empty());
    let repeated = committed(&mut connection, |db| delete(db, &registry, &request)).unwrap();
    assert_eq!(repeated.result.affected.get(), 0);
    assert!(repeated.catalog.is_none());
}

#[test]
fn failed_registry_invalidation_rolls_back_catalog_and_assignment_together() {
    let workspace = WorkspaceId::random();
    let (mut connection, mut registry) = database(&[(workspace, State::Active)]);
    registry.revision = Revision::Closed;
    let result = committed(&mut connection, |db| {
        assign(
            db,
            &registry,
            &request(workspace, "Ready", LabelColor::Violet, Assignment::Assigned),
        )
    });
    assert!(matches!(result, Err(StoreError::Closed)));
    assert!(catalog(&connection).unwrap().is_empty());
    assert!(names(&connection, workspace).unwrap().is_empty());
    assert_eq!(revision(&connection), 0);
}

#[test]
fn assignment_order_exhaustion_rolls_back_a_new_definition_instead_of_wrapping() {
    let workspace = WorkspaceId::random();
    let (mut connection, registry) = database(&[(workspace, State::Active)]);
    let _created = committed(&mut connection, |db| {
        assign(
            db,
            &registry,
            &request(
                workspace,
                "Existing",
                LabelColor::Violet,
                Assignment::Assigned,
            ),
        )
    })
    .unwrap();
    let _changed = connection
        .execute(
            "UPDATE workspace_label_assignments SET position=?1 WHERE workspace=?2",
            params![i64::MAX, workspace.to_string()],
        )
        .unwrap();
    let before = revision(&connection);
    let result = committed(&mut connection, |db| {
        assign(
            db,
            &registry,
            &request(
                workspace,
                "Never created",
                LabelColor::Sky,
                Assignment::Assigned,
            ),
        )
    });
    assert!(matches!(result, Err(StoreError::WorkspaceLabelOrder)));
    assert_eq!(catalog(&connection).unwrap().len(), 1);
    assert_eq!(
        names(&connection, workspace).unwrap(),
        vec!["Existing".parse().unwrap()]
    );
    assert_eq!(revision(&connection), before);
}

#[test]
fn corrupt_key_and_color_are_refused_whole_instead_of_silently_skipping_rows() {
    let workspace = WorkspaceId::random();
    let (mut connection, registry) = database(&[(workspace, State::Active)]);
    let _created = committed(&mut connection, |db| {
        assign(
            db,
            &registry,
            &request(workspace, "Ready", LabelColor::Violet, Assignment::Assigned),
        )
    })
    .unwrap();
    let _changed = connection
        .execute("UPDATE workspace_label_catalog SET name='Different'", [])
        .unwrap();
    assert!(matches!(
        catalog(&connection),
        Err(StoreError::WorkspaceLabelIdentity)
    ));
    assert!(matches!(
        names(&connection, workspace),
        Err(StoreError::WorkspaceLabelIdentity)
    ));
    let _changed = connection
        .execute(
            "UPDATE workspace_label_catalog SET name='Ready',color='invalid-json'",
            [],
        )
        .unwrap();
    assert!(matches!(
        catalog(&connection),
        Err(StoreError::WorkspaceLabelColor(_))
    ));
}
