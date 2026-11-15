//! Connected notification admission and explicit terminal attention (ADR 0089).
//! Calls live only in the shared method table; the integrator assigns their
//! protocol after routing, semantic producers and consumers are reviewed.
use crate::inbox::{InboxCursor, InboxKey};
use crate::key::secret;
use crate::{ProtocolVersion, TerminalId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

secret! {
    /// A local terminal's activity-only capability. Never grants session,
    /// filesystem, inbox, push, or normal node-operation authority.
    TerminalActivitySecret, 32
}
/// Local IPC authentication for an activity-only terminal producer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct TerminalActivityAuth {
    /// The only terminal this capability can report on.
    pub terminal: TerminalId,
    /// Short-lived capability, redacted by Debug and invalid after terminal end.
    pub secret: TerminalActivitySecret,
}
/// Environment name of the exact terminal activity target.
pub const TERMINAL_ACTIVITY_ID_ENV: &str = "VESSEL_TERMINAL_ACTIVITY_ID";
/// Environment name of the terminal's activity-only credential.
pub const TERMINAL_ACTIVITY_SECRET_ENV: &str = "VESSEL_TERMINAL_ACTIVITY_SECRET";
/// Local socket or named-pipe endpoint; never a newly opened hook listener.
pub const TERMINAL_ACTIVITY_SOCKET_ENV: &str = "VESSEL_TERMINAL_ACTIVITY_SOCKET";

/// Unassigned until node admission, terminal producers, native rendering and
/// exact-source older-peer refusal are available together at integration.
pub const ATTENTION_NOTIFICATIONS_PROTOCOL: Option<ProtocolVersion> = None;

/// A non-reused revision of explicit activity on one terminal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct TerminalAttentionRevision(u64);
impl TerminalAttentionRevision {
    /// Carry the runtime's revision; zero is the initial unknown state.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    /// Its numeric wire representation.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
    /// Advance without reusing an earlier clear/notification identity.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

/// Why an explicitly instrumented terminal needs the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TerminalAttentionReason {
    /// A known command was working and returned to idle.
    Finished,
    /// The producer explicitly reports that human input is required.
    NeedsInput,
    /// Newer producer state remains readable and never authorizes an action.
    #[serde(other)]
    Unknown,
}

/// Unknown is distinct from idle: raw PTY bytes cannot identify a shell job.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TerminalActivity {
    /// Known plain shell without semantic instrumentation. No terminal indicator;
    /// neutral only in workspace projection, never a terminal completion claim.
    Untracked,
    /// Producer reports idle with no outstanding attention.
    Idle,
    /// Producer reports active work.
    Working,
    /// Explicit attention remains until cleared by exact revision.
    Attention {
        /// Reason supplied by the typed activity producer.
        reason: TerminalAttentionReason,
    },
    /// Lost/missing producer or newer state; never an Untracked proof.
    #[default]
    #[serde(other)]
    Unknown,
}

/// The node-assigned current revision, never a client-supplied timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TerminalAttention {
    /// Terminal owning this state.
    pub terminal: TerminalId,
    /// Exact revision to clear or claim.
    pub revision: TerminalAttentionRevision,
    /// State reported by the trusted terminal producer boundary.
    pub activity: TerminalActivity,
    /// When that revision became current, assigned by the node.
    pub changed_at: DateTime<Utc>,
}

/// Exact attention identity; unattached terminals never use fabricated sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "target", rename_all = "snake_case")]
pub enum NotificationSource {
    /// A durable session inbox source.
    Session {
        /// Exact source record, independent of currently loaded pages.
        key: InboxKey,
        /// Monotonic position assigned by this node.
        cursor: InboxCursor,
    },
    /// An explicitly reported terminal attention revision.
    Terminal {
        /// Terminal owning the producer state.
        terminal: TerminalId,
        /// Exact attention being requested, never merely its latest process.
        revision: TerminalAttentionRevision,
    },
}

