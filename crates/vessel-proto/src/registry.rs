//! Independent project/workspace registry contracts (ADR 0082).
//! No IO or inferred cwd identity. The six metadata calls are protocol 25;
//! the other values here have no method row and remain unavailable.

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

/// Protocol 25 (2026-11-15): the six metadata methods only, `workspace.registry`,
/// `.list`, `.get`, `.rename`, `.pin` and `.release_automation` (Read for the
/// first three, Operate for the rest, all unary and fleet-forwardable). Their
/// effects are complete: reads, a title or pin compare-and-set, and clearing
/// the retained blocked-source gate. The earlier threshold, every lifecycle,
/// receipt and UI/CLI effect validated before any call ships, is superseded
/// for these six alone. Filesystem provisioning, generation, resource start
/// and exit, archive, restore and deletion, config IO, editor launch and D2
/// effects stay out: `CreateWorkspace`, `ArchiveWorkspace`, `RestoreWorkspace`
/// and the config and directory values have no row and stay unavailable.
/// Assigned in merge order at the node line; it was None while reviewed.
pub const PROJECT_WORKSPACE_REGISTRY: Option<ProtocolVersion> = Some(ProtocolVersion::new(25));

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
