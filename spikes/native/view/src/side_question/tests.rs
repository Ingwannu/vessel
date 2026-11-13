//! GOAT `sideChat.logic.test.ts` cases, plus Vessel's terminal/lifecycle and
//! bounded-input differences. No UI or transport substitutes these rules.

use chrono::DateTime;
use vessel_proto::{
    AttachmentLimits, HarnessKind, PermissionId, PermissionRequest, PromptQueue, Question,
    QuestionId, SessionBoard, SessionSpec, TerminalSize, ToolCallId,
};

use super::*;

fn session(id: u128, created_at: &str) -> SessionInfo {
    let mut info = SessionInfo::new(
        format!("00000000-0000-4000-8000-{id:012x}")
            .parse()
            .unwrap(),
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        created_at.parse().unwrap(),
    );
    info.status = SessionStatus::Running;
    info
}

fn conversation() -> Conversation {
    Conversation {
        queue: PromptQueue::default(),
        commands: Vec::new(),
        usage: None,
        permissions: Vec::new(),
        questions: Vec::new(),
        abilities: None,
        limits: AttachmentLimits::DEFAULT,
    }
}

#[test]
fn reads_questions_empty_commands_and_ignores_prose_like_the_reference() {
    let cases = [
        (
            "/side what does this test cover?",
            Some("what does this test cover?"),
        ),
        (
            "  /SIDE\nfirst line\nsecond line ",
            Some("first line\nsecond line"),
        ),
        ("/side", Some("")),
        ("/sidebar", None),
        ("please /side this", None),
        ("한글 /side", None),
        ("/side\u{85}question", None),
        ("\u{feff}/side\u{feff}question\u{feff}", Some("question")),
    ];
    for (input, expected) in cases {
        let expected = expected.map(|text| {
            Ok(if text.is_empty() {
                SideCommand::Open
            } else {
                SideCommand::Ask(text.parse().unwrap())
            })
        });
        assert_eq!(command(input), expected, "{input}");
    }
    assert!(
        command(&format!("/side {}", "한".repeat(6_000)))
            .unwrap()
            .is_err()
    );
}

#[test]
fn chooses_newest_unarchived_side_including_ended_threads() {
    let parent = session(1, "2026-10-13T00:00:00Z");
    let mut older = session(2, "2026-10-13T01:00:00Z");
    older.spec.side_question_of = Some(parent.id);
    let mut newer = session(3, "2026-10-13T02:00:00Z");
    newer.spec.side_question_of = Some(parent.id);
    newer.status = SessionStatus::Interrupted;
    let mut archived = session(4, "2026-10-13T03:00:00Z");
    archived.spec.side_question_of = Some(parent.id);
    archived.marks.archived_at = Some(DateTime::UNIX_EPOCH);
    let mut other = session(5, "2026-10-13T04:00:00Z");
    other.spec.side_question_of = Some(older.id);
    let sessions = vec![parent.clone(), older, newer.clone(), archived, other];
    assert_eq!(
        newest(&sessions, parent.id).map(|info| info.id),
        Some(newer.id)
    );
    assert!(newest(&sessions, SessionId::random()).is_none());
    assert!(can_open(Some(&parent)));
    assert!(!can_open(Some(&newer)));
    assert!(!can_open(None));
    let mut terminal = parent;
    terminal.spec.mode = SessionMode::Terminal {
        args: Vec::new(),
        size: TerminalSize::default(),
    };
    assert!(!can_open(Some(&terminal)));
}

#[test]
fn attention_precedes_work_and_idle_failure_and_interrupted_are_distinct() {
    let mut parent = session(1, "2026-10-24T07:30:00Z");
    parent.board = Some(SessionBoard::new(
        vessel_proto::KanbanColumn::Building,
        vessel_proto::DisplayStatus::Working,
        Activity::Active,
        DateTime::UNIX_EPOCH,
    ));
    let mut conversation = conversation();
    assert_eq!(parent_status(&parent, &conversation), ParentStatus::Working);
    conversation.questions.push(Question {
        id: QuestionId::random(),
        message: "Which file?".to_owned(),
        tool_call: None,
        fields: Vec::new(),
    });
    assert_eq!(
        parent_status(&parent, &conversation),
        ParentStatus::NeedsInput
    );
    conversation.permissions.push(PermissionRequest {
        id: PermissionId::random(),
        tool_call: ToolCallId::new("read"),
        options: Vec::new(),
    });
    assert_eq!(
        parent_status(&parent, &conversation),
        ParentStatus::NeedsApproval
    );
    conversation.permissions.clear();
    conversation.questions.clear();
    parent.board = None;
    let cases = [
        (
            SessionStatus::Starting,
            ParentStatus::Starting,
            ParentTone::Active,
        ),
        (SessionStatus::Running, ParentStatus::Idle, ParentTone::Idle),
        (
            SessionStatus::Interrupted,
            ParentStatus::Interrupted,
            ParentTone::Idle,
        ),
        (
            SessionStatus::Exited { code: Some(0) },
            ParentStatus::Idle,
            ParentTone::Idle,
        ),
        (
            SessionStatus::Failed {
                message: "failed".to_owned(),
            },
            ParentStatus::Failed,
            ParentTone::Error,
        ),
    ];
    for (status, expected, tone) in cases {
        parent.status = status;
        let actual = parent_status(&parent, &conversation);
        assert_eq!(actual, expected);
        assert_eq!(actual.tone(), tone);
    }
    assert_eq!(ParentStatus::NeedsApproval.tone(), ParentTone::Attention);
    assert_eq!(ParentStatus::NeedsInput.tone(), ParentTone::Attention);
}

