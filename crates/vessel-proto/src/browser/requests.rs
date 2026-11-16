//! Human and agent entry points carry different authority selectors (ADR 0078).
//!
//! Agent workspace/session comes from its authenticated principal. Human
//! selectors are verified against the registry and never mint service authority.

use super::{BrowserCommand, BrowserId, BrowserRevision, BrowserTab};
use crate::workspace::ServicePort;
use crate::{SessionId, SubscriptionId, WorkspaceId};
use serde::{Deserialize, Serialize};

/// Agent tools have no caller-controlled workspace, cwd or credential scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BrowserCall {
    /// One of the twenty-two typed operations, still subject to node admission.
    pub command: BrowserCommand,
}

/// HTTP and HTTPS are distinct service attestations, never guessed from a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ServiceScheme {
    /// Plain HTTP to an exact workspace-owned loopback listener.
    Http,
    /// HTTPS still requires normal certificate verification.
    Https,
}

/// A hint for the node's service lookup, never an access capability by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PreviewServiceHint {
    /// Must match a complete current owned-service observation.
    pub port: ServicePort,
    /// Must be verified by the node for that exact live listener.
    pub scheme: ServiceScheme,
}

/// Panel control is direct-human-only, even when the agent-tools switch is off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BrowserControl {
    /// Stable registry identity, verified server-side rather than inferred from cwd.
    pub workspace: WorkspaceId,
    /// Required for Preview and checked against the selected workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening_session: Option<SessionId>,
    /// Input/selection/control vocabulary is distinct from arbitrary CDP.
    pub command: BrowserControlCommand,
}

/// A fixed control vocabulary prevents exposing a general host-debugging tunnel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", content = "args", rename_all = "snake_case")]
pub enum BrowserControlCommand {
    /// Reuse the same real targets/actions without granting an agent this entry point.
    Drive(BrowserCommand),
    /// Service authority is minted only after node-owned lifetime/scheme checks.
    OpenPreview(PreviewServiceHint),
    /// A loading page can be stopped without pretending reload has completed.
    Stop {
        /// The existing target remains bound to its original profile.
        browser_id: BrowserId,
    },
}

/// Watching a signed-in page needs Operate; Read alone never receives frames.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WatchBrowser {
    /// The descriptor must resolve to this logical workspace.
    pub workspace: WorkspaceId,
    /// Required for a session-bound Preview watch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening_session: Option<SessionId>,
    /// The node verifies ownership before starting a frame producer.
    pub browser_id: BrowserId,
}

/// A stream starts from one current descriptor/revision, not an unrelated cached frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BrowserWatched {
    /// Owned by the authenticated connection that opened the watch.
    pub subscription: SubscriptionId,
    /// The target state from the same admission boundary.
    pub tab: BrowserTab,
    /// Later state/frame updates cannot replace it with an older revision.
    pub revision: BrowserRevision,
}

/// Clearing is an Admin/direct-human operation on node-owned profile locations only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClearBrowserProfile {}
