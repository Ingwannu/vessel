//! Real Store adapter and actor publication, with isolated data and no process/RPC.

use super::Labels;
use chrono::DateTime;
use vessel_proto::{
    ProjectId, WorkspaceId,
    registry::{
        RegistryRevision, WorkspaceAutomation, WorkspaceBacking, WorkspaceDescriptor,
        WorkspaceLifecycle, WorkspacePin, WorkspaceVisibility,
    },
    workspace_labels::{
        AssignWorkspaceLabel, Assignment, CatalogMode, Label, LabelColor, ListWorkspaceLabels,
        UpdateWorkspaceLabel,
    },
};
use vessel_store::{DataDir, RegistryRecord, Store, StoreConfig};

fn record(workspace: WorkspaceId) -> RegistryRecord {
    RegistryRecord {
        descriptor: WorkspaceDescriptor {
            id: workspace,
            project: ProjectId::random(),
            path: "/fixture/labels".into(),
            title: None,
            automation: WorkspaceAutomation::Allowed,
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

#[tokio::test]
async fn workspace_labels_real_backend_reopens_committed_catalog_with_new_generation() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = DataDir::new(temporary.path());
    directory.prepare().unwrap();
    let store = Store::open(&directory, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let workspace = WorkspaceId::random();
    store
        .put_registry_workspace(record(workspace), None)
        .await
        .unwrap();
    let labels = Labels::open(store.clone()).await.unwrap();
    let initial = labels.list(ListWorkspaceLabels::default()).await.unwrap();
    let _assigned = labels
        .assign(AssignWorkspaceLabel {
            workspace,
            label: Label {
                name: "Ready".parse().unwrap(),
                color: LabelColor::Violet,
            },
            assignment: Assignment::Assigned,
        })
        .await
        .unwrap();
    let _renamed = labels
        .update(UpdateWorkspaceLabel {
            name: "Ready".parse().unwrap(),
            new_name: Some("Done".parse().unwrap()),
            color: Some(LabelColor::Sky),
        })
        .await
        .unwrap();
    let changes = labels
        .list(ListWorkspaceLabels {
            cursor: Some(initial.head),
            ..ListWorkspaceLabels::default()
        })
        .await
        .unwrap();
    assert!(matches!(changes.data, CatalogMode::Changes { updates, .. } if updates.len() == 2));
    let snapshot = store.workspace_registry().await.unwrap();
    assert_eq!(snapshot.labels.first().unwrap().workspace, workspace);
    assert_eq!(
        snapshot.labels.first().unwrap().labels,
        ["Done".parse().unwrap()]
    );
    labels.close().await.unwrap();
    drop(labels);
    store.close().await;
    drop(store);
    let store = Store::open(&directory, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let labels = Labels::open(store.clone()).await.unwrap();
    let reopened = labels.list(ListWorkspaceLabels::default()).await.unwrap();
    assert_ne!(reopened.head.generation, initial.head.generation);
    assert_eq!(
        reopened.data,
        CatalogMode::Snapshot {
            labels: vec![Label {
                name: "Done".parse().unwrap(),
                color: LabelColor::Sky,
            }]
        }
    );
    labels.close().await.unwrap();
    store.close().await;
}
