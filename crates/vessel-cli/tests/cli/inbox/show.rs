//! The selected CLI form must survive independently of its pruned source.
use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn show_recovers_every_choice_after_pruning_without_marking_or_answering() {
    within(async {
        let fixture = Fixture::start_with(|config, _| {
            config.ring_capacity = std::num::NonZeroUsize::MIN;
            config.store.retention.max_session_bytes = vessel_node::ByteCount::new(1);
        })
        .await;
        let owner = fixture.owner().await;
        let session = structured(&fixture, &owner).await;
        let mut stream = owner.follow(session, EventSeq::ZERO).await.unwrap();
        owner
            .send(
                session,
                SessionInput::Prompt {
                    content: vec![ContentBlock::Text {
                        text: "ask large".to_owned(),
                    }],
                },
            )
            .await
            .unwrap();
        drop(wait_for(&mut stream, Wanted::Pruned).await);
        let retained = owner.read(session, EventSeq::ZERO, 100).await.unwrap();
        let page = owner.inbox(ListInbox::default()).await.unwrap();
        let entry = &page.entries[0];
        let InboxItem::InSession {
            request: vessel_proto::inbox::InboxRequest::Question { id },
            ..
        } = entry.item
        else {
            panic!("the list must carry the exact compact request");
        };
        assert!(!retained.records.is_empty());
        assert!(
            retained
                .records
                .iter()
                .all(|record| record.seq > entry.key.seq)
        );
        let cursor = entry.cursor.to_string();
        let shown = fixture.vessel(&["inbox", "show", &cursor], b"").await;
        assert!(shown.ok, "{}", shown.stderr);
        let lines = shown.stdout.lines().collect::<Vec<_>>();
        let header = format!("  [question {id}] Choose every field, including the final option.");
        assert_eq!(lines.iter().filter(|line| **line == header).count(), 1);
        for field in 0..32 {
            let expected = format!("  field-{field} (required): Field {field} (choose)");
            assert_eq!(lines.iter().filter(|line| **line == expected).count(), 1);
        }
        for option in 0..64 {
            let expected = format!(
                "    choice-{option}: Choice {option}: {}",
                "long description ".repeat(50)
            );
            assert_eq!(lines.iter().filter(|line| **line == expected).count(), 32);
        }
        let after = owner.inbox(ListInbox::default()).await.unwrap();
        assert_eq!(after.entries, page.entries);
        assert_eq!(after.counts, page.counts);
        assert_eq!(after.entries[0].read, InboxRead::Unread);
        let declined = fixture
            .vessel(&["inbox", "answer", &cursor, "--decline"], b"")
            .await;
        assert!(declined.ok, "{}", declined.stderr);
        let _committed = owner.read(session, EventSeq::ZERO, 1).await.unwrap();
        assert!(owner.inbox_entry(entry.key).await.unwrap().is_none());
        fixture.stop().await;
    })
    .await;
}
