//! Real list-state splices cover close/reveal while a working tail is mounted.

use super::{ConversationArea, Turns};
use vessel_proto::{EventRecord, EventSeq, SessionEvent, SessionId};

fn records() -> Vec<EventRecord> {
    use vessel_proto::{AgentEvent, ContentBlock, StopReason, ToolCall, ToolCallStatus, ToolKind};
    [
        AgentEvent::TurnStarted {
            content: vec![ContentBlock::Text {
                text: "Go".to_owned(),
            }],
        },
        AgentEvent::ToolCall(ToolCall {
            id: vessel_proto::ToolCallId::new("read"),
            title: "Read".to_owned(),
            kind: ToolKind::Read,
            status: ToolCallStatus::Completed,
            content: Vec::new(),
            locations: Vec::new(),
            raw_input: None,
        }),
        AgentEvent::MessageChunk {
            content: ContentBlock::Text {
                text: "Done".to_owned(),
            },
        },
        AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: None,
            point: None,
        },
    ]
    .into_iter()
    .enumerate()
    .map(|(seq, event)| EventRecord {
        seq: EventSeq::new(u64::try_from(seq).unwrap()),
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent { event },
    })
    .collect()
}

#[gpui::test]
fn closing_and_revealing_keep_one_tail_and_the_same_final_answer(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        let session = SessionId::random();
        let mut area = ConversationArea::new(cx);
        area.open("fake", session);
        area.turns = Some(Turns::default());
        let records = records();
        assert!(area.take(&session, &records[..3]));
        assert_eq!(area.rows.len(), 3);
        area.sync_tail(true);
        assert_eq!(area.list.rows(), 4);
        assert!(area.take(&session, &records[3..]));
        assert_eq!(area.rows.entry(1), Some(2));
        assert_eq!(area.list.rows(), 4);
        area.sync_tail(true);
        assert_eq!(area.list.rows(), 4);
        area.reveal_turn(1);
        assert_eq!(area.rows.entry(1), Some(1));
        assert_eq!(area.rows.entry(2), Some(2));
        area.sync_tail(true);
        assert_eq!(area.list.rows(), 5);
        area.toggle_turn(0);
        area.sync_tail(true);
        assert_eq!(area.rows.entry(1), Some(2));
        assert_eq!(area.list.rows(), 4);
        area.sync_tail(false);
        assert_eq!(area.list.rows(), 3);
    });
}
