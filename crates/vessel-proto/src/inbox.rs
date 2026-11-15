//! Durable per-node attention, folded across nodes by a client (ADR 0040).
//! Answers deliberately use the existing session input protocol: the inbox
//! cannot create a second permission or question validation boundary.

use crate::{EventSeq, PermissionRequest, ProjectId, Question, SessionId, SessionTitle, ToolCall};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A stable position in a node's inbox, never reused after removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct InboxCursor(u64);
impl InboxCursor {
    /// Wrap a stored cursor.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    /// Its wire value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl std::str::FromStr for InboxCursor {
    type Err = std::num::ParseIntError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}
impl std::fmt::Display for InboxCursor {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(output)
    }
}

/// The exact source record, also used to open the conversation at that point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InboxKey {
    /// Session owning the entry.
    pub session: SessionId,
    /// Record that first asked for attention.
    pub seq: EventSeq,
}

/// Shared attention state; reading never resolves a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum InboxRead {
    /// Seen on one of this node's paired devices.
    Read,
    /// Attention has not been seen, or was explicitly marked unread.
    Unread,
}

/// Whether a tool's entire detail fits in an inbox page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum InboxDetail {
    /// The supplied tool is complete.
    Complete,
    /// Some detail is omitted; only retained session history may contain it.
    InSession,
}

/// A request whose unusually large form stays in its session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InboxRequest {
    /// Permission awaiting an answer.
    Permission {
        /// The actor's request id.
        id: crate::PermissionId,
    },
    /// Question awaiting an answer.
    Question {
        /// The actor's request id.
        id: crate::QuestionId,
    },
}

/// What needs attention. Unknown future kinds remain readable, not actionable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InboxItem {
    /// Reuses the agent's own choices and the tool's own title and detail.
    Permission {
        /// Request the session actor still owns.
        request: PermissionRequest,
        /// Last announced tool state, when available.
        tool: Option<ToolCall>,
        /// Large tool bodies stay in the session.
        detail: InboxDetail,
    },
    /// The agent's existing typed question form.
    Question {
        /// Request the session actor still owns.
        question: Question,
    },
    /// A request above the inbox payload budget; open its source session.
    InSession {
        /// The request remains pending, and is never silently dropped.
        request: InboxRequest,
    },
    /// A normal turn finished.
    Finished,
    /// A turn or process failed; details remain inside the session.
    Failed,
    /// The turn hit a token or request limit.
    Limit,
    /// A future kind: open its session to see it.
    #[serde(other)]
    Unknown,
}
impl InboxItem {
    /// Whether this item waits for an answer rather than reporting an outcome.
    #[must_use]
    pub const fn waiting(&self) -> bool {
        match self {
            Self::Permission { .. } | Self::Question { .. } | Self::InSession { .. } => true,
            Self::Finished | Self::Failed | Self::Limit | Self::Unknown => false,
        }
    }
}

/// A count of inbox entries, independent of the currently loaded page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct InboxCount(u64);
impl InboxCount {
    /// Wrap a count read from the node's index.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    /// The count, suitable for localization as a number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}
/// Exact counts for the whole node, never inferred from a partial page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InboxCounts {
    /// Every current request and outcome, including read ones.
    pub total: InboxCount,
    /// Requests waiting for a person's answer, including read ones.
    pub waiting: InboxCount,
    /// Entries whose attention remains unread.
    pub unread: InboxCount,
}

/// An entry and the session's current identifying metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InboxEntry {
    /// Page position, local to this node.
    pub cursor: InboxCursor,
    /// Exact source record.
    pub key: InboxKey,
    /// Current session title.
    pub title: Option<SessionTitle>,
    /// Project, when the session was placed in one.
    pub project: Option<ProjectId>,
    /// Current project name for a person, when the project still exists.
    pub project_name: Option<crate::workspace::ProjectName>,
    /// When attention was first requested.
    pub at: DateTime<Utc>,
    /// Shared read state.
    pub read: InboxRead,
    /// Request or outcome.
    pub item: InboxItem,
}

