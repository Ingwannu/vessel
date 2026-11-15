use super::*;
use crate::{
    Creator, HarnessKind, NodeId, PROTOCOL_VERSION, SessionId, SessionMode, SessionSpec,
    WorkspaceId,
    subagents::{DetachReceipt, SUBAGENT_RELATIONS},
};

#[test]
fn workspace_roles_preserve_unknowns_and_full_node_scoped_ownership() {
    let node = NodeId::random();
    let workspace = WorkspaceId::random();
    let local = (node, workspace);
    let elsewhere = (node, WorkspaceId::random());
    let same_uuid_other_node = (NodeId::random(), workspace);
    let parent = SessionRef {
        node,
        session: SessionId::random(),
    };
    for (relation, child, owner, expected) in [
        (Relation::Known(None), None, None, WorkspaceRole::Root),
        (
            Relation::Known(Some(parent)),
            Some(local),
            Some(local),
            WorkspaceRole::Child,
        ),
        (
            Relation::Known(Some(parent)),
            Some(local),
            Some(elsewhere),
            WorkspaceRole::Root,
        ),
        (
            Relation::Known(Some(SessionRef {
                node: same_uuid_other_node.0,
                ..parent
            })),
            Some(local),
            Some(same_uuid_other_node),
            WorkspaceRole::Root,
        ),
        (
            Relation::Known(Some(parent)),
            Some(local),
            None,
            WorkspaceRole::Unknown,
        ),
        (
            Relation::Known(Some(parent)),
            None,
            Some(local),
            WorkspaceRole::Unknown,
        ),
        (
            Relation::Unknown,
            Some(local),
            Some(elsewhere),
            WorkspaceRole::Unknown,
        ),
    ] {
        assert_eq!(
            relation.workspace_role(child.as_ref(), owner.as_ref()),
            expected
        );
    }
}

fn info() -> SessionInfo {
    SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    )
}

#[test]
fn session_confidence_requires_the_assigned_capability_and_valid_receipt() {
    let parent = SessionRef {
        node: NodeId::random(),
        session: SessionId::random(),
    };
    let mut attached = info();
    attached.created_by = Creator::Agent {
        node: parent.node,
        session: parent.session,
    };
    let mut detached = attached.clone();
    detached.detach_receipt = Some(DetachReceipt::new(parent, chrono::DateTime::UNIX_EPOCH));
    let mut corrupt = info();
    corrupt.detach_receipt = detached.detach_receipt;
    let supported = SUBAGENT_RELATIONS.is_some_and(|since| since <= PROTOCOL_VERSION);
    for (source, known) in [
        (info(), Relation::Known(None)),
        (attached.clone(), Relation::Known(Some(parent))),
        (detached.clone(), Relation::Known(None)),
        (corrupt, Relation::Unknown),
    ] {
        assert_eq!(
            Relation::for_session(&source, PROTOCOL_VERSION),
            if supported { known } else { Relation::Unknown }
        );
        if let Some(older) = SUBAGENT_RELATIONS
            .and_then(|since| since.get().checked_sub(1))
            .map(ProtocolVersion::new)
        {
            assert_eq!(Relation::for_session(&source, older), Relation::Unknown);
        }
    }
    assert_eq!(detached.created_by, attached.created_by);
    assert_eq!(Relation::default(), Relation::Unknown);
    assert_eq!(WorkspaceRole::default(), WorkspaceRole::Unknown);
}

#[test]
fn relation_roundtrip_keeps_the_complete_parent_reference() {
    let parent = SessionRef {
        node: NodeId::random(),
        session: SessionId::random(),
    };
    for relation in [
        Relation::Known(None),
        Relation::Known(Some(parent)),
        Relation::Unknown,
    ] {
        let encoded = serde_json::to_string(&relation).unwrap();
        assert_eq!(
            serde_json::from_str::<Relation>(&encoded).unwrap(),
            relation
        );
    }
}
