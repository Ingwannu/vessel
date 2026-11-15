//! One index drives child order, workspace placement and jump expansion.

use super::*;
use vessel_proto::{NodeId, SessionId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Workspace {
    One,
    Two,
}
fn id(node: NodeId) -> SessionRef {
    SessionRef {
        node,
        session: SessionId::random(),
    }
}
fn row(
    session: SessionRef,
    parent: Option<SessionRef>,
    workspace: Workspace,
    second: i64,
) -> ManagedSnapshot<Workspace> {
    ManagedSnapshot {
        session,
        relation: Relation::Known(parent),
        workspace,
        created_at: DateTime::from_timestamp(second, 0).unwrap(),
        visibility: Visibility::Active,
    }
}

#[test]
fn child_order_is_oldest_first_across_hosts_and_aliases_do_not_duplicate_rows() {
    let a = NodeId::random();
    let b = NodeId::random();
    let root = id(a);
    let older = id(b);
    let newer = id(a);
    let first = row(older, Some(root), Workspace::One, 1);
    let tree = ManagedTree::new(
        vec![
            row(root, None, Workspace::One, 0),
            row(newer, Some(root), Workspace::One, 2),
            first.clone(),
            first,
        ],
        TreeCoverage::Complete,
    )
    .unwrap();
    assert_eq!(tree.children(root), [older, newer]);
    let expanded = HashSet::from([root]);
    let rows = tree.rows(&Workspace::One, &expanded);
    assert_eq!(
        rows.iter().map(|row| row.session).collect::<Vec<_>>(),
        [root, older, newer]
    );
    assert_eq!(
        rows.iter().map(|row| row.depth.level()).collect::<Vec<_>>(),
        [0, 1, 1]
    );
    assert_eq!(tree.rows(&Workspace::One, &HashSet::new()).len(), 1);
}

#[test]
fn same_session_uuid_on_another_node_is_a_different_target() {
    let a = NodeId::random();
    let b = NodeId::random();
    let root = id(a);
    let foreign = SessionRef {
        node: b,
        session: root.session,
    };
    let child = id(b);
    let tree = ManagedTree::new(
        vec![
            row(root, None, Workspace::One, 0),
            row(foreign, None, Workspace::One, 1),
            row(child, Some(foreign), Workspace::One, 2),
        ],
        TreeCoverage::Complete,
    )
    .unwrap();
    assert_eq!(tree.children(root), []);
    assert_eq!(tree.children(foreign), [child]);
    assert_eq!(tree.reveal(child).unwrap().ancestors, [foreign]);
}

#[test]
fn a_cross_workspace_child_stays_a_normal_row_in_its_own_workspace() {
    let root = id(NodeId::random());
    let child = id(NodeId::random());
    let tree = ManagedTree::new(
        vec![
            row(root, None, Workspace::One, 0),
            row(child, Some(root), Workspace::Two, 1),
        ],
        TreeCoverage::Complete,
    )
    .unwrap();
    assert_eq!(tree.children(root), [child]);
    let expanded = HashSet::from([root]);
    assert_eq!(
        tree.rows(&Workspace::One, &expanded)
            .iter()
            .map(|row| row.session)
            .collect::<Vec<_>>(),
        [root]
    );
    let own = tree.rows(&Workspace::Two, &expanded);
    assert_eq!(
        own.iter().map(|row| row.session).collect::<Vec<_>>(),
        [child]
    );
    assert_eq!(own.first().unwrap().depth.level(), 0);
    let reveal = tree.reveal(child).unwrap();
    assert_eq!(reveal.workspace, Workspace::Two);
    assert!(reveal.ancestors.is_empty());
}

#[test]
fn archive_intent_excludes_managed_rows_and_detached_children_are_roots() {
    let node = NodeId::random();
    let root = id(node);
    let child = id(node);
    let pending = id(node);
    let archived = id(node);
    let mut hidden = row(pending, Some(root), Workspace::One, 2);
    hidden.visibility = Visibility::PendingArchive;
    let mut old = row(archived, Some(root), Workspace::One, 3);
    old.visibility = Visibility::Archived;
    // Known(None) is the resolved result for both a root and a detached child.
    let tree = ManagedTree::new(
        vec![
            row(root, None, Workspace::One, 0),
            row(child, None, Workspace::One, 1),
            hidden,
            old,
        ],
        TreeCoverage::Complete,
    )
    .unwrap();
    assert!(tree.children(root).is_empty());
    assert_eq!(tree.rows(&Workspace::One, &HashSet::new()).len(), 2);
    assert_eq!(tree.parent(child), Relation::Known(None));
    assert!(tree.reveal(pending).is_none());
}

#[test]
fn cyclic_or_unknown_relations_stay_visible_without_fabricating_root_authority() {
    let node = NodeId::random();
    let a = id(node);
    let b = id(node);
    let c = id(node);
    let mut unknown = row(c, None, Workspace::One, 2);
    unknown.relation = Relation::Unknown;
    let tree = ManagedTree::new(
        vec![
            row(a, Some(b), Workspace::One, 0),
            row(b, Some(a), Workspace::One, 1),
            unknown,
        ],
        TreeCoverage::Complete,
    )
    .unwrap();
    assert_eq!(tree.coverage(), TreeCoverage::Partial);
    assert_eq!(tree.parent(a), Relation::Unknown);
    assert_eq!(tree.parent(b), Relation::Unknown);
    assert_eq!(tree.parent(c), Relation::Unknown);
    assert_eq!(
        tree.rows(&Workspace::One, &HashSet::from([a, b, c])).len(),
        3
    );
    assert!(tree.children(a).is_empty());
}

#[test]
fn conflicting_aliases_are_refused_instead_of_arbitrarily_selecting_a_relation() {
    let node = NodeId::random();
    let root = id(node);
    let child = id(node);
    let a = row(child, Some(root), Workspace::One, 1);
    let b = row(child, None, Workspace::One, 1);
    assert!(
        matches!(ManagedTree::new(vec![a,b],TreeCoverage::Complete),Err(TreeProblem::Ambiguous(id)) if id==child)
    );
}

#[test]
fn jump_expansion_and_visible_rows_read_the_same_nested_order() {
    let node = NodeId::random();
    let root = id(node);
    let child = id(node);
    let grandchild = id(node);
    let tree = ManagedTree::new(
        vec![
            row(root, None, Workspace::One, 0),
            row(child, Some(root), Workspace::One, 1),
            row(grandchild, Some(child), Workspace::One, 2),
        ],
        TreeCoverage::Complete,
    )
    .unwrap();
    let reveal = tree.reveal(grandchild).unwrap();
    assert_eq!(reveal.ancestors, [root, child]);
    let expanded: HashSet<_> = reveal.ancestors.into_iter().collect();
    assert_eq!(
        tree.rows(&reveal.workspace, &expanded)
            .last()
            .unwrap()
            .session,
        reveal.target
    );
}

#[test]
fn old_and_unassigned_peers_never_turn_audit_creator_into_a_root_fact() {
    use vessel_proto::{
        Creator, HarnessKind, ProtocolVersion, SessionInfo, SessionMode, SessionSpec,
    };
    let parent = id(NodeId::random());
    let info = SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![],
            },
        ),
        DateTime::UNIX_EPOCH,
    )
    .with_creator(Creator::Agent {
        node: parent.node,
        session: parent.session,
    });
    assert_eq!(
        Relation::for_session(&info, ProtocolVersion::new(1)),
        Relation::Unknown
    );
    let legacy = crate::Card::from_session("fixture", &info);
    assert_eq!(legacy.detail.relation, Relation::Unknown);
    assert_eq!(legacy.detail.created_by, info.created_by);
    if vessel_proto::subagents::SUBAGENT_RELATIONS.is_none() {
        assert_eq!(
            Relation::for_session(&info, vessel_proto::PROTOCOL_VERSION),
            Relation::Unknown
        );
    }
    let mut json = serde_json::to_value(&legacy).unwrap();
    let _legacy_relation = json
        .get_mut("detail")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("relation");
    let cached: crate::Card = serde_json::from_value(json).unwrap();
    assert_eq!(cached.detail.relation, Relation::Unknown);
}

#[test]
fn workspace_root_role_keeps_cross_workspace_children_visible_and_unknown_ownership_explicit() {
    use crate::layout::AgentRole;
    let relation = Relation::Known(Some(id(NodeId::random())));
    assert_eq!(
        Relation::Known(None).workspace_role::<Workspace>(None, None),
        AgentRole::Root
    );
    assert_eq!(
        relation.workspace_role(Some(&Workspace::One), Some(&Workspace::One)),
        AgentRole::Child
    );
    assert_eq!(
        relation.workspace_role(Some(&Workspace::One), Some(&Workspace::Two)),
        AgentRole::Root
    );
    assert_eq!(
        relation.workspace_role(Some(&Workspace::One), None),
        AgentRole::Unknown
    );
    assert_eq!(
        relation.workspace_role(None, Some(&Workspace::Two)),
        AgentRole::Unknown
    );
    assert_eq!(
        Relation::Unknown.workspace_role(Some(&Workspace::One), Some(&Workspace::Two)),
        AgentRole::Unknown
    );
}
