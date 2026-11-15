//! A new reader recovers the full live request after both log and ring prune.
use super::*;
use crate::support::step;
use vessel_proto::conversation::{ChoiceOption, FieldKind, QuestionField};
use vessel_proto::{EventRecord, Question};

/// Missing records are the point of this fixture, not an accidental failure.
async fn until(stream: &mut vessel_client::SessionStream, wanted: impl Fn(&EventRecord) -> bool) {
    step("pruned stream reaches its marker", async {
        loop {
            match stream.next().await.unwrap() {
                Ok(record) if wanted(&record) => return,
                Ok(_) | Err(vessel_client::StreamError::Missing { .. }) => {}
                Err(error) => panic!("unexpected stream failure: {error}"),
            }
        }
    })
    .await;
}
fn assert_exact_form(question: &Question) {
    let mut offered = (0..32).collect::<Vec<_>>();
    // ACP property names are a BTreeMap; preserve the mapper’s lexical order.
    offered.sort_by_key(|field| format!("field-{field}"));
    let fields = offered
        .into_iter()
        .map(|field| QuestionField {
            name: format!("field-{field}").parse().unwrap(),
            title: Some(format!("Field {field}")),
            description: None,
            required: true,
            kind: FieldKind::Choice {
                options: (0..64)
                    .map(|option| ChoiceOption {
                        value: format!("choice-{option}"),
                        title: format!("Choice {option}: {}", "long description ".repeat(50)),
                        description: None,

                        preview: None,
                    })
                    .collect(),
                multiple: false,
                min_items: None,
                max_items: None,
                default: Vec::new(),
            },
        })
        .collect();
    assert_eq!(
        question,
        &Question {
            id: question.id,
            message: "Choose every field, including the final option.".to_owned(),
            tool_call: None,
            fields,
            lifetime: Default::default(),
            delivery_state: Default::default(),
        }
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn large_live_form_remains_exact_and_answerable_after_source_pruning_for_a_new_client() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start_with(|config| {
            config.ring_capacity = std::num::NonZeroUsize::MIN;
            config.store.retention.max_session_bytes = vessel_node::ByteCount::new(1);
        }).await;
        let owner = fixture.connect(&node).await;
        let session = owner.create_session(structured_spec(&fixture, None)).await.unwrap().id;
        let mut stream = owner.follow(session, EventSeq::ZERO).await.unwrap();
        owner.send(session, prompt("ask large")).await.unwrap();
        until(&mut stream, |record| matches!(agent_of(record), Some(AgentEvent::MessageChunk { content: vessel_proto::ContentBlock::Text { text } }) if text == "form source may now be pruned")).await;
        let fresh = fixture.connect(&node).await;
        let records = fresh.read(session, EventSeq::ZERO, 100).await.unwrap();
        let entry = fresh.inbox(ListInbox::default()).await.unwrap().entries.remove(0);
        let InboxItem::InSession { request: vessel_proto::inbox::InboxRequest::Question { id } } = entry.item else { panic!("large form must remain an exact reference"); };
        assert!(!records.records.is_empty());
        assert!(records.records.iter().all(|record| record.seq > entry.key.seq), "source must be absent from both log and ring");
        let expanded = fresh.inbox_request(entry.key).await.unwrap().unwrap();
        let InboxItem::Question { question } = &expanded.item else { panic!("one full live form must remain"); };
        assert_eq!(question.id, id);
        assert_exact_form(question);
        let values = (0..32).map(|field| (format!("field-{field}").parse().unwrap(), vessel_proto::conversation::AnswerValue::Choice("choice-63".to_owned()))).collect();
        fresh.answer_question(session, id, QuestionAnswer::Accept { values }).await.unwrap();
        until(&mut stream, |record| matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))).await;
        durable(&fresh, session).await;
        assert!(fresh.inbox_entry(entry.key).await.unwrap().is_none());
        node.stop().await.unwrap();
    }).await;
}
