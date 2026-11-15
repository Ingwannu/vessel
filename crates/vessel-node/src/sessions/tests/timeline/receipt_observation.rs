//! The real private fake proves read-only lookup and exact fast-turn ticket binding.

use super::{Fixture, TIMELINE_RUNTIME_BOUND, create, prompt, until};
use vessel_proto::timeline::{
    ObservePromptReceipt, ObservePromptReceiptReply, PromptReceiptBinding, SubmitPrompt,
    SubmitPromptReply,
};
use vessel_proto::{AgentEvent, ContentBlock, Creator, EventSeq, SessionEvent};

async fn observe(fixture: &Fixture, request: &SubmitPrompt) -> ObservePromptReceiptReply {
    fixture
        .sessions
        .observe_prompt_receipt(
            ObservePromptReceipt {
                request: request.clone(),
            },
            Creator::Owner,
        )
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn missing_observation_never_loads_attachments_wakes_or_dispatches() {
    vessel_testkit::within(TIMELINE_RUNTIME_BOUND, async {
        let fixture = Fixture::new().await;
        let id = create(&fixture).await;
        let mut request = prompt(id, "observe only");
        request.content.push(ContentBlock::Attachment {
            attachment: vessel_proto::AttachmentRef {
                id: vessel_proto::AttachmentId::random(),
                name: "absent.txt".parse().unwrap(),
                mime: "text/plain".parse().unwrap(),
                kind: vessel_proto::AttachmentKind::File,
                size: 1,
            },
        });
        assert!(matches!(
            observe(&fixture, &request).await,
            ObservePromptReceiptReply::Unobserved { .. }
        ));
        assert!(
            fixture
                .store
                .prompt_receipt(request.clone(), Creator::Owner)
                .await
                .unwrap()
                .is_none()
        );
        fixture.sessions.live(id).unwrap().kill();
        fixture.settle().await;
        assert!(matches!(
            observe(&fixture, &request).await,
            ObservePromptReceiptReply::Unobserved { .. }
        ));
        let page = fixture
            .sessions
            .read(id, EventSeq::ZERO, 256)
            .await
            .unwrap();
        assert!(!page.records.iter().any(|record| matches!(
            record.event,
            SessionEvent::Agent {
                event: AgentEvent::TurnStarted { .. }
            }
        )));
        fixture.stop().await;
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_fast_completed_prompt_keeps_its_own_binding_after_a_later_turn_and_handover() {
    vessel_testkit::within(TIMELINE_RUNTIME_BOUND, async {
        let fixture = Fixture::new().await;
        let id = create(&fixture).await;
        let request = prompt(id, "first");
        assert!(matches!(fixture.sessions.submit_prompt(request.clone(), Creator::Owner).await.unwrap(), SubmitPromptReply::Accepted { .. }));
        let _completed = until(&fixture, id, |event| matches!(event, AgentEvent::TurnEnded { .. })).await;
        assert!(matches!(fixture.sessions.submit_prompt(prompt(id, "later"), Creator::Owner).await.unwrap(), SubmitPromptReply::Accepted { .. }));
        let page = fixture.sessions.read(id, EventSeq::ZERO, 256).await.unwrap();
        let first = page.records.iter().find(|record| matches!(&record.event, SessionEvent::Agent { event: AgentEvent::TurnStarted { client_message_id: Some(message), .. } } if message == &request.message)).unwrap().seq;
        assert!(page.records.iter().any(|record| record.seq != first && matches!(record.event, SessionEvent::Agent { event: AgentEvent::TurnStarted { .. } })));
        fixture.sessions.live(id).unwrap().kill();
        fixture.settle().await;
        assert!(matches!(observe(&fixture, &request).await, ObservePromptReceiptReply::Accepted { binding: PromptReceiptBinding::Turn { started }, .. } if started == first));
        fixture.stop().await;
    }).await.unwrap();
}
