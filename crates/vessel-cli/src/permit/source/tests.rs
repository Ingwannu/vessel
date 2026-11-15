//! Complete canonical snapshots include stored/archived requests without Inbox attention.
use super::*;
use std::collections::VecDeque;
use vessel_proto::conversation::{FieldKind, QuestionField};
use vessel_proto::{
    AttachmentLimits, HarnessKind, PermissionId, PermissionOption, PermissionOptionId,
    PermissionOptionKind, QuestionId, SessionSpec, SessionStatus, TerminalSize, ToolCallId,
};

#[derive(Default)]
struct Memory {
    sessions: Option<Result<Vec<SessionInfo>, CallError>>,
    snapshots: VecDeque<(SessionId, Result<Conversation, CallError>)>,
    read: Vec<SessionId>,
}
impl Source for Memory {
    async fn sessions(&mut self) -> Result<Vec<SessionInfo>, CallError> {
        self.sessions
            .take()
            .expect("scheduled canonical session list")
    }
    async fn conversation(&mut self, session: SessionId) -> Result<Conversation, CallError> {
        self.read.push(session);
        let (expected, result) = self
            .snapshots
            .pop_front()
            .expect("scheduled canonical snapshot");
        assert_eq!(session, expected);
        result
    }
}
fn session(id: SessionId, mode: SessionMode) -> SessionInfo {
    SessionInfo::new(
        id,
        SessionSpec::new(HarnessKind::Codex, mode),
        chrono::DateTime::UNIX_EPOCH,
    )
}
fn structured(id: SessionId) -> SessionInfo {
    session(
        id,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    )
}
fn permission() -> PermissionRequest {
    PermissionRequest {
        id: PermissionId::random(),
        tool_call: ToolCallId::new("actual-tool"),
        options: vec![PermissionOption {
            id: PermissionOptionId::new("yes"),
            name: "Proceed".into(),
            kind: PermissionOptionKind::AllowOnce,
        }],
    }
}
fn question() -> Question {
    Question {
        id: QuestionId::random(),
        message: "Retained question".into(),
        tool_call: None,
        fields: vec![QuestionField {
            name: "answer".parse().unwrap(),
            title: None,
            description: None,
            required: true,
            kind: FieldKind::Boolean { default: None },
        }],
    }
}
fn held(permissions: Vec<PermissionRequest>, questions: Vec<Question>) -> Conversation {
    Conversation {
        queue: Default::default(),
        commands: Vec::new(),
        usage: None,
        permissions,
        questions,
        abilities: None,
        limits: AttachmentLimits::DEFAULT,
    }
}

#[tokio::test]
async fn archived_structured_sessions_are_listed_and_terminals_are_not_queried() {
    let live = structured(SessionId::random());
    let mut archived = structured(SessionId::random());
    archived.marks.archived_at = Some(chrono::DateTime::UNIX_EPOCH);
    archived.status = SessionStatus::Exited { code: Some(0) };
    let terminal = session(
        SessionId::random(),
        SessionMode::Terminal {
            args: Vec::new(),
            size: TerminalSize::default(),
        },
    );
    let request = permission();
    let retained = question();
    let mut source = Memory {
        sessions: Some(Ok(vec![live.clone(), archived.clone(), terminal])),
        snapshots: VecDeque::from([
            (live.id, Ok(held(vec![request.clone()], Vec::new()))),
            (archived.id, Ok(held(Vec::new(), vec![retained.clone()]))),
        ]),
        ..Memory::default()
    };
    let rows = list(&mut source).await.unwrap();
    assert_eq!(source.read, [live.id, archived.id]);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, PendingId::Permission(request.id));
    assert_eq!(rows[1].id, PendingId::Question(retained.id));
    assert_eq!(rows[1].agent_id, archived.id);
}

#[tokio::test]
async fn a_later_snapshot_failure_has_no_partial_success_result() {
    let a = structured(SessionId::random());
    let b = structured(SessionId::random());
    let mut source = Memory {
        sessions: Some(Ok(vec![a.clone(), b.clone()])),
        snapshots: VecDeque::from([
            (a.id, Ok(held(vec![permission()], Vec::new()))),
            (b.id, Err(CallError::Disconnected)),
        ]),
        ..Memory::default()
    };
    assert!(matches!(
        list(&mut source).await,
        Err(SourceError::Node(CallError::Disconnected))
    ));
    assert_eq!(source.read, [a.id, b.id]);
}

