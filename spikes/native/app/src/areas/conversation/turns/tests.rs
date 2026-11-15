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

#[gpui::test]
fn changing_reasoning_default_preserves_the_visible_answer_and_exact_scroll_offset(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(|cx| {
        use vessel_app_kit::Reveal;
        use vessel_app_view::prefs::ReasoningExpansion;
        use vessel_proto::{AgentEvent, ContentBlock};

        let session = SessionId::random();
        let mut area = ConversationArea::new(cx);
        area.open("fake", session);
        area.turns = None; // This case specifically exercises Collapse tool calls.
        let mut events: Vec<_> = records().into_iter().map(|record| record.event).collect();
        let mut second_tool = events[1].clone();
        if let SessionEvent::Agent {
            event: AgentEvent::ToolCall(call),
        } = &mut second_tool
        {
            call.id = vessel_proto::ToolCallId::new("read-again");
        }
        events.insert(
            2,
            SessionEvent::Agent {
                event: AgentEvent::ThoughtChunk {
                    content: ContentBlock::Text {
                        text: "Reasoning".to_owned(),
                    },
                },
            },
        );
        events.insert(3, second_tool);
        let records: Vec<_> = events
            .into_iter()
            .enumerate()
            .map(|(index, event)| EventRecord {
                seq: EventSeq::new(u64::try_from(index).unwrap()),
                at: chrono::DateTime::UNIX_EPOCH,
                event,
            })
            .collect();
        assert!(area.take(&session, &records));
        assert_eq!(area.rows.len(), 4);
        area.list.reveal(area.at(2), Reveal::Top);
        let before = area.list.top();
        area.set_reasoning(ReasoningExpansion::Expanded);
        assert_eq!(area.rows.len(), 6);
        assert_eq!(area.rows.entry(4), Some(4));
        assert_eq!(area.list.top(), (area.at(4), before.1));
        assert!(!area.list.following());
        let kept = area.list.top();
        area.set_reasoning(ReasoningExpansion::Expanded);
        assert_eq!(area.list.top(), kept);
    });
}

#[gpui::test]
fn an_explicitly_opened_lone_call_preopens_its_future_group(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        use vessel_app_view::prefs::ReasoningExpansion;
        use vessel_proto::{AgentEvent, ToolCallId};
        let session = SessionId::random();
        let mut area = ConversationArea::new(cx);
        area.open("fake", session);
        area.turns = None;
        let first = records();
        assert!(area.take(&session, &first[..2]));
        assert!(area.folds.as_ref().unwrap().run_of(1).is_none());
        area.toggle_entry(1);
        assert!(area.entry_expanded(1));
        assert!(area.open_runs.contains(&1));
        let mut second = first[1].clone();
        second.seq = EventSeq::new(2);
        if let SessionEvent::Agent {
            event: AgentEvent::ToolCall(call),
        } = &mut second.event
        {
            call.id = ToolCallId::new("second");
        }
        assert!(area.take(&session, &[second]));
        assert_eq!(area.folds.as_ref().unwrap().run_of(2).unwrap().lead, 1);
        assert_eq!(area.rows.entry(2), Some(2));
        // A call inside the group has independent disclosure state.
        area.toggle_entry(1);
        assert!(!area.entry_expanded(1));
        assert!(area.open_runs.contains(&1));
        area.set_reasoning(ReasoningExpansion::Expanded);
        assert!(area.open_runs.contains(&1));
        assert_eq!(area.rows.entry(2), Some(2));
    });
}

#[gpui::test]
fn running_tools_start_closed_and_explicitly_opened_tools_stay_open_after_completion(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(|cx| {
        use vessel_proto::{AgentEvent, ToolCallStatus};
        let session = SessionId::random();
        let mut area = ConversationArea::new(cx);
        area.open("fake", session);
        area.turns = None;
        let completed = records()[1].clone();
        let mut running = completed.clone();
        if let SessionEvent::Agent {
            event: AgentEvent::ToolCall(call),
        } = &mut running.event
        {
            call.status = ToolCallStatus::InProgress;
        }
        assert!(area.take(&session, &[running]));
        assert!(!area.entry_expanded(0));
        area.toggle_entry(0);
        assert!(area.entry_expanded(0));
        let completed = EventRecord {
            seq: EventSeq::new(2),
            ..completed
        };
        assert!(area.take(&session, &[completed]));
        assert!(area.entry_expanded(0));
    });
}