/// A bounded descending page. Omitted cursor starts at the newest entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListInbox {
    /// Exclusive upper bound, returned as `next` by the preceding page.
    pub before: Option<InboxCursor>,
}

/// At most sixteen entries, with an exclusive cursor for older entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InboxPage {
    /// Whole-node counts, from the same read transaction as this page.
    pub counts: InboxCounts,
    /// Entries, newest first.
    pub entries: Vec<InboxEntry>,
    /// Fetch the next page when present.
    pub next: Option<InboxCursor>,
}

/// Whether a get is a cheap projection recheck or recovers one live form.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum InboxFetch {
    /// Bounded durable projection; the default for confirmation/background reads.
    #[default]
    Summary,
    /// One current actor request, independent of pruned history or other forms.
    LiveRequest,
}
impl InboxFetch {
    fn summary(&self) -> bool {
        *self == Self::Summary
    }
}

/// Look up one exact current entry, including after it leaves a loaded page.
/// An absent entry is resolved/withdrawn, not a successful duplicate answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetInboxEntry {
    /// Source the client is still showing or awaiting confirmation for.
    pub key: InboxKey,
    /// Opt-in full live form. A single reply is capped at 15 MiB; larger
    /// content stays an explicit compact reference, never shortened choices.
    #[serde(default, skip_serializing_if = "InboxFetch::summary")]
    pub fetch: InboxFetch,
}

/// Change shared read attention for exactly one current entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MarkInbox {
    /// Exact source, so an old mark never consumes newer attention.
    pub key: InboxKey,
    /// New shared state.
    pub read: InboxRead,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Call, Method, ProtocolVersion, Request as _, Scope};
    #[test]
    fn inbox_calls_round_trip_and_keep_the_read_scope_boundary() {
        let list: Call = ListInbox {
            before: Some(InboxCursor::new(17)),
        }
        .into_call();
        let mark: Call = MarkInbox {
            key: InboxKey {
                session: SessionId::random(),
                seq: EventSeq::new(5),
            },
            read: InboxRead::Unread,
        }
        .into_call();
        for call in [list, mark] {
            assert_eq!(
                serde_json::from_str::<Call>(&serde_json::to_string(&call).unwrap()).unwrap(),
                call
            );
            // The inbox is protocol 22 (`rpc.rs`), not the newest number.
            assert_eq!(call.since(), Some(ProtocolVersion::new(22)));
        }
        assert_eq!(ListInbox::METHOD, Method::ListInbox);
        assert_eq!(Method::ListInbox.scope(), Some(Scope::Read));
        assert_eq!(Method::MarkInbox.scope(), Some(Scope::Read));
    }
    #[test]
    fn live_form_fetch_is_opt_in_and_old_summary_shape_stays_compatible() {
        let key = InboxKey {
            session: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
            seq: EventSeq::new(7),
        };
        let summary = GetInboxEntry {
            key,
            fetch: InboxFetch::Summary,
        };
        let encoded = serde_json::to_string(&summary).unwrap();
        assert_eq!(
            encoded,
            r#"{"key":{"session":"00000000-0000-4000-8000-000000000001","seq":7}}"#
        );
        assert_eq!(
            serde_json::from_str::<GetInboxEntry>(&encoded).unwrap(),
            summary
        );
        let live = GetInboxEntry {
            key,
            fetch: InboxFetch::LiveRequest,
        };
        assert_eq!(
            serde_json::from_str::<GetInboxEntry>(&serde_json::to_string(&live).unwrap()).unwrap(),
            live
        );
    }
    #[test]
    fn future_items_remain_readable_and_cannot_be_answered() {
        let item: InboxItem =
            serde_json::from_str(r#"{"kind":"future","details":"ignored"}"#).unwrap();
        assert_eq!(item, InboxItem::Unknown);
        assert!(!item.waiting());
    }
}
