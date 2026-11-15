//! Why a call about projects, git or files did not answer: the page's
//! `Problem` (`data/outcome.ts`), kept typed so a screen can tell a
//! refusal to write over a change (`conflict`) from a machine that is gone,
//! without reading text. The link makes these from `vessel-client`'s
//! errors; nothing here knows that crate.

use vessel_proto::ErrorCode;

/// A call that did not answer with its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The node answered with an error.
    Node {
        /// Its code.
        code: ErrorCode,
        /// Its words (the node's, in English).
        message: String,
    },
    /// The machine speaks a protocol older than the call.
    Older,
    /// The machine is not connected, or the connection dropped; why.
    Gone(String),
    /// The app's link did not take the ask (its queue full, or stopped):
    /// nothing reached the machine; trying again may work.
    Unsent,
}

impl Problem {
    /// The node's error code, when the node answered with one (`conflict`
    /// for a write the file changed under, `unsupported` for a folder
    /// without git, `not_found`, `unavailable` while git holds a lock).
    #[must_use]
    pub const fn code(&self) -> Option<ErrorCode> {
        match self {
            Self::Node { code, .. } => Some(*code),
            Self::Older | Self::Gone(_) | Self::Unsent => None,
        }
    }
}
