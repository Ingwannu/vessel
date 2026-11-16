//! The frames between the browser and `vessel ui` (ADR 0017): JSON text
//! frames on `/api/ws`, one message each, and the HTTP API's bodies.
//!
//! The browser's first frame must be [`ClientFrame::Hello`] with the
//! proof of its bootstrap/session key after the server challenge, within
//! a short deadline. Invalid proof or a misplaced frame closes with
//! [`CLOSE_UNAUTHORIZED`]; elapsed waiting uses retryable close 1013.
//! After [`ServerFrame::Welcome`] the browser:
//!
//! - calls methods on nodes ([`ClientFrame::Call`]), each answered once by
//!   a [`ServerFrame::Reply`] with the same id;
//! - follows sessions ([`ClientFrame::Follow`]) and terminals
//!   ([`ClientFrame::FollowTerminal`], protocol 11): records come as
//!   [`ServerFrame::Event`], the stream's ups and downs as
//!   [`ServerFrame::Stream`];
//! - asks the bridge itself ([`ClientFrame::Op`]) for what only this
//!   machine knows: the book's nodes, pairing, QR codes, the book's edits
//!   (forget, rename, default), and the page's own preferences and drafts
//!   (`prefs`, ADR 0028).
//!
//! Every node's connection state is pushed as [`ServerFrame::Node`], and
//! every state of the preferences as [`ServerFrame::Preferences`].
//!
//! Ids (`CallId`, `StreamId`) are the browser's: the bridge only echoes
//! them, so they need to be unique among one socket's open calls and
//! streams, nothing more.

mod auth;
mod node;
mod prefs;
pub use auth::{AuthMode, InvalidAuthValue, LaunchId, Nonce};
pub use node::{BookName, InvalidBookName, InvalidNodeKey, NodeKey};
pub use prefs::{
    ActionId, AgentDefaults, AgentKind, Alerts, Appearance, Bindings, Bounded, ChoiceKey,
    ChoiceKeyTooLong, Choices, ChoicesError, CodeSize, Composer, ContentWidth, ConversationDisplay,
    Draft, DraftKey, DraftText, DraftTooLong, DraftsState, Grouping, InterfaceSize,
    InvalidActionId, InvalidKeyBinding, Kept, KeyBinding, LaunchOnly, Layout, MAX_ACTIONS,
    MAX_BINDINGS, MAX_CHOICE_TEXT, MAX_CHOICES, MAX_DRAFT, MAX_EDITS, MAX_MACHINES, MAX_USAGE_PINS,
    MachineDefaults, Notifications, OutOfRange, Panel, PreferenceChange, Preferences,
    PreferencesState, Revision, SendKey, SessionSort, ShortcutEdit, ShortcutEdits, ShortcutTarget,
    SidebarView, Theme, TooMany, TurnDelivery, UsageDisplay, UsagePin, UsagePins, UsagePreferences,
};

use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use vessel_proto::{
    EventRecord, EventSeq, Method, NodeId, NodeInfo, PairingOffer, ProtocolVersion, RawJson, Reply,
    SessionId, TerminalId, WireError,
};

/// Closes a websocket that presented invalid proof or an unexpected first frame.
pub const CLOSE_UNAUTHORIZED: u16 = 4401;

/// A frame from the browser.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientFrame {
    /// The client's response to the server challenge.
    Hello {
        /// Bootstrap or a session resumed by this tab.
        mode: AuthMode,
        /// The browser's fresh challenge; binds the welcome to this socket.
        nonce: Nonce,
        /// HMAC of both challenges, launch id and role (never the key).
        proof: PresentedToken,
    },
    /// Call a method on a node.
    Call {
        /// The browser's id for the call, echoed in the reply.
        id: CallId,
        /// The node.
        node: NodeKey,
        /// The protocol's own `Call` (`{ "method", "params" }`). Kept as
        /// text until the frame is read, then decoded into the typed
        /// `Call`, so that a call this bridge cannot read is still
        /// answered, by id.
        #[cfg_attr(feature = "ts", ts(as = "vessel_proto::Call"))]
        call: RawJson,
    },
    /// Follow a session from a record on.
    Follow {
        /// The browser's id for the stream.
        stream: StreamId,
        /// The node.
        node: NodeKey,
        /// The session.
        session: SessionId,
        /// The first record wanted.
        from: EventSeq,
    },
    /// Follow a terminal (protocol 11, ADR 0031) from a record on: its
    /// records and its stream's states come as a followed session's do.
    FollowTerminal {
        /// The browser's id for the stream.
        stream: StreamId,
        /// The node.
        node: NodeKey,
        /// The terminal.
        terminal: TerminalId,
        /// The first record wanted.
        from: EventSeq,
    },
    /// Stop following.
    Unfollow {
        /// The stream.
        stream: StreamId,
    },
    /// Ask the bridge itself.
    Op {
        /// The browser's id for the request, echoed in the answer.
        id: CallId,
        /// What to do.
        op: Op,
    },
}

