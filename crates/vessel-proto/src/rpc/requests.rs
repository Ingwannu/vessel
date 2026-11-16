//! The request and reply payloads named in the method table (`rpc.rs`).

use serde::{Deserialize, Serialize};

use crate::event::EventRecord;
use crate::id::{EventSeq, SessionId, SubscriptionId};
use crate::node::ProtocolVersion;
use crate::session::{SessionInfo, SessionInput, SessionSpec};
use crate::title::SessionTitle;

/// `node.info`: describe the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetNode;

/// `harness.list`: discover installed harnesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListHarnesses;

/// `session.list`: preserve the legacy full collection unless a caller
/// explicitly supplies the versioned discovery selection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct ListSessions(Option<crate::SessionSelection>);

impl ListSessions {
    /// A present selection must pass shared admission; it never becomes null.
    #[must_use]
    pub fn selected(selection: crate::SessionSelection) -> Self {
        Self(Some(selection))
    }

    /// None is the original full-list wire contract, not implicit discovery.
    #[must_use]
    pub const fn selection(&self) -> Option<&crate::SessionSelection> {
        self.0.as_ref()
    }
}

/// `session.get`: one session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetSession {
    /// Which session.
    pub session: SessionId,
}

/// `session.create`: start a harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateSession {
    /// What to run.
    pub spec: SessionSpec,
}

/// `session.send`: send input to a live session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SendInput {
    /// Which session.
    pub session: SessionId,
    /// What to send.
    pub input: SessionInput,
}

/// `session.kill`: stop a session's process (and its process group).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct KillSession {
    /// Which session.
    pub session: SessionId,
}

/// `session.rename` (protocol 9): give a session a title of its own. Every
/// client sees the change as a `renamed` event and in `SessionInfo.title`;
/// the reply is the session as it is now. Needs `operate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RenameSession {
    /// Which session.
    pub session: SessionId,
    /// The title.
    pub title: SessionTitle,
}

/// A validated nonempty patch for current labels (ADR 0063), prepared before
/// registration of `session.labels`. This payload alone sends no request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetLabels {
    /// Exact session; discovery filters cannot retarget a metadata mutation.
    pub session: SessionId,
    /// Named keys to set, retaining every unmentioned current key.
    pub labels: crate::LabelPatch,
}

/// `session.read`: a page of a session's log. The authoritative way to fill
/// gaps; live subscriptions are an optimisation on top of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadEvents {
    /// Which session.
    pub session: SessionId,
    /// First sequence number wanted.
    pub from: EventSeq,
    /// Maximum number of records; the node may return fewer and caps it.
    pub limit: u32,
    /// With nothing at `from` yet, wait this many seconds for something to
    /// arrive before answering (an empty page when nothing did), at most
    /// [`ReadEvents::MAX_WAIT`] (protocol 6). A long poll, so waiting on
    /// another node's session needs no subscription across hops.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait: Option<u32>,
}

impl ReadEvents {
    /// The longest a read waits, in seconds; the node caps `wait` to it.
    pub const MAX_WAIT: u32 = 60;
}

/// Reply to `session.read`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EventPage {
    /// Records in order, starting at the requested `from` (or the oldest
    /// retained record if older ones were pruned).
    pub records: Vec<EventRecord>,
    /// Sequence number to ask for next.
    pub next: EventSeq,
    /// Whether `next` is the end of the log right now.
    pub at_end: bool,
}

impl EventPage {
    /// This page as a client speaking `protocol` reads it, or `None` when no
    /// record needs a stand-in (see `SessionEvent::for_protocol`).
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        let changed = self
            .records
            .iter()
            .any(|record| record.event.for_protocol(protocol).is_some());
        changed.then(|| Self {
            records: self
                .records
                .iter()
                .map(|record| {
                    record
                        .for_protocol(protocol)
                        .unwrap_or_else(|| record.clone())
                })
                .collect(),
            next: self.next,
            at_end: self.at_end,
        })
    }
}

/// `session.subscribe`: stream a session's events from `from` onwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Subscribe {
    /// Which session.
    pub session: SessionId,
    /// First sequence number wanted (zero for the whole history).
    pub from: EventSeq,
}

/// Reply to `session.subscribe`. Events follow as `ServerFrame::Event`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Subscribed {
    /// Id that tags the events of this subscription.
    pub subscription: SubscriptionId,
    /// The session at the moment of subscribing.
    pub info: SessionInfo,
}

/// `session.unsubscribe`: stop a subscription on this connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Unsubscribe {
    /// Which subscription.
    pub subscription: SubscriptionId,
}

/// When `node.stop` takes effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum StopWhen {
    /// Stop now: live sessions end `Interrupted`, as on a signal.
    Now,
    /// Drain: refuse new sessions and stop once none is live.
    WhenIdle,
}

/// `node.stop` (protocol 3): stop the node, now or once idle. Used by
/// remote update; a client that negotiated protocol 2 never sends it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StopNode {
    /// When to stop.
    pub when: StopWhen,
}

/// Empty success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Ack;

#[cfg(test)]
mod labels_tests;
