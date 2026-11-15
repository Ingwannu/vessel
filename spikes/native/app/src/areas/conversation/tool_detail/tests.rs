//! Canonical producer fixtures use the same admission-to-details rule as rows.

use super::can_open;
use vessel_app_view::timeline::{ToolDetailSource, ToolView};
use vessel_proto::timeline::{
    ShellDetail, SubAgentDetail, ToolCallDetail, ToolFailure, UnknownToolDetail,
};
use vessel_proto::{ToolCallId, ToolCallStatus, ToolKind};

struct Fake {
    view: ToolView,
    detail: ToolCallDetail,
    failure: Option<ToolFailure>,
}

impl ToolDetailSource for Fake {
    fn tool(&self) -> &ToolView {
        &self.view
    }
    fn detail(&self) -> Option<&ToolCallDetail> {
        Some(&self.detail)
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
