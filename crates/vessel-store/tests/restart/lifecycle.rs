//! New-run and late-finish boundaries of the durable recovery projection.
use super::*;

#[tokio::test]
async fn manual_resume_consumes_the_old_intent_without_rearming_an_idle_run() {
    case(async {
        let (_dir, data) = data_dir();
        let config = StoreConfig::default();
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&data, config, true),
        )
        .await
        .unwrap()
        .store;
        let session = phase("Store working", working(&store)).await;
        phase("Store snapshot", snapshot(&store, session.id)).await;
        phase(
            "Store append",
            store.append(session.id, vec![status(2, SessionStatus::Interrupted)]),
        )
        .await
        .unwrap();
        assert!(
            phase("Store next_restart", store.next_restart())
                .await
                .unwrap()
                .is_some()
        );
        let _marks = phase("Store reopen", store.reopen(session.id, EventSeq::new(3)))
            .await
            .unwrap();
        phase(
            "Store append",
            store.append(
                session.id,
                vec![
                    status(3, SessionStatus::Running),
                    status(4, SessionStatus::Interrupted),
                ],
            ),
        )
        .await
        .unwrap();
        assert!(
            phase("Store next_restart", store.next_restart())
                .await
                .unwrap()
                .is_none()
        );
        phase("Store close", store.close()).await;
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&data, config, true),
        )
        .await
        .unwrap()
        .store;
        assert!(
            phase("Store next_restart", store.next_restart())
                .await
                .unwrap()
                .is_none()
        );
        phase("Store close", store.close()).await;
    })
    .await;
}
#[tokio::test]
async fn finishing_an_old_attempt_preserves_a_new_actual_turns_shutdown_intent() {
    case(async {
        let (_dir, data) = data_dir();
        let config = StoreConfig::default();
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&data, config, true),
        )
        .await
        .unwrap()
        .store;
        let session = phase("Store working", working(&store)).await;
        phase("Store snapshot", snapshot(&store, session.id)).await;
        phase(
            "Store append",
            store.append(session.id, vec![status(2, SessionStatus::Interrupted)]),
        )
        .await
        .unwrap();
        let original = phase("Store next_restart", store.next_restart())
            .await
            .unwrap()
            .unwrap();
        assert!(
            phase("Store claim_restart", store.claim_restart(original))
                .await
                .unwrap()
        );
        let _marks = phase(
            "Store reopen_for_restart",
            store.reopen_for_restart(session.id, EventSeq::new(3)),
        )
        .await
        .unwrap();
        assert!(
            phase(
                "Store checkpoint_restart",
                store.checkpoint_restart(
                    session.id,
                    vec![status(3, SessionStatus::Running), started(4)],
                    RestartCheckpoint::Arm {
                        generation: original.generation,
                        turn: EventSeq::new(4)
                    }
                )
            )
            .await
            .unwrap()
        );
        assert!(
            phase(
                "Store checkpoint_restart",
                store.checkpoint_restart(
                    session.id,
                    Vec::new(),
                    RestartCheckpoint::Shutdown {
                        generation: original.generation,
                        turn: Some(EventSeq::new(4)),
                        parked: false
                    }
                )
            )
            .await
            .unwrap()
        );
        phase(
            "Store append",
            store.append(session.id, vec![status(5, SessionStatus::Interrupted)]),
        )
        .await
        .unwrap();
        phase("Store finish_restart", store.finish_restart(session.id))
            .await
            .unwrap();
        assert_eq!(
            phase("Store next_restart", store.next_restart())
                .await
                .unwrap()
                .unwrap()
                .turn,
            EventSeq::new(4)
        );
        phase("Store close", store.close()).await;
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&data, config, true),
        )
        .await
        .unwrap()
        .store;
        assert_eq!(
            phase("Store next_restart", store.next_restart())
                .await
                .unwrap()
                .unwrap()
                .turn,
            EventSeq::new(4)
        );
        phase("Store close", store.close()).await;
    })
    .await;
}
#[tokio::test]
async fn suppressing_an_ended_run_survives_archive_and_unarchive_and_does_not_touch_a_new_run() {
    case(async {
        let (_dir, data) = data_dir();
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&data, StoreConfig::default(), true),
        )
        .await
        .unwrap()
        .store;
        let session = phase("Store working", working(&store)).await;
        phase("Store snapshot", snapshot(&store, session.id)).await;
        phase(
            "Store append",
            store.append(session.id, vec![status(2, SessionStatus::Interrupted)]),
        )
        .await
        .unwrap();
        assert!(
            phase(
                "Store suppress_ended_restart",
                store.suppress_ended_restart(session.id, EventSeq::new(3))
            )
            .await
            .unwrap()
        );
        let _marks = phase(
            "Store mark",
            store.mark(session.id, vessel_proto::Mark::Archive, at(5)),
        )
        .await
        .unwrap();
        let _marks = phase(
            "Store mark",
            store.mark(session.id, vessel_proto::Mark::Unarchive, at(6)),
        )
        .await
        .unwrap();
        assert!(
            phase("Store next_restart", store.next_restart())
                .await
                .unwrap()
                .is_none()
        );
        let _marks = phase("Store reopen", store.reopen(session.id, EventSeq::new(3)))
            .await
            .unwrap();
        assert!(
            !phase(
                "Store suppress_ended_restart",
                store.suppress_ended_restart(session.id, EventSeq::new(3))
            )
            .await
            .unwrap(),
            "the same cursor cannot suppress a newer live run"
        );
        assert!(
            phase(
                "Store checkpoint_restart",
                store.checkpoint_restart(
                    session.id,
                    vec![status(3, SessionStatus::Running), started(4)],
                    RestartCheckpoint::Arm {
                        generation: phase(
                            "Store restart_generation",
                            store.restart_generation(session.id)
                        )
                        .await
                        .unwrap(),
                        turn: EventSeq::new(4)
                    }
                )
            )
            .await
            .unwrap()
        );
        phase("Store close", store.close()).await;
        let store = phase(
            "Store open_with_restart_policy",
            Store::open_with_restart_policy(&data, StoreConfig::default(), true),
        )
        .await
        .unwrap()
        .store;
        assert_eq!(
            phase("Store next_restart", store.next_restart())
                .await
                .unwrap()
                .unwrap()
                .turn,
            EventSeq::new(4)
        );
        phase("Store close", store.close()).await;
    })
    .await;
}
#[tokio::test]
async fn a_stop_between_resume_preparation_and_reopen_is_a_conflict_then_a_fresh_resume_rearms() {
    case(async {
    let (_dir, data) = data_dir();
    let store = phase("Store open_with_restart_policy", Store::open_with_restart_policy(&data, StoreConfig::default(), true))
        .await
        .unwrap()
        .store;
    let session = phase("Store working", working(&store)).await;
    phase("Store snapshot", snapshot(&store, session.id)).await;
    phase("Store append", store
        .append(session.id, vec![status(2, SessionStatus::Interrupted)]))
        .await
        .unwrap();
    let prepared = phase("Store restart_generation", store.restart_generation(session.id)).await.unwrap();
    assert!(
        phase("Store suppress_ended_restart", store
            .suppress_ended_restart(session.id, EventSeq::new(3)))
            .await
            .unwrap()
    );
    assert!(
        matches!(phase("Store reopen_exact", store.reopen_exact(session.id,EventSeq::new(3),prepared,vessel_store::ReopenMode::Manual)).await,Err(vessel_store::StoreError::RestartChanged(id)) if id==session.id)
    );
    let fresh = phase("Store restart_generation", store.restart_generation(session.id)).await.unwrap();
    let _marks = phase("Store reopen_exact", store
        .reopen_exact(
            session.id,
            EventSeq::new(3),
            fresh,
            vessel_store::ReopenMode::Manual,
        ))
        .await
        .unwrap();
    assert!(
        phase("Store checkpoint_restart", store
            .checkpoint_restart(
                session.id,
                vec![status(3, SessionStatus::Running), started(4)],
                RestartCheckpoint::Arm {
                    generation: fresh,
                    turn: EventSeq::new(4)
                }
            ))
            .await
            .unwrap()
    );
    phase("Store close", store.close()).await;
    let store = phase("Store open_with_restart_policy", Store::open_with_restart_policy(&data, StoreConfig::default(), true))
        .await
        .unwrap()
        .store;
    assert_eq!(
        phase("Store next_restart", store.next_restart()).await.unwrap().unwrap().turn,
        EventSeq::new(4)
    );
    phase("Store close", store.close()).await;
    }).await;
}
