//! Real writer/CAS with an in-memory effect boundary; no agents or files launch.
use super::*;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use vessel_proto::creation::{Key, Kind, Resource};
use vessel_proto::{BlobHash, SessionId, WorkspaceId};
use vessel_store::{CreationAdmission, CreationIdentity, DataDir, StoreConfig};

struct Fake {
    store: Store,
    reserved: Reservation,
    trace: Mutex<Vec<Stage>>,
    refuse_prompt: AtomicBool,
    uncertain_resource: bool,
}
impl Runtime for Fake {
    fn fingerprint(&self) -> Fingerprint {
        Fingerprint::new(BlobHash::from_bytes([7; 32]))
    }
    async fn execute(&self, stage: Stage, reserved: &Reservation) -> Result<(), StepFailure> {
        let actual = self
            .store
            .creation_operation(reserved.operation)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            actual.in_flight(),
            Some(stage),
            "effect escaped the durable marker acknowledgement"
        );
        assert_eq!(
            reserved, &self.reserved,
            "a retry changed reserved identities"
        );
        self.trace.lock().unwrap().push(stage);
        if stage == Stage::Resource && self.uncertain_resource {
            return Err(StepFailure::Uncertain(ErrorCode::Internal));
        }
        if stage == Stage::Prompt && self.refuse_prompt.swap(false, Ordering::AcqRel) {
            return Err(StepFailure::Refused(ErrorCode::Conflict));
        }
        Ok(())
    }
}
async fn fixture() -> (tempfile::TempDir, DataDir, Store, Reservation) {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let reserved = Reservation {
        operation: WorkspaceOperationId::random(),
        workspace: WorkspaceId::random(),
        workspace_claim: WorkspaceClaim::Create,
        resource: Some(Resource::Agent {
            session: SessionId::random(),
        }),
        first_input: FirstInput::Identified {
            message: vessel_proto::timeline::ClientMessageId::random(),
        },
    };
    let receipt = Receipt::admitted(
        Kind::Workspace,
        reserved.clone(),
        Fingerprint::new(BlobHash::from_bytes([7; 32])),
    )
    .unwrap();
    let _receipt = store
        .admit_creation(CreationAdmission {
            identity: CreationIdentity::new(Kind::Workspace, Key::parse("operation").unwrap()),
            receipt,
        })
        .await
        .unwrap();
    (temp, dir, store, reserved)
}
fn fake(store: &Store, reserved: Reservation, refuse: bool, uncertain: bool) -> Fake {
    Fake {
        store: store.clone(),
        reserved,
        trace: Mutex::new(Vec::new()),
        refuse_prompt: AtomicBool::new(refuse),
        uncertain_resource: uncertain,
    }
}
#[tokio::test]
async fn every_effect_observes_its_committed_marker_and_prompt_retry_reuses_registration() {
    let (_temp, _dir, store, reserved) = fixture().await;
    let runtime = fake(&store, reserved.clone(), true, false);
    let failed = drive(&store, reserved.operation, &runtime).await.unwrap();
    assert!(matches!(
        failed.phase,
        Phase::Failed {
            stage: Stage::Prompt,
            certainty: Certainty::Known,
            ..
        }
    ));
    assert_eq!(failed.reserved, reserved);
    let complete = drive(&store, reserved.operation, &runtime).await.unwrap();
    assert_eq!(complete.phase, Phase::Completed);
    assert_eq!(complete.reserved, reserved);
    assert_eq!(
        *runtime.trace.lock().unwrap(),
        [
            Stage::Workspace,
            Stage::Resource,
            Stage::Prompt,
            Stage::Prompt
        ]
    );
    let _same = drive(&store, reserved.operation, &runtime).await.unwrap();
    assert_eq!(runtime.trace.lock().unwrap().len(), 4);
    store.close().await;
}
#[tokio::test]
async fn unknown_resource_effect_is_never_dispatched_again() {
    let (_temp, _dir, store, reserved) = fixture().await;
    let runtime = fake(&store, reserved.clone(), false, true);
    let unknown = drive(&store, reserved.operation, &runtime).await.unwrap();
    assert!(matches!(
        unknown.phase,
        Phase::Failed {
            stage: Stage::Resource,
            certainty: Certainty::Unknown,
            ..
        }
    ));
    assert_eq!(
        drive(&store, reserved.operation, &runtime).await.unwrap(),
        unknown
    );
    assert_eq!(
        *runtime.trace.lock().unwrap(),
        [Stage::Workspace, Stage::Resource]
    );
    store.close().await;
}
#[tokio::test]
async fn crash_after_begin_before_effect_is_unknown_but_idle_accepted_retains_original_ids() {
    for began in [false, true] {
        let (_temp, dir, store, reserved) = fixture().await;
        if began {
            let _marked = store
                .advance_creation(
                    reserved.operation,
                    vessel_proto::creation::Revision::INITIAL,
                    Advance::Begin(Stage::Workspace),
                )
                .await
                .unwrap();
        }
        store.close().await;
        drop(store);
        let store = Store::open(&dir, StoreConfig::default())
            .await
            .unwrap()
            .store;
        let runtime = fake(&store, reserved.clone(), false, false);
        let observed = store
            .creation_operation(reserved.operation)
            .await
            .unwrap()
            .unwrap();
        assert!(runtime.trace.lock().unwrap().is_empty());
        assert_eq!(observed.snapshot().reserved, reserved);
        let result = drive(&store, reserved.operation, &runtime).await.unwrap();
        if began {
            assert!(matches!(
                result.phase,
                Phase::Failed {
                    certainty: Certainty::Unknown,
                    ..
                }
            ));
            assert!(runtime.trace.lock().unwrap().is_empty());
        } else {
            assert_eq!(result.phase, Phase::Completed);
            assert_eq!(
                *runtime.trace.lock().unwrap(),
                [Stage::Workspace, Stage::Resource, Stage::Prompt]
            );
        }
        store.close().await;
    }
}
