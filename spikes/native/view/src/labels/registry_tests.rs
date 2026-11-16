use vessel_proto::registry::{
    RegistryRevision, RegistrySnapshot, WorkspaceMembership, WorkspaceResource,
};
use vessel_proto::workspace_labels::WorkspaceLabelAssignments;
use vessel_proto::{NodeId, SessionId, TerminalId, WorkspaceId};

use super::Assignments;
use crate::labels::Name;

fn name(text: &str) -> Name {
    text.parse().unwrap()
}

fn snapshot(
    revision: u64,
    memberships: Vec<WorkspaceMembership>,
    labels: Vec<WorkspaceLabelAssignments>,
) -> RegistrySnapshot {
    let mut revision_at = RegistryRevision::ZERO;
    for _ in 0..revision {
        revision_at = revision_at.next().unwrap();
    }
    RegistrySnapshot {
        labels,
        node: NodeId::random(),
        revision: revision_at,
        workspaces: Vec::new(),
        memberships,
    }
}

#[test]
fn a_session_reaches_its_workspace_and_its_labels_through_the_registry() {
    let session = SessionId::random();
    let workspace = WorkspaceId::random();
    let read = Assignments::read(&snapshot(
        3,
        vec![
            WorkspaceMembership {
                workspace,
                resource: WorkspaceResource::Session { session },
            },
            WorkspaceMembership {
                workspace,
                resource: WorkspaceResource::Terminal {
                    terminal: TerminalId::random(),
                },
            },
        ],
        vec![WorkspaceLabelAssignments {
            workspace,
            labels: vec![name("Ready"), name("Blocked")],
        }],
    ));
    assert_eq!(read.workspace(session), Some(workspace));
    assert_eq!(read.labels(workspace), [name("Ready"), name("Blocked")]);
    assert_eq!(read.revision().map(RegistryRevision::get), Some(3));
    // An older session (no membership) has no workspace.
    assert_eq!(read.workspace(SessionId::random()), None);
    assert!(read.labels(WorkspaceId::random()).is_empty());
}

#[test]
fn a_committed_assignment_shows_at_once_and_the_next_snapshot_replaces_it() {
    let workspace = WorkspaceId::random();
    let mut assignments = Assignments::read(&snapshot(1, Vec::new(), Vec::new()));
    assignments.assigned(workspace, vec![name("Ready")]);
    assert_eq!(assignments.labels(workspace), [name("Ready")]);
    assignments.assigned(workspace, Vec::new());
    assert!(assignments.labels(workspace).is_empty());
    assignments.assigned(workspace, vec![name("Ready")]);
    let next = Assignments::read(&snapshot(2, Vec::new(), Vec::new()));
    assert!(next.labels(workspace).is_empty());
    assert_eq!(next.revision().map(RegistryRevision::get), Some(2));
}