/// A frame to the browser.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerFrame {
    /// The server's fresh challenge, before authentication.
    Challenge {
        /// This UI launch.
        launch: LaunchId,
        /// A fresh random value, used on this connection only.
        nonce: Nonce,
    },
    /// Both peers prove their key before the browser opens the bridge.
    Welcome {
        /// The server's version (`vessel`'s).
        version: String,
        /// Must match the challenge and the tab's previous launch.
        launch: LaunchId,
        /// HMAC including the client nonce and any issued session key.
        proof: PresentedToken,
        /// An independent key, only when redeeming the one-time code.
        session: Option<PresentedToken>,
    },
    /// The answer to a call.
    Reply {
        /// The call's id.
        id: CallId,
        /// The node's reply, or why there is none.
        outcome: CallOutcome,
    },
    /// The answer to an op.
    OpReply {
        /// The op's id.
        id: CallId,
        /// Its result, or why there is none.
        outcome: OpOutcome,
    },
    /// A node's connection changed.
    Node {
        /// The node.
        node: NodeKey,
        /// Where its connection is now.
        status: NodeStatus,
    },
    /// A record of a followed session.
    Event {
        /// The stream.
        stream: StreamId,
        /// The record (gap-free and in order, within one stream).
        record: EventRecord,
    },
    /// A followed session's stream changed.
    Stream {
        /// The stream.
        stream: StreamId,
        /// What happened.
        state: StreamState,
    },
    /// A frame the bridge could not read, and could not answer by id.
    Unreadable {
        /// Why.
        message: String,
    },
    /// The preferences as the bridge publishes them: once after the
    /// welcome, then after every change (from any tab, or found in the
    /// file). The latest only: a tab that reads slowly skips states it
    /// would have overwritten anyway.
    Preferences {
        /// The state, with its revision.
        #[cfg_attr(feature = "ts", ts(as = "PreferencesState"))]
        preferences: Arc<PreferencesState>,
    },
}

/// A call's answer.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CallOutcome {
    /// The node's reply (boxed: a reply is several hundred bytes, a
    /// failure a few words).
    Ok(Box<Reply>),
    /// No reply.
    Err(Failure),
}

/// Why a call has no reply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Failure {
    /// The node answered with an error.
    Remote {
        /// The node's error.
        error: WireError,
    },
    /// The node's protocol is older than the method; it was not sent.
    NotInProtocol {
        /// The method.
        method: Method,
        /// The version it needs.
        needs: ProtocolVersion,
        /// The node's.
        negotiated: ProtocolVersion,
    },
    /// The connection closed with the call in flight. It may or may not
    /// have run on the node, and it is not repeated.
    Disconnected,
    /// The node cannot be reached now.
    Unreachable {
        /// Why.
        message: String,
    },
    /// The bridge does not pass this call on: not a call it can read, a
    /// method a browser does not call, or too many calls at once.
    Refused {
        /// Why.
        message: String,
    },
    /// Anything else (a broken peer).
    Failed {
        /// What happened.
        message: String,
    },
}

/// An op's answer.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum OpOutcome {
    /// It worked.
    Ok(OpResult),
    /// It did not.
    Err {
        /// Which kind of failure, for the page to act on without reading
        /// `message`.
        reason: OpFailure,
        /// Why, in words.
        message: String,
    },
}

