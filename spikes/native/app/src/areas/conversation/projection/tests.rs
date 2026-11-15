//! A prefix rebuild moves one projection without fabricating a managed session.
use super::*;
use chrono::DateTime;
use vessel_app_kit::Reveal;
use vessel_proto::{
    AgentEvent, ContentBlock, EventRecord, EventSeq, SessionEvent, SessionId, ToolCall, ToolCallId,
    ToolCallStatus, ToolKind,
};

fn record(seq: u64, event: AgentEvent) -> EventRecord {
    EventRecord {
        seq: EventSeq::new(seq),
        at: DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent { event },
    }
}
fn projection(prefix: bool) -> Timeline {
    let mut records = Vec::new();
    if prefix {
        records.push(record(
            2,
            AgentEvent::ThoughtChunk {
                content: ContentBlock::Text {
                    text: "Earlier".to_owned(),
                },
            },
        ));
    }
    let tool = ToolCall {
        id: ToolCallId::new("stable-call"),
        title: "Read".to_owned(),
        kind: ToolKind::Read,
        status: ToolCallStatus::Completed,
        content: Vec::new(),
        locations: Vec::new(),
        raw_input: None,
    };
    records.push(record(10, AgentEvent::ToolCall(tool)));
    records.push(record(
        11,
        AgentEvent::ThoughtChunk {
            content: ContentBlock::Text {
                text: "Reasoning".to_owned(),
            },
        },
    ));
    let mut timeline = Timeline::default();
    let _change = timeline.apply(&records);
    timeline
}
#[gpui::test]
fn prepend_preserves_explicit_choice_run_host_and_reading_anchor_by_entry_key(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(|cx| {
        let mut area = ConversationArea::new(cx);
        area.turns = None;
        area.collapse = false;
        assert!(
            area.replace_readonly_projection(projection(false), OwnerReadiness::Ready)
                .is_ok()
        );
        area.toggle_entry(0);
        assert!(area.entry_expanded(0));
        assert!(area.open_runs.contains(&0));
        area.list.reveal(0, Reveal::Top);
        assert!(
            area.replace_readonly_projection(projection(true), OwnerReadiness::Ready)
                .is_ok()
        );
        assert!(area.shown.is_none());
        assert!(area.info.is_none());
        assert!(area.entry_expanded(1));
        assert!(!area.entry_expanded(0));
        assert!(area.open_runs.contains(&1));
        assert!(!area.open_runs.contains(&0));
        assert_eq!(area.list.top().0, 1);
        assert_eq!(area.timeline.entries().len(), 3);
    });
}
#[gpui::test]
fn managed_receiver_returns_the_projection_without_mutating_either_side(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(|cx| {
        let mut area = ConversationArea::new(cx);
        area.open("fake", SessionId::random());
        let Err(unapplied) =
            area.replace_readonly_projection(projection(false), OwnerReadiness::Ready)
        else {
            panic!("managed receiver must refuse")
        };
        assert_eq!(unapplied.timeline.entries().len(), 2);
        assert!(area.timeline.entries().is_empty());
        assert!(area.shown.is_some());
    });
}

#[gpui::test]
fn an_open_group_keeps_its_intent_when_an_earlier_member_becomes_the_host(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(|cx| {
        let mut area = ConversationArea::new(cx);
        area.turns = None;
        area.collapse = true;
        assert!(
            area.replace_readonly_projection(projection(false), OwnerReadiness::Ready)
                .is_ok()
        );
        area.toggle_run(0);
        assert!(area.open_runs.contains(&0));
        assert!(
            area.replace_readonly_projection(projection(true), OwnerReadiness::Ready)
                .is_ok()
        );
        assert_eq!(area.folds.as_ref().unwrap().run_of(1).unwrap().lead, 0);
        assert!(area.open_runs.contains(&0));
        assert!(!area.open_runs.contains(&1));
        assert_eq!(area.rows.len(), 3);
    });
}

#[gpui::test]
fn protected_navigation_returns_the_new_projection_without_changing_the_installed_rows(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(|cx| {
        let mut area = ConversationArea::new(cx);
        area.turns = None;
        area.collapse = false;
        assert!(
            area.replace_readonly_projection(projection(false), OwnerReadiness::Ready)
                .is_ok()
        );
        area.toggle_entry(0);
        area.list.reveal(0, Reveal::Top);
        let top = area.list.top();
        let old = area.timeline.entries().to_vec();
        let Err(unapplied) = area.replace_readonly_projection(
            projection(true),
            OwnerReadiness::Hold(vessel_app_view::residents::HoldReason::ActiveEditor),
        ) else {
            panic!("protected navigation must hold replacement")
        };
        assert_eq!(unapplied.timeline.entries().len(), 3);
        assert_eq!(area.timeline.entries(), old);
        assert!(area.entry_expanded(0));
        assert_eq!(area.list.top(), top);
    });
}
