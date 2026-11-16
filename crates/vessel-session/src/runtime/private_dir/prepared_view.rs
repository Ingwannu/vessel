//! The public view retains the original runtime and claim, while the canonical
//! node/runtime admission remains the producer's single close observation.

use super::*;
use crate::{PreparationAdmission, PreparationCancellation, PreparationGate};
use vessel_proto::{HarnessSessionId, SessionId, SessionInfo};

const VIEW_STEP: Duration = Duration::from_secs(20);

fn ended(fixture: &Fixture) -> Ended {
    let mut info = SessionInfo::new(SessionId::random(), fixture.spec(), chrono::Utc::now());
    info.harness_session = Some(HarnessSessionId::new("prepared-view-conversation"));
    info.status = SessionStatus::Interrupted;
    Ended {
        info,
        choices: Vec::new(),
        queue: PromptQueue::default(),
        usage: None,
        attention: None,
        moved_through: BTreeSet::new(),
    }
}

fn admit_live(
    _effect: crate::PreparationEffect,
    live: PreparationCancellation<'_>,
) -> PreparationAdmission {
    match live.check() {
        Ok(()) => PreparationAdmission::Admitted,
        Err(reason) => PreparationAdmission::Refused(reason),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn temporary_view_returns_a_prepared_session_with_its_original_claim() {
    within(async {
        let fixture = Fixture::new();
        let original = ended(&fixture);
        let id = original.info.id;
        let prepared = tokio::time::timeout(
            VIEW_STEP,
            fixture.runtime
                .with_preparation(PreparationGate::new(CancellationToken::new(), admit_live))
                .prepare_again(original.clone(), Relaunch::default()),
        ).await.expect("public view preparation must settle").unwrap();
        assert!(fixture.runtime.get(id).is_none());
        assert!(fixture.runtime.pids().is_empty());
        let mut duplicate = original;
        duplicate.info.id = SessionId::random();
        assert!(matches!(
            tokio::time::timeout(VIEW_STEP, fixture.runtime.prepare_again(duplicate, Relaunch::default()))
                .await.expect("duplicate claim must be refused without preparing"),
            Err(crate::SessionError::ConversationTaken(owner)) if owner == id
        ));
        let Reopening {
            created: Created {
                handle, durable, ..
            },
            gate,
        } = prepared.start().unwrap();
        assert_eq!(handle.id(), id);
        assert!(fixture.runtime.get(id).is_some());
        drain(durable);
        gate.open();
        tokio::time::timeout(VIEW_STEP, until_running(&handle))
            .await.expect("registered prepared session must become ready");
        assert_eq!(tokio::time::timeout(VIEW_STEP, carried(&handle))
            .await.expect("actual actor launch context must answer"), Some(fixture.private()));
        handle.kill();
        let _ended = tokio::time::timeout(VIEW_STEP, handle.ended())
            .await.expect("prepared run must end after kill");
        tokio::time::timeout(VIEW_STEP, fixture.runtime.shutdown())
            .await.expect("prepared runtime must drain");
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prepared_start_uses_the_original_runtime_node_admission() {
    within(async {
        let mut fixture = Fixture::new();
        let node_stop = CancellationToken::new();
        fixture.runtime = fixture.runtime.with_admission(node_stop.clone());
        let original = ended(&fixture);
        let id = original.info.id;
        let prepared = tokio::time::timeout(
            VIEW_STEP,
            fixture
                .runtime
                .with_preparation(PreparationGate::new(CancellationToken::new(), admit_live))
                .prepare_again(original, Relaunch::default()),
        )
        .await
        .expect("pre-close public view preparation must settle")
        .unwrap();
        node_stop.cancel();
        assert!(matches!(
            prepared.start(),
            Err(crate::SessionError::ShuttingDown)
        ));
        assert!(fixture.runtime.get(id).is_none());
        assert!(fixture.runtime.pids().is_empty());
        tokio::time::timeout(VIEW_STEP, fixture.runtime.shutdown())
            .await
            .expect("node-closed runtime must drain");
    })
    .await;
}
