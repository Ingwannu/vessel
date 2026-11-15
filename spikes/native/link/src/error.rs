//! The existing public link failure type, separate from the transport actor.
//!
//! [Decision Log]
//! - Purpose: make a small existing vocabulary independently readable while
//!   feature registrations leave the listed root module smaller.
//! - Constraint: root public path, enum body, diagnostics and actor stay.
//! - Alternatives: enlarge the ratchet or move the whole command/event bus.
//! - Choice: move this type unchanged with its original root public re-export.
//! - Why and cost: a smaller existing boundary; one additional module only.

// Documentation resolves the original root event link without changing the
// moved enum's bytes or introducing a runtime import.
#[cfg(doc)]
use crate::Event;

/// Something the link could not do.
#[derive(Debug, Clone, thiserror::Error)]
pub enum LinkError {
    /// No home or configuration directory.
    #[error("no configuration directory")]
    NoHome,
    /// The book could not be read or written.
    #[error("the node book: {0}")]
    Book(String),
    /// The book does not read: where and what, never its text (it may
    /// hold a token). The app says it in its own words
    /// ([`Event::BookUnreadable`]).
    #[error("the node book {} does not read", .0.path)]
    BookFault(vessel_app_view::files::FileFault),
    /// A websocket node without a token file.
    #[error("no token file in the book for this node")]
    NoToken,
    /// The token file does not hold a token.
    #[error("the token: {0}")]
    Token(String),
    /// A paired node without a credential file.
    #[error("no device credential in the book for this node")]
    NoCredential,
    /// The credential could not be read or written.
    #[error("the device credential: {0}")]
    Credential(String),
    /// Reading a file failed.
    #[error("reading a file: {0}")]
    Io(String),
    /// The node could not be reached.
    #[error("{0}")]
    Connect(String),
    /// An SSH node (not in this build).
    #[error("SSH routes are not in this build yet; pair the node instead")]
    SshNotYet,
    /// The name is not a valid machine name.
    #[error("the name: {0}")]
    Name(String),
    /// A call to the node failed.
    #[error("{0}")]
    Call(String),
    /// The machine is not connected.
    #[error("{0} is not connected")]
    NotConnected(String),
}
