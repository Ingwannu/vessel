//! Pure canonical values only; no actor, provider, directory or path lookup.
use super::*;
use crate::{
    connection::{ConnectionEpochs, MachineKey},
    layout::{WorkspaceLocation, WorkspaceLookup},
};
use vessel_proto::{
    NodeId, ProjectId, SessionId, WorkspaceId,
    registry::{
        RegistryRevision, WorkspaceAutomation, WorkspaceBacking, WorkspaceDescriptor,
        WorkspaceLifecycle, WorkspaceMembership, WorkspacePin, WorkspaceResource,
        WorkspaceVisibility,
    },
    workspace::{GitSupport, ProjectInfo, ProjectState, WorkspaceRoot},
};

fn context(epochs: &mut ConnectionEpochs, name: &str, node: NodeId) -> ConnectionContext {
    epochs.admit(MachineKey::from_book(name), node).unwrap()
}

fn data(node: NodeId) -> RegistryData {
    let project = ProjectInfo {
        id: ProjectId::random(),
        name: "registered".parse().unwrap(),
        root: "/fixture/project".into(),
        git: false,
        default_branch: None,
        state: ProjectState::Ready,
        created_at: chrono::DateTime::UNIX_EPOCH,
        revision: 0,
    };
    let workspace = WorkspaceDescriptor {
        id: WorkspaceId::random(),
        project: project.id,
        path: project.root.clone(),
        title: None,
        automation: WorkspaceAutomation::Allowed,
        pin: WorkspacePin::Unpinned,
        backing: WorkspaceBacking::Local,
        lifecycle: WorkspaceLifecycle::Active,
        visibility: WorkspaceVisibility::Foreground,
        created_at: chrono::DateTime::UNIX_EPOCH,
        revision: RegistryRevision::ZERO,
    };
    RegistryData {
        registry: RegistrySnapshot {
            node,
            revision: RegistryRevision::ZERO,
            workspaces: vec![workspace],
            memberships: Vec::new(),
        },
        projects: ProjectList {
            git: GitSupport::Missing,
            projects: vec![project],
            more: false,
        },
        worktrees: WorktreeList {
            worktrees: Vec::new(),
            more: false,
        },
    }
}

#[test]
fn empty_workspaces_sharing_a_directory_keep_distinct_layout_keys_and_memberships() {
    let mut epochs = ConnectionEpochs::default();
    let node = NodeId::random();
    let context = context(&mut epochs, "fixture", node);
    let mut data = data(node);
    let first = data.registry.workspaces[0].id;
    let second = WorkspaceId::random();
    data.registry.workspaces.push(WorkspaceDescriptor {
        id: second,
        ..data.registry.workspaces[0].clone()
    });
    let session = SessionId::random();
    data.registry.memberships.push(WorkspaceMembership {
        workspace: second,
        resource: WorkspaceResource::Session { session },
    });
    let mut cache = RegistryProjection::default();
    assert_eq!(
        cache.session_workspace(node, session),
        Err(LookupProblem::NotHydrated)
    );
    let key = cache.begin(context.clone()).unwrap();
    assert!(cache.finish(&key, Ok(data)).unwrap());
    assert_eq!(cache.session_workspace(node, session), Ok(Some(second)));
    assert_eq!(cache.session_workspace(node, SessionId::random()), Ok(None));
    let rows: Vec<_> = cache.rows(&context).unwrap().collect();
    assert_ne!(rows[0].key(), rows[1].key());
    assert_eq!(rows[0].placement(), rows[1].placement());
    assert_eq!(rows[0].workspace().id, first);
    assert_eq!(rows[0].resources().count(), 0);
    assert_eq!(
        rows[1].resources().collect::<Vec<_>>(),
        vec![WorkspaceResource::Session { session }]
    );
    assert!(rows.iter().all(WorkspaceRow::visible));
}

