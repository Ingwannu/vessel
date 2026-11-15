//! Offline metadata fixtures carry no filesystem, process or backing attestation.
use chrono::DateTime;
use vessel_proto::registry::{
    BlockedSource, RegistryRevision, WorkspaceAutomation, WorkspaceBacking, WorkspaceDescriptor,
    WorkspaceLifecycle, WorkspacePin, WorkspaceVisibility,
};
use vessel_proto::{ProjectId, WorkspaceId};
use vessel_store::{RegistryRecord, Store, StoreConfig};

use crate::support::Fixture;

pub(super) fn source() -> BlockedSource {
    BlockedSource::ChangeRequest {
        number: vessel_proto::forge::PullNumber::new(1).unwrap(),
        head_repository: vessel_proto::forge::RepoRef {
            kind: vessel_proto::forge::ForgeKind::Github,
            host: "github.com".parse().unwrap(),
            path: "fork/project".parse().unwrap(),
        },
    }
}

pub(super) fn descriptor(project: ProjectId) -> WorkspaceDescriptor {
    WorkspaceDescriptor {
        id: WorkspaceId::random(),
        project,
        path: "metadata-only/nonexistent".into(),
        automation: WorkspaceAutomation::Allowed,
        title: None,
        pin: WorkspacePin::Unpinned,
        backing: WorkspaceBacking::Local,
        lifecycle: WorkspaceLifecycle::Active,
        visibility: WorkspaceVisibility::Foreground,
        created_at: DateTime::UNIX_EPOCH,
        revision: RegistryRevision::ZERO,
    }
}

pub(super) async fn seed(fixture: &Fixture, descriptors: &[WorkspaceDescriptor]) {
    let opened = Store::open(&fixture.data_dir(), StoreConfig::default())
        .await
        .unwrap();
    for descriptor in descriptors {
        opened
            .store
            .put_registry_workspace(
                RegistryRecord {
                    descriptor: descriptor.clone(),
                    recovery: None,
                },
                None,
            )
            .await
            .unwrap();
    }
    opened.store.close().await;
}
