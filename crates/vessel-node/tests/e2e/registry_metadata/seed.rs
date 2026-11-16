//! Offline metadata fixtures carry no filesystem, process or backing attestation.
use chrono::DateTime;
use vessel_proto::registry::{
    BlockedSource, RegistryRevision, WorkspaceAutomation, WorkspaceBacking, WorkspaceDescriptor,
    WorkspaceLifecycle, WorkspacePin, WorkspaceVisibility,
};
use vessel_proto::{ProjectId, WorkspaceId};
use vessel_store::{RegistryRecord, Store, StoreConfig};

use crate::support::{Fixture, step};
use futures_util::FutureExt as _;
use std::panic::{AssertUnwindSafe, resume_unwind};

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
    let opened = step(
        "registry offline Store open/migrations",
        Store::open(&fixture.data_dir(), StoreConfig::default()),
    )
    .await
    .unwrap();
    let result = AssertUnwindSafe(async {
        for descriptor in descriptors {
            step(
                "registry offline seed commit ACK",
                opened.store.put_registry_workspace(
                    RegistryRecord {
                        descriptor: descriptor.clone(),
                        recovery: None,
                    },
                    None,
                ),
            )
            .await
            .unwrap();
        }
    })
    .catch_unwind()
    .await;
    step("registry offline seed writer close", opened.store.close()).await;
    if let Err(panic) = result {
        resume_unwind(panic);
    }
}

pub(super) fn requests(workspace: WorkspaceId) -> [vessel_proto::Call; 6] {
    use vessel_proto::Request as _;
    use vessel_proto::registry::{
        GetRegistry, GetWorkspace, ListWorkspaces, PinWorkspace, ReleaseWorkspaceAutomation,
        RenameWorkspace,
    };
    [
        GetRegistry::default().into_call(),
        ListWorkspaces::default().into_call(),
        GetWorkspace { workspace }.into_call(),
        RenameWorkspace {
            workspace,
            title: None,
        }
        .into_call(),
        PinWorkspace {
            workspace,
            pin: WorkspacePin::Pinned,
        }
        .into_call(),
        ReleaseWorkspaceAutomation {
            workspace,
            expected: RegistryRevision::ZERO,
            source: source(),
        }
        .into_call(),
    ]
}
