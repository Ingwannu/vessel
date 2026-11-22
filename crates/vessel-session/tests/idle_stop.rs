//! Automatic archive shares admission with ordinary session deliveries.

use support::{Fixture, Log, structured, within};
use vessel_proto::{AgentEvent, ContentBlock, Creator, Delivery, SessionEvent, SessionInput};
use vessel_session::{IdleStop, SessionError};

#[cfg(test)]
#[path = "support/runtime.rs"]
#[expect(
    dead_code,
    reason = "only the session helpers of the shared fixture are used"
)]
mod support;

fn text(text: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: text.to_owned(),
    }]
}

#[tokio::test]
async fn idle_stop_preserves_admitted_work_and_refuses_later_deliveries() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        let _started = handle
            .prompt(text("wait"), Delivery::Start, Creator::Owner)
            .await
            .unwrap();
        assert_eq!(handle.stop_if_idle().await.unwrap(), IdleStop::Busy);
        let _queued = handle
            .prompt(text("plain"), Delivery::Queue, Creator::Owner)
            .await
            .unwrap();
        handle
            .send(SessionInput::CancelTurn { turn: None })
            .await
            .unwrap();
        log.until(|event| {
            matches!(
                event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded { .. }
                }
            )
        })
        .await;
        assert_eq!(handle.stop_if_idle().await.unwrap(), IdleStop::Busy);
        let queued = handle.conversation().unwrap().queue.entries[0].id;
        let _empty = handle
            .change_queue(vessel_proto::QueueChange::Remove { entry: queued })
            .await
            .unwrap();
        assert_eq!(handle.stop_if_idle().await.unwrap(), IdleStop::Stopped);
        assert!(matches!(
            handle
                .prompt(text("late"), Delivery::Start, Creator::Owner)
                .await,
            Err(SessionError::Ended)
        ));
        let _end = handle.ended().await;
    }))
    .await;
}
