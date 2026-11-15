//! Independent project/workspace registry contracts (ADR 0082).
//! No IO or inferred cwd identity; calls remain unavailable until integration.

mod automation;
mod config;
mod project;
mod snapshot;
mod workspace;

pub use automation::{BlockedSource, ReleaseWorkspaceAutomation, WorkspaceAutomation};
pub use config::{
    ConfigDocument, ConfigRevision, ConfigText, ConfigTextError, ConfigValidity, MAX_CONFIG_BYTES,
    ReadProjectConfig, WriteProjectConfig,
};
pub use project::{
    CreateProjectDirectory, DirectoryName, DirectoryNameError, ProjectMetadata,
    ReadProjectMetadata, RenameProject,
};
pub use workspace::{
    ArchiveWorkspace, ArchiveWorkspaces, CreateWorkspace, GetWorkspace, ListWorkspaces,
    PinWorkspace, RegistryRevision, RenameWorkspace, RestoreWorkspace, WorkspaceBacking,
    WorkspaceDescriptor, WorkspaceIsolation, WorkspaceLifecycle, WorkspaceList, WorkspacePin,
    WorkspaceVisibility, WorkspaceVisibilityFilter,
};

pub use snapshot::{GetRegistry, RegistrySnapshot, WorkspaceMembership, WorkspaceResource};

use crate::ProtocolVersion;

/// Assign only after the node lifecycle, durable receipts and all UI/CLI
/// consumers validate workspace ownership and partial cleanup outcomes.
/// None refuses registry calls before dispatch, including forwarded calls.
pub const PROJECT_WORKSPACE_REGISTRY: Option<ProtocolVersion> = None;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Method, Request as _, WorkspaceId};
    #[test]
    fn registry_methods_share_one_feature_gate_and_preserve_typed_response_targets() {
        let requests = [
            ReleaseWorkspaceAutomation {
                workspace: WorkspaceId::random(),
                expected: RegistryRevision::ZERO,
                source: BlockedSource::ChangeRequest {
                    number: crate::forge::PullNumber::new(1).unwrap(),
                    head_repository: crate::forge::RepoRef {
                        kind: crate::forge::ForgeKind::Github,
                        host: "github.com".parse().unwrap(),
                        path: "fork/project".parse().unwrap(),
                    },
                },
            }
            .into_call(),
            GetRegistry::default().into_call(),
            ListWorkspaces::default().into_call(),
            GetWorkspace {
                workspace: WorkspaceId::random(),
            }
            .into_call(),
            RenameWorkspace {
                workspace: WorkspaceId::random(),
                title: None,
            }
            .into_call(),
            PinWorkspace {
                workspace: WorkspaceId::random(),
                pin: WorkspacePin::Unpinned,
            }
            .into_call(),
        ];
        for call in requests {
            assert_eq!(call.required_protocol(), PROJECT_WORKSPACE_REGISTRY);
            assert_eq!(call.method().since(), PROJECT_WORKSPACE_REGISTRY);
            assert_eq!(call.method().shape(), crate::MethodShape::Unary);
        }
        assert_eq!(GetWorkspace::METHOD, Method::GetWorkspace);
    }
}
