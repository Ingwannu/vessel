//! [`Attachments`]: what a prompt's attachments are to the agent (protocol
//! 10, ADR 0025), handed over with the prompt by whoever loaded them.
//!
//! [Decision Log]
//! - Purpose: a prompt names an attachment by reference
//!   (`ContentBlock::Attachment`), which is what the log keeps; the agent
//!   needs an image's or audio's bytes in the prompt, and a file's
//!   location to read it with its own tools.
//! - Constraint: this crate does no file I/O and knows no data directory;
//!   the node keeps the files and reads them back (never through a link).
//! - Choice: the caller loads each attachment's body (base64 bytes, or a
//!   `file:` URI) and passes them with the content; the mapper puts each
//!   body where its reference is, checked against what the agent said it
//!   takes in its handshake. The bytes are base64 already, so the driver
//!   only moves them into the request.
//! - Alternatives: the driver reading the files (file I/O on the
//!   connection's loop, and the node's read-back rules duplicated here);
//!   bytes inside the content blocks (the log would then carry them).

use std::collections::HashMap;
use std::fmt;

use vessel_proto::AttachmentId;

/// How one attachment reaches the agent.
#[derive(Clone, PartialEq, Eq)]
pub enum AttachmentBody {
    /// An image's or audio's bytes, base64, sent inside the prompt.
    Inline(String),
    /// A file the agent reads itself: a `file:` URI to where it is kept.
    Link(String),
}

/// The bytes are not shown, only how many.
impl fmt::Debug for AttachmentBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inline(data) => write!(f, "Inline({} base64 bytes)", data.len()),
            Self::Link(uri) => f.debug_tuple("Link").field(uri).finish(),
        }
    }
}

/// The bodies of one prompt's attachments, by id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attachments(HashMap<AttachmentId, AttachmentBody>);

impl Attachments {
    /// None: a prompt without attachments.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Give attachment `id` its body.
    pub fn insert(&mut self, id: AttachmentId, body: AttachmentBody) {
        drop(self.0.insert(id, body));
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Take `id`'s body for the request (each is sent once).
    pub(crate) fn take(&mut self, id: AttachmentId) -> Option<AttachmentBody> {
        self.0.remove(&id)
    }
}