#[test]
fn full_side_input_uses_dedicated_start_and_never_implicit_resume_or_queue() {
    assert_eq!(submission(None, None, " \u{feff}\n"), Submission::Noop);
    let parent = session(1, "2026-10-24T07:30:00Z");
    let mut side = session(2, "2026-10-24T07:31:00Z");
    side.spec.side_question_of = Some(parent.id);
    assert_eq!(
        submission(Some(&parent), None, "ordinary question"),
        Submission::Normal
    );
    assert_eq!(
        submission(Some(&parent), None, "/side"),
        Submission::Open(SideCommand::Open)
    );
    assert_eq!(
        submission(Some(&side), None, "ordinary question"),
        Submission::Ask {
            side: side.id,
            question: "ordinary question".parse().unwrap(),
        }
    );
    assert_eq!(
        submission(Some(&side), None, "/side nested"),
        Submission::Hold(SubmitProblem::Nested)
    );
    assert_eq!(
        submission(Some(&side), None, "  /compact"),
        Submission::Hold(SubmitProblem::NativeCommand)
    );
    assert_eq!(
        submission(None, None, "ordinary question"),
        Submission::Hold(SubmitProblem::Loading)
    );
    side.status = SessionStatus::Starting;
    assert_eq!(
        submission(Some(&side), None, "question"),
        Submission::Hold(SubmitProblem::Loading)
    );
    for ended in [
        SessionStatus::Interrupted,
        SessionStatus::Exited { code: Some(0) },
        SessionStatus::Failed {
            message: "failed".to_owned(),
        },
    ] {
        side.status = ended;
        assert_eq!(
            submission(Some(&side), None, "question"),
            Submission::Hold(SubmitProblem::Resume)
        );
    }
    side.status = SessionStatus::Running;
    for activity in [Activity::Active, Activity::WaitingInput] {
        side.board = Some(SessionBoard::new(
            vessel_proto::KanbanColumn::Building,
            vessel_proto::DisplayStatus::Working,
            activity,
            DateTime::UNIX_EPOCH,
        ));
        assert_eq!(
            submission(Some(&side), None, "question"),
            Submission::Hold(SubmitProblem::Busy)
        );
    }
    side.board = None;
    assert!(matches!(
        submission(Some(&side), None, &"한".repeat(6_000)),
        Submission::Hold(SubmitProblem::Question(_))
    ));
    let mut terminal = parent;
    terminal.spec.mode = SessionMode::Terminal {
        args: Vec::new(),
        size: TerminalSize::default(),
    };
    assert_eq!(
        submission(Some(&terminal), None, "/side question"),
        Submission::Hold(SubmitProblem::Terminal)
    );
}

#[test]
fn current_stream_status_overrides_stale_info_only_after_catching_up() {
    use crate::timeline::Timeline;
    use vessel_proto::{AgentEvent, EventRecord, EventSeq, SessionEvent, StopReason};
    let mut side = session(2, "2026-10-24T07:31:00Z");
    side.spec.side_question_of = Some(session(1, "2026-10-24T07:30:00Z").id);
    side.status = SessionStatus::Starting;
    side.next_seq = EventSeq::new(1);
    let mut timeline = Timeline::default();
    let record = |seq, event| EventRecord {
        seq: EventSeq::new(seq),
        at: DateTime::UNIX_EPOCH,
        event,
    };
    timeline.apply(&[record(
        0,
        SessionEvent::Status {
            status: SessionStatus::Running,
        },
    )]);
    assert!(matches!(
        submission(Some(&side), Some(&timeline), "question"),
        Submission::Ask { .. }
    ));
    timeline.apply(&[record(
        1,
        SessionEvent::Agent {
            event: AgentEvent::TurnStarted {
                content: Vec::new(),
            },
        },
    )]);
    assert_eq!(
        submission(Some(&side), Some(&timeline), "question"),
        Submission::Hold(SubmitProblem::Busy)
    );
    timeline.apply(&[record(
        2,
        SessionEvent::Agent {
            event: AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
                usage: None,
            },
        },
    )]);
    assert!(matches!(
        submission(Some(&side), Some(&timeline), "question"),
        Submission::Ask { .. }
    ));
    side.next_seq = EventSeq::new(10);
    side.status = SessionStatus::Interrupted;
    assert_eq!(
        submission(Some(&side), Some(&timeline), "question"),
        Submission::Hold(SubmitProblem::Resume)
    );
}
