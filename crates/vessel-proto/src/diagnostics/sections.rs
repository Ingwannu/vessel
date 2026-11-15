//! Allowlisted facts; no field accepts a path, arbitrary label or message.
use super::{
    AcpProtocolVersion, CustomAgentSlot, DiagnosticCount, DiagnosticSchemaVersion,
    DiagnosticVersion,
};
use crate::{DurationMs, ErrorCode, HarnessKind, NodeId, NodeState, agents::UnknownReason};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One source succeeded at a stated time, or failed for a closed reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiagnosticSection<T> {
    /// Facts from one source, not a cross-node global transaction.
    Available {
        /// When these facts were observed.
        at: DateTime<Utc>,
        /// Allowlisted facts.
        value: T,
    },
    /// No success/zero is inferred from missing facts.
    Unavailable {
        /// A safe reason, with no raw error.
        reason: DiagnosticUnavailable,
    },
}

/// Why one section cannot make a claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticUnavailable {
    /// No observation yet.
    NotChecked,
    /// Only expired facts exist.
    Stale,
    /// The collector's deadline expired.
    TimedOut,
    /// A source read failed; no path or message is included.
    ReadFailed,
    /// Inventory exceeds its named row limit.
    TooManyAgents,
    /// The node is stopping.
    Stopping,
    /// This platform/source does not provide it.
    Unsupported,
}

/// The home is represented by a fixed marker, never by its path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticHome {
    /// Render as `~` without reading or retaining a real home string.
    Redacted,
}

/// Platform classes contain no operating-system release/environment text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticOs {
    /// Linux.
    Linux,
    /// macOS.
    MacOs,
    /// Windows.
    Windows,
    /// Another target; no unbounded target name is copied.
    Other,
}

/// CPU architecture classes from the build target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticArch {
    /// x86-64.
    X86_64,
    /// 64-bit Arm.
    Aarch64,
    /// x86.
    X86,
    /// Arm.
    Arm,
    /// Another target, without a copied target string.
    Other,
}

/// Public process facts, projected rather than cloning `NodeInfo`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticNode {
    /// Stable machine identity; no key/hostname.
    pub id: NodeId,
    /// Normalized package version or explicit unknown.
    pub version: Option<DiagnosticVersion>,
    /// Build platform.
    pub os: DiagnosticOs,
    /// Build architecture.
    pub arch: DiagnosticArch,
    /// Node lifecycle, not transport reachability.
    pub state: NodeState,
    /// Elapsed monotonic node-instance time.
    pub uptime: DurationMs,
    /// Fixed privacy marker.
    pub home: DiagnosticHome,
}

/// Stored session aggregates, from the same `SQLite` transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticStoredSessions {
    /// All retained sessions.
    pub total: DiagnosticCount,
    /// Sessions still marked live in durable state.
    pub live: DiagnosticCount,
    /// Ended sessions, including failures/interruption.
    pub ended: DiagnosticCount,
}

/// Successful bounded store reads do not imply a full integrity check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticStore {
    /// Schema supported by this binary.
    pub expected_schema: DiagnosticSchemaVersion,
    /// Schema observed on the opened connection.
    pub observed_schema: DiagnosticSchemaVersion,
    /// Durable session aggregates; no session identifiers/content.
    pub sessions: DiagnosticStoredSessions,
}

/// Runtime sessions are distinct from durable state during publication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticRuntimeSessions {
    /// Live session actors, including parked ones.
    pub live: DiagnosticCount,
    /// Live sessions holding no running harness process.
    pub parked: DiagnosticCount,
}

/// A safe agent identifier in this report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiagnosticAgentId {
    /// A statically known agent label.
    Builtin {
        /// The builtin kind.
        harness: HarnessKind,
    },
    /// A numeric report-local slot, not a custom agent's chosen name.
    Custom {
        /// Ordinal valid only for this report.
        slot: CustomAgentSlot,
    },
}

/// Executable presence says nothing about sign-in or successful turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticPresence {
    /// Metadata lookup found an executable.
    Found,
    /// Metadata lookup did not find one.
    Missing,
    /// No trustworthy lookup is available.
    Unknown,
}

