//! Tokens distinguish a resource lifetime from each transport attempt.
use crate::layout::{LayoutKey, PaneId, TabId};
use std::collections::BTreeSet;
use vessel_proto::{NodeId, SessionId};

/// Unique window registry instance; not a backend session id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistryId(SessionId);
impl RegistryId {
    /// Mint once for a window, including after window recreation.
    #[must_use]
    pub fn random() -> Self {
        Self(SessionId::random())
    }
}
/// Authenticated resource address; aliases are only locators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionAddress {
    /// Welcome identity of the expected node.
    pub node: NodeId,
    /// Node-owned session.
    pub session: SessionId,
}
/// Monotonic resource incarnation within a registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidentEpoch(pub(super) u64);
/// One resource lifetime, unaffected by hide/move or same-node reconnect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidentToken {
    /// Owning window instance.
    pub registry: RegistryId,
    /// Expected authenticated resource.
    pub address: SessionAddress,
    /// Incarnation fences released/recreated resource callbacks.
    pub epoch: ResidentEpoch,
}
/// Stable presentation claim; moving panes keeps it unchanged.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PanelClaim {
    /// Canonical workspace or explicit legacy session bridge.
    pub workspace: LayoutKey,
    /// Local tab id within that layout.
    pub tab: TabId,
}
/// Whether a panel may mark itself read or receive pointer input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Currently drawn in a pane.
    Visible,
    /// Resources retained without focus/read claims.
    RetainedHidden,
}
/// Exactly one claim in a window may own keyboard/composer focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    /// The registry's current focused claim.
    Focused,
    /// Visible or hidden without keyboard ownership.
    Background,
}
/// Scoped owner access; lookup validates token, claim and current pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionContext {
    /// Exact resource incarnation.
    pub resident: ResidentToken,
    /// Exact layout claim.
    pub claim: PanelClaim,
    /// Current pane; changing placement invalidates old view callbacks.
    pub pane: PaneId,
    /// Current projection.
    pub visibility: Visibility,
    /// Current keyboard authority, never network authorization.
    pub focus: PanelFocus,
}
/// Per-resident attach generation, advanced before launch and never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowGeneration(pub(super) u64);
/// Every network event carries this attempt as well as the resource token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBinding {
    /// Exact resource incarnation.
    pub resident: ResidentToken,
    /// Admitted attempt; old attach metadata cannot recreate admission.
    pub follow: FollowGeneration,
}
/// An owner obtains one opaque admission before sending an operation.
/// Only the registry mints its monotonic ordinal; consumed tickets cannot be readmitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationTicket {
    pub(super) binding: FollowBinding,
    pub(super) kind: OperationKind,
    pub(super) ordinal: u64,
}
/// Owner namespaces prevent a reply settling another feature's pending state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationKind {
    /// Timeline read, rewind, fork or origin action.
    Timeline,
    /// Prompt or provider/plugin command submission.
    Composer,
    /// File upload or attachment payload effect.
    Attachment,
    /// Permission or question answer.
    Request,
    /// Parent-owned provider-child query/read operation.
    Subagents,
    /// Archive, close or other session lifecycle effect.
    Lifecycle,
}
impl OperationKind {
    pub(super) const fn owner(self) -> ResourceOwner {
        match self {
            Self::Timeline => ResourceOwner::Timeline,
            Self::Composer => ResourceOwner::Composer,
            Self::Attachment => ResourceOwner::Attachments,
            Self::Request => ResourceOwner::Requests,
            Self::Subagents => ResourceOwner::Subagents,
            Self::Lifecycle => ResourceOwner::Lifecycle,
        }
    }
}
/// The concrete resource owners must all agree before clean dehydration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceOwner {
    /// Timeline and its transient operations.
    Timeline,
    /// Editor and composer policy resources.
    Composer,
    /// Trays, uploads and context payload.
    Attachments,
    /// Permission/question forms and drafts.
    Requests,
    /// Parent-owned child query, readonly area and navigation holds.
    Subagents,
    /// Confirmation and lifecycle operations.
    Lifecycle,
}
impl ResourceOwner {
    /// Every concrete owner must transfer a clean snapshot before retirement.
    pub const ALL: [Self; 6] = [
        Self::Timeline,
        Self::Composer,
        Self::Attachments,
        Self::Requests,
        Self::Subagents,
        Self::Lifecycle,
    ];
}
/// Protected state is retained even beyond the warm target or hard-cap pressure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldReason {
    /// An owner has not produced a safe concrete snapshot yet.
    Loading,
    /// Includes whitespace, recall and permission/question draft text.
    DirtyInput,
    /// Input method composition cannot be reconstructed safely.
    Ime,
    /// Active editor/focus must not be discarded.
    ActiveEditor,
    /// Fullscreen, voice, lightbox or confirmation still owns resources.
    Overlay,
    /// An admitted operation is in flight.
    PendingEffect,
    /// Wire mutation outcome is unknown; do not retry or clear input.
    UncertainEffect,
}
/// App owners retain their concrete snapshot; no untyped payload crosses here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerReadiness {
    /// A concrete owner snapshot is ready for transfer.
    Ready,
    /// Preserve the live resources until the owner explicitly releases its hold.
    Hold(HoldReason),
}
/// Typed refusal never redirects a callback to globally selected state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ResidentProblem {
    /// No current resource exists.
    #[error("resident is missing")]
    Missing,
    /// Window, incarnation or claim changed.
    #[error("resident context is stale")]
    StaleEpoch,
    /// A different authenticated node claims the same session id.
    #[error("resident owner conflicts")]
    OwnerConflict,
    /// No protected resource is silently removed for a new admission.
    #[error("resident capacity reached")]
    Capacity,
    /// Never wrap a lifetime or attempt counter.
    #[error("resident generation exhausted")]
    Exhausted,
    /// The active attach attempt was retired/replaced.
    #[error("follow binding is stale")]
    StaleBinding,
    /// A reply is duplicate, unknown or belongs to another operation kind.
    #[error("operation is not admitted")]
    UnadmittedOperation,
    /// A visible/focused/protected resource cannot dehydrate.
    #[error("resident resources are held")]
    Held,
}
pub(super) type Admissions = BTreeSet<(OperationKind, u64)>;
