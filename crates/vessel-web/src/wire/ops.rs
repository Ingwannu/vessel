//! Browser-to-bridge operation contracts; execution stays in the existing ops owner.
//! This module owns only values/serde/TS shape, not node calls or stored preferences.

#[cfg(doc)]
use super::StreamStop;
use super::{
    BookName, BookNode, DraftKey, DraftText, DraftsState, NodeKey, PreferenceChange,
    PreferencesState, SecretText,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vessel_proto::timeline::SubmitPrompt;
use vessel_proto::{NodeId, PairingOffer};

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
    /// Check this one prepared submission's protocol and recipient without dispatch.
    CheckSubmit {
        /// Book key currently selected by the page.
        node: NodeKey,
        /// Captured authenticated node identity; a retarget can only refuse.
        expected_node: NodeId,
        /// Its final policy/identity, before attachment references are available.
        request: Box<SubmitPrompt>,
    },
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
    /// Result of the local SDK preflight; no node call was sent.
    CheckSubmit {
        /// Typed support/refusal, never an inferred acceptance.
        check: SubmitCheck,
    },
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

/// Local preflight checks support and recipient only; it grants no input authority.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SubmitCheck {
    /// The actual connection supports this call and still names the captured node.
    Ready {
        /// Frozen recipient for every prepared upload and submission frame.
        node: NodeId,
    },
    /// No submission was sent; legacy compatibility may be selected before preparation.
    Unsupported {
        /// The SDK's typed protocol refusal.
        failure: super::Failure,
    },
    /// No submission was sent, but recipient/connectivity refused preparation.
    Refused {
        /// The actual local refusal, not provider output.
        failure: super::Failure,
    },
}
