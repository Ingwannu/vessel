//! Real actor pause preserves the conversation, held queue and parked route.
use vessel_proto::{
    AgentEvent, ContentBlock, Creator, Delivered, Delivery, QueueChange, SessionEvent, SessionInput,
};
use vessel_session::SessionError;
#[cfg(test)]
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
        assert!(handle.info().parked.is_some());
        handle.send(prompt("plain")).await.unwrap();
        let mut info = handle.watch_info();
        assert!(info.wait_for(|info| info.parked.is_none()).await.is_ok());
        fixture.runtime.shutdown().await;
    })
    .await;
}

#[tokio::test]
async fn checkpoint_owned_pause_publishes_one_result_and_keeps_queue_held_until_resume() {
    use vessel_proto::checkpoint::{CheckpointEvent, CheckpointId, PathCount, RestoreId};
    within(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        ready(&mut log).await;
        let _held = handle.change_queue(QueueChange::Hold).await.unwrap();
        let _queued = handle.prompt(text("queued"), Delivery::Queue, Creator::Owner).await.unwrap();
        let queue = handle.change_queue(QueueChange::Hold).await.unwrap().queue;
        let conversation = handle.info().harness_session.clone();
        let mut pause = handle.pause_for_checkpoint().await.unwrap();
        let restore = RestoreId::random();
        let checkpoint = CheckpointId::random();
        let paths = PathCount::new(1).unwrap();
        let seq = pause.files_restored(restore, checkpoint, paths).await.unwrap();
        assert_eq!(pause.files_restored(restore, checkpoint, paths).await.unwrap(), seq);
        assert!(matches!(pause.files_restored(RestoreId::random(), checkpoint, paths).await, Err(SessionError::CheckpointBusy)));
        log.until(|event| matches!(event, SessionEvent::Checkpoint { event: CheckpointEvent::FilesRestored { restore: id, .. } } if *id == restore)).await;
        assert_eq!(log.records.iter().filter(|record| matches!(&record.event, SessionEvent::Checkpoint { event: CheckpointEvent::FilesRestored { .. } })).count(), 1);
        assert_eq!(handle.info().harness_session, conversation);
        assert!(matches!(handle.send(prompt("still paused")).await, Err(SessionError::FilesPaused)));
        pause.resume().await.unwrap();
        assert_eq!(handle.change_queue(QueueChange::Hold).await.unwrap().queue, queue);
        let _resumed = handle.change_queue(QueueChange::Resume).await.unwrap();
        log.turn_over().await;
        fixture.runtime.shutdown().await;
    }).await;
}
