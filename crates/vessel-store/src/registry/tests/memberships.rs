//! A5 reads the caller's uncommitted writer state, without a second read transaction.
use super::{db, put, record};
use crate::StoreError;
use crate::registry::{RegistryWrite, apply, load, workspace_memberships, write};
use vessel_proto::TerminalId;
use vessel_proto::registry::{RegistryRevision, WorkspaceMembership, WorkspaceResource};
use vessel_proto::{SessionId, WorkspaceId};

#[test]
fn scoped_membership_reads_include_pending_ids_and_observe_the_same_transaction() {
    let mut db = db();
    let first = record(WorkspaceId::random());
    let second = record(WorkspaceId::random());
    let a = first.descriptor.id;
    let b = second.descriptor.id;
    put(&mut db, first, None).unwrap();
    put(&mut db, second, None).unwrap();
    let session = WorkspaceResource::Session {
        session: SessionId::random(),
    };
    let terminal = WorkspaceResource::Terminal {
        terminal: TerminalId::random(),
    };
    let transaction = db.transaction().unwrap();
    write::require_active(&transaction, a).unwrap();
    let _changed = apply(
        &transaction,
        &RegistryWrite::Claim {
            workspace: a,
            resource: session,
            expected: RegistryRevision::ZERO,
        },
    )
    .unwrap();
    let _changed = apply(
        &transaction,
        &RegistryWrite::Claim {
            workspace: b,
            resource: terminal,
            expected: RegistryRevision::ZERO,
        },
    )
    .unwrap();
    assert_eq!(
        workspace_memberships(&transaction, a).unwrap(),
        vec![WorkspaceMembership {
            workspace: a,
            resource: session
        }]
    );
    assert_eq!(
        workspace_memberships(&transaction, b).unwrap(),
        vec![WorkspaceMembership {
            workspace: b,
            resource: terminal
        }]
    );
    let all = workspace_memberships(&transaction, None::<WorkspaceId>).unwrap();
    assert_eq!(all.len(), 2);
    assert!(all.contains(&WorkspaceMembership {
        workspace: a,
        resource: session
    }));
    assert!(all.contains(&WorkspaceMembership {
        workspace: b,
        resource: terminal
    }));
    transaction.rollback().unwrap();
    assert!(workspace_memberships(&db, a).unwrap().is_empty());
    assert!(workspace_memberships(&db, b).unwrap().is_empty());
}

#[test]
fn unknown_selected_resource_kind_is_refused_without_leaking_another_workspaces_ids() {
    let mut db = db();
    let first = record(WorkspaceId::random());
    let second = record(WorkspaceId::random());
    let a = first.descriptor.id;
    let b = second.descriptor.id;
    put(&mut db, first, None).unwrap();
    put(&mut db, second, None).unwrap();
    db.execute_batch("PRAGMA ignore_check_constraints=ON;")
        .unwrap();
    db.execute(
        "INSERT INTO workspace_registry_resources(kind,resource,workspace) VALUES ('future',?1,?2)",
        rusqlite::params![SessionId::random().to_string(), a.to_string()],
    )
    .unwrap();
    assert!(matches!(
        workspace_memberships(&db, a),
        Err(StoreError::RegistryConflict)
    ));
    assert!(workspace_memberships(&db, b).unwrap().is_empty());
    assert!(matches!(load(&db), Err(StoreError::RegistryConflict)));
}
