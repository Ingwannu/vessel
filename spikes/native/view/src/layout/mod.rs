//! Client-local pane contract. Features own resources and buffers; this module
//! owns identity and placement and cannot call a node or tear down a resource.

mod action_context;
pub use action_context::LayoutActionContext;
mod aliases;
mod catalog;
pub use aliases::{forget_alias, rename_alias};
mod descriptor;
mod draft;
pub use draft::DraftId;
mod fraction;
pub use descriptor::{PanelDescriptor, PanelStatus, TitleState};
pub use fraction::SplitFraction;
mod close;
mod compact;
mod drop_zone;
pub use drop_zone::{DROP_CENTER_RATIO, DROP_EDGE_RATIO, DropPoint};
mod movement;
mod ownership;
pub use ownership::{LookupProblem, WorkspaceLocation, WorkspaceLookup, session_layout_key};
mod persistence;
#[cfg(test)]
mod persistence_groups;
pub use persistence::{RememberedLayout, decode_layouts};
mod reconcile;
mod retarget;
pub use retarget::RetargetRequest;
mod rename;
pub use rename::{RenamePhase, RenameProblem, RenameSubmission, RenameTarget, RenameTicket};
mod state;
mod target;
mod tree;
mod values;

pub use catalog::{Availability, Launch, LaunchPurpose, catalog};
pub use close::{CloseApproval, CloseEffect, ClosePlan, Modified};
pub use movement::{CloseScope, Direction, LayoutAction, TabIndex, TabInsertion};
pub use reconcile::{EntitySnapshot, Hydration, SessionTab};
pub use state::{
    ExplorerVisibility, Layout, LayoutMode, OpenIntent, Placement, Sheet, SheetPromotion, Tab,
};
pub use target::{PaneHost, Target};
pub use tree::{Axis, Pane, Position, SplitNode};
pub use values::{GroupId, HostAlias, LayoutKey, PaneId, ResourceId, TabId, WorkspaceId};
/// Compatibility name for the canonical workspace-relative metadata role. This
/// adds no membership lookup, freshness or permission to close a resource.
pub use vessel_proto::subagents::WorkspaceRole as AgentRole;

/// Includes the Explorer bookkeeping level, as Paseo's layout store does.
pub const MAX_TREE_DEPTH: usize = 5;
/// Smallest fraction of a two-way split (Paseo workspace-layout-constants).
pub const MIN_SPLIT_SIZE: f32 = 0.1;
/// Host title bound; the current protocol is stricter than Paseo's rename modal.
pub const MAX_TITLE_CHARS: usize = vessel_proto::SessionTitle::MAX_CHARS;

/// Typed refusals let screens show an actionable failure without guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LayoutError {
    /// Persisted or user-entered identity was not meaningful.
    #[error(
        "layout identity must be nonempty, contain no controls and use at most {} bytes",
        values::MAX_ID_BYTES
    )]
    InvalidIdentity,
    /// An event referred to a pane no longer present.
    #[error("pane no longer exists")]
    MissingPane,
    /// An event referred to a tab no longer present.
    #[error("tab no longer exists")]
    MissingTab,
    /// A panel manifest refuses this host.
    #[error("panel does not support this pane host")]
    UnsupportedHost,
    /// Compact/touch mode does not expose desktop splits.
    #[error("this layout does not support cross-pane movement or splitting")]
    SplitsUnavailable,
    /// Nesting is limited to prevent unusable or malicious restored trees.
    #[error("pane tree exceeds depth limit {MAX_TREE_DEPTH}")]
    TreeTooDeep,
    /// A stale close cannot discard changed work.
    #[error("tab contains modified work")]
    Modified,
    /// Validated persistence never installs broken references.
    #[error("saved layout has inconsistent pane or tab references")]
    InvalidState,
    /// Local counters never wrap into an existing identity.
    #[error("layout identity space is exhausted")]
    Exhausted,
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod canonical_role_tests;
