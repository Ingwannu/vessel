//! Real list-state splices cover close/reveal while a working tail is mounted.

use super::{ConversationArea, Turns};
use vessel_proto::{EventRecord, EventSeq, SessionEvent, SessionId};

fn records() -> Vec<EventRecord> {
    [
        r#"{"event":"turn_started","content":[{"type":"text","text":"Go"}]}"#,
        r#"{"event":"tool_call","id":"read","title":"Read","kind":"read","status":"completed"}"#,
        r#"{"event":"message_chunk","content":{"type":"text","text":"Done"}}"#,
        r#"{"event":"turn_ended","reason":"end_turn"}"#,
    ]
    .into_iter()
    .enumerate()
    .map(|(seq, event)| EventRecord {
        seq: EventSeq::new(u64::try_from(seq).unwrap()),
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: serde_json::from_str(event).unwrap(),
        },
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
