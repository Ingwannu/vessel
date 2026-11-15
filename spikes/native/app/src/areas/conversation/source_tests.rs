//! Real list navigation follows an exact contributing user chunk across metadata.

use super::ConversationArea;
use chrono::DateTime;
use vessel_proto::{AgentEvent, ContentBlock, EventRecord, EventSeq, SessionEvent, SessionId};

fn text(value: &str) -> ContentBlock {
    ContentBlock::Text {
        text: value.to_owned(),
    }
}

#[gpui::test]
fn a_later_chunk_search_hit_reveals_its_prompt_instead_of_the_intervening_log(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(|cx| {
        let session = SessionId::random();
        let mut area = ConversationArea::new(cx);
        area.turns = None;
        area.open("fake", session);
        let records = [
            (
                1,
                SessionEvent::Agent {
                    event: AgentEvent::TurnStarted {
                        content: vec![text("First ")],
                    },
                },
            ),
            (
                2,
                SessionEvent::Agent {
                    event: AgentEvent::HarnessLog {
                        line: "diagnostic".to_owned(),
                    },
                },
            ),
            (3, SessionEvent::Woken),
            (
                8,
                SessionEvent::Agent {
                    event: AgentEvent::UserMessageChunk {
                        content: text("part"),
                    },
                },
            ),
            (
                10,
                SessionEvent::Agent {
                    event: AgentEvent::MessageChunk {
                        content: text("Other reply"),
                    },
                },
            ),
        ]
        .map(|(seq, event)| EventRecord {
            seq: EventSeq::new(seq),
            at: DateTime::UNIX_EPOCH,
            event,
        });
        assert!(area.take(&session, &records));
        area.reveal(EventSeq::new(8));
        assert_eq!(area.list.top().0, 0);
        assert_eq!(area.rows.entry(0), Some(0));
    });
}
