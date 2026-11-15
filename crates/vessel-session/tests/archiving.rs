//! Conditional archive races actual mailbox deliveries using only the fake harness.
use support::{Fixture, Log, prompt, structured, within};
use vessel_proto::{SessionInput, session_archive::ArchiveCondition};
use vessel_session::{ArchiveRefusal, SessionError};

#[cfg(test)]
#[path = "support/runtime.rs"]
#[expect(
    dead_code,
    reason = "shared fake runtime fixture provides helpers used by other suites"
)]
mod support;

#[tokio::test]
async fn idle_archive_stops_the_actor_and_no_later_prompt_can_start() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        handle.send(prompt("plain")).await.unwrap();
        log.turn_over().await;
        let ended = handle
            .stop_for_archive(ArchiveCondition::IfIdle)
            .await
            .unwrap();
        assert_eq!(ended.id, handle.id());
        assert!(!ended.status.is_live());
        assert!(matches!(
            handle.send(prompt("wait")).await,
            Err(SessionError::Ended)
        ));
        assert!(
            ended.marks.archived_at.is_none(),
            "only the node may commit the archive timestamp"
        );
    }))
    .await;
}

#[tokio::test]
async fn active_turn_refusal_makes_no_stop_and_force_is_explicit() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        handle.send(prompt("wait")).await.unwrap();
        let refused = handle.stop_for_archive(ArchiveCondition::IfIdle).await;
        assert!(
            matches!(
                refused,
                Err(SessionError::NotArchived(ArchiveRefusal::TurnRunning))
            ),
            "{refused:?}"
        );
        assert!(handle.info().status.is_live());
        assert!(handle.turn_running().await.unwrap());
        let ended = handle
            .stop_for_archive(ArchiveCondition::Force)
            .await
            .unwrap();
        assert!(!ended.status.is_live());
    }))
    .await;
}

#[tokio::test]
async fn racing_new_prompt_and_idle_archive_cannot_both_be_admitted() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        handle.send(prompt("plain")).await.unwrap();
        log.turn_over().await;
        let (archived, sent) = tokio::join!(
            handle.stop_for_archive(ArchiveCondition::IfIdle),
            handle.send(prompt("wait"))
        );
        if let Ok(ended) = &archived {
            assert!(!ended.status.is_live());
            assert!(
                sent.is_err(),
                "an actor-admitted archive must fence a concurrent new turn"
            );
        } else {
            assert!(
                matches!(
                    archived,
                    Err(SessionError::NotArchived(ArchiveRefusal::TurnRunning))
                ),
                "{archived:?}"
            );
            sent.unwrap();
            let _ended = handle
                .stop_for_archive(ArchiveCondition::Force)
                .await
                .unwrap();
        }
    }))
    .await;
}

#[tokio::test]
async fn parked_actor_can_archive_without_waking_or_sending_any_prompt() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        handle.send(prompt("plain")).await.unwrap();
        log.turn_over().await;
        let parked = handle.park().await.unwrap();
        assert!(parked.parked.is_some());
        let ended = handle
            .stop_for_archive(ArchiveCondition::IfIdle)
            .await
            .unwrap();
        assert!(!ended.status.is_live());
        assert!(
            handle
                .send(SessionInput::Prompt {
                    content: Vec::new()
                })
                .await
                .is_err()
        );
    }))
    .await;
}
