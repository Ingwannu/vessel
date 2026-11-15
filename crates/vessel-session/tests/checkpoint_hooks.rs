//! Actual actor delivery: no queued/steered phantom capture, and a timed-out node.
use vessel_proto::{
    AgentEvent, ContentBlock, Creator, Delivered, Delivery, EventSeq, QueueChange, SessionEvent,
    SessionInput,
    checkpoint::{Boundary, CHECKPOINT_CAPTURE_DEADLINE, CheckpointError, CheckpointEvent},
};
use vessel_session::{CheckpointAdvance, CheckpointHooks};
#[cfg(test)]
#[path = "support/runtime.rs"]
#[expect(
    dead_code,
    reason = "the shared fake runtime has helpers used by other suites"
)]
mod support;
use support::{Fixture, Log, structured, within};
fn text(value: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: value.to_owned(),
    }]
}
#[tokio::test]
async fn checkpoint_hooks_capture_real_delivery_and_turn_completion_but_not_injected_steers() {
    within(async {
        let (hooks, mut requests) = CheckpointHooks::channel();
        let fixture = Fixture::checkpoint_hooks(hooks);
        let node = tokio::spawn(async move {
            let mut seen = Vec::new();
            for _boundary in 0..4 {
                let request = requests.recv().await.unwrap();
                let boundary = request.boundary();
                seen.push(boundary);
                let advanced = request.acknowledge(Err(CheckpointError::StorageFull));
                assert_eq!(advanced.await.unwrap(), match boundary {
                    Boundary::BeforeTurn { .. } => CheckpointAdvance::PromptSent,
                    Boundary::AfterTurn { .. } => CheckpointAdvance::TurnComplete,
                });
            }
            seen
        });
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        assert_eq!(handle.prompt(text("wait"), Delivery::Start, Creator::Owner).await.unwrap(), Delivered::Started);
        log.until(|event| matches!(event, SessionEvent::Agent { event: AgentEvent::TurnStarted { .. } })).await;
        assert!(matches!(handle.prompt(text("plain"), Delivery::Queue, Creator::Owner).await.unwrap(), Delivered::Queued { .. }));
        assert_eq!(handle.prompt(text("now"), Delivery::Steer, Creator::Owner).await.unwrap(), Delivered::Steered);
        handle.send(SessionInput::CancelTurn { turn: None }).await.unwrap();
        log.turn_over().await;
        let resumed = handle.change_queue(QueueChange::Resume).await.unwrap();
        assert_eq!(resumed.delivered, Some(Delivered::Started));
        let seen = node.await.unwrap();
        assert!(matches!(seen.as_slice(), [Boundary::BeforeTurn { .. }, Boundary::AfterTurn { .. }, Boundary::BeforeTurn { .. }, Boundary::AfterTurn { .. }]));
        let last = match seen.last().unwrap() { Boundary::AfterTurn { ended } => *ended, Boundary::BeforeTurn { .. } => panic!("no final after-image") };
        log.until(|event| matches!(event, SessionEvent::Checkpoint { event: CheckpointEvent::TurnAssociated { .. } })).await;
        let follow = handle.follow(EventSeq::ZERO).unwrap();
        assert!(follow.cached.iter().any(|record| record.seq == last && matches!(record.event, SessionEvent::Agent { event: AgentEvent::TurnEnded { .. } })));
        for boundary in seen {
            if let Boundary::BeforeTurn { prompt } = boundary {
                assert!(follow.cached.iter().any(|record| record.seq == prompt && matches!(record.event, SessionEvent::Checkpoint { event: CheckpointEvent::PromptAccepted })));
                assert!(follow.cached.iter().any(|record| matches!(record.event, SessionEvent::Checkpoint { event: CheckpointEvent::TurnAssociated { prompt: anchor, .. } } if anchor == prompt)));
            }
        }
        drop(follow);
        fixture.runtime.shutdown().await;
    }).await;
}
#[tokio::test]
async fn checkpoint_capture_timeout_releases_real_actor_delivery_and_late_ack_is_abandoned() {
    vessel_testkit::within(std::time::Duration::from_secs(120), async {
        let (hooks, mut requests) = CheckpointHooks::channel();
        let fixture = Fixture::checkpoint_hooks(hooks);
        let handle = fixture.start(structured(Vec::new())).await;
        let sent = handle.clone();
        let prompt = tokio::spawn(async move {
            sent.prompt(text("plain"), Delivery::Start, Creator::Owner)
                .await
        });
        let request = requests.recv().await.unwrap();
        assert!(matches!(request.boundary(), Boundary::BeforeTurn { .. }));
        tokio::time::pause();
        tokio::time::advance(CHECKPOINT_CAPTURE_DEADLINE).await;
        assert_eq!(prompt.await.unwrap().unwrap(), Delivered::Started);
        assert!(request.abandoned());
        assert!(
            request
                .acknowledge(Err(CheckpointError::CaptureTimedOut))
                .await
                .is_err()
        );
        // Closing this bridge makes the after-image unavailable immediately too.
        drop(requests);
        tokio::time::resume();
        fixture.runtime.shutdown().await;
    })
    .await
    .unwrap();
}
