//! Canonical relation output enters layout without a second classifier/type.
use super::*;
use vessel_proto::subagents::{Relation, WorkspaceRole};
use vessel_proto::{NodeId, SessionId, SessionRef};

fn opened(role: WorkspaceRole) -> (Layout, TabId, SessionId) {
    let mut layout = Layout::default();
    let session = SessionId::random();
    // This direct canonical-typed argument is intentional: a copied native enum
    // would require another conversion and no longer satisfy the adapter seam.
    let tab = layout
        .open(
            Target::Agent(session),
            OpenIntent::Reveal,
            Placement::Focused,
            role,
            LayoutMode::Compact,
        )
        .unwrap();
    (layout, tab, session)
}

#[test]
fn canonical_node_scoped_membership_keeps_child_hide_and_cross_node_root_close_distinct() {
    let node = NodeId::random();
    let workspace = WorkspaceId::random();
    let child_owner = (node, workspace);
    let relation = Relation::Known(Some(SessionRef {
        node,
        session: SessionId::random(),
    }));
    let (child, tab, session) =
        opened(relation.workspace_role(Some(&child_owner), Some(&child_owner)));
    assert_eq!(
        child.request_close(tab).unwrap().effect(),
        CloseEffect::HideChild(session)
    );
    assert_eq!(child.sheets().last().map(|sheet| sheet.tab), Some(tab));

    let parent_owner = (NodeId::random(), workspace);
    let relation = Relation::Known(Some(SessionRef {
        node: parent_owner.0,
        session: SessionId::random(),
    }));
    let (root, tab, session) =
        opened(relation.workspace_role(Some(&child_owner), Some(&parent_owner)));
    assert_eq!(
        root.request_close(tab).unwrap().effect(),
        CloseEffect::ArchiveAgent(session)
    );
    assert!(root.sheets().is_empty());
}

#[test]
fn missing_owner_metadata_keeps_unknown_close_unresolved_and_legacy_role_path_available() {
    let node = NodeId::random();
    let owner = (node, WorkspaceId::random());
    let relation = Relation::Known(Some(SessionRef {
        node,
        session: SessionId::random(),
    }));
    let role: AgentRole = relation.workspace_role(Some(&owner), None);
    let (layout, tab, session) = opened(role);
    assert!(layout.sheets().is_empty());
    assert_eq!(
        layout.request_close(tab).unwrap().effect(),
        CloseEffect::ResolveAgent(session)
    );
    assert_eq!(role, AgentRole::Unknown);
}