/// Why an op has no result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum OpFailure {
    /// The op could not be read: not one the bridge knows, or a node key
    /// or book name that is not one (this machine's node where only a book
    /// entry will do, a name the book would not take). Nothing was done.
    Unreadable,
    /// Too many calls and ops at once; nothing was done.
    Busy,
    /// It was tried and did not work: the book's own refusal (no such
    /// node, a name taken), a link that does not decode, a node that
    /// cannot be reached.
    Failed,
}

/// What the browser asks the bridge itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// The nodes to show: the book's and this machine's.
    Nodes,
    /// Connect to a node again now (after it failed, or while waiting to
    /// retry).
    Retry {
        /// The node.
        node: NodeKey,
    },
    /// What a pairing link offers, before pairing with it. Nothing is
    /// sent anywhere.
    Inspect {
        /// The link (`vessel://pair#...`).
        link: SecretText,
    },
    /// Pair this machine with the node a link offers and add it to the
    /// book.
    Pair {
        /// The name to give it in the book.
        name: BookName,
        /// The link.
        link: SecretText,
    },
    /// Draw `text` as a QR code.
    Qr {
        /// What to encode (a pairing link).
        text: SecretText,
    },
    /// Remove a node from the book (`vessel nodes remove`) and close the
    /// bridge's connection to it; streams following it end as
    /// [`StreamStop::Removed`]. This machine's node is not in the book,
    /// so it is not a [`BookName`] and cannot be named here.
    Forget {
        /// The node.
        node: BookName,
    },
    /// Give a node in the book another name (`vessel nodes rename`); the
    /// default follows it. Its connection and streams end under the old
    /// name, as when it is forgotten: follow it again under the new one.
    Rename {
        /// The node.
        node: BookName,
        /// Its new name. The same name changes nothing.
        name: BookName,
    },
    /// Make a node the one commands use when none is named (`vessel nodes
    /// default`); [`NodeKey::HERE`] clears the book's default, which is
    /// what makes this machine's node the default (`--none`).
    Default {
        /// The node.
        node: NodeKey,
    },
    /// The preferences, read from where they are kept again (a change
    /// another process made there is pushed to every tab).
    Preferences,
    /// Change one setting; the answer is the whole document after it.
    Prefer {
        /// The change.
        change: PreferenceChange,
    },
    /// Every draft, read from where they are kept again.
    Drafts,
    /// One draft, read from where it is kept again.
    Draft {
        /// Whose.
        key: DraftKey,
    },
    /// Keep a draft (the empty text removes it). Answered when queued; the
    /// write follows, at most once a second.
    SaveDraft {
        /// Whose.
        key: DraftKey,
        /// The text.
        text: DraftText,
    },
}

/// An op's result.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum OpResult {
    /// [`Op::Nodes`].
    Nodes {
        /// The nodes, in the order to show them.
        nodes: Vec<BookNode>,
    },
    /// [`Op::Retry`].
    Retry,
    /// [`Op::Inspect`].
    Inspect {
        /// The offer.
        offer: PairingOffer,
        /// The fingerprint of its key, to compare with what the node
        /// shows.
        fingerprint: String,
    },
    /// [`Op::Pair`].
    Pair {
        /// The node as the book has it now.
        node: BookNode,
    },
    /// [`Op::Qr`].
    Qr {
        /// The code as an SVG document.
        svg: String,
    },
    /// [`Op::Forget`].
    Forget {
        /// The nodes as the book has them now, in the order to show them.
        nodes: Vec<BookNode>,
    },
    /// [`Op::Rename`].
    Rename {
        /// The nodes as the book has them now, in the order to show them.
        nodes: Vec<BookNode>,
    },
    /// [`Op::Default`].
    Default {
        /// The nodes as the book has them now, in the order to show them
        /// (the default moved: two entries changed).
        nodes: Vec<BookNode>,
    },
    /// [`Op::Preferences`].
    Preferences {
        /// The state now.
        #[cfg_attr(feature = "ts", ts(as = "PreferencesState"))]
        preferences: Arc<PreferencesState>,
    },
    /// [`Op::Prefer`].
    Prefer {
        /// The state after the change (also pushed to every tab).
        #[cfg_attr(feature = "ts", ts(as = "PreferencesState"))]
        preferences: Arc<PreferencesState>,
    },
    /// [`Op::Drafts`].
    Drafts {
        /// Every draft.
        drafts: DraftsState,
    },
    /// [`Op::Draft`].
    Draft {
        /// Its text; `None` when there is no such draft.
        text: Option<DraftText>,
    },
    /// [`Op::SaveDraft`].
    SaveDraft,
}

