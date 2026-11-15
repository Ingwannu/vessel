//! Feature-owned presentation is separate from target identity and panel buffers.
use std::sync::Arc;
/// Loading a label does not create a different target or remount its resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleState {
    /// Label is authoritative for the latest resource snapshot.
    Ready,
    /// The owning feature is resolving a label.
    Loading,
}
/// A tab's status is independent from whether its pane has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelStatus {
    /// No background work or attention.
    Idle,
    /// Owning resource has active work.
    Running,
    /// Resource needs the user.
    Attention,
    /// Capability/contribution no longer available; retain its slot.
    Unavailable,
}
/// The registry asks each owner for this cached projection, never its raw state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelDescriptor {
    /// Free-form display text; never used as an identifier.
    pub label: Arc<str>,
    /// URL, location or provider hint, when meaningful.
    pub subtitle: Option<Arc<str>>,
    /// Full title/path when the rail truncates the label.
    pub tooltip: Option<Arc<str>>,
    /// Hydration state of the label only.
    pub title_state: TitleState,
    /// Activity/capability projection supplied by the owner.
    pub status: PanelStatus,
}
