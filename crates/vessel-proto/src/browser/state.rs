//! Browser facts are distinct from network/workspace authority and pane geometry.

use serde::{Deserialize, Serialize};

use super::{BrowserFailure, BrowserId, BrowserUrl, PageText, ScreenshotPng, ViewportSize};
use crate::{SessionId, WorkspaceId};

/// History eligibility is one closed state rather than an ambiguous pair of flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum BrowserHistory {
    /// Neither direction has an entry.
    #[default]
    Empty,
    /// A preceding entry exists.
    Back,
    /// A later entry exists.
    Forward,
    /// Both directions are available.
    Both,
}

/// An error is retained as a typed fact rather than shown as a ready page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", content = "failure", rename_all = "snake_case")]
pub enum BrowserLoad {
    /// Navigation or resource loading is still in flight.
    Loading,
    /// Loading settled successfully.
    Ready,
    /// Failed loading never supplies a synthetic screenshot.
    Failed(BrowserFailure),
}

/// A responsive viewport retains its effective size without becoming a fixed preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "mode", content = "size", rename_all = "snake_case")]
pub enum BrowserViewport {
    /// Current pane bounds, or the resident default while hidden.
    Responsive(ViewportSize),
    /// An agent resize or a selected device preset.
    Fixed(ViewportSize),
}

/// Only the node can bind a Preview profile; this descriptor grants no capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BrowserProfile {
    /// Node-shared persistent sign-ins for public destinations.
    General,
    /// Isolated sign-ins and one exact private origin tied to the opening session.
    Preview {
        /// Other sessions cannot drive this target merely by sharing its workspace.
        opening_session: SessionId,
    },
}

/// A favicon must not make the GPUI app fetch a page-selected URL outside policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", content = "image", rename_all = "snake_case")]
pub enum BrowserIcon {
    /// Safe fallback while no bounded PNG icon is available.
    Globe,
    /// Content obtained through the browser's own permitted network context.
    Png(ScreenshotPng),
}

/// Inventory facts are scoped by node RPC/workspace checks, not a caller-provided cwd.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BrowserTab {
    /// Stable node-owned id, independent of a CDP target's lifetime.
    pub browser_id: BrowserId,
    /// Logical ownership comes from the reviewed workspace identity contract.
    pub workspace: WorkspaceId,
    /// Current address, never copied into diagnostic fields or persisted app layout.
    pub url: BrowserUrl,
    /// The panel falls back to hostname while this is empty.
    pub title: PageText,
    /// One loading/error fact drives panel and toolbar state.
    pub load: BrowserLoad,
    /// Buttons and shortcuts use the same engine history fact.
    pub history: BrowserHistory,
    /// CSS viewport; physical image/pane sizes are a separate boundary.
    pub viewport: BrowserViewport,
    /// Indicates the credential-sharing scope without returning the internal lease id.
    pub profile: BrowserProfile,
    /// Remote image URLs are never delegated to a client-side image loader.
    pub icon: BrowserIcon,
}

/// Frame/state revisions cannot wrap into an earlier update after a long-lived watch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BrowserRevision(u64);

impl BrowserRevision {
    /// A newly created target has no earlier visible update.
    pub const ZERO: Self = Self(0);
    /// Checked advancement forces the actor to close rather than reuse an old revision.
    #[must_use]
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
    /// Explicit transport/capture adapters may inspect the revision.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A measurement describes a snapshot without using an untyped numeric field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SnapshotMetric(u32);

impl SnapshotMetric {
    /// All unsigned measurements are meaningful; traversal bounds live in the engine.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
    /// Formatting happens at the presentation edge, not through a guessed unit.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Stats are measurements, never proof that the full DOM fit the snapshot budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SnapshotStats {
    /// Nodes considered by the bounded traversal.
    pub nodes: SnapshotMetric,
    /// Refs assigned in this snapshot only.
    pub references: SnapshotMetric,
    /// UTF-8 bytes in the bounded rendered text.
    pub text_bytes: TextByteCount,
    /// Frames encountered while constructing the snapshot.
    pub frames: SnapshotMetric,
}

/// Byte measurements cannot be passed where a DOM/ref count is expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TextByteCount(u32);
impl TextByteCount {
    /// A measured unsigned byte length, with no implied source completeness.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
    /// Formatting and wire adapters can explicitly read its unit.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