#[tokio::test]
async fn duplicate_sessions_or_requests_refuse_a_complete_listing() {
    let owner = structured(SessionId::random());
    let request = permission();
    let mut duplicate_session = Memory {
        sessions: Some(Ok(vec![owner.clone(), owner.clone()])),
        snapshots: VecDeque::from([(owner.id, Ok(held(Vec::new(), Vec::new())))]),
        ..Memory::default()
    };
    assert!(matches!(
        list(&mut duplicate_session).await,
        Err(SourceError::Duplicate)
    ));
    assert_eq!(duplicate_session.read, [owner.id]);
    let mut duplicate_request = Memory {
        sessions: Some(Ok(vec![owner.clone()])),
        snapshots: VecDeque::from([(
            owner.id,
            Ok(held(vec![request.clone(), request], Vec::new())),
        )]),
        ..Memory::default()
    };
    assert!(matches!(
        list(&mut duplicate_request).await,
        Err(SourceError::Duplicate)
    ));
}

#[tokio::test]
async fn preflight_rereads_one_exact_session_and_requires_every_selected_body() {
    let owner = SessionId::random();
    let mut request = permission();
    let question = question();
    let rows = [
        Row::permission(owner, &request),
        Row::question(owner, &question),
    ];
    request.options[0].name = "Current canonical option".into();
    let mut source = Memory {
        snapshots: VecDeque::from([(
            owner,
            Ok(held(vec![request.clone()], vec![question.clone()])),
        )]),
        ..Memory::default()
    };
    assert_eq!(
        preflight(&mut source, &rows).await.unwrap(),
        [
            Ready::Permission {
                session: owner,
                request: request.clone()
            },
            Ready::Question {
                session: owner,
                question
            },
        ]
    );
    assert_eq!(source.read, [owner]);
    let mut gone = Memory {
        snapshots: VecDeque::from([(owner, Ok(held(vec![request], Vec::new())))]),
        ..Memory::default()
    };
    assert!(matches!(
        preflight(&mut gone, &rows).await,
        Err(SourceError::Gone)
    ));
    assert_eq!(gone.read, [owner]);
}

#[tokio::test]
async fn mixed_or_duplicate_scope_is_refused_before_any_snapshot_read() {
    let owner = SessionId::random();
    let other = SessionId::random();
    let request = permission();
    let row = Row::permission(owner, &request);
    let mut source = Memory::default();
    assert!(matches!(
        preflight(
            &mut source,
            &[row.clone(), Row::permission(other, &request)]
        )
        .await,
        Err(SourceError::Mismatch)
    ));
    assert!(matches!(
        preflight(&mut source, &[row.clone(), row]).await,
        Err(SourceError::Duplicate)
    ));
    assert!(preflight(&mut source, &[]).await.unwrap().is_empty());
    assert!(source.read.is_empty());
}

#[tokio::test]
async fn an_oversized_session_list_is_refused_before_conversation_reads() {
    let terminal = session(
        SessionId::random(),
        SessionMode::Terminal {
            args: Vec::new(),
            size: TerminalSize::default(),
        },
    );
    let mut source = Memory {
        sessions: Some(Ok(vec![terminal; MAX_SESSIONS.saturating_add(1)])),
        ..Memory::default()
    };
    assert!(matches!(list(&mut source).await, Err(SourceError::Limit)));
    assert!(source.read.is_empty());
}

#[test]
fn preflight_retains_typed_peer_failure_without_printing_its_private_message() {
    let canary = "synthetic-private-peer-diagnostic";
    let error = SourceError::from(CallError::Remote(vessel_proto::WireError::new(
        vessel_proto::ErrorCode::Forbidden,
        canary,
    )));
    let SourceError::Node(CallError::Remote(original)) = &error else {
        panic!("original typed failure retained")
    };
    assert_eq!(original.code, vessel_proto::ErrorCode::Forbidden);
    assert!(original.message.contains(canary));
    assert!(!error.to_string().contains(canary));
    assert!(!format!("{error:?}").contains(canary));
    assert!(std::error::Error::source(&error).is_none());
}

#[test]
fn retained_preflight_bodies_are_not_embedded_in_diagnostics() {
    let mut question = question();
    question.message = "synthetic-private-form-detail".into();
    let ready = Ready::Question {
        session: SessionId::random(),
        question,
    };
    assert!(!format!("{ready:?}").contains("synthetic-private-form-detail"));
    let Ready::Question { question, .. } = ready else {
        panic!("question")
    };
    assert_eq!(question.message, "synthetic-private-form-detail");
}

#[tokio::test]
async fn a_fresh_snapshot_with_duplicate_identity_cannot_choose_its_first_body() {
    let owner = SessionId::random();
    let request = permission();
    let row = Row::permission(owner, &request);
    let mut changed = request.clone();
    changed.options[0].name = "A different current body".into();
    let mut source = Memory {
        snapshots: VecDeque::from([(owner, Ok(held(vec![request, changed], Vec::new())))]),
        ..Memory::default()
    };
    assert!(matches!(
        preflight(&mut source, &[row]).await,
        Err(SourceError::Duplicate)
    ));
    assert_eq!(source.read, [owner]);
}
