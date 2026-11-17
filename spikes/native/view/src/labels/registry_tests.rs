use chrono::{DateTime, Duration, Utc};
use vessel_proto::registry::{
    RegistryRevision, RegistrySnapshot, WorkspaceAutomation, WorkspaceBacking,
    WorkspaceDescriptor, WorkspaceLifecycle, WorkspaceMembership, WorkspacePin,
    WorkspaceResource, WorkspaceVisibility,
};
use vessel_proto::workspace_labels::WorkspaceLabelAssignments;
use vessel_proto::{NodeId, ProjectId, SessionId, TerminalId, WorkspaceId, WorktreeId};

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
    assert_eq!(read.workspace(session, None), Some(workspace));
    assert_eq!(read.labels(workspace), [name("Ready"), name("Blocked")]);
    assert_eq!(read.revision().map(RegistryRevision::get), Some(3));
    // An older session (no membership) has no workspace.
    assert_eq!(read.workspace(SessionId::random(), None), None);
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

#[test]
fn an_assignment_outlives_a_snapshot_read_before_its_commit() {
    let workspace = WorkspaceId::random();
    let labelled = |labels: Vec<Name>| vec![WorkspaceLabelAssignments { workspace, labels }];
    let mut assignments = Assignments::read(&snapshot(4, Vec::new(), Vec::new()));
    assignments.assigned(workspace, vec![name("Ready")]);
    // Read before the commit (no newer than the revision held): kept.
    assignments.refreshed(&snapshot(4, Vec::new(), Vec::new()));
    assert_eq!(assignments.labels(workspace), [name("Ready")]);
    // One newer snapshot still without it: kept once more.
    assignments.refreshed(&snapshot(5, Vec::new(), Vec::new()));
    assert_eq!(assignments.labels(workspace), [name("Ready")]);
    // Another newer one without it: the registry wins (someone else
    // changed it since).
    assignments.refreshed(&snapshot(6, Vec::new(), Vec::new()));
    assert!(assignments.labels(workspace).is_empty());
    // A snapshot carrying the commit confirms it and ends the override.
    assignments.assigned(workspace, vec![name("Blocked")]);
    assignments.refreshed(&snapshot(7, Vec::new(), labelled(vec![name("Blocked")])));
    assert_eq!(assignments.labels(workspace), [name("Blocked")]);
    assignments.refreshed(&snapshot(8, Vec::new(), Vec::new()));
    assert!(assignments.labels(workspace).is_empty());
}

fn descriptor(
    project: ProjectId,
    created_at: DateTime<Utc>,
    backing: WorkspaceBacking,
    lifecycle: WorkspaceLifecycle,
    visibility: WorkspaceVisibility,
) -> WorkspaceDescriptor {
    WorkspaceDescriptor {
        id: WorkspaceId::random(),
        project,
        path: "/work/shop".into(),
        title: None,
        automation: WorkspaceAutomation::Allowed,
        pin: WorkspacePin::Unpinned,
        backing,
        lifecycle,
        visibility,
        created_at,
        revision: RegistryRevision::ZERO,
    }
}

#[test]
fn a_session_placed_in_a_project_belongs_to_its_oldest_live_local_workspace() {
    let project = ProjectId::random();
    let start = Utc::now();
    let at = |minutes| start + Duration::minutes(minutes);
    let local = WorkspaceBacking::Local;
    let active = WorkspaceLifecycle::Active;
    let front = WorkspaceVisibility::Foreground;
    let owner = descriptor(project, at(3), local, active.clone(), front);
    let owner_id = owner.id;
    let mut registry = snapshot(1, Vec::new(), Vec::new());
    registry.workspaces = vec![
        // Older ones that do not own: archived, background, a worktree.
        descriptor(project, at(1), local, WorkspaceLifecycle::Archived { at: at(0) }, front),
        descriptor(project, at(1), local, active.clone(), WorkspaceVisibility::Background),
        descriptor(
            project,
            at(2),
            WorkspaceBacking::Managed {
                worktree: WorktreeId::random(),
            },
            active.clone(),
            front,
        ),
        descriptor(project, at(4), local, active, front),
        owner,
    ];
    let session = SessionId::random();
    let read = Assignments::read(&registry);
    assert_eq!(read.workspace(session, Some(project)), Some(owner_id));
    // No project (a plain folder, a worktree or archived): no workspace.
    assert_eq!(read.workspace(session, None), None);
    assert_eq!(read.workspace(session, Some(ProjectId::random())), None);
    // An explicit row wins over the derived one.
    let explicit = WorkspaceId::random();
    registry.memberships = vec![WorkspaceMembership {
        workspace: explicit,
        resource: WorkspaceResource::Session { session },
    }];
    let read = Assignments::read(&registry);
    assert_eq!(read.workspace(session, Some(project)), Some(explicit));
}