#[test]
fn retirement_and_same_node_reconnect_reject_delayed_reads_without_empty_fallback() {
    let mut epochs = ConnectionEpochs::default();
    let node = NodeId::random();
    let old = context(&mut epochs, "fixture", node);
    let next = context(&mut epochs, "fixture", node);
    let mut cache = RegistryProjection::default();
    let stale = cache.begin(old.clone()).unwrap();
    cache.reconcile(std::slice::from_ref(&next));
    assert!(!cache.finish(&stale, Ok(data(node))).unwrap());
    assert_eq!(
        cache.session_workspace(node, SessionId::random()),
        Err(LookupProblem::Unavailable)
    );
    let fresh = cache.begin(next.clone()).unwrap();
    assert!(!cache.finish(&stale, Ok(data(node))).unwrap());
    assert!(
        cache
            .finish(&fresh, Err(RegistryReadFailure::Unsupported))
            .unwrap()
    );
    assert_eq!(cache.phase(&next), Some(RegistryPhase::Unsupported));
    assert_eq!(
        cache.session_workspace(node, SessionId::random()),
        Err(LookupProblem::Unavailable)
    );
}

#[test]
fn conflicting_membership_or_peer_identity_is_not_exposed_as_a_partial_snapshot() {
    let mut epochs = ConnectionEpochs::default();
    let node = NodeId::random();
    let context = context(&mut epochs, "fixture", node);
    let mut cache = RegistryProjection::default();
    let key = cache.begin(context.clone()).unwrap();
    assert_eq!(
        cache.finish(&key, Ok(data(NodeId::random()))),
        Err(ProjectionRefusal::Invalid)
    );
    let mut duplicate = data(node);
    let member = WorkspaceMembership {
        workspace: duplicate.registry.workspaces[0].id,
        resource: WorkspaceResource::Session {
            session: SessionId::random(),
        },
    };
    duplicate.registry.memberships = vec![member, member];
    let key = cache.begin(context.clone()).unwrap();
    assert_eq!(
        cache.finish(&key, Ok(duplicate)),
        Err(ProjectionRefusal::Invalid)
    );
    assert_eq!(cache.phase(&context), Some(RegistryPhase::Refused));
}

#[test]
fn lower_revisions_and_old_tickets_cannot_replace_current_facts() {
    let mut epochs = ConnectionEpochs::default();
    let node = NodeId::random();
    let context = context(&mut epochs, "fixture", node);
    let mut cache = RegistryProjection::default();
    let mut newer = data(node);
    newer.registry.revision = RegistryRevision::from(2);
    let key = cache.begin(context.clone()).unwrap();
    assert!(cache.finish(&key, Ok(newer)).unwrap());
    let next = cache.begin(context.clone()).unwrap();
    assert!(
        !cache
            .finish(&key, Err(RegistryReadFailure::Offline))
            .unwrap()
    );
    assert!(!cache.finish(&next, Ok(data(node))).unwrap());
    assert_eq!(
        cache.changed_from(&context),
        Some(RegistryRevision::from(2))
    );
    assert_eq!(cache.phase(&context), Some(RegistryPhase::Ready));
}

