//! Capacity is a lifetime reservation: full history still permits exact reconciliation.

use super::*;
use rusqlite::params;

#[test]
fn retained_workspace_limit_refuses_fresh_ids_but_reconciles_existing_reservations() {
    let mut db = db();
    let first = record(WorkspaceId::random());
    let intent = WorkspaceProvision {
        project: first.descriptor.project,
        isolation: WorkspaceIsolation::Local,
        automation: vessel_proto::registry::WorkspaceAutomation::Allowed,
        title: None,
        visibility: WorkspaceVisibility::Foreground,
    };
    let retained = RegistryReservation {
        workspace: first.descriptor.id,
        intent: intent.clone(),
        worktree: None,
        stage: ReservationStage::Reserved,
    };
    apply(&mut db, &RegistryWrite::Reserve(retained.clone())).unwrap();
    for _ in 1..MAX_WORKSPACES {
        let _inserted = db.execute("INSERT INTO workspace_registry_reservations(id,intent,stage) VALUES (?1,?2,'uncertain')", params![WorkspaceId::random().to_string(),crate::codec::encode(&intent).unwrap()]).unwrap();
    }
    let fresh = RegistryReservation {
        workspace: WorkspaceId::random(),
        ..retained.clone()
    };
    let before = load(&db).unwrap();
    assert!(matches!(
        apply(&mut db, &RegistryWrite::Reserve(fresh)),
        Err(StoreError::RegistryFull)
    ));
    assert_eq!(load(&db).unwrap(), before);
    apply(&mut db, &RegistryWrite::Reserve(retained)).unwrap();
    put(&mut db, first, None).unwrap();
}

#[test]
fn resource_capacity_never_evicts_ended_owners_and_existing_lease_still_settles() {
    let mut db = db();
    let first = record(WorkspaceId::random());
    let id = first.descriptor.id;
    put(&mut db, first, None).unwrap();
    let resource = WorkspaceResource::Session {
        session: SessionId::random(),
    };
    apply(
        &mut db,
        &RegistryWrite::Claim {
            workspace: id,
            resource,
            expected: RegistryRevision::ZERO,
        },
    )
    .unwrap();
    for _ in 1..MAX_RESOURCES {
        let _inserted = db.execute("INSERT INTO workspace_registry_resources(kind,resource,workspace,stage) VALUES ('session',?1,?2,'failed')", params![SessionId::random().to_string(),id.to_string()]).unwrap();
    }
    let expected = workspace(&db, id).unwrap().unwrap().descriptor.revision;
    let before = load(&db).unwrap();
    assert!(matches!(
        apply(
            &mut db,
            &RegistryWrite::Claim {
                workspace: id,
                resource: WorkspaceResource::Session {
                    session: SessionId::random()
                },
                expected
            }
        ),
        Err(StoreError::RegistryFull)
    ));
    assert_eq!(load(&db).unwrap(), before);
    let lease = WorkspaceStartLease::new(id, resource, expected);
    apply(
        &mut db,
        &RegistryWrite::Settle {
            lease,
            settled: RegistryStartSettlement::Ready,
        },
    )
    .unwrap();
    assert_eq!(resource_owner(&db, resource).unwrap(), Some(id));
    let previous = resource_run(&db, resource).unwrap().unwrap();
    assert_eq!(previous.readiness, RegistryResourceReadiness::Ready);
    let expected = workspace(&db, id).unwrap().unwrap().descriptor.revision;
    apply(
        &mut db,
        &RegistryWrite::Restart {
            previous: previous.lease,
            expected,
        },
    )
    .unwrap();
    assert_eq!(load(&db).unwrap().memberships.len(), MAX_RESOURCES);
}

#[test]
fn oversized_descriptor_does_not_consume_a_slot_or_advance_revision() {
    let mut db = db();
    let mut first = record(WorkspaceId::random());
    first.descriptor.path = "x".repeat(MAX_DESCRIPTOR_BYTES).into();
    let before = load(&db).unwrap();
    assert!(matches!(
        put(&mut db, first, None),
        Err(StoreError::RegistryRecordTooLarge)
    ));
    assert_eq!(load(&db).unwrap(), before);
    assert_eq!(
        MAX_RETAINED_BYTES,
        MAX_WORKSPACES * RESERVED_BYTES_PER_WORKSPACE + MAX_RESOURCES * RESERVED_BYTES_PER_RESOURCE
    );
}
