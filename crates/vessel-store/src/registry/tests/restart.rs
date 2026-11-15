//! Stable terminal identity retains ownership while each admitted run has a distinct lease.

use super::*;
use vessel_proto::TerminalId;

fn registered(
    db: &mut Connection,
    id: WorkspaceId,
    resource: WorkspaceResource,
) -> WorkspaceStartLease {
    let expected = workspace(db, id).unwrap().unwrap().descriptor.revision;
    apply(
        db,
        &RegistryWrite::Claim {
            workspace: id,
            resource,
            expected,
        },
    )
    .unwrap();
    let lease = WorkspaceStartLease::new(id, resource, expected.next().unwrap());
    apply(
        db,
        &RegistryWrite::Settle {
            lease: lease.clone(),
            settled: RegistryStartSettlement::Ready,
        },
    )
    .unwrap();
    lease
}

#[test]
fn restarting_a_ready_terminal_requires_a_new_fenced_lease_and_keeps_its_owner() {
    let mut db = db();
    let record = record(WorkspaceId::random());
    let id = record.descriptor.id;
    let resource = WorkspaceResource::Terminal {
        terminal: TerminalId::random(),
    };
    put(&mut db, record, None).unwrap();
    let first = registered(&mut db, id, resource);
    let expected = workspace(&db, id).unwrap().unwrap().descriptor.revision;
    let before = load(&db).unwrap();
    assert!(matches!(
        apply(
            &mut db,
            &RegistryWrite::Claim {
                workspace: id,
                resource,
                expected
            }
        ),
        Err(StoreError::RegistryConflict)
    ));
    assert_eq!(load(&db).unwrap(), before);
    apply(
        &mut db,
        &RegistryWrite::Restart {
            previous: first.clone(),
            expected,
        },
    )
    .unwrap();
    let current = resource_run(&db, resource).unwrap().unwrap();
    assert_eq!(current.lease.resource(), resource);
    assert_eq!(current.lease.workspace(), id);
    assert_eq!(current.lease.revision(), expected.next().unwrap());
    assert_eq!(current.readiness, RegistryResourceReadiness::Pending);
    let expected = workspace(&db, id).unwrap().unwrap().descriptor.revision;
    for mutation in [
        RegistryWrite::Settle {
            lease: first.clone(),
            settled: RegistryStartSettlement::Ready,
        },
        RegistryWrite::Restart {
            previous: first,
            expected,
        },
    ] {
        assert!(matches!(
            apply(&mut db, &mutation),
            Err(StoreError::RegistryConflict)
        ));
        assert_eq!(resource_run(&db, resource).unwrap(), Some(current.clone()));
    }
    apply(&mut db, &RegistryWrite::RecoverCreating).unwrap();
    assert_eq!(
        resource_run(&db, resource).unwrap().unwrap().readiness,
        RegistryResourceReadiness::Uncertain
    );
    assert!(matches!(
        apply(
            &mut db,
            &RegistryWrite::Restart {
                previous: current.lease.clone(),
                expected
            }
        ),
        Err(StoreError::RegistryConflict)
    ));
    apply(
        &mut db,
        &RegistryWrite::Settle {
            lease: current.lease,
            settled: RegistryStartSettlement::Ready,
        },
    )
    .unwrap();
    assert_eq!(load(&db).unwrap().memberships.len(), 1);
}

#[test]
fn foreign_owner_and_archiving_refuse_a_stable_terminal_restart_without_mutation() {
    let mut db = db();
    let first = record(WorkspaceId::random());
    let other = record(WorkspaceId::random());
    let id = first.descriptor.id;
    let resource = WorkspaceResource::Terminal {
        terminal: TerminalId::random(),
    };
    put(&mut db, first, None).unwrap();
    put(&mut db, other.clone(), None).unwrap();
    let lease = registered(&mut db, id, resource);
    let forged = WorkspaceStartLease::new(other.descriptor.id, resource, lease.revision());
    let before = load(&db).unwrap();
    assert!(matches!(
        apply(
            &mut db,
            &RegistryWrite::Restart {
                previous: forged,
                expected: RegistryRevision::ZERO
            }
        ),
        Err(StoreError::RegistryConflict)
    ));
    assert_eq!(load(&db).unwrap(), before);
    let mut record = workspace(&db, id).unwrap().unwrap();
    let expected = record.descriptor.revision;
    record.descriptor.revision = expected.next().unwrap();
    record.descriptor.lifecycle = WorkspaceLifecycle::Archiving {
        at: DateTime::UNIX_EPOCH,
    };
    put(&mut db, record, Some(expected)).unwrap();
    let before = load(&db).unwrap();
    assert!(matches!(
        apply(
            &mut db,
            &RegistryWrite::Restart {
                previous: lease,
                expected: expected.next().unwrap()
            }
        ),
        Err(StoreError::RegistryInactive(_))
    ));
    assert_eq!(load(&db).unwrap(), before);
}
