//! Runtime registration (ADR 0106): one workspace per project root, archived by
//! legacy removal, outside the live cap, with members derived from placement.
use super::*;
use rusqlite::params;
use vessel_proto::registry::WorkspaceMembership;

fn root(project: ProjectId) -> Register {
    Register {
        placement: Placement {
            project,
            path: "/work/project".into(),
        },
        fresh: WorkspaceId::random(),
        at: DateTime::UNIX_EPOCH,
    }
}

fn registered(db: &mut Connection, request: &Register) -> WorkspaceId {
    let transaction = db.transaction().unwrap();
    let (id, _change) = register(&transaction, request).unwrap();
    transaction.commit().unwrap();
    id
}

/// A stored session placed in `project`, created at `at` nanoseconds.
fn placed(db: &Connection, project: ProjectId, at: i64, archived: bool) -> SessionId {
    let session = SessionId::random();
    let spec = serde_json::json!({ "place": { "kind": "project", "project": project } });
    let _inserted = db
        .execute(
            "INSERT INTO sessions (id,spec,status,live,created_at,next_seq,first_seq,retained_bytes,archived_at) \
             VALUES (?1,?2,'{}',0,?3,0,0,0,?4)",
            params![session.to_string(), spec.to_string(), at, archived.then_some(at)],
        )
        .unwrap();
    session
}

/// Legacy removal of `project` at `at` nanoseconds.
fn archive(db: &mut Connection, project: ProjectId, at: i64) -> RegistryChange {
    let transaction = db.transaction().unwrap();
    let change = archive_stranded(&transaction, project, DateTime::from_timestamp_nanos(at)).unwrap();
    transaction.commit().unwrap();
    change
}

#[test]
fn a_root_finds_its_oldest_active_workspace_and_an_archived_one_is_not_restored() {
    let mut db = db();
    let project = ProjectId::random();
    let first = root(project);
    assert_eq!(registered(&mut db, &first), first.fresh);
    // A second start at the same root, with another fresh id, finds the first.
    let again = root(project);
    assert_eq!(registered(&mut db, &again), first.fresh);
    assert_eq!(load(&db).unwrap().workspaces.len(), 1);
    assert_eq!(archive(&mut db, project, 1), RegistryChange::Changed);
    // Archived is kept and not restored; the next start creates a new one.
    let after = root(project);
    assert_eq!(registered(&mut db, &after), after.fresh);
    let stored = load(&db).unwrap().workspaces;
    assert_eq!(stored.len(), 2);
    assert!(stored.iter().any(|item| item.id == first.fresh
        && matches!(item.lifecycle, WorkspaceLifecycle::Archived { .. })));
}

#[test]
fn removal_archives_only_local_active_workspaces_and_bumps_the_revision_once() {
    let mut db = db();
    let local = record(WorkspaceId::random());
    let project = local.descriptor.project;
    let mut managed = record(WorkspaceId::random());
    managed.descriptor.project = project;
    managed.descriptor.backing = WorkspaceBacking::Managed {
        worktree: vessel_proto::WorktreeId::random(),
    };
    let other = record(WorkspaceId::random());
    for item in [&local, &managed, &other] {
        put(&mut db, item.clone(), None).unwrap();
    }
    let before = load(&db).unwrap().revision;
    assert_eq!(archive(&mut db, project, 1), RegistryChange::Changed);
    let after = load(&db).unwrap();
    assert_eq!(after.revision, before.next().unwrap());
    let lifecycle = |id| {
        after
            .workspaces
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .lifecycle
            .clone()
    };
    assert!(matches!(
        lifecycle(local.descriptor.id),
        WorkspaceLifecycle::Archived { .. }
    ));
    assert_eq!(lifecycle(managed.descriptor.id), WorkspaceLifecycle::Active);
    assert_eq!(lifecycle(other.descriptor.id), WorkspaceLifecycle::Active);
    // Nothing left to archive: no write, no revision.
    assert_eq!(archive(&mut db, project, 2), RegistryChange::Unchanged);
    assert_eq!(load(&db).unwrap().revision, after.revision);
}

#[test]
fn archived_records_never_refuse_a_live_create_and_are_pruned_oldest_first() {
    let mut db = db();
    let mut oldest = None;
    for at in 0..MAX_WORKSPACES {
        let project = ProjectId::random();
        let id = registered(&mut db, &root(project));
        oldest.get_or_insert(id);
        let _change = archive(&mut db, project, i64::try_from(at).unwrap());
    }
    // The live cap counts no archived record.
    let live = root(ProjectId::random());
    assert_eq!(registered(&mut db, &live), live.fresh);
    // One archive past the bound prunes the oldest archived record.
    let last = i64::try_from(MAX_WORKSPACES).unwrap();
    assert_eq!(
        archive(&mut db, live.placement.project, last),
        RegistryChange::Changed
    );
    let stored = load(&db).unwrap().workspaces;
    assert_eq!(stored.len(), MAX_ARCHIVED_WORKSPACES);
    assert!(!stored.iter().any(|item| Some(item.id) == oldest));
    assert!(stored.iter().any(|item| item.id == live.fresh));
}

#[test]
fn placed_sessions_are_derived_members_of_the_oldest_active_root_workspace() {
    let mut db = db();
    let project = ProjectId::random();
    let open = placed(&db, project, 2, false);
    let _archived = placed(&db, project, 1, true);
    let _elsewhere = placed(&db, ProjectId::random(), 3, false);
    let workspace = registered(&mut db, &root(project));
    let mut newer = record(WorkspaceId::random());
    newer.descriptor.project = project;
    newer.descriptor.created_at = DateTime::from_timestamp_nanos(1);
    put(&mut db, newer, None).unwrap();
    let snapshot = load(&db).unwrap();
    assert_eq!(
        snapshot.memberships,
        vec![WorkspaceMembership {
            workspace,
            resource: WorkspaceResource::Session { session: open },
        }]
    );
    // Once removal archives the project's workspaces, nothing is derived.
    assert_eq!(archive(&mut db, project, 5), RegistryChange::Changed);
    assert!(load(&db).unwrap().memberships.is_empty());
}

#[test]
fn bootstrap_lists_each_unregistered_project_once_at_its_earliest_session() {
    let mut db = db();
    let project = ProjectId::random();
    let _late = placed(&db, project, 9, false);
    let _early = placed(&db, project, 4, false);
    let _gone = placed(&db, ProjectId::random(), 1, true);
    assert_eq!(
        unregistered_placed(&db).unwrap(),
        vec![(project, DateTime::from_timestamp_nanos(4))]
    );
    let _id = registered(&mut db, &root(project));
    assert!(unregistered_placed(&db).unwrap().is_empty());
    // A project whose workspace removal archived stays registered: one shot.
    let _change = archive(&mut db, project, 10);
    assert!(unregistered_placed(&db).unwrap().is_empty());
}
