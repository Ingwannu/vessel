//! Feature producers seed a draft without opening it or replacing another payload.
//!
//! Destination identity is captured separately from its mutable book alias.
//! A selected project root is presentation context supplied by that registry;
//! the node still resolves membership/provisioning and classifies automation.
//! No cwd or source session is promoted into a workspace identity here.

use super::Launch;
use crate::{
    composer::context::{ChatHistory, ContextText},
    machine_book::Name,
};
use std::{fmt, path::PathBuf};
use vessel_proto::{NodeId, ProjectId, WorkspaceId};

/// A verified destination when the initiating feature accepted its action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftDestination {
    /// Routing alias, rechecked against the stable node before effects.
    pub machine: Name,
    /// Welcome identity, never a project path or tab identity.
    pub node: NodeId,
}

/// Project context preserves the selected registered root, including subfolders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSeed {
    /// Existing project registry identity.
    pub project: ProjectId,
    /// Exact remote root; never normalize it using the client's operating system.
    pub root: PathBuf,
}

/// New workspace versus a new agent in an existing workspace is explicit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedPlace {
    /// A host-only entry may ask for a directory; a project preserves its root.
    NewWorkspace(Option<ProjectSeed>),
    /// The node's authoritative registry owns this identity and its resources.
    ExistingWorkspace(WorkspaceId),
}

/// Complete bounded input accepted before the caller opens W12's draft tab.
/// File/context ownership remains W6's; history uses its canonical wrapper.
#[derive(Clone, PartialEq, Eq)]
pub struct DraftSeed {
    /// Explicit host identity captured at the initiating action.
    pub destination: DraftDestination,
    /// No source-session/cwd fallback may replace this placement.
    pub place: SeedPlace,
    /// The choice remains editable before the draft's first submission.
    pub launch: Launch,
    /// Initial editor content, already bounded by canonical context validation.
    pub text: ContextText,
    /// A fork's recent-history selection/truncation belongs to its producer.
    pub history: Option<ChatHistory>,
}

impl fmt::Debug for DraftSeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DraftSeed([redacted])")
    }
}

/// Seeding is accepted atomically or leaves the current draft untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SeedProblem {
    /// A workspace form cannot consume an existing-workspace draft seed.
    #[error("the seed belongs to a different creation target")]
    TargetMismatch,
    /// The canonical payload owner refused intake without losing its source.
    #[error(transparent)]
    Context(#[from] crate::composer::context::ContextProblem),
    /// An existing draft/payload is occupied or an uncertain operation owns it.
    #[error("the destination draft already contains unsent or pending work")]
    Busy,
    /// The captured destination is no longer the connected node.
    #[error("the selected draft destination changed")]
    DestinationChanged,
    /// A producer's project or workspace is no longer in the current registry.
    #[error("the selected creation placement is no longer available")]
    PlacementGone,
}

/// Selection for the top-level workspace form, before any workspace id exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceForm {
    /// Verified host captured by the initiating feature.
    pub destination: DraftDestination,
    /// Exact selected project context, if the entry supplied one.
    pub project: Option<ProjectSeed>,
    /// A blank shell is distinct from an initial chat.
    pub launch: Launch,
}
impl WorkspaceForm {
    /// Header actions seed only placement; the person chooses content afterwards.
    #[must_use]
    pub fn new(destination: DraftDestination, project: Option<ProjectSeed>) -> Self {
        Self {
            destination,
            project,
            launch: Launch::default(),
        }
    }
}
/// The top-level opener does not require or fabricate a layout workspace key.
pub type NewWorkspaceSeed = WorkspaceForm;
