//! Why an upload, or a read of an attachment, was refused (protocol 10),
//! and the code each answers with.

use vessel_proto::conversation::MimeType;
use vessel_proto::{AttachmentId, ErrorCode, SessionId, UploadId};
use vessel_store::StoreError;

/// Why an upload or a read of an attachment was refused.
#[derive(Debug, thiserror::Error)]
pub(crate) enum UploadError {
    /// A kind the session's agent does not take (or has not said yet).
    #[error(
        "this session's agent does not take {0} attachments (see `session.conversation`'s abilities)"
    )]
    NotTaken(&'static str),
    /// Larger than a limit.
    #[error("{what} would be {size} bytes, over the limit of {limit} bytes")]
    OverLimit {
        /// What would be over.
        what: &'static str,
        /// Its size with this one.
        size: u64,
        /// The limit.
        limit: u64,
    },
    /// No place for another upload.
    #[error(
        "{limit} uploads are open {}; finish one, or wait for one to be dropped",
        if *.yours { "for this caller" } else { "on this node" }
    )]
    Busy {
        /// The bound reached.
        limit: usize,
        /// Whether it is the caller's own bound.
        yours: bool,
    },
    /// No open upload of that id for this caller (or none at all).
    #[error("no open upload {0}")]
    Unknown(UploadId),
    /// A piece larger than a write takes.
    #[error("a piece is at most {limit} bytes; this one is {len}")]
    PieceTooLarge {
        /// Its length.
        len: usize,
        /// The limit.
        limit: usize,
    },
    /// A piece that does not start where the received bytes end.
    #[error("the next piece starts at {received}, not {offset}")]
    OutOfOrder {
        /// Where it starts.
        offset: u64,
        /// What has come.
        received: u64,
    },
    /// A piece past the declared size.
    #[error("the piece would end at {end}, past the declared size of {size} bytes")]
    PastSize {
        /// Where it would end.
        end: u64,
        /// The declared size.
        size: u64,
    },
    /// `finish` before every byte came.
    #[error("the upload has {received} of its {size} bytes")]
    Incomplete {
        /// What has come.
        received: u64,
        /// The declared size.
        size: u64,
    },
    /// An image whose first bytes are not its type's; the upload is
    /// dropped.
    #[error("the file does not start as {0} does; it was not kept")]
    NotAnImage(MimeType),
    /// No attachment of that id in that session.
    #[error("session {session} has no attachment {id}")]
    NoAttachment {
        /// The session.
        session: SessionId,
        /// The attachment.
        id: AttachmentId,
    },
    /// A read past the end.
    #[error("the attachment is {size} bytes; cannot read from {offset}")]
    PastEnd {
        /// Where the read starts.
        offset: u64,
        /// Its size.
        size: u64,
    },
    /// The staging file could not be made or written.
    #[error("the upload's staging file")]
    Staging(#[source] std::io::Error),
    /// The store refused or failed.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// A blocking task failed.
    #[error("a node task failed")]
    Task(#[source] tokio::task::JoinError),
}

impl UploadError {
    /// The wire code.
    pub(crate) const fn code(&self) -> ErrorCode {
        match self {
            Self::NotTaken(_) => ErrorCode::Unsupported,
            Self::OverLimit { .. }
            | Self::PieceTooLarge { .. }
            | Self::OutOfOrder { .. }
            | Self::PastSize { .. }
            | Self::Incomplete { .. }
            | Self::NotAnImage(_)
            | Self::PastEnd { .. } => ErrorCode::InvalidRequest,
            Self::Busy { .. } => ErrorCode::Unavailable,
            Self::Unknown(_) | Self::NoAttachment { .. } => ErrorCode::NotFound,
            Self::Store(error) => error.code(),
            Self::Staging(_) | Self::Task(_) => ErrorCode::Internal,
        }
    }
}
