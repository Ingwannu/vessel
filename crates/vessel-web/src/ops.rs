//! The ops: what a tab asks the bridge itself rather than a node (the
//! book's nodes, a retry, pairing, a QR code, and the book's edits).
//! [`route`] sends the page's preferences and drafts elsewhere: they are
//! `keep`'s, answered in order by the task that owns each file rather
//! than as one of the tab's concurrent tasks (ADR 0028).
//!
//! [Decision Log] The book's edits from the page (`forget`, `rename`,
//! `default`), milestone 10, track E
//! - Purpose: the page could add a machine (`pair`) and had no way back
//!   out, nor to rename one or make it the default; a one-way door.
//! - Constraints: the book is the CLI's file, written by `BookFile` and
//!   nothing else; a change made with `vessel nodes` while the page is
//!   open must survive the page's next edit; a forgotten node must not
//!   keep a connection (or an `ssh` process) alive.
//! - Alternatives: the bridge editing the book itself (a second writer
//!   and a dependency on the book's format here); keeping a copy of the
//!   book in the registry and writing it back (a CLI edit made meanwhile
//!   would be overwritten); one `edit` op carrying a closed enum of edits
//!   (no simpler for the page, and the replies differ).
//! - Choice: three ops on the [`NodeSource`] seam, which the CLI
//!   implements with `BookFile`'s own `remove`, `rename` and
//!   `set_default`: each opens the file right before the edit, so nothing
//!   is written from a stale copy. Then the registry reads the book again
//!   ([`Registry::nodes`]), which cancels every entry no longer in it:
//!   its supervisor stops, its connection closes, and streams following
//!   it end as [`StreamStop::Removed`](crate::wire::StreamStop::Removed). The reply is the book as it is
//!   then, so the page needs no second `nodes` round trip.
//! - Refusals: an op reaches these edits exactly where it reaches `pair`
//!   (after the socket's mutual proof, within the tab's `MAX_CALLS`).
//!   Node names are parsed when the frame is decoded (`wire::BookName`):
//!   this machine's node, which is not in the book, and names the book
//!   would not take never reach an op ([`OpFailure::Unreadable`]);
//!   unknown nodes and taken names are the book's own errors
//!   ([`OpFailure::Failed`]).
//! - Costs: a rename ends the node's connection and streams under the old
//!   name, as a forget does, and the page follows again under the new
//!   one. Moving a live entry between keys would have to re-key a running
//!   supervisor and every stream's view of it, for a rare edit. Two
//!   processes editing the book in the same instant can still lose one
//!   edit (`BookFile` has no lock, by its own decision).

use std::sync::Arc;

use vessel_proto::PairingLink;

use crate::calls::failure_text;
use crate::keep::{DraftsAsk, KeptOp, PrefsAsk};
use crate::registry::{Registry, chain};
use crate::source::NodeSource;
use crate::wire::{BookName, BookNode, NodeKey, Op, OpFailure, OpResult, SecretText};

/// The longest text drawn as a QR code (a pairing link is a few hundred
/// bytes; the code's own limit is about 2900).
const MAX_QR_TEXT: usize = 2048;

/// Why an op has no result: the page's `OpOutcome::Err`.
#[derive(Debug)]
pub(crate) struct OpError {
    pub(crate) reason: OpFailure,
    pub(crate) message: String,
}

impl OpError {
    /// It was tried and did not work.
    pub(crate) const fn failed(message: String) -> Self {
        Self {
            reason: OpFailure::Failed,
            message,
        }
    }

    /// Too much at once; nothing was done.
    pub(crate) fn busy() -> Self {
        Self {
            reason: OpFailure::Busy,
            message: "too many requests at once".to_owned(),
        }
    }
}

/// Where an op is answered.
#[derive(Debug)]
pub(crate) enum Routed {
    /// By the bridge, as a task of the tab's (within its `MAX_CALLS`).
    Bridge(BridgeOp),
    /// By the task that keeps the page's preferences or drafts, in the
    /// order the tab sent them (`keep`).
    Kept(KeptOp),
}

/// An op [`answer`] answers: the book's nodes and edits, pairing, QR
/// codes ([`Op`]'s variants of the same names).
#[derive(Debug)]
pub(crate) enum BridgeOp {
    CheckSubmit {
        node: NodeKey,
        expected_node: vessel_proto::NodeId,
        request: Box<vessel_proto::timeline::SubmitPrompt>,
    },
    Nodes,
    Retry {
        node: NodeKey,
    },
    Inspect {
        link: SecretText,
    },
    Pair {
        name: BookName,
        link: SecretText,
    },
    Qr {
        text: SecretText,
    },
    Forget {
        node: BookName,
    },
    Rename {
        node: BookName,
        name: BookName,
    },
    Default {
        node: NodeKey,
    },
}

