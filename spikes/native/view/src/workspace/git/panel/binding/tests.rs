//! Same-alias reconnects and equal tab numbers never share discovery authority.
use super::*;
use crate::connection::{ConnectionEpochs, MachineKey};
use crate::layout::HostAlias;
use crate::workspace::git::RequestKeys;
use vessel_proto::workspace::SessionWorkspace;
use vessel_proto::{HarnessKind, NodeId, ProjectId, SessionMode, SessionSpec};

fn address(machine: &str, session: SessionId) -> PanelAddress {
    PanelAddress {
        workspace: LayoutKey::Session {
            machine: HostAlias::parse(machine).unwrap(),
            session,
        },
        tab: serde_json::from_str("1").unwrap(),
    }
}
fn session(id: SessionId, project: Option<ProjectId>) -> Box<SessionInfo> {
    let mut session = SessionInfo::new(
        id,
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        "2026-01-01T00:00:00Z".parse().unwrap(),
    );
    session.workspace = project.map(|project| {
        Box::new(SessionWorkspace {
            project,
            worktree: None,
            branch: None,
        })
    });
    Box::new(session)
}
#[test]
fn same_alias_node_and_root_cannot_revive_a_retired_discovery() {
    let mut epochs = ConnectionEpochs::default();
    let node = NodeId::random();
    let first = epochs
        .admit(MachineKey::from_book("fixture"), node)
        .unwrap();
    let next = epochs
        .admit(MachineKey::from_book("fixture"), node)
        .unwrap();
    let id = SessionId::random();
    let owner = address("fixture", id);
    let project = ProjectId::random();
    let mut keys = RequestKeys::default();
    let mut binding = PanelBinding::default();
    let old = binding
        .begin(owner.clone(), first.clone(), keys.issue().unwrap())
        .unwrap();
    assert!(binding.reconcile(Some(&next)));
    assert!(!binding.finish(&old, Ok(session(id, Some(project)))));
    assert_eq!(binding.repository(), None);
    let current = binding
        .begin(owner, next.clone(), keys.issue().unwrap())
        .unwrap();
    assert!(!binding.finish(&old, Ok(session(id, Some(project)))));
    assert!(binding.finish(&current, Ok(session(id, Some(project)))));
    assert_eq!(
        binding.repository(),
        Some(RepositoryKey {
            node,
            root: WorkspaceRoot::Project { project }
        })
    );
    assert_eq!(binding.context(), Some(&next));
    assert!(!binding.finish(&current, Ok(session(id, None))));
    assert!(!binding.reconcile(Some(&next)));
    assert!(binding.reconcile(Some(&first)));
    assert_eq!(binding.repository(), None);
}
#[test]
fn explicit_layout_owners_and_session_ids_are_checked_before_installation() {
    let mut epochs = ConnectionEpochs::default();
    let context = epochs
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let id = SessionId::random();
    let other = SessionId::random();
    let mut keys = RequestKeys::default();
    let mut binding = PanelBinding::default();
    let before = binding
        .begin(
            address("fixture", id),
            context.clone(),
            keys.issue().unwrap(),
        )
        .unwrap();
    let after = binding
        .begin(address("fixture", other), context, keys.issue().unwrap())
        .unwrap();
    assert_eq!(before.address().tab, after.address().tab);
    assert_ne!(before.address().workspace, after.address().workspace);
    assert!(!binding.finish(&before, Ok(session(id, None))));
    assert!(binding.finish(&after, Ok(session(id, None))));
    assert!(
        matches!(binding, PanelBinding::Failed(ReadFailure::UnexpectedSession { requested, received }) if requested == other && received == id)
    );
}
#[test]
fn a_different_alias_or_unhydrated_workspace_never_issues_a_discovery() {
    let mut epochs = ConnectionEpochs::default();
    let context = epochs
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let mut keys = RequestKeys::default();
    let mut binding = PanelBinding::default();
    assert_eq!(
        binding.begin(
            address("another", SessionId::random()),
            context.clone(),
            keys.issue().unwrap()
        ),
        Err(LocationRefusal::DestinationChanged)
    );
    assert!(matches!(binding, PanelBinding::Idle));
    let workspace = PanelAddress {
        workspace: LayoutKey::Workspace {
            node: context.node(),
            workspace: vessel_proto::WorkspaceId::random(),
        },
        tab: serde_json::from_str("1").unwrap(),
    };
    assert_eq!(
        binding.begin(workspace, context, keys.issue().unwrap()),
        Err(LocationRefusal::NeedsRegistry)
    );
    assert!(matches!(binding, PanelBinding::Idle));
}
#[test]
fn genuine_outside_and_unsent_are_settled_without_guessing_another_root() {
    let mut epochs = ConnectionEpochs::default();
    let context = epochs
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let id = SessionId::random();
    let mut keys = RequestKeys::default();
    let mut binding = PanelBinding::default();
    let sent = binding
        .begin(
            address("fixture", id),
            context.clone(),
            keys.issue().unwrap(),
        )
        .unwrap();
    assert!(binding.finish(&sent, Ok(session(id, None))));
    assert_eq!(binding.repository(), None);
    assert_eq!(binding.context(), Some(&context));
    let unsent = binding
        .begin(address("fixture", id), context, keys.issue().unwrap())
        .unwrap();
    assert!(binding.finish(
        &unsent,
        Err(ReadFailure::Call(
            crate::workspace::problem::Problem::Unsent
        ))
    ));
    assert!(matches!(
        binding,
        PanelBinding::Failed(ReadFailure::Call(
            crate::workspace::problem::Problem::Unsent
        ))
    ));
    assert_eq!(binding.context(), None);
}
#[test]
fn immutable_commit_identity_participates_in_every_comparison_and_scope() {
    let first: vessel_proto::workspace::CommitId = "a".repeat(40).parse().unwrap();
    let second: vessel_proto::workspace::CommitId = "b".repeat(40).parse().unwrap();
    let one = super::super::Comparison::at_commit(first.clone(), super::super::Whitespace::Show);
    let two = super::super::Comparison::at_commit(second, super::super::Whitespace::Show);
    assert_ne!(one, two);
    assert_eq!(one.mode(), None);
    assert_eq!(one.base(), None);
    assert_eq!(
        one.legacy_scope(),
        Some(vessel_proto::workspace::DiffScope::Commit { id: first })
    );
}

#[test]
fn an_old_answer_never_retires_the_new_admission_at_the_same_address() {
    let mut epochs = ConnectionEpochs::default();
    let first = epochs
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let newer = epochs.admit(first.machine().clone(), first.node()).unwrap();
    assert_eq!(
        answer_admission(Some(&newer), Some(&newer), &first),
        AnswerAdmission::Ignore
    );
    assert_eq!(
        answer_admission(Some(&newer), None, &first),
        AnswerAdmission::Ignore
    );
    assert_eq!(
        answer_admission(None, Some(&newer), &first),
        AnswerAdmission::Ignore
    );
    assert_eq!(
        answer_admission(Some(&first), Some(&newer), &first),
        AnswerAdmission::Retire
    );
    assert_eq!(
        answer_admission(Some(&first), None, &first),
        AnswerAdmission::Retire
    );
    assert_eq!(
        answer_admission(Some(&newer), Some(&newer), &newer),
        AnswerAdmission::MatchTicket
    );
}
