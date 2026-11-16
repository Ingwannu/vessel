//! Legacy deletion accounts for owned backings, transitions AND every reservation stage.
use super::{apply, db, put, record};
use crate::registry::{RegistryWrite, project_reference};
use crate::{RegistryReservation, ReservationStage, WorkspaceProvision};
use vessel_proto::registry::{
    WorkspaceAutomation, WorkspaceIsolation, WorkspaceLifecycle, WorkspaceVisibility,
};
use vessel_proto::{ProjectId, WorkspaceId};

// ADR 0106 narrows W11's fence: a `Local` record that is Active or Archived
// owns no directory (removal archives it), while `Managed` backings and a
// `Local` record mid-transition still fence.
#[test]
fn legacy_project_reference_covers_owned_backings_and_transitions_only() {
    let mut connection = db();
    let mut item = record(WorkspaceId::random());
    let project = item.descriptor.project;
    assert_eq!(project_reference(&connection, project).unwrap(), None);
    put(&mut connection, item.clone(), None).unwrap();
    assert_eq!(project_reference(&connection, project).unwrap(), None);
    let at = chrono::DateTime::UNIX_EPOCH;
    let steps = [
        (WorkspaceLifecycle::Archiving { at }, true),
        (WorkspaceLifecycle::Archived { at }, false),
        (WorkspaceLifecycle::Restoring { at }, true),
    ];
    for (lifecycle, fences) in steps {
        let expected = item.descriptor.revision;
        item.descriptor.revision = expected.next().unwrap();
        item.descriptor.lifecycle = lifecycle;
        put(&mut connection, item.clone(), Some(expected)).unwrap();
        assert_eq!(
            project_reference(&connection, project).unwrap(),
            fences.then_some(item.descriptor.id)
        );
    }
    let mut managed = record(WorkspaceId::random());
    managed.descriptor.project = project;
    managed.descriptor.backing = vessel_proto::registry::WorkspaceBacking::Managed {
        worktree: vessel_proto::WorktreeId::random(),
    };
    put(&mut connection, managed.clone(), None).unwrap();
    assert!(project_reference(&connection, project).unwrap().is_some());
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
