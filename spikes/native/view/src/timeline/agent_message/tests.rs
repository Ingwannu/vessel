//! Delivery events stay separate from tool completion; summaries use source units.
use super::*;
use crate::localization::{ENGLISH, LanguagePreference, Translator, resolve_language};
use vessel_proto::timeline::AgentMessageSender;

#[test]
fn sender_title_falls_back_to_external_identity_and_body_is_not_interpreted() {
    let mut message = AgentMessageProvenance {
        event: AgentMessageEvent::Finished,
        sender: AgentMessageSender {
            id: "source-agent".parse().unwrap(),
            server_id: None,
            title: None,
        },
        text: "<b>literal</b> **body**".to_owned(),
    };
    let display = agent_message_display(&message);
    assert_eq!(
        display.label,
        ToolLabel::AgentMessage(AgentMessageEvent::Finished)
    );
    assert_eq!(display.summary.as_deref(), Some("source-agent"));
    message.sender.title = Some("  Source   title  ".parse().unwrap());
    assert_eq!(
        agent_message_display(&message).summary.as_deref(),
        Some("Source title")
    );
    assert_eq!(message.text, "<b>literal</b> **body**");
}
#[test]
fn summaries_bound_the_scan_and_never_split_a_non_bmp_scalar() {
    assert_eq!(
        summary("  first  second  ").as_deref(),
        Some("first second")
    );
    assert_eq!(summary(&" ".repeat(481)), None);
    let value = format!("{}😀x", "a".repeat(118));
    assert_eq!(summary(&value), Some(format!("{}…", "a".repeat(118))));
    let value = format!("short{}later", " ".repeat(500));
    assert_eq!(summary(&value).as_deref(), Some("short…"));
}
#[test]
fn all_delivery_event_labels_and_sender_line_are_complete_in_both_languages() {
    let en = Translator::get(ENGLISH).unwrap().timeline_tool();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap()
    .timeline_tool();
    for (event, english, korean) in [
        (
            AgentMessageEvent::Message,
            "Message received",
            "메시지 수신",
        ),
        (
            AgentMessageEvent::Finished,
            "Agent finished",
            "에이전트 완료",
        ),
        (AgentMessageEvent::Errored, "Agent errored", "에이전트 오류"),
        (
            AgentMessageEvent::PermissionRequired,
            "Agent needs permission",
            "에이전트 권한 필요",
        ),
        (AgentMessageEvent::Closed, "Agent closed", "에이전트 종료"),
    ] {
        assert_eq!(en.agent_message(event), english);
        assert_eq!(ko.agent_message(event), korean);
    }
    assert_eq!(
        en.from_agent("sender").unwrap(),
        "From agent: \u{2068}sender\u{2069}"
    );
    assert_eq!(
        ko.from_agent("sender").unwrap(),
        "보낸 에이전트: \u{2068}sender\u{2069}"
    );
}