#[test]
fn roots_require_explicit_matching_placement_and_archived_ownership_stays_historical() {
    let mut epochs = ConnectionEpochs::default();
    let node = NodeId::random();
    let context = context(&mut epochs, "fixture", node);
    let mut data = data(node);
    let workspace = data.registry.workspaces[0].id;
    let project = data.registry.workspaces[0].project;
    let mut cache = RegistryProjection::default();
    let key = cache.begin(context.clone()).unwrap();
    assert!(cache.finish(&key, Ok(data.clone())).unwrap());
    assert_eq!(
        cache.workspace_location(node, workspace),
        Ok(WorkspaceLocation::Root(WorkspaceRoot::Project { project }))
    );
    data.registry.revision = RegistryRevision::from(1);
    data.registry.workspaces[0].revision = RegistryRevision::from(1);
    data.registry.workspaces[0].path = "/fixture/project/nested".into();
    let key = cache.begin(context.clone()).unwrap();
    assert!(cache.finish(&key, Ok(data.clone())).unwrap());
    assert_eq!(
        cache.workspace_location(node, workspace),
        Ok(WorkspaceLocation::External)
    );
    data.registry.revision = RegistryRevision::from(2);
    data.registry.workspaces[0].revision = RegistryRevision::from(2);
    data.registry.workspaces[0].lifecycle = WorkspaceLifecycle::Archived {
        at: chrono::DateTime::UNIX_EPOCH,
    };
    let session = SessionId::random();
    data.registry.memberships.push(WorkspaceMembership {
        workspace,
        resource: WorkspaceResource::Session { session },
    });
    let key = cache.begin(context.clone()).unwrap();
    assert!(cache.finish(&key, Ok(data)).unwrap());
    assert_eq!(cache.session_workspace(node, session), Ok(Some(workspace)));
    assert_eq!(
        cache.workspace_location(node, workspace),
        Err(LookupProblem::Unavailable)
    );
    assert!(!cache.rows(&context).unwrap().next().unwrap().visible());
}

#[test]
fn host_and_content_capacity_never_evict_another_hosts_current_snapshot() {
    let mut epochs = ConnectionEpochs::default();
    let node = NodeId::random();
    let original = context(&mut epochs, "original", node);
    let mut cache = RegistryProjection::default();
    let key = cache.begin(original.clone()).unwrap();
    assert!(cache.finish(&key, Ok(data(node))).unwrap());
    let other_node = NodeId::random();
    let protected = context(&mut epochs, "protected", other_node);
    let protected_key = cache.begin(protected.clone()).unwrap();
    assert!(cache.finish(&protected_key, Ok(data(other_node))).unwrap());
    for n in 2..MAX_REGISTRY_HOSTS {
        let _key = cache
            .begin(context(&mut epochs, &format!("host-{n}"), NodeId::random()))
            .unwrap();
    }
    assert_eq!(
        cache.begin(context(&mut epochs, "full", NodeId::random())),
        Err(ProjectionRefusal::Full)
    );
    assert_eq!(cache.phase(&original), Some(RegistryPhase::Ready));
    let mut oversized = data(node);
    oversized.registry.workspaces[0].path =
        format!("/{}", "x".repeat(MAX_REGISTRY_CACHE_BYTES)).into();
    let key = cache.begin(original.clone()).unwrap();
    assert_eq!(
        cache.finish(&key, Ok(oversized)),
        Err(ProjectionRefusal::Full)
    );
    assert_eq!(cache.hosts.len(), MAX_REGISTRY_HOSTS);
    assert_eq!(cache.phase(&protected), Some(RegistryPhase::Ready));
}

#[test]
fn git_facts_use_the_explicit_project_and_managed_backing_consistency() {
    let mut epochs = ConnectionEpochs::default();
    let node = NodeId::random();
    let context = context(&mut epochs, "fixture", node);
    let mut data = data(node);
    let mut cache = RegistryProjection::default();
    let key = cache.begin(context.clone()).unwrap();
    assert!(cache.finish(&key, Ok(data.clone())).unwrap());
    assert_eq!(
        cache.rows(&context).unwrap().next().unwrap().is_git(),
        Ok(false)
    );
    let id = vessel_proto::WorktreeId::random();
    data.registry.revision = RegistryRevision::from(1);
    data.registry.workspaces[0].revision = RegistryRevision::from(1);
    data.registry.workspaces[0].backing = WorkspaceBacking::Managed { worktree: id };
    data.worktrees
        .worktrees
        .push(vessel_proto::workspace::WorktreeInfo {
            id,
            project: data.registry.workspaces[0].project,
            path: data.registry.workspaces[0].path.clone(),
            branch: None,
            created_branch: None,
            base: None,
            created_at: chrono::DateTime::UNIX_EPOCH,
            created_by: vessel_proto::Creator::Owner,
            created_for: None,
            sessions: Vec::new(),
            setup: vessel_proto::workspace::SetupState::None,
            kept: None,
        });
    let key = cache.begin(context.clone()).unwrap();
    assert!(cache.finish(&key, Ok(data.clone())).unwrap());
    assert_eq!(
        cache.rows(&context).unwrap().next().unwrap().is_git(),
        Err(LookupProblem::Unavailable)
    );
    data.registry.revision = RegistryRevision::from(2);
    data.registry.workspaces[0].revision = RegistryRevision::from(2);
    data.projects.projects[0].git = true;
    let key = cache.begin(context.clone()).unwrap();
    assert!(cache.finish(&key, Ok(data)).unwrap());
    let row = cache.rows(&context).unwrap().next().unwrap();
    assert_eq!(row.is_git(), Ok(true));
    assert_eq!(
        row.location(),
        Ok(WorkspaceLocation::Root(WorkspaceRoot::Worktree {
            worktree: id
        }))
    );
}