/// A node the UI lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BookNode {
    /// How the browser names it.
    pub key: NodeKey,
    /// The book's name for it, or a description of this machine's node.
    pub label: String,
    /// Where it is, as the book has it (an address, an SSH target, a
    /// paired node's key fingerprint).
    pub address: String,
    /// How it is reached.
    pub transport: Transport,
    /// Whether commands use it when none is named.
    pub default: bool,
    /// Its id, once seen.
    pub id: Option<NodeId>,
}

/// How a node is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    /// A websocket and a token.
    Ws,
    /// SSH.
    Ssh,
    /// A paired node's routes (direct or through relays), end to end
    /// encrypted.
    Paired,
}

/// Where a node's connection is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum NodeStatus {
    /// Nothing has asked for it yet.
    Idle,
    /// Dialling it.
    Connecting,
    /// Connected.
    Connected {
        /// The node, as it introduced itself.
        node: NodeInfo,
        /// The protocol version in use.
        protocol: ProtocolVersion,
    },
    /// The connection is down; dialling again in `retry_in_ms`.
    Reconnecting {
        /// Why it is down.
        message: String,
        /// Which attempt comes next, from 1.
        attempt: u32,
        /// Milliseconds until it.
        retry_in_ms: u64,
    },
    /// It cannot be reached in a way that waiting does not fix (no such
    /// node, a wrong token, a changed identity); `Op::Retry`, or the next
    /// call, tries again.
    Failed {
        /// Why.
        message: String,
    },
}

/// What happened to a followed session's stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum StreamState {
    /// Records `expected..resumed_at` were pruned by the node; the stream
    /// continues at `resumed_at`.
    Missing {
        /// The first record that was wanted.
        expected: EventSeq,
        /// Where it continues.
        resumed_at: EventSeq,
    },
    /// The node's connection is down; the stream resumes when it is back.
    Reconnecting {
        /// Why.
        message: String,
    },
    /// Connected again; the next record is `at`, nothing repeated.
    Resumed {
        /// The next record.
        at: EventSeq,
    },
    /// The session ended; no more records.
    Ended,
    /// The stream stopped (the session is gone, the node cannot be
    /// reached again, it left the book, or the UI is stopping); `reason`
    /// says which kind, `message` the details. Follow again to retry.
    Failed {
        /// Why.
        message: String,
        /// Which kind of stop, for the page to act on without reading
        /// `message`.
        reason: StreamStop,
    },
}

/// Why a followed session's stream stopped for good.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum StreamStop {
    /// The node left the book (forgotten or renamed from the page, or
    /// removed with `vessel nodes`); its connection is closed. Following
    /// it again fails unless it is back in the book.
    Removed,
    /// `vessel ui` is stopping.
    Stopping,
    /// Anything else: the session is gone, the node cannot be reached
    /// again, or the bridge refused the follow.
    Error,
}

/// `GET /api/status`: what is serving.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Status {
    /// The server's version (`vessel`'s).
    pub version: String,
    /// Whether the real UI is embedded (else the placeholder page).
    pub bundled: bool,
}

/// The browser's id for a call or an op.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct CallId(u64);

impl CallId {
    /// An id.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// The browser's id for a followed session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct StreamId(u64);

impl StreamId {
    /// An id.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// A token as a browser presented it: not yet checked, never printed.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct PresentedToken(String);

impl PresentedToken {
    /// The token as given.
    #[must_use]
    pub fn new(token: String) -> Self {
        Self(token)
    }

