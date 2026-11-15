//! Real ConversationArea ingestion, with independent main/side sequences.
use super::*;
use crate::areas::conversation::ConversationArea;
use vessel_app_view::devices::MachineKey;
use vessel_app_view::side_question::Target;
use vessel_proto::{
    AgentEvent, ContentBlock, EventRecord, EventSeq, RequestId, SessionEvent, SessionId,
};

fn message(seq: u64, text: &str) -> EventRecord {
    EventRecord {
        seq: EventSeq::new(seq),
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: AgentEvent::MessageChunk {
                content: ContentBlock::Text {
                    text: text.to_owned(),
                },
            },
        },
    }
}

#[gpui::test]
fn side_records_are_independent_and_old_streams_cannot_repopulate_a_reopened_panel(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(|cx| {
        let mut main = ConversationArea::new(cx);
        let mut side = ConversationArea::new(cx);
        let parent = SessionId::random();
        let side_id = SessionId::random();
        main.open("one", parent);
        side.open("one", side_id);
        let follow = Follow {
            target: Target {
                machine: MachineKey::from_book("one"),
                parent,
            },
            side: side_id,
            ticket: RequestId::new(1),
        };
        assert!(main.take(&parent, &[message(0, "main reply")]));
        let before = main.timeline.entries().to_vec();
        assert!(ingest(
            &mut side,
            Some(&follow),
            &follow,
            &[message(0, "side reply")]
        ));
        assert_eq!(main.timeline.entries(), before);
        assert_ne!(main.timeline.entries(), side.timeline.entries());
        let side_before = side.timeline.entries().to_vec();
        let reopened = Follow {
            ticket: RequestId::new(2),
            ..follow.clone()
        };
        assert!(!ingest(
            &mut side,
            Some(&reopened),
            &follow,
            &[message(1, "stale")]
        ));
        assert!(!ingest(&mut side, None, &follow, &[message(1, "closed")]));
        let elsewhere = Follow {
            target: Target {
                machine: MachineKey::from_book("other"),
                parent,
            },
            ..follow.clone()
        };
        assert!(!ingest(
            &mut side,
            Some(&follow),
            &elsewhere,
            &[message(1, "wrong machine")]
        ));
        assert_eq!(side.timeline.entries(), side_before);
        assert!(ingest(
            &mut side,
            Some(&reopened),
            &reopened,
            &[message(1, " current")]
        ));
        assert_eq!(main.timeline.entries(), before);
    });
}
