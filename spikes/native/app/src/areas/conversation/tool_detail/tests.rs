//! Canonical producer fixtures use the same admission-to-details rule as rows.

use super::can_open;
use vessel_app_view::timeline::{ToolDetailSource, ToolView};
use vessel_proto::timeline::{
    ShellDetail, SubAgentDetail, ToolCallDetail, ToolFailure, UnknownToolDetail,
};
use vessel_proto::{ToolCallId, ToolCallStatus, ToolKind};

#[test]
fn borrowed_request_sections_preserve_structural_parent_identity() {
    use vessel_app_kit::ui::ElementId;
    let request = ElementId::NamedInteger("permit".into(), 1);
    let text_alias = ElementId::Name("permit-1".into());
    assert_eq!(request.to_string(), text_alias.to_string());
    assert_ne!(
        super::child_id(&request, "section-0"),
        super::child_id(&text_alias, "section-0")
    );
    assert_ne!(
        super::child_id(&request, "section-0"),
        super::child_id(&request, "failure")
    );
    assert_eq!(
        super::child_id(&request, "section-0"),
        super::child_id(&request, "section-0")
    );
}

struct Fake {
    view: ToolView,
    detail: ToolCallDetail,
    failure: Option<ToolFailure>,
    message: Option<vessel_proto::timeline::AgentMessageProvenance>,
}

impl ToolDetailSource for Fake {
    fn tool(&self) -> &ToolView {
        &self.view
    }
    fn detail(&self) -> Option<&ToolCallDetail> {
        Some(&self.detail)
    }
    fn agent_message(&self) -> Option<&vessel_proto::timeline::AgentMessageProvenance> {
        self.message.as_ref()
    }
    fn failure(&self) -> Option<&ToolFailure> {
        self.failure.as_ref()
    }
}

fn fixture(detail: ToolCallDetail, status: ToolCallStatus) -> Fake {
    Fake {
        view: ToolView {
            id: ToolCallId::new("fake"),
            title: "Provider tool".to_owned(),
            kind: ToolKind::Other,
            status,
            content: Vec::new(),
            locations: Vec::new(),
            input: None,
            output: None,
        },
        detail,
        failure: None,
        message: None,
    }
}

#[test]
fn launch_markers_have_no_empty_detail_action_but_an_explicit_failure_remains_readable() {
    let mut marker = fixture(
        ToolCallDetail::SubAgent(SubAgentDetail {
            sub_agent_type: None,
            description: Some("Review".to_owned()),
            child_session_id: None,
            log: None,
            actions: None,
        }),
        ToolCallStatus::InProgress,
    );
    assert!(!can_open(&marker));
    marker.view.status = ToolCallStatus::Failed;
    marker.failure = Some(ToolFailure {
        message: "Could not launch".to_owned(),
    });
    assert!(can_open(&marker));
}

#[test]
fn empty_running_details_offer_loading_but_empty_settled_details_have_no_dead_action() {
    let mut empty = fixture(
        ToolCallDetail::Unknown(UnknownToolDetail {
            input: None,
            output: None,
        }),
        ToolCallStatus::InProgress,
    );
    assert!(can_open(&empty));
    empty.view.status = ToolCallStatus::Completed;
    assert!(!can_open(&empty));
    let shell = fixture(
        ToolCallDetail::Shell(ShellDetail {
            command: "pwd".to_owned(),
            cwd: None,
            output: None,
            exit_code: None,
        }),
        ToolCallStatus::Completed,
    );
    assert!(can_open(&shell));
}

#[test]
fn delivery_provenance_wins_over_a_plan_or_marker_without_turning_into_approval() {
    use vessel_app_view::timeline::plan_text;
    use vessel_proto::timeline::{
        AgentMessageEvent, AgentMessageProvenance, AgentMessageSender, PlanDetail,
    };
    let mut source = fixture(
        ToolCallDetail::Plan(PlanDetail {
            text: "# Plan".to_owned(),
        }),
        ToolCallStatus::Completed,
    );
    assert!(plan_text(&source).is_some());
    source.message = Some(AgentMessageProvenance {
        event: AgentMessageEvent::PermissionRequired,
        sender: AgentMessageSender {
            id: "external-sender".parse().unwrap(),
            server_id: None,
            title: None,
        },
        text: "Please inspect".to_owned(),
    });
    assert!(plan_text(&source).is_none());
    assert!(can_open(&source));
    source.detail = ToolCallDetail::SubAgent(SubAgentDetail {
        sub_agent_type: None,
        description: None,
        child_session_id: None,
        log: None,
        actions: None,
    });
    assert!(
        can_open(&source),
        "provenance still has its literal body when the underlying call is a launch marker"
    );
}