/// Which way `op` goes. Exhaustive, so a new op must choose.
pub(crate) fn route(op: Op) -> Routed {
    let bridge = Routed::Bridge;
    let kept = Routed::Kept;
    match op {
        Op::Preferences => kept(KeptOp::Preferences(PrefsAsk::Read)),
        Op::Prefer { change } => kept(KeptOp::Preferences(PrefsAsk::Change(change))),
        Op::Drafts => kept(KeptOp::Drafts(DraftsAsk::All)),
        Op::Draft { key } => kept(KeptOp::Drafts(DraftsAsk::One(key))),
        Op::SaveDraft { key, text } => kept(KeptOp::Drafts(DraftsAsk::Save(key, text))),
        Op::CheckSubmit {
            node,
            expected_node,
            request,
        } => bridge(BridgeOp::CheckSubmit {
            node,
            expected_node,
            request,
        }),
        Op::Nodes => bridge(BridgeOp::Nodes),
        Op::Retry { node } => bridge(BridgeOp::Retry { node }),
        Op::Inspect { link } => bridge(BridgeOp::Inspect { link }),
        Op::Pair { name, link } => bridge(BridgeOp::Pair { name, link }),
        Op::Qr { text } => bridge(BridgeOp::Qr { text }),
        Op::Forget { node } => bridge(BridgeOp::Forget { node }),
        Op::Rename { node, name } => bridge(BridgeOp::Rename { node, name }),
        Op::Default { node } => bridge(BridgeOp::Default { node }),
    }
}

/// Answer an op.
pub(crate) async fn answer<S: NodeSource>(
    registry: &Arc<Registry<S>>,
    op: BridgeOp,
) -> Result<OpResult, OpError> {
    let failed = |message| OpError::failed(message);
    let source = |error: S::Error| OpError::failed(chain(&error));
    match op {
        BridgeOp::CheckSubmit {
            node,
            expected_node,
            request,
        } => Ok(OpResult::CheckSubmit {
            check: crate::submission::check(registry, &node, expected_node, *request).await,
        }),
        BridgeOp::Nodes => book_now(registry)
            .await
            .map(|nodes| OpResult::Nodes { nodes }),
        BridgeOp::Retry { node } => registry
            .retry(&node)
            .await
            .map(|()| OpResult::Retry)
            .map_err(|failure| failed(failure_text(&failure))),
        BridgeOp::Inspect { link } => {
            let link: PairingLink = link
                .expose()
                .parse()
                .map_err(|error| failed(chain(&error)))?;
            let offer = link.decode().map_err(|error| failed(chain(&error)))?;
            let fingerprint = vessel_client::fingerprint(&offer.key);
            Ok(OpResult::Inspect { offer, fingerprint })
        }
        BridgeOp::Pair { name, link } => registry
            .source()
            .pair(name.node_name(), &link)
            .await
            .map(|node| OpResult::Pair { node })
            .map_err(source),
        BridgeOp::Qr { text } => qr(text.expose())
            .map(|svg| OpResult::Qr { svg })
            .map_err(failed),
        BridgeOp::Forget { node } => {
            registry
                .source()
                .forget(node.node_name())
                .await
                .map_err(source)?;
            book_now(registry)
                .await
                .map(|nodes| OpResult::Forget { nodes })
        }
        BridgeOp::Rename { node, name } => {
            registry
                .source()
                .rename(node.node_name(), name.node_name())
                .await
                .map_err(source)?;
            book_now(registry)
                .await
                .map(|nodes| OpResult::Rename { nodes })
        }
        BridgeOp::Default { node } => {
            registry.source().set_default(&node).await.map_err(source)?;
            book_now(registry)
                .await
                .map(|nodes| OpResult::Default { nodes })
        }
    }
}

/// The book after an edit; reading it is also what releases the entries
/// of nodes no longer in it (their connections and streams).
async fn book_now<S: NodeSource>(registry: &Registry<S>) -> Result<Vec<BookNode>, OpError> {
    registry
        .nodes()
        .await
        .map_err(|failure| OpError::failed(failure_text(&failure)))
}

/// `text` as a QR code, an SVG document.
fn qr(text: &str) -> Result<String, String> {
    if text.len() > MAX_QR_TEXT {
        return Err(format!(
            "too long for a QR code ({} bytes; at most {MAX_QR_TEXT})",
            text.len()
        ));
    }
    let code = qrcode::QrCode::new(text.as_bytes()).map_err(|error| error.to_string())?;
    Ok(code
        .render::<qrcode::render::svg::Color<'_>>()
        .min_dimensions(240, 240)
        .quiet_zone(true)
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_qr_code_is_an_svg_and_long_text_is_refused() {
        let svg = qr("vessel://pair#abc").unwrap();
        assert!(svg.starts_with("<?xml") && svg.contains("<svg"), "{svg}");
        assert!(qr(&"x".repeat(MAX_QR_TEXT + 1)).is_err());
    }
}