/// Request shape staged until the shared method table registers it. Recipient
/// identity is the authenticated connection, never an argument from a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ClaimNotification {
    /// Source the client has freshly observed and has not delivered before.
    pub source: NotificationSource,
}

/// Only Admitted authorizes best-effort generic OS display. A lost reply is
/// not a reason to repeat the claim; the durable inbox is still available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum NotificationClaim {
    /// This exact source was atomically admitted for this connection once.
    Admitted,
    /// A more recently active eligible client is the local recipient.
    NotRecipient,
    /// A present visible client is already focused on the target.
    Suppressed,
    /// Attention was read, resolved, withdrawn or changed revision.
    NoAttention,
    /// This exact source has already had its local delivery opportunity.
    AlreadyClaimed,
    /// No recently active local client exists; preserve the push opportunity.
    NoPresentClient,
    /// A newer outcome never authorizes local display.
    #[serde(other)]
    Unknown,
}

/// Clear one exact terminal attention revision, independent of closing a tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ClearTerminalAttention {
    /// Terminal whose current state is being viewed.
    pub terminal: TerminalId,
    /// Stale visibility/read callbacks cannot consume newer attention.
    pub revision: TerminalAttentionRevision,
}

/// A producer's semantic activity input. Arbitrary PTY output never maps to
/// these variants; the service validates terminal ownership before applying it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TerminalActivitySignal {
    /// Report idle; a prior Working state becomes Finished attention.
    Idle,
    /// Explicitly begin working.
    Working,
    /// Explicitly request input.
    NeedsInput,
    /// Producer disconnected; revert to unknown, not a finished outcome.
    Clear,
    /// User interrupted active work; revert only Working to unknown.
    Interrupt,
}

/// Semantic transition for one terminal. Normal clients name the exact
/// revision; a capability-only producer may advance its own current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ReportTerminalActivity {
    /// Terminal owned by the authenticated producer or person.
    pub terminal: TerminalId,
    /// Normal clients must name the observed revision. An activity-only
    /// producer may omit it to advance its own terminal's current state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<TerminalAttentionRevision>,
    /// Explicit semantic lifecycle signal, never decoded PTY output text.
    pub signal: TerminalActivitySignal,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revision_exhaustion_cannot_reuse_an_identity() {
        assert_eq!(TerminalAttentionRevision::new(u64::MAX).next(), None);
    }
    #[test]
    fn explicit_untracked_and_future_wire_states_remain_distinct() {
        let untracked = serde_json::to_string(&TerminalActivity::Untracked).unwrap();
        assert_eq!(untracked, r#"{"state":"untracked"}"#);
        assert_eq!(
            serde_json::from_str::<TerminalActivity>(&untracked).unwrap(),
            TerminalActivity::Untracked
        );
        assert_eq!(
            serde_json::from_str::<TerminalActivity>(r#"{"state":"future"}"#).unwrap(),
            TerminalActivity::Unknown
        );
        assert_ne!(TerminalActivity::default(), TerminalActivity::Untracked);
    }
    #[test]
    fn future_claim_and_activity_never_authorize_delivery() {
        assert_eq!(
            serde_json::from_str::<NotificationClaim>("\"new-outcome\"").unwrap(),
            NotificationClaim::Unknown
        );
        assert_eq!(
            serde_json::from_str::<TerminalActivity>(r#"{"state":"new-state"}"#).unwrap(),
            TerminalActivity::Unknown
        );
        assert!(serde_json::from_str::<ClaimNotification>(r#"{"source":{"target":"terminal","terminal":"00000000-0000-4000-8000-000000000001","revision":1},"recipient":"another-device"}"#).is_err());
    }
    #[test]
    fn session_and_terminal_sources_keep_independent_exact_identity() {
        let source = NotificationSource::Terminal {
            terminal: TerminalId::random(),
            revision: TerminalAttentionRevision::new(5),
        };
        assert_eq!(
            serde_json::from_str::<NotificationSource>(&serde_json::to_string(&source).unwrap())
                .unwrap(),
            source
        );
    }
}