    /// The text, for comparing.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PresentedToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PresentedToken(..)")
    }
}

/// Text from the page that may be a credential: a pairing link (to
/// inspect or pair with), or the text of a QR code (a pairing link to
/// show). A link holds a single-use invitation until it is redeemed, so
/// `Debug` never prints it (an `Op` or a `ClientFrame` in a log line, a
/// test failure or a panic message shows a placeholder), and there is no
/// `Display`. It is not checked here: explaining a bad link is
/// `inspect`'s job, with the book's own parser.
///
/// [`SecretText::expose`] is called only where the text is parsed or
/// drawn (`ops`: inspect, pair, qr; the source's pairing); nothing on
/// those paths logs it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct SecretText(String);

impl SecretText {
    /// The text as the page sent it.
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self(text)
    }

    /// The text, to parse or draw; never to log or format into a message.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretText(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pairing link is a credential until redeemed: whatever formats an
    /// op or a frame with `{:?}` (a log line, a failing test, a panic)
    /// shows a placeholder, whether the frame was built or decoded.
    #[test]
    fn debug_never_prints_a_link_or_a_qr_text() {
        const LINK: &str = "vessel://pair#c2VjcmV0LWludml0YXRpb24";
        let pair = Op::Pair {
            name: "lab".parse().unwrap(),
            link: SecretText::new(LINK.to_owned()),
        };
        let frame = ClientFrame::Op {
            id: CallId::new(1),
            op: pair.clone(),
        };
        let decoded: ClientFrame = serde_json::from_str(&format!(
            r#"{{"type":"op","id":2,"op":{{"op":"inspect","link":"{LINK}"}}}}"#
        ))
        .unwrap();
        let qr = Op::Qr {
            text: SecretText::new(LINK.to_owned()),
        };
        for text in [
            format!("{pair:?}"),
            format!("{frame:?}"),
            format!("{decoded:?}"),
            format!("{qr:?}"),
            format!("{frame:#?}"),
        ] {
            assert!(!text.contains("c2VjcmV0"), "{text}");
            assert!(text.contains("<redacted>"), "{text}");
        }
        // It still travels: the page's text, as sent.
        let json = serde_json::to_string(&frame).unwrap();
        assert!(json.contains(LINK), "{json}");
    }
}

/// The bridge's frames in TypeScript, beside the protocol's (ADR 0017).
#[cfg(feature = "ts")]
pub mod ts {
    use vessel_proto::ts::{Declarations, NameClash};

    use super::{ClientFrame, ServerFrame, Status};

    /// Where the web UI keeps them, from the workspace's root.
    pub const BRIDGE_FILE: &str = "web/src/wire/bridge.ts";

    /// The bridge's module: its frames, importing the protocol's types
    /// from `./proto`.
    ///
    /// # Errors
    /// Two types share a TypeScript name.
    pub fn module() -> Result<String, NameClash> {
        let proto = Declarations::wire()?;
        let mut bridge = Declarations::new();
        bridge.add::<ClientFrame>()?;
        bridge.add::<ServerFrame>()?;
        bridge.add::<Status>()?;
        Ok(bridge.render(Some((&proto, "./proto"))))
    }

    #[cfg(test)]
    mod tests {
        use std::path::Path;

        use vessel_proto::ts::{ALIASES, check_file, check_strict};

        use super::*;

        #[test]
        fn every_primitive_alias_is_classified_and_every_entry_still_declared() {
            let proto = Declarations::wire().unwrap();
            let mut bridge = Declarations::new();
            bridge.add::<ClientFrame>().unwrap();
            bridge.add::<ServerFrame>().unwrap();
            bridge.add::<Status>().unwrap();
            let unclassified: Vec<&str> = bridge.unclassified().collect();
            assert!(
                unclassified.is_empty(),
                "classify these in vessel-proto's ts/aliases.rs: {unclassified:?}"
            );
            let declared: Vec<&str> = proto.aliases().chain(bridge.aliases()).collect();
            let stale: Vec<&str> = ALIASES
                .iter()
                .map(|(name, _alias)| *name)
                .filter(|name| !declared.contains(name))
                .collect();
            assert!(
                stale.is_empty(),
                "no longer declared, remove from ts/aliases.rs: {stale:?}"
            );
        }

        #[test]
        fn typescript_declarations_are_current() {
            let generated = module().unwrap();
            check_strict(&generated).unwrap();
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(BRIDGE_FILE);
            check_file(&path, &generated).unwrap();
        }
    }
}
