//! Real Store/registry/writer integration, isolated data directory, no processes.
use chrono::DateTime;
use tempfile::TempDir;
use vessel_proto::{
    ProjectId, WorkspaceId,
    registry::{
        RegistryRevision, WorkspaceAutomation, WorkspaceBacking, WorkspaceDescriptor,
        WorkspaceLifecycle, WorkspacePin, WorkspaceVisibility,
    },
    workspace_labels::{
        AssignWorkspaceLabel, Assignment, DeleteWorkspaceLabel, InspectWorkspaceLabelDelete, Label,
        LabelColor, LabelName, LabelNameError, UpdateWorkspaceLabel, WorkspaceLabelFailure,
    },
};
use vessel_store::{DataDir, RegistryRecord, Store, StoreConfig, StoreError, StoredRegistry};

#[derive(Debug, thiserror::Error)]
enum FixtureError {
    #[error("temporary label store directory could not be created")]
    Temporary(#[from] std::io::Error),
    #[error(transparent)]
    Store(#[from] StoreError),
}

async fn opened() -> Result<(TempDir, DataDir, Store), FixtureError> {
    let temporary = tempfile::tempdir()?;
    let dir = DataDir::new(temporary.path());
    dir.prepare()?;
    let store = Store::open(&dir, StoreConfig::default()).await?.store;
    Ok((temporary, dir, store))
}
fn record() -> RegistryRecord {
    RegistryRecord {
        descriptor: WorkspaceDescriptor {
            id: WorkspaceId::random(),
            project: ProjectId::random(),
            path: "/work/labels-fixture".into(),
            automation: WorkspaceAutomation::Allowed,
            title: None,
            pin: WorkspacePin::Unpinned,
            backing: WorkspaceBacking::Local,
            lifecycle: WorkspaceLifecycle::Active,
            visibility: WorkspaceVisibility::Foreground,
            created_at: DateTime::UNIX_EPOCH,
            revision: RegistryRevision::ZERO,
        },
        recovery: None,
    }
}
fn assign(workspace: WorkspaceId, name: &str) -> Result<AssignWorkspaceLabel, LabelNameError> {
    Ok(AssignWorkspaceLabel {
        workspace,
        label: Label {
            name: name.parse()?,
            color: LabelColor::Violet,
        },
        assignment: Assignment::Assigned,
    })
}
fn names(snapshot: &StoredRegistry, workspace: WorkspaceId) -> Vec<&str> {
    snapshot
        .labels
        .iter()
        .find(|group| group.workspace == workspace)
        .map_or_else(Vec::new, |group| {
            group.labels.iter().map(LabelName::as_str).collect()
        })
}

#[tokio::test]
async fn workspace_labels_real_registry_refuses_missing_and_each_inactive_state_atomically() {
    let (_temporary, _dir, store) = opened().await.unwrap();
    assert!(matches!(
        store
            .assign_workspace_label(assign(WorkspaceId::random(), "Ready").unwrap())
            .await,
        Err(StoreError::WorkspaceLabelRefused(
            WorkspaceLabelFailure::WorkspaceNotFound
        ))
    ));
    for lifecycle in [
        WorkspaceLifecycle::Archiving {
            at: DateTime::UNIX_EPOCH,
        },
        WorkspaceLifecycle::Restoring {
            at: DateTime::UNIX_EPOCH,
        },
        WorkspaceLifecycle::Archived {
            at: DateTime::UNIX_EPOCH,
        },
    ] {
        let mut record = record();
        let id = record.descriptor.id;
        store
            .put_registry_workspace(record.clone(), None)
            .await
            .unwrap();
        record.descriptor.lifecycle = lifecycle;
        record.descriptor.revision = RegistryRevision::ZERO.next().unwrap();
        store
            .put_registry_workspace(record, Some(RegistryRevision::ZERO))
            .await
            .unwrap();
        let before = store.workspace_registry().await.unwrap();
        let changes = store.registry_changes();
        assert!(matches!(
            store
                .assign_workspace_label(assign(id, "Ready").unwrap())
                .await,
            Err(StoreError::WorkspaceLabelRefused(
                WorkspaceLabelFailure::WorkspaceInactive
            ))
        ));
        assert!(!changes.has_changed().unwrap());
        assert_eq!(store.workspace_registry().await.unwrap(), before);
        assert!(store.workspace_label_catalog().await.unwrap().is_empty());
    }
    store.close().await;
}

#[tokio::test]
async fn workspace_labels_watch_silence_and_collision_rollback_use_the_real_writer() {
    let (_temporary, _dir, store) = opened().await.unwrap();
    let record = record();
    let id = record.descriptor.id;
    store.put_registry_workspace(record, None).await.unwrap();
    let mut changes = store.registry_changes();
    let committed = store
        .assign_workspace_label(assign(id, "Ready").unwrap())
        .await
        .unwrap();
    assert!(committed.delta.is_some());
    assert!(changes.has_changed().unwrap());
    changes.mark_unchanged();
    let before = store.workspace_registry().await.unwrap();
    assert!(
        store
            .assign_workspace_label(assign(id, " ready ").unwrap())
            .await
            .unwrap()
            .delta
            .is_none()
    );
    assert!(!changes.has_changed().unwrap());
    assert_eq!(store.workspace_registry().await.unwrap(), before);
    let _recolor = store
        .update_workspace_label(UpdateWorkspaceLabel {
            name: "Ready".parse().unwrap(),
            new_name: None,
            color: Some(LabelColor::Blue),
        })
        .await
        .unwrap();
    assert!(!changes.has_changed().unwrap());
    assert_eq!(
        store.workspace_registry().await.unwrap().revision,
        before.revision
    );
    let _other = store
        .assign_workspace_label(assign(id, "Other").unwrap())
        .await
        .unwrap();
    changes.mark_unchanged();
    let before = store.workspace_registry().await.unwrap();
    assert!(matches!(
        store
            .update_workspace_label(UpdateWorkspaceLabel {
                name: "Ready".parse().unwrap(),
                new_name: Some("Other".parse().unwrap()),
                color: Some(LabelColor::Red)
            })
            .await,
        Err(StoreError::WorkspaceLabelRefused(
            WorkspaceLabelFailure::NameTaken
        ))
    ));
    assert!(!changes.has_changed().unwrap());
    assert_eq!(store.workspace_registry().await.unwrap(), before);
    assert_eq!(
        store
            .workspace_label_catalog()
            .await
            .unwrap()
            .into_iter()
            .find(|label| label.name.as_str() == "Ready")
            .unwrap()
            .color,
        LabelColor::Blue
    );
    store.close().await;
}

#[tokio::test]
async fn workspace_labels_failed_real_revision_bump_rolls_back_both_tables_and_wake() {
    let (_temporary, directory, store) = opened().await.unwrap();
    let record = record();
    let workspace = record.descriptor.id;
    store.put_registry_workspace(record, None).await.unwrap();
    let original = store.workspace_registry().await.unwrap().revision;
    let connection = rusqlite::Connection::open(directory.database_path()).unwrap();
    let _changed = connection
        .execute(
            "UPDATE workspace_registry_meta SET revision=?1 WHERE singleton=1",
            [i64::MAX],
        )
        .unwrap();
    let before = store.workspace_registry().await.unwrap();
    let changes = store.registry_changes();
    assert!(matches!(
        store
            .assign_workspace_label(assign(workspace, "Ready").unwrap())
            .await,
        Err(StoreError::RegistryRevisionExhausted)
    ));
    assert!(store.workspace_label_catalog().await.unwrap().is_empty());
    assert_eq!(store.workspace_registry().await.unwrap(), before);
    assert!(!changes.has_changed().unwrap());
    let _reset = connection
        .execute(
            "UPDATE workspace_registry_meta SET revision=?1 WHERE singleton=1",
            [i64::try_from(original.get()).unwrap()],
        )
        .unwrap();
    drop(connection);
    let committed = store
        .assign_workspace_label(assign(workspace, "Ready").unwrap())
        .await
        .unwrap();
    assert!(committed.delta.is_some());
    assert!(changes.has_changed().unwrap());
    assert_eq!(
        names(&store.workspace_registry().await.unwrap(), workspace),
        ["Ready"]
    );
    store.close().await;
}

#[tokio::test]
async fn workspace_labels_rename_delete_order_and_snapshot_survive_real_reopen() {
    let (_temporary, dir, store) = opened().await.unwrap();
    let first = record();
    let second = record();
    let (a, b) = (first.descriptor.id, second.descriptor.id);
    store
        .put_registry_workspace(first.clone(), None)
        .await
        .unwrap();
    store
        .put_registry_workspace(second.clone(), None)
        .await
        .unwrap();
    for (id, name) in [(a, "Ready"), (a, "Next"), (b, "Ready")] {
        let _commit = store
            .assign_workspace_label(assign(id, name).unwrap())
            .await
            .unwrap();
    }
    let mut changes = store.registry_changes();
    let renamed = store
        .update_workspace_label(UpdateWorkspaceLabel {
            name: "Ready".parse().unwrap(),
            new_name: Some("Done".parse().unwrap()),
            color: None,
        })
        .await
        .unwrap();
    assert_eq!(renamed.result.affected.get(), 2);
    assert!(changes.has_changed().unwrap());
    let snapshot = store.workspace_registry().await.unwrap();
    assert_eq!(names(&snapshot, a), ["Done", "Next"]);
    assert_eq!(names(&snapshot, b), ["Done"]);
    assert_eq!(
        store.registry_workspace(a).await.unwrap(),
        Some(first.clone())
    );
    let count = store
        .inspect_workspace_label(InspectWorkspaceLabelDelete {
            name: "Done".parse().unwrap(),
        })
        .await
        .unwrap();
    assert_eq!(count.affected.get(), 2);
    changes.mark_unchanged();
    let deleted = store
        .delete_workspace_label(DeleteWorkspaceLabel {
            name: "Done".parse().unwrap(),
        })
        .await
        .unwrap();
    assert_eq!(deleted.result.affected.get(), 2);
    assert!(changes.has_changed().unwrap());
    let snapshot = store.workspace_registry().await.unwrap();
    assert_eq!(names(&snapshot, a), ["Next"]);
    assert!(names(&snapshot, b).is_empty());
    store.close().await;
    drop(store);
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let snapshot = store.workspace_registry().await.unwrap();
    assert_eq!(names(&snapshot, a), ["Next"]);
    assert!(names(&snapshot, b).is_empty());
    assert_eq!(store.registry_workspace(a).await.unwrap(), Some(first));
    assert_eq!(store.registry_workspace(b).await.unwrap(), Some(second));
    store.close().await;
}
