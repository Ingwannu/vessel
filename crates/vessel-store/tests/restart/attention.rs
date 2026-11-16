//! Checkpoint batches preserve the ordinary append's post-commit attention wake.
use super::*;

#[tokio::test]
async fn checkpoint_records_wake_attention_after_commit_even_when_the_checkpoint_is_stale() {
    case(async {
        let (_temp, dir) = data_dir();
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&dir, StoreConfig::default(), true),
        )
        .await
        .unwrap()
        .store;
        let session = phase("Store working", working(&store)).await;
        let mut notifications = store.inbox_changes();
        drop(notifications.borrow_and_update());
        let before = phase("Store inbox_head", store.inbox_head()).await.unwrap();
        assert!(!notifications.has_changed().unwrap());
        let matched = phase(
            "Store checkpoint_restart",
            store.checkpoint_restart(
                session.id,
                vec![record(
                    2,
                    SessionEvent::Agent {
                        event: AgentEvent::TurnEnded {
                            reason: StopReason::EndTurn,
                            usage: None,
                            point: None,
                        },
                    },
                )],
                RestartCheckpoint::Arm {
                    generation: RestartGeneration::ZERO,
                    turn: EventSeq::new(1),
                },
            ),
        )
        .await
        .unwrap();
        // The turn ended before its delayed Arm: it cannot rearm recovery,
        // while its preceding event still commits and reaches attention readers.
        assert!(!matched);
        assert!(notifications.has_changed().unwrap());
        assert!(phase("Store inbox_head", store.inbox_head()).await.unwrap() > before);
        phase("Store close", store.close()).await;
    })
    .await;
}
