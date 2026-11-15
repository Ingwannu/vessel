//! Real Store/project primitives, isolated git configuration, no agent or bootstrap.
use super::*;
use crate::workspace::provisioning::Provisioned;
use vessel_proto::creation::{
    Advance, Fingerprint, FirstInput, Key, Kind, Receipt, Reservation, Stage, WorkspaceClaim,
};
use vessel_proto::registry::{
    WorkspaceAutomation, WorkspaceBacking, WorkspaceIsolation, WorkspaceVisibility,
};
use vessel_proto::{BlobHash, WorkspaceId, WorkspaceOperationId};
use vessel_store::{CreationAdmission, CreationIdentity, ReservationStage, WorkspaceProvision};

async fn accept(fixture: &Fixture, key: &str) -> Receipt {
    fixture
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
                Fingerprint::new(BlobHash::from_bytes([1; 32])),
            )
            .unwrap(),
        })
        .await
        .unwrap()
}
async fn begin(fixture: &Fixture, receipt: &Receipt) {
    let _acknowledged = fixture
        .store
        .advance_creation(
            receipt.snapshot().reserved.operation,
            receipt.snapshot().revision,
            Advance::Begin(Stage::Workspace),
        )
        .await
        .unwrap();
}
fn local(project: ProjectId) -> WorkspaceProvision {
    WorkspaceProvision {
        project,
        isolation: WorkspaceIsolation::Local,
        automation: WorkspaceAutomation::Allowed,
        title: None,
        visibility: WorkspaceVisibility::Foreground,
    }
}
fn ready(result: Provisioned) -> vessel_proto::registry::WorkspaceDescriptor {
    match result {
        Provisioned::Ready(descriptor) => *descriptor,
        Provisioned::NeedsApproval(commands) => panic!("unexpected approval: {commands:?}"),
        Provisioned::Refused { reason, certainty } => {
            panic!("unexpected refusal: {reason:?} ({certainty:?})")
        }
        Provisioned::InProgress(id) | Provisioned::Uncertain(id) => {
            panic!("unsettled backing: {id}")
        }
    }
}

#[tokio::test]
async fn provisioning_requires_the_committed_receipt_marker_before_reserving_backing() {
    let fixture = Fixture::new().await;
    let project = fixture.project("marker-project", None).await;
    let receipt = accept(&fixture, "marker-required").await;
    let reserved = &receipt.snapshot().reserved;
    let result = fixture
        .workspaces
        .provision_workspace(Creator::Owner, reserved.operation, local(project.id))
        .await;
    assert!(matches!(result, Err(WorkspaceError::Conflict(_))));
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
            .workspaces
            .list_worktrees(None, &[])
            .worktrees
            .is_empty()
    );
    begin(&fixture, &receipt).await;
    let descriptor = ready(
        fixture
            .workspaces
            .provision_workspace(Creator::Owner, reserved.operation, local(project.id))
            .await
            .unwrap(),
    );
    assert_eq!(descriptor.id, reserved.workspace);
    assert_eq!(descriptor.path, project.root);
    assert_eq!(descriptor.backing, WorkspaceBacking::Local);
    fixture.stop.cancel();
}

#[tokio::test]
async fn same_root_keeps_distinct_workspace_ids_and_same_operation_retains_its_descriptor() {
    let fixture = Fixture::new().await;
    let project = fixture.project("shared-root", None).await;
    let first = accept(&fixture, "local-first").await;
    let second = accept(&fixture, "local-second").await;
    begin(&fixture, &first).await;
    begin(&fixture, &second).await;
    let intent = local(project.id);
    let one = ready(
        fixture
            .workspaces
            .provision_workspace(
                Creator::Owner,
                first.snapshot().reserved.operation,
                intent.clone(),
            )
            .await
            .unwrap(),
    );
    let repeated = ready(
        fixture
            .workspaces
            .provision_workspace(
                Creator::Owner,
                first.snapshot().reserved.operation,
                intent.clone(),
            )
            .await
            .unwrap(),
    );
    let two = ready(
        fixture
            .workspaces
            .provision_workspace(
                Creator::Owner,
                second.snapshot().reserved.operation,
                intent.clone(),
            )
            .await
            .unwrap(),
    );
    assert_eq!(one, repeated);
    assert_ne!(one.id, two.id);
    assert_eq!(one.path, two.path);
    let mut changed = intent;
    changed.visibility = WorkspaceVisibility::Background;
    assert!(matches!(
        fixture
            .workspaces
            .provision_workspace(Creator::Owner, first.snapshot().reserved.operation, changed)
            .await,
        Err(WorkspaceError::Conflict(_))
    ));
    assert_eq!(
        fixture
            .store
            .workspace_registry()
            .await
            .unwrap()
            .workspaces
            .len(),
        2
    );
    fixture.stop.cancel();
}

