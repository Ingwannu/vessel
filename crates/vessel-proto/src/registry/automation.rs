//! Cross-repository automation is released explicitly for one retained source/revision.
//! Releasing does not approve command digests, execute setup, or replay any work.

use super::RegistryRevision;
use crate::{
    WorkspaceId,
    forge::{PullNumber, RepoRef},
};
use serde::{Deserialize, Serialize};

/// Exact external source that caused automatic setup/naming to be withheld.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BlockedSource {
    /// A change request whose checkout may contain untrusted automation.
    ChangeRequest {
        /// Forge repository includes its validated host/kind/path, not a free URL.
        head_repository: RepoRef,
        /// Stable request number, positive and exactly representable on the wire.
        number: PullNumber,
    },
}

/// Applies to automatic setup, named services and metadata work, not explicit user shells.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceAutomation {
    /// Proven ordinary placement or an explicitly released source; digest checks still apply.
    Allowed,
    /// Source is retained durably until the person releases this exact version.
    Blocked {
        /// Render the actual source; never derive it from cwd, branch or approval digest.
        source: BlockedSource,
    },
}

/// Compare source and descriptor revision before releasing this workspace's automation gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReleaseWorkspaceAutomation {
    /// Exact lifecycle owner on this node.
    pub workspace: WorkspaceId,
    /// A changed source or metadata revision requires reinspection and another explicit choice.
    pub expected: RegistryRevision,
    /// Must equal the retained blocked source, not a generic approve-all flag.
    pub source: BlockedSource,
}
