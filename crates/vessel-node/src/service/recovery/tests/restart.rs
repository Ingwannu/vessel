//! Clean-stop behavior with the real node, journal writer and fake provider.
use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_clean_stop_continues_the_same_conversation_once_and_keeps_pending_input_held() {
    vessel_testkit::within(CASE_DEADLINE, Box::pin(async {
        let f = Fixture::new();
        let node = f.start().await;
        let owner = f.client(&node).await;
        set(&node, true).await;
        let original = f.active(&node, &owner).await;
        assert!(matches!(owner.prompt(original.id, vec![ContentBlock::Text { text: "not sent".into() }], Delivery::Queue).await.unwrap(), vessel_proto::Delivered::Queued { .. }));
        node.stop().await.unwrap();
        let node = f.start().await;
        let owner = f.client(&node).await;
        done(&node).await;
        let continued = owner.session(original.id).await.unwrap();
        assert_eq!(continued.harness_session, original.harness_session);
        assert_eq!(continued.created_by, original.created_by);
        assert_eq!(continued.created_at, original.created_at);
        assert_eq!(continued.profile, original.profile);
        assert_eq!(continued.model, original.model);
        let mut events = owner.follow(original.id, original.next_seq).await.unwrap();
        loop {
            let record = events.next().await.unwrap().unwrap();
            if matches!(record.event, SessionEvent::Agent { event: AgentEvent::TurnEnded { .. } }) { break; }
        }
        let conversation = owner.conversation(original.id).await.unwrap();
        assert_eq!(conversation.queue.held, Some(vessel_proto::QueueHold::Ended));
        assert_eq!(conversation.queue.entries.len(), 1);
        let before = owner.read(original.id, EventSeq::ZERO, EVENT_PAGE).await.unwrap().records;
        assert_eq!(before.iter().filter(|record| continuation(record)).count(), 1);
        assert!(!before.iter().any(|record| matches!(&record.event, SessionEvent::Agent { event: AgentEvent::TurnStarted { content } } if content == &vec![ContentBlock::Text { text: "not sent".into() }])));
        node.stop().await.unwrap();
        let node = f.start().await;
        let owner = f.client(&node).await;
        done(&node).await;
        let after = owner.read(original.id, EventSeq::ZERO, EVENT_PAGE).await.unwrap().records;
        assert_eq!(after.iter().filter(|record| continuation(record)).count(), 1);
        node.stop().await.unwrap();
    })).await.unwrap();
}
pub(super) fn continuation(record: &EventRecord) -> bool {
    matches!(&record.event, SessionEvent::Agent { event: AgentEvent::TurnStarted { content } } if content == &vec![ContentBlock::Text { text: "loaded".into() }])
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn acknowledged_person_stop_is_not_recovered_at_the_next_start() {
    vessel_testkit::within(
        CASE_DEADLINE,
        Box::pin(async {
            let f = Fixture::new();
            let node = f.start().await;
            let owner = f.client(&node).await;
            set(&node, true).await;
            let original = f.active(&node, &owner).await;
            owner.kill(original.id).await.unwrap();
            node.stop().await.unwrap();
            let node = f.start().await;
            let owner = f.client(&node).await;
            done(&node).await;
            let records = owner
                .read(original.id, EventSeq::ZERO, EVENT_PAGE)
                .await
                .unwrap()
                .records;
            assert!(!records.iter().any(continuation));
            assert!(!owner.session(original.id).await.unwrap().status.is_live());
            node.stop().await.unwrap();
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unassigned_client_fields_refuse_before_mutating_the_real_saved_policy() {
    vessel_testkit::within(
        CASE_DEADLINE,
        Box::pin(async {
            let f = Fixture::new();
            let node = f.start().await;
            let owner = f.client(&node).await;
            set(&node, true).await;
            let before = f.data.read_settings_record().unwrap().unwrap();
            assert!(matches!(
                owner.restart_recovery().await,
                Err(vessel_client::CallError::UnassignedProtocol {
                    method: vessel_proto::Method::GetNodeSettings
                })
            ));
            assert!(matches!(
                owner
                    .set_restart_recovery(SetRestartRecovery {
                        enabled: Some(false),
                        prompt: None
                    })
                    .await,
                Err(vessel_client::CallError::UnassignedProtocol {
                    method: vessel_proto::Method::SetNodeSettings
                })
            ));
            assert_eq!(f.data.read_settings_record().unwrap().unwrap(), before);
            let legacy = owner
                .call(vessel_proto::settings::GetNodeSettings::default())
                .await
                .unwrap();
            assert!(
                legacy.restart_recovery.is_default(),
                "a hidden default is not a capability acknowledgement"
            );
            assert!(
                node.secure
                    .service
                    .0
                    .settings
                    .current()
                    .await
                    .restart_recovery
                    .enabled
            );
            node.stop().await.unwrap();
        }),
    )
    .await
    .unwrap();
}