#[test]
fn node_lookup_exposes_its_exact_metadata_source_and_refuses_equal_revision_conflict() {
    let mut epochs = ConnectionEpochs::default();
    let node = NodeId::random();
    let first = context(&mut epochs, "first", node);
    let second = context(&mut epochs, "second", node);
    let original = data(node);
    let mut newer = original.clone();
    newer.registry.revision = RegistryRevision::from(1);
    let mut cache = RegistryProjection::default();
    let key = cache.begin(first.clone()).unwrap();
    assert!(cache.finish(&key, Ok(original)).unwrap());
    let key = cache.begin(second.clone()).unwrap();
    assert!(cache.finish(&key, Ok(newer.clone())).unwrap());
    assert_eq!(cache.read_context(node), Ok(&second));
    let mut conflict = newer;
    conflict.registry.workspaces[0].title = Some("different".parse().unwrap());
    let key = cache.begin(first.clone()).unwrap();
    assert!(cache.finish(&key, Ok(conflict)).unwrap());
    assert_eq!(cache.read_context(node), Err(LookupProblem::Unavailable));
    assert_eq!(
        cache.session_workspace(node, SessionId::random()),
        Err(LookupProblem::Unavailable)
    );
}

#[test]
fn pending_read_keeps_its_ticket_and_first_hydration_does_not_need_prior_data() {
    let mut epochs = ConnectionEpochs::default();
    let node = NodeId::random();
    let context = context(&mut epochs, "fixture", node);
    let snapshot = data(node);
    let mut cache = RegistryProjection::default();
    let key = cache.begin(context.clone()).unwrap();
    assert_eq!(cache.phase(&context), Some(RegistryPhase::Loading));
    assert_eq!(
        cache.begin(context.clone()),
        Err(ProjectionRefusal::Pending)
    );
    assert!(cache.finish(&key, Ok(snapshot)).unwrap());
    assert_eq!(cache.phase(&context), Some(RegistryPhase::Ready));
    assert!(cache.rows(&context).unwrap().next().is_some());
    assert!(
        !cache
            .finish(&key, Err(RegistryReadFailure::Offline))
            .unwrap()
    );
    assert_eq!(cache.phase(&context), Some(RegistryPhase::Ready));
}

#[test]
fn remote_windows_path_is_metadata_not_the_ui_platforms_absolute_path() {
    let mut epochs = ConnectionEpochs::default();
    let node = NodeId::random();
    let context = context(&mut epochs, "windows-node", node);
    let mut snapshot = data(node);
    let remote_root = std::path::PathBuf::from(r"C:\work\project");
    snapshot.projects.projects[0].root = remote_root.clone();
    snapshot.registry.workspaces[0].path = remote_root;
    let workspace = snapshot.registry.workspaces[0].id;
    let project = snapshot.projects.projects[0].id;
    let mut cache = RegistryProjection::default();
    let key = cache.begin(context).unwrap();
    assert!(cache.finish(&key, Ok(snapshot)).unwrap());
    assert_eq!(
        cache.workspace_location(node, workspace),
        Ok(WorkspaceLocation::Root(WorkspaceRoot::Project { project }))
    );
}
