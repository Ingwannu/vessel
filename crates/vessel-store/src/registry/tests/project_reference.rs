//! Legacy deletion accounts for empty descriptors AND every retained reservation stage.
use super::{apply, db, put, record};
use crate::registry::{RegistryWrite, project_reference};
use crate::{RegistryReservation, ReservationStage, WorkspaceProvision};
use vessel_proto::registry::{
    RegistryRevision, WorkspaceAutomation, WorkspaceIsolation, WorkspaceLifecycle,
    WorkspaceVisibility,
};
use vessel_proto::{ProjectId, WorkspaceId};

#[test]
fn legacy_project_reference_covers_active_and_archived_descriptors() {
    let mut connection = db();
    let mut item = record(WorkspaceId::random());
    let project = item.descriptor.project;
    assert_eq!(project_reference(&connection, project).unwrap(), None);
    put(&mut connection, item.clone(), None).unwrap();
    assert_eq!(
        project_reference(&connection, project).unwrap(),
        Some(item.descriptor.id)
    );
    item.descriptor.revision = RegistryRevision::ZERO.next().unwrap();
    item.descriptor.lifecycle = WorkspaceLifecycle::Archived {
        at: chrono::DateTime::UNIX_EPOCH,
    };
    put(&mut connection, item.clone(), Some(RegistryRevision::ZERO)).unwrap();
    assert_eq!(
        project_reference(&connection, project).unwrap(),
        Some(item.descriptor.id)
    );
    assert_eq!(
        project_reference(&connection, ProjectId::random()).unwrap(),
        None
    );
}

#[test]
fn legacy_project_reference_covers_all_prepublication_backing_reservation_stages() {
    let mut connection = db();
    for stage in [
        ReservationStage::Reserved,
        ReservationStage::Creating,
        ReservationStage::Uncertain,
        ReservationStage::Ready,
    ] {
        let project = ProjectId::random();
        let workspace = WorkspaceId::random();
        apply(
            &mut connection,
            &RegistryWrite::Reserve(RegistryReservation {
                workspace,
                worktree: Some(vessel_proto::WorktreeId::random()),
                stage,
                intent: WorkspaceProvision {
                    project,
                    isolation: WorkspaceIsolation::Local,
                    automation: WorkspaceAutomation::Allowed,
                    title: None,
                    visibility: WorkspaceVisibility::Foreground,
                },
            }),
        )
        .unwrap();
        assert_eq!(
            project_reference(&connection, project).unwrap(),
            Some(workspace)
        );
    }
}

#[test]
fn legacy_project_reference_never_treats_corrupt_retained_intent_as_no_owner() {
    let connection = db();
    let changed = connection.execute(
        "INSERT INTO workspace_registry_reservations(id,intent,stage) VALUES (?1,'{','uncertain')",
        [WorkspaceId::random().to_string()],
    ).unwrap();
    assert_eq!(changed, 1);
    assert!(project_reference(&connection, ProjectId::random()).is_err());
}
