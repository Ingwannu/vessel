//! One workspace-relative metadata policy for store and native projections.
use crate::{ProtocolVersion, SessionInfo, SessionRef};
use serde::{Deserialize, Serialize};

/// A workspace-local presentation role, never operation or connection authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WorkspaceRole {
    /// No current parent, or an attached session in a known different workspace.
    Root,
    /// Unsupported or incomplete relationship/ownership observations.
    #[default]
    Unknown,
    /// An attached session with the same known owning workspace as its parent.
    Child,
}

/// Confidence in current parentage, distinct from immutable creation provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Relation {
    /// A capable authoritative lookup resolved the current parent, including detach.
    Known(Option<SessionRef>),
    /// Unsupported, missing, stale or conflicting current-parent information.
    #[default]
    Unknown,
}

impl Relation {
    /// Join current parentage with authoritative owning-workspace observations.
    ///
    /// `W` is the caller's canonical node-scoped key, normally
    /// `(NodeId, WorkspaceId)`, not a cwd, alias or workspace UUID alone.
    /// Resolve parent ownership by the complete parent `SessionRef`. Missing
    /// ownership remains unknown; this does not prove membership or freshness.
    #[must_use]
    pub fn workspace_role<W: Eq>(
        self,
        child_workspace: Option<&W>,
        parent_workspace: Option<&W>,
    ) -> WorkspaceRole {
        match (self, child_workspace, parent_workspace) {
            (Self::Known(None), _, _) => WorkspaceRole::Root,
            (Self::Known(Some(_)), Some(child), Some(parent)) if child != parent => {
                WorkspaceRole::Root
            }
            (Self::Known(Some(_)), Some(_), Some(_)) => WorkspaceRole::Child,
            (Self::Known(Some(_)) | Self::Unknown, _, _) => WorkspaceRole::Unknown,
        }
    }

    /// Derive confidence from validated session metadata and its exact source peer.
    ///
    /// A missing/unassigned capability is not evidence of a root. The caller
    /// owns authentication, snapshot freshness and same-writer effect admission;
    /// the protocol version is observation metadata, not an authorization grant.
    #[must_use]
    pub fn for_session(info: &SessionInfo, peer: ProtocolVersion) -> Self {
        if super::SUBAGENT_RELATIONS.is_none_or(|since| since > peer) {
            return Self::Unknown;
        }
        info.parentage()
            .map_or(Self::Unknown, |parentage| Self::Known(parentage.parent()))
    }
}

#[cfg(test)]
mod tests;
