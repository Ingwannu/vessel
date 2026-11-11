//! [`NodeSource`]: where the bridge's nodes come from.
//!
//! [Decision Log]
//! - Purpose: the browser names nodes; something must turn a name into a
//!   connection. That needs the CLI's knowledge: the book, token files,
//!   SSH, device credentials, pinned ids (ADR 0017: the browser never
//!   holds any of it).
//! - Alternatives: the bridge resolving book entries itself (it would
//!   duplicate `vessel-cli`'s connect logic and need `vessel-remote`); a
//!   boxed `dyn` source (one production implementation and test fakes
//!   need no dynamic dispatch, AGENTS.md section 2).
//! - Choice: a small generic trait. `vessel-cli` implements it with the
//!   book and its `NodeConnector`; the bridge's tests with in-process
//!   nodes. The bridge only routes: what a node allows is the node's to
//!   decide.
//! - The book's edits (pair, forget, rename, default) are the source's
//!   too, so the book has one writer, the CLI's (`BookFile`), and the
//!   page cannot do to the book what `vessel nodes` cannot.
//! - Where the page's preferences and drafts are kept is the source's to
//!   say too ([`NodeSource::preferences`], ADR 0025): only the CLI knows
//!   `--config-dir`, `--data-dir` and where the book is, and a library
//!   never works out the user's home itself (AGENTS.md section 8).

use std::future::Future;
use std::path::PathBuf;

use vessel_client::Connector;
use vessel_client::book::NodeName;
use vessel_proto::NodeId;

use crate::wire::{BookNode, NodeKey, SecretText};

/// The nodes `vessel ui` offers, and how to reach each.
pub trait NodeSource: Send + Sync + 'static {
    /// Opens connections to one node.
    type Connector: Connector + 'static;
    /// Why the source could not answer (a book that does not parse, no
    /// such node, a missing token file). Shown to the user as text.
    type Error: std::error::Error + Send + Sync + 'static;

    /// The nodes to list, in order.
    fn nodes(&self) -> impl Future<Output = Result<Vec<BookNode>, Self::Error>> + Send;

    /// How to reach `node`.
    fn connector(
        &self,
        node: &NodeKey,
    ) -> impl Future<Output = Result<Self::Connector, Self::Error>> + Send;

    /// `node` answered as `id`: a book node seen for the first time gets
    /// its id pinned, as the CLI does on first contact.
    fn seen(&self, node: &NodeKey, id: NodeId) -> impl Future<Output = ()> + Send;

    /// Pair with the node `link` offers and add it to the book as `name`;
    /// the node as the book has it then. The link is a credential until
    /// redeemed: parse it, never log it or echo it in an error.
    fn pair(
        &self,
        name: &NodeName,
        link: &SecretText,
    ) -> impl Future<Output = Result<BookNode, Self::Error>> + Send;

    /// Remove `node` from the book, as `vessel nodes remove` does.
    fn forget(&self, node: &NodeName) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Rename `node` to `to` in the book, as `vessel nodes rename` does
    /// (the same name changes nothing).
    fn rename(
        &self,
        node: &NodeName,
        to: &NodeName,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Make `node` the default, as `vessel nodes default` does;
    /// [`NodeKey::Here`] clears the book's default (`--none`), which makes
    /// this machine's node the default.
    fn set_default(&self, node: &NodeKey) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Where the page's preferences and drafts are kept. A field that is
    /// `None` is kept in memory for this launch. The default keeps both
    /// in memory (a source with no disk, such as the bridge's tests).
    /// Asked once, when `vessel ui` starts; an error is logged once and
    /// both are kept in memory, the page saying why.
    ///
    /// # Errors
    /// Where they would be kept cannot be worked out (no home directory,
    /// say).
    fn preferences(&self) -> Result<PageFiles, Self::Error> {
        Ok(PageFiles::default())
    }
}

/// Where the page's preferences and drafts are kept: full paths of the
/// files, or `None` for memory only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageFiles {
    /// The preferences (`vessel` keeps them beside the node book).
    pub preferences: Option<PathBuf>,
    /// The drafts (`vessel` keeps them in its local data directory).
    pub drafts: Option<PathBuf>,
}
