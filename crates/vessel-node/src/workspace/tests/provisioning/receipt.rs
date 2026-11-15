//! Join actual receipt commits to actual backing publication; no public ingress.
use super::*;
use crate::service::creation::empty::{PreparedEmpty, drive_empty};
use std::sync::atomic::{AtomicUsize, Ordering};
use vessel_proto::{
    ErrorCode,
    creation::{Certainty, Phase},
};

async fn admit(fixture: &Fixture, project: ProjectId, key: &str) -> (Receipt, PreparedEmpty) {
    let prepared = PreparedEmpty::new(local(project)).unwrap();
    let receipt = fixture
        .store
        .admit_creation(CreationAdmission {
            identity: CreationIdentity::new(Kind::Workspace, Key::parse(key).unwrap()),
            receipt: Receipt::admitted(
                Kind::Workspace,
                Reservation {
                    operation: WorkspaceOperationId::random(),
                    workspace: WorkspaceId::random(),
                    workspace_claim: WorkspaceClaim::Create,
                    resource: None,
                    first_input: FirstInput::Absent,
                },
                prepared.fingerprint(),
            )
            .unwrap(),
        })
        .await
        .unwrap();
    (receipt, prepared)
}

#[tokio::test]
async fn empty_receipt_completes_only_after_real_descriptor_and_completed_retry_does_no_work() {
    let fixture = Fixture::new().await;
    let project = fixture.project("empty-joined", None).await;
    let (receipt, prepared) = admit(&fixture, project.id, "joined-empty").await;
    let reserved = receipt.snapshot().reserved.clone();
    let checks = AtomicUsize::new(0);
    let completed = drive_empty(
        &fixture.store,
        &fixture.workspaces,
        reserved.operation,
        &prepared,
        || {
            checks.fetch_add(1, Ordering::SeqCst);
            Ok(Creator::Owner)
        },
    )
    .await
    .unwrap();
    assert_eq!(completed.phase, Phase::Completed);
    assert_eq!(completed.reserved, reserved);
    let record = fixture
        .store
        .registry_workspace(reserved.workspace)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(record.descriptor.path, project.root);
    assert_eq!(record.descriptor.id, reserved.workspace);
    assert_eq!(checks.load(Ordering::SeqCst), 1);
    assert_eq!(
        drive_empty(
            &fixture.store,
            &fixture.workspaces,
            reserved.operation,
            &prepared,
            || Err(ErrorCode::Forbidden)
        )
        .await
        .unwrap(),
        completed
    );
    assert_eq!(
        fixture
            .store
            .workspace_registry()
            .await
            .unwrap()
            .workspaces
            .len(),
        1
    );
    assert!(
        fixture
            .workspaces
            .list_worktrees(None, &[])
            .worktrees
            .is_empty()
    );
    fixture.stop.cancel();
}

#[tokio::test]
async fn denied_entry_keeps_the_same_receipt_and_has_no_backing_reservation() {
    let fixture = Fixture::new().await;
    let project = fixture.project("empty-denied", None).await;
    let (receipt, prepared) = admit(&fixture, project.id, "joined-denied").await;
    let reserved = receipt.snapshot().reserved.clone();
    let denied = drive_empty(
        &fixture.store,
        &fixture.workspaces,
        reserved.operation,
        &prepared,
        || Err(ErrorCode::Forbidden),
    )
    .await
    .unwrap();
    assert_eq!(
        denied.phase,
        Phase::Failed {
            stage: Stage::Workspace,
            code: ErrorCode::Forbidden,
            certainty: Certainty::Known
        }
    );
    assert!(
        fixture
            .store
            .registry_reservation(reserved.workspace)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .store
            .registry_workspace(reserved.workspace)
            .await
            .unwrap()
            .is_none()
    );
    let ready = drive_empty(
        &fixture.store,
        &fixture.workspaces,
        reserved.operation,
        &prepared,
        || Ok(Creator::Owner),
    )
    .await
    .unwrap();
    assert_eq!(ready.phase, Phase::Completed);
    assert_eq!(ready.reserved, reserved);
    fixture.stop.cancel();
}

#[test]
fn backing_only_adapter_refuses_setup_instead_of_completing_unperformed_work() {
    let mut intent = local(ProjectId::random());
    intent.isolation = WorkspaceIsolation::Worktree {
        branch: WorktreeBranch::New {
            name: None,
            base: None,
        },
        setup: SetupChoice::Run,
    };
    assert!(PreparedEmpty::new(intent).is_err());
}
