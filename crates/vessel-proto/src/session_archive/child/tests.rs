//! Activity evidence cannot be replaced by a PR display phrase or absent metadata.
use super::*;
use crate::{
    DisplayStatus, HarnessKind, KanbanColumn, NodeId, SessionBoard, SessionId, SessionMode,
    SessionSpec,
};

fn info() -> SessionInfo {
    SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        DateTime::UNIX_EPOCH,
    )
}

#[test]
fn eligibility_requires_current_idle_or_error_evidence_and_no_archive_mark() {
    let mut info = info();
    info.status = SessionStatus::Running;
    assert!(!finished_child_candidate(&info));
    let mut board = SessionBoard::new(
        KanbanColumn::Ready,
        DisplayStatus::Merged,
        Activity::Active,
        DateTime::UNIX_EPOCH,
    );
    for activity in [Activity::Active, Activity::WaitingInput, Activity::Exited] {
        board.activity = activity;
        info.board = Some(board.clone());
        assert!(!finished_child_candidate(&info));
    }
    board.activity = Activity::Idle;
    info.board = Some(board);
    assert!(finished_child_candidate(&info));
    for status in [
        SessionStatus::Starting,
        SessionStatus::Exited { code: None },
        SessionStatus::Interrupted,
    ] {
        info.status = status;
        assert!(!finished_child_candidate(&info));
    }
    info.status = SessionStatus::failed("fixture error");
    info.board = None;
    assert!(finished_child_candidate(&info));
    info.marks.archived_at = Some(DateTime::UNIX_EPOCH);
    assert!(!finished_child_candidate(&info));
}

#[test]
fn cross_node_parent_identity_and_unknown_outcomes_remain_explicit() {
    let session = SessionId::random();
    let request = ArchiveChildSession {
        parent: SessionRef {
            node: NodeId::random(),
            session,
        },
        child: SessionRef {
            node: NodeId::random(),
            session,
        },
    };
    assert_ne!(request.parent, request.child);
    assert_eq!(
        serde_json::from_slice::<ArchiveChildSession>(&serde_json::to_vec(&request).unwrap())
            .unwrap(),
        request
    );
    assert_eq!(
        serde_json::from_str::<ChildArchiveOutcome>(r#"{"kind":"future"}"#).unwrap(),
        ChildArchiveOutcome::Unknown
    );
    assert_eq!(
        serde_json::from_str::<ChildArchiveSkip>(r#""future""#).unwrap(),
        ChildArchiveSkip::Unknown
    );
}
