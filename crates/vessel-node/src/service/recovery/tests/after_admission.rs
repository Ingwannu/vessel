//! Effects already admitted keep their explicit in-flight boundary.
use super::*;
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn off_after_prompt_admission_does_not_kill_that_turn_or_reconstruct_it_later() {
    vessel_testkit::within(
        CASE_DEADLINE,
        Box::pin(async {
            let f = Fixture::new();
            let node = f.start().await;
            let owner = f.client(&node).await;
            let mut policy = change(true);
            policy.restart_recovery.as_mut().unwrap().prompt = Some("wait".parse().unwrap());
            let installed = node.secure.service.0.settings.change(policy).await.unwrap();
            assert!(installed.restart_recovery.enabled);
            assert_eq!(installed.restart_recovery.prompt.as_str(), "wait");
            let original = f.active(&node, &owner).await;
            let after_initial = after_initial_turn(&owner, original.id).await;
            node.stop().await.unwrap();
            let node = f.start().await;
            // Listening does not promise that the stored session has been
            // reopened: its ordinary subscription may still end at that log.
            done(&node).await;
            let owner = f.client(&node).await;
            let mut stream = owner.follow(original.id, after_initial).await.unwrap();
            loop {
                let record = stream.next().await.unwrap().unwrap();
                if waited(&record) {
                    break;
                }
            }
            set(&node, false).await;
            assert_eq!(
                owner.session(original.id).await.unwrap().status,
                SessionStatus::Running
            );
            let refused = owner
                .prompt(
                    original.id,
                    vec![ContentBlock::Text {
                        text: "not admitted".into(),
                    }],
                    Delivery::Start,
                )
                .await;
            assert!(refused.is_err());
            node.stop().await.unwrap();
            let node = f.start().await;
            let owner = f.client(&node).await;
            done(&node).await;
            let records = owner
                .read(original.id, EventSeq::ZERO, EVENT_PAGE)
                .await
                .unwrap()
                .records;
            assert_eq!(records.iter().filter(|record| waited(record)).count(), 2);
            assert_eq!(
                owner.session(original.id).await.unwrap().status,
                SessionStatus::Interrupted
            );
            node.stop().await.unwrap();
        }),
    )
    .await
    .unwrap();
}
async fn after_initial_turn(owner: &NodeClient, session: SessionId) -> EventSeq {
    // Use the actual committed event, not a possibly earlier info-watch
    // snapshot, so the second wait cannot observe the original turn again.
    owner
        .read(session, EventSeq::ZERO, EVENT_PAGE)
        .await
        .unwrap()
        .records
        .into_iter()
        .find(waited)
        .unwrap()
        .seq
        .next()
        .unwrap()
}
fn waited(record: &EventRecord) -> bool {
    matches!(&record.event, SessionEvent::Agent {
        event: AgentEvent::TurnStarted { content },
    } if content == &vec![ContentBlock::Text { text: "wait".into() }])
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_admitted_current_grant_fences_revoke_and_a_new_grant_reads_revocation() {
    vessel_testkit::within(
        CASE_DEADLINE,
        Box::pin(async {
            let f = Fixture::new();
            let node = f.start().await;
            let owner = f.client(&node).await;
            let (id, device) = grants::pair(&node, &owner).await;
            let devices = &node.secure.service.0.devices;
            let creator = vessel_proto::Creator::Device { id };
            let grant = devices.recovery_grant(&creator).await.unwrap();
            let mut revoking = Box::pin(devices.revoke(id));
            assert!(futures_util::poll!(revoking.as_mut()).is_pending());
            assert!(devices.scopes_now(id).await.unwrap().is_some());
            drop(grant);
            revoking.await.unwrap();
            assert_eq!(
                devices.recovery_grant(&creator).await.unwrap_err().code,
                ErrorCode::Forbidden
            );
            assert!(devices.scopes_now(id).await.unwrap().is_none());
            drop(device);
            node.stop().await.unwrap();
        }),
    )
    .await
    .unwrap();
}