#[tokio::test]
async fn managed_backing_uses_the_reserved_id_and_never_runs_legacy_bootstrap() {
    let fixture = Fixture::new().await;
    let file = "[worktree]\nsetup = [\"echo must-not-run > bootstrap-must-not-run\"]\n";
    let project = fixture.project("managed-backing", Some(file)).await;
    assert!(matches!(
        fixture.workspaces.commands(project.id, None).await.unwrap(),
        ProjectCommands::Listed { setup, .. } if !setup.is_empty()
    ));
    let receipt = accept(&fixture, "managed").await;
    begin(&fixture, &receipt).await;
    let intent = WorkspaceProvision {
        isolation: WorkspaceIsolation::Worktree {
            branch: WorktreeBranch::New {
                name: None,
                base: None,
            },
            setup: SetupChoice::Run,
        },
        ..local(project.id)
    };
    let first = ready(
        fixture
            .workspaces
            .provision_workspace(
                Creator::Owner,
                receipt.snapshot().reserved.operation,
                intent.clone(),
            )
            .await
            .unwrap(),
    );
    let reservation = fixture
        .store
        .registry_reservation(first.id)
        .await
        .unwrap()
        .unwrap();
    let fixed = reservation.worktree.unwrap();
    assert_eq!(first.backing, WorkspaceBacking::Managed { worktree: fixed });
    assert_eq!(reservation.stage, ReservationStage::Ready);
    let stored = fixture
        .store
        .registry_workspace(first.id)
        .await
        .unwrap()
        .unwrap();
    let recovery = stored.recovery.unwrap();
    assert_eq!(recovery.id, fixed);
    assert!(matches!(recovery.setup, SetupState::Skipped));
    let repeated = ready(
        fixture
            .workspaces
            .provision_workspace(
                Creator::Owner,
                receipt.snapshot().reserved.operation,
                intent,
            )
            .await
            .unwrap(),
    );
    assert_eq!(first, repeated);
    assert_eq!(
        fixture
            .workspaces
            .list_worktrees(Some(project.id), &[])
            .worktrees
            .len(),
        1
    );
    assert!(!first.path.join("bootstrap-must-not-run").exists());
    fixture.stop.cancel();
}

#[tokio::test]
async fn retained_creating_and_uncertain_backing_are_not_recreated() {
    let fixture = Fixture::new().await;
    let project = fixture.project("uncertain-backing", None).await;
    let receipt = accept(&fixture, "do-not-replay").await;
    begin(&fixture, &receipt).await;
    let reserved = &receipt.snapshot().reserved;
    let intent = local(project.id);
    fixture
        .store
        .reserve_registry_workspace(vessel_store::RegistryReservation {
            workspace: reserved.workspace,
            intent: intent.clone(),
            worktree: None,
            stage: ReservationStage::Reserved,
        })
        .await
        .unwrap();
    fixture
        .store
        .set_registry_reservation_stage(
            reserved.workspace,
            ReservationStage::Reserved,
            ReservationStage::Creating,
        )
        .await
        .unwrap();
    assert!(
        matches!(fixture.workspaces.provision_workspace(Creator::Owner, reserved.operation, intent.clone()).await.unwrap(), Provisioned::InProgress(id) if id == reserved.workspace)
    );
    fixture.store.recover_registry_reservations().await.unwrap();
    assert!(
        matches!(fixture.workspaces.provision_workspace(Creator::Owner, reserved.operation, intent).await.unwrap(), Provisioned::Uncertain(id) if id == reserved.workspace)
    );
    assert!(
        fixture
            .store
            .registry_workspace(reserved.workspace)
            .await
            .unwrap()
            .is_none()
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
