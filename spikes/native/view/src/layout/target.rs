//! Panel manifests and resource identity. Renderer choice is panel-owned state.
use super::{DraftId, ResourceId, WorkspaceId};
use serde::{Deserialize, Serialize};
use vessel_proto::plugins::{ContributionAddress, PluginPanelContext};
use vessel_proto::workspace::{CommitId, RelPath, WorkspaceRoot};
use vessel_proto::{SessionId, TerminalId};

/// Host constraints are independent of window width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaneHost {
    /// Ordinary tabbed pane.
    Main,
    /// Retained Explorer dock.
    Explorer,
    /// Ephemeral compact overlay.
    Sheet,
}
/// Typed resource handed from a feature to the shared pane host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    /// A fresh launcher always receives a distinct tab id.
    NewTab,
    /// Unsent agent configuration remains with the composer owner.
    Draft(DraftId),
    /// Root or child role is resolved from hydrated ownership facts.
    Agent(SessionId),
    /// Provider-level children have their own identity within a session.
    ProviderSubagent {
        /// Owning parent.
        parent: SessionId,
        /// Provider-minted resource.
        child: ResourceId,
    },
    /// A standalone shell; closing requires the terminal owner.
    Terminal(TerminalId),
    /// Desktop browser resource.
    Browser(ResourceId),
    /// Singleton Explorer changes list.
    Changes,
    /// Singleton Explorer file tree.
    Files,
    /// Workspace script/service menu hosted by the native panel owner.
    Scripts,
    /// Workspace's pull request.
    PullRequest,
    /// Root prevents two checkouts' equal paths sharing editor buffers.
    File {
        /// Server-validated filesystem root.
        root: WorkspaceRoot,
        /// Normalized, validated path inside that root.
        path: RelPath,
    },
    /// One working-diff panel; focused path belongs to its own state.
    WorkingDiff,
    /// A registered plugin contribution.
    Plugin {
        /// Exact node, installed runtime generation and contribution namespace.
        address: ContributionAddress,
        /// Workspace/agent/file context participates in equality.
        context: PluginPanelContext,
    },
    /// Closing does not cancel workspace setup.
    Setup(WorkspaceId),
    /// Ephemeral because rebasing may remove this commit.
    CommitDiff(CommitId),
}
impl Target {
    /// Whether a target may be placed in this host (Paseo panel manifests).
    #[must_use]
    pub const fn supports(&self, host: PaneHost) -> bool {
        match self {
            Self::Changes | Self::Files => matches!(host, PaneHost::Explorer),
            Self::Browser(_) | Self::Setup(_) => matches!(host, PaneHost::Main),
            Self::File { .. } | Self::ProviderSubagent { .. } | Self::Agent(_) => true,
            Self::NewTab
            | Self::Draft(_)
            | Self::Terminal(_)
            | Self::Scripts
            | Self::PullRequest
            | Self::WorkingDiff
            | Self::Plugin { .. }
            | Self::CommitDiff(_) => !matches!(host, PaneHost::Sheet),
        }
    }
    /// New launchers are instances, all other targets reveal equal resources.
    #[must_use]
    pub fn same_resource(&self, other: &Self) -> bool {
        !matches!(self, Self::NewTab) && self == other
    }
    /// Commit diffs and overlays must not resurrect stale resources on restart.
    #[must_use]
    pub const fn persistent(&self) -> bool {
        !matches!(self, Self::CommitDiff(_))
    }
}
