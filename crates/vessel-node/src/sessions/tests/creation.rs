//! The real fake actor settles the same count, start lease and creation receipt.
use super::*;
use crate::principal::Principal;
use vessel_proto::creation::{
    Advance, Fingerprint, FirstInput, Key, Kind, Phase, Receipt, Reservation, Resource, Stage,
    WorkspaceClaim,
};
use vessel_proto::registry::{
    RegistryRevision, WorkspaceAutomation, WorkspaceBacking, WorkspaceDescriptor,
    WorkspaceLifecycle, WorkspacePin, WorkspaceResource, WorkspaceVisibility,
};
use vessel_proto::{BlobHash, ProjectId, WorkspaceId, WorkspaceOperationId};
use vessel_store::{CreationAdmission, CreationIdentity, RegistryRecord};

async fn workspace(fixture: &Fixture) -> WorkspaceId {
    let workspace = WorkspaceId::random();
    fixture
        .store
        .put_registry_workspace(
            RegistryRecord {
                descriptor: WorkspaceDescriptor {
                    id: workspace,
                    project: ProjectId::random(),
                    path: fixture.data.path().to_path_buf(),
                    title: None,
                    automation: WorkspaceAutomation::Allowed,
                    pin: WorkspacePin::Unpinned,
                    backing: WorkspaceBacking::Local,
                    lifecycle: WorkspaceLifecycle::Active,
                    visibility: WorkspaceVisibility::Foreground,
                    created_at: chrono::Utc::now(),
                    revision: RegistryRevision::ZERO,
                },
                recovery: None,
            },
            None,
        )
        .await
        .unwrap();
    workspace
}

async fn admit(fixture: &Fixture, workspace: WorkspaceId, session: SessionId) -> Receipt {
    let admitted = fixture
        .store
        .admit_creation(CreationAdmission {
            identity: CreationIdentity::new(
                Kind::Agent,
                Key::parse("actor-registration-join").unwrap(),
            ),
            receipt: Receipt::admitted(
                Kind::Agent,
                Reservation {
                    operation: WorkspaceOperationId::random(),
                    workspace,
                    workspace_claim: WorkspaceClaim::Existing,
                    resource: Some(Resource::Agent { session }),
                    first_input: FirstInput::Identified {
                        message: vessel_proto::timeline::ClientMessageId::random(),
                    },
                },
                Fingerprint::new(BlobHash::from_bytes([5; 32])),
            )
            .unwrap(),
        })
        .await
        .unwrap();
    admitted
}

#[tokio::test]
async fn resource_ready_requires_real_actor_registration_and_durable_workspace_settlement() {
    vessel_testkit::within(std::time::Duration::from_secs(30), async {
        let fixture = Fixture::new().await;
        let workspace = workspace(&fixture).await;
        let session = SessionId::random();
        let mut count = fixture
            .sessions
            .agents
            .reserve_creation(session, &Principal::Owner, false)
            .unwrap();
        let admitted = admit(&fixture, workspace, session).await;
        let resource = WorkspaceResource::Session { session };
        let start = fixture
            .store
            .claim_registry_start(workspace, resource, RegistryRevision::ZERO)
            .await
            .unwrap();
        let begin = fixture
            .store
            .advance_creation(
                admitted.snapshot().reserved.operation,
                admitted.snapshot().revision,
                Advance::Begin(Stage::Resource),
            )
            .await
            .unwrap();
        let ready = fixture
            .sessions
            .create_receipted(
                fixture.structured(),
                LaunchOptions::default(),
                &mut count,
                start,
                &begin,
            )
            .await
            .unwrap();
        assert_eq!(ready.snapshot().phase, Phase::ResourceReady);
        assert_eq!(ready.snapshot().reserved, admitted.snapshot().reserved);
        assert_eq!(ready.in_flight(), None);
        assert_eq!(fixture.sessions.get(session).await.unwrap().id, session);
        assert!(fixture.runtime.get(session).is_some());
        assert_eq!(
            fixture
                .store
                .registry_resource_owner(resource)
                .await
                .unwrap(),
            Some(workspace)
        );
        assert!(
            fixture
                .store
                .pending_registry_starts(workspace)
                .await
                .unwrap()
                .is_empty()
        );
        drop(count);
        assert!(fixture.sessions.agents.is_live(session));
        assert!(
            fixture
                .sessions
                .agents
                .reserve_creation(session, &Principal::Owner, false)
                .is_err()
        );
        // No prompt owner was called and no Prompt Begin/acceptance was minted.
        assert_eq!(
            fixture
                .store
                .creation_operation(admitted.snapshot().reserved.operation)
                .await
                .unwrap()
                .unwrap(),
            ready
        );
        fixture.stop().await;
    })
    .await
    .unwrap();
}
