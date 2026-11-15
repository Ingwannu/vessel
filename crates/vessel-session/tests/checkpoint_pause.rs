//! Real actor pause preserves the conversation, held queue and parked route.
use vessel_proto::{
    AgentEvent, ContentBlock, Creator, Delivered, Delivery, QueueChange, SessionEvent, SessionInput,
};
use vessel_session::SessionError;
#[path = "support/runtime.rs"]
#[expect(
    dead_code,
    reason = "the shared fake runtime has helpers used by other suites"
)]
mod support;
use support::{Fixture, Log, prompt, select, structured, within};
fn text(value: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: value.to_owned(),
    }]
}
async fn ready(log: &mut Log) {
    log.until(|event| {
        matches!(
            event,
            SessionEvent::Agent {
                event: AgentEvent::Ready { .. }
            }
        )
    })
    .await;
}
#[tokio::test]
async fn checkpoint_pause_refuses_inputs_and_preserves_the_existing_held_queue() {
    within(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        ready(&mut log).await;
        let _held = handle.change_queue(QueueChange::Hold).await.unwrap();
        let queued = handle
            .prompt(text("queued"), Delivery::Queue, Creator::Owner)
            .await
            .unwrap();
        assert!(matches!(queued, Delivered::Queued { .. }));
        let before = handle.change_queue(QueueChange::Hold).await.unwrap().queue;
        let conversation = handle.info().harness_session.clone();
        let pause = handle.pause_for_checkpoint().await.unwrap();
        assert!(matches!(
            handle.send(prompt("must not start")).await,
            Err(SessionError::FilesPaused)
        ));
        assert!(matches!(
            handle.change_queue(QueueChange::Resume).await,
            Err(SessionError::FilesPaused)
        ));
        assert!(matches!(
            handle.set_option(select("mode", "plan")).await,
            Err(SessionError::FilesPaused)
        ));
        assert!(matches!(
            handle.park().await,
            Err(SessionError::FilesPaused)
        ));
        assert!(matches!(
            handle.pause_for_checkpoint().await,
            Err(SessionError::FilesPaused)
        ));
        pause.resume().await.unwrap();
        assert_eq!(handle.info().harness_session, conversation);
        let after = handle.change_queue(QueueChange::Hold).await.unwrap().queue;
        assert_eq!(before, after);
        let resumed = handle.change_queue(QueueChange::Resume).await.unwrap();
        assert_eq!(resumed.delivered, Some(Delivered::Started));
        log.turn_over().await;
        fixture.runtime.shutdown().await;
    })
    .await;
}
#[tokio::test]
async fn checkpoint_pause_refuses_a_running_turn_and_keeps_the_parked_route() {
    within(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        handle.send(prompt("wait")).await.unwrap();
        log.until(|event| {
            matches!(
                event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnStarted { .. }
                }
            )
        })
        .await;
        assert!(matches!(
            handle.pause_for_checkpoint().await,
            Err(SessionError::CheckpointBusy)
        ));
        handle
            .send(SessionInput::CancelTurn { turn: None })
            .await
            .unwrap();
        log.turn_over().await;
        let _parked = handle.park().await.unwrap();
        let pause = handle.pause_for_checkpoint().await.unwrap();
        assert!(matches!(
            handle.wake().await,
            Err(SessionError::FilesPaused)
        ));
        pause.resume().await.unwrap();
        assert!(handle.parked());
        handle.send(prompt("plain")).await.unwrap();
        assert!(!handle.parked());
        fixture.runtime.shutdown().await;
    })
    .await;
}
