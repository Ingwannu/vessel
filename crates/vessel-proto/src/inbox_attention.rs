//! Session badges are a rollup of the durable inbox, not log read watermarks.
//! Exact-entry reads must not consume unrelated unread permissions/questions.
use crate::{SessionId, inbox::InboxCount};
use serde::{Deserialize, Serialize};

/// Maximum requested session identities in one bounded badge snapshot.
pub const ATTENTION_BATCH: usize = 256;

/// Read authoritative attention for specified sessions, including entries
/// outside the currently loaded sixteen-entry inbox page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct GetInboxAttention {
    /// At most `ATTENTION_BATCH` distinct session identities on this node.
    pub sessions: Vec<SessionId>,
}

/// Independent unread and waiting counts. A read request can remain waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SessionAttention {
    /// Existing source session. Missing/deleted sessions are omitted.
    pub session: SessionId,
    /// All current inbox entries that have not been seen.
    pub unread: InboxCount,
    /// Deliberate whole-session unread mark, including an empty/pruned inbox.
    /// Exact-entry Seen and resolution do not clear this separate flag.
    #[serde(default)]
    pub explicit_unread: bool,
    /// All current requests awaiting a person, including seen ones.
    pub waiting: InboxCount,
    /// Current failed outcomes, independent of whether they have been read.
    pub failed: InboxCount,
}

/// A person's passive read through an actually observed exclusive watermark.
/// Never pin/archive/unread or answer; pending requests veto the operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ReadInboxThrough {
    /// Session the person actually viewed.
    pub session: SessionId,
    /// Only source records strictly before this observed point may be read.
    pub through: crate::EventSeq,
}