/// A capability refusal has no executable/path/output text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "reason", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiagnosticAgentRefusal {
    /// No executable.
    MissingExecutable,
    /// No installed adapter.
    MissingAdapter,
    /// The agent speaks an older protocol.
    TooOld {
        /// Reported numeric protocol.
        speaks: AcpProtocolVersion,
        /// Required numeric protocol.
        needs: AcpProtocolVersion,
    },
    /// The agent speaks a newer protocol.
    TooNew {
        /// Reported numeric protocol.
        speaks: AcpProtocolVersion,
        /// Required numeric protocol.
        needs: AcpProtocolVersion,
    },
    /// This agent does not offer the mode.
    Unsupported,
}

/// Known availability or the exact safe reason it cannot be asserted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiagnosticAvailability {
    /// The observed capability is available (not proof of sign-in).
    Available,
    /// The capability was refused for a closed reason.
    Unavailable {
        /// Never a raw reason string.
        reason: DiagnosticAgentRefusal,
    },
    /// No fresh observation; existing typed readiness reasons are safe.
    Unknown {
        /// No refresh/probe is performed to fill this fact.
        reason: UnknownReason,
    },
}

/// Cached sign-in facts for the default profile, without sign-in help.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiagnosticSignIn {
    /// A previous explicit check found credentials; no turn was run.
    Present,
    /// A previous explicit check requested sign-in; help is omitted.
    Missing,
    /// The configured default profile does not require sign-in.
    NotRequired,
    /// No fresh cached check; collecting diagnostics never refreshes it.
    Unknown {
        /// Existing closed readiness reason, never output or a path.
        reason: UnknownReason,
    },
}

/// One agent with no path, arbitrary name or output field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticAgent {
    /// Safe identity.
    pub id: DiagnosticAgentId,
    /// Metadata-only executable observation.
    pub executable: DiagnosticPresence,
    /// Conversation availability.
    pub structured: DiagnosticAvailability,
    /// Terminal availability.
    pub terminal: DiagnosticAvailability,
    /// Cached sign-in of this agent's default profile only.
    pub sign_in: DiagnosticSignIn,
    /// Numeric cached version; free-form output is absent.
    pub version: Option<DiagnosticVersion>,
    /// When a cached protocol observation was made, if one exists.
    pub checked_at: Option<DateTime<Utc>>,
    /// Whether the cached observation has expired.
    pub stale: bool,
}

/// Local transport kinds, without a socket/pipe name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLocalKind {
    /// Unix local socket.
    Socket,
    /// Windows named pipe.
    Pipe,
}

/// Observed binding state, never a claim of outside reachability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticBinding {
    /// Not configured.
    Disabled,
    /// Startup has not published a successful bind.
    Starting,
    /// Successfully bound; address and port are omitted.
    Bound,
}

/// Listener metadata never contains a route/address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticListeners {
    /// Local transport kind.
    pub local_kind: DiagnosticLocalKind,
    /// Local bind state.
    pub local: DiagnosticBinding,
    /// Direct TCP bind state; not an advertised route.
    pub direct: DiagnosticBinding,
}

/// Counts from relay watches; no route, refusal text or credential.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticRelayCounts {
    /// Connecting or registering, including reconnect attempts.
    pub connecting: DiagnosticCount,
    /// Registered and accepting relayed channels.
    pub registered: DiagnosticCount,
    /// Registration refused; normal retry policy is unchanged.
    pub refused: DiagnosticCount,
    /// Link stopped.
    pub stopped: DiagnosticCount,
    /// Future non-exhaustive link states, without invented meaning.
    pub unrecognized: DiagnosticCount,
}

/// Configured links and fleet callbacks remain distinct.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticRelays {
    /// Links from startup configuration.
    pub configured: DiagnosticRelayCounts,
    /// Current home's callback links.
    pub callback: DiagnosticRelayCounts,
}

/// One observed failed local RPC completion; no request/method/message data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticFailure {
    /// Existing closed error class.
    pub code: ErrorCode,
    /// When the failed completion was observed.
    pub at: DateTime<Utc>,
}

/// Explicit error observation coverage; never "all node errors".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticErrorCoverage {
    /// Typed local RPC completions only; envelope wrappers are excluded.
    LocalRpcCompletions,
}
