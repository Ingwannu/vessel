//! Files a person sends with a prompt (protocol 10, ADR 0025).
//!
//! [Decision Log]
//! - Purpose: images and files go to the agent with a prompt, offered by
//!   what the agent says it takes, from any client (a phone included), and
//!   the conversation shows them to every other client afterwards.
//! - Constraint: frames are at most 16 MiB, and a call in flight holds one
//!   of its connection's slots; a 50 MiB file cannot be one frame, and one
//!   large frame would hold up every other reply on the connection.
//! - Alternatives: inline base64 in the prompt (agent-orchestrator: 25
//!   MiB of images in one request, no files); a signed upload URL outside
//!   the RPC (GOAT: a second listener and its own auth, which the relay
//!   and the SSH transport do not carry); chunked upload calls (Paseo,
//!   128 KiB binary chunks).
//! - Choice: chunked calls over the same connection, the reverse of
//!   protocol 6's `transfer.read`: `attachment.begin` declares the file,
//!   `attachment.write` sends it in order in pieces of at most
//!   [`AttachmentLimits::chunk`], `attachment.finish` checks it and hands
//!   back an [`AttachmentRef`], which a prompt then names. Works through
//!   every transport and the fleet's envelope, and other calls keep
//!   flowing between pieces.
//! - The bytes are the session's (under the node's data directory) and go
//!   with it; the log keeps the reference only, so a subscriber never
//!   receives an image it did not ask for.

use serde::{Deserialize, Serialize};

use super::text::{AttachmentName, MimeType};
use crate::id::{AttachmentId, SessionId, UploadId};

/// What an attachment is to the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AttachmentKind {
    /// An image the model sees (`image/png`, `image/jpeg`, `image/gif`,
    /// `image/webp`), for agents that take images.
    Image,
    /// Audio (`audio/*`), for agents that take audio.
    Audio,
    /// Any other file, handed to the agent as a link to where the node
    /// keeps it; the agent reads it with its own tools.
    File,
}

impl AttachmentKind {
    /// The kind a file of type `mime` is.
    #[must_use]
    pub fn of(mime: &MimeType) -> Self {
        match mime.as_str() {
            "image/png" | "image/jpeg" | "image/gif" | "image/webp" => Self::Image,
            _ if mime.top() == "audio" => Self::Audio,
            _ => Self::File,
        }
    }

    /// The largest attachment of this kind, in bytes.
    #[must_use]
    pub const fn max_size(self, limits: &AttachmentLimits) -> u64 {
        match self {
            Self::Image => limits.image,
            Self::Audio => limits.audio,
            Self::File => limits.file,
        }
    }

    /// Whether its bytes go to the agent inside the prompt (and so count
    /// against [`AttachmentLimits::inline_per_prompt`]).
    #[must_use]
    pub const fn is_inline(self) -> bool {
        match self {
            Self::Image | Self::Audio => true,
            Self::File => false,
        }
    }
}

/// An attachment a prompt names, as `attachment.finish` returned it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AttachmentRef {
    /// Its id within its session.
    pub id: AttachmentId,
    /// Its file name.
    pub name: AttachmentName,
    /// What it is to the agent.
    pub kind: AttachmentKind,
    /// Its type.
    pub mime: MimeType,
    /// Its size in bytes.
    pub size: u64,
}

/// How large attachments may be, as a node enforces it; a client reads
/// them (`session.conversation`) before it uploads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AttachmentLimits {
    /// Largest image, in bytes.
    pub image: u64,
    /// Largest audio file, in bytes.
    pub audio: u64,
    /// Largest other file, in bytes.
    pub file: u64,
    /// Most attachments one prompt names.
    pub per_prompt: u32,
    /// Most bytes of images and audio one prompt carries to the agent.
    pub inline_per_prompt: u64,
    /// Most bytes of attachments one session keeps.
    pub per_session: u64,
    /// Largest piece `attachment.write` takes, in bytes.
    pub chunk: u32,
}

impl AttachmentLimits {
    /// The limits this build enforces: images and audio of 10 MiB
    /// (agent-orchestrator's and GOAT's image limit), other files of 50
    /// MiB (GOAT's and Paseo's), 16 a prompt with 25 MiB of images and
    /// audio (agent-orchestrator's total), 100 MiB a session, pieces of 1
    /// MiB (`BlobChunk::MAX_LEN`).
    pub const DEFAULT: Self = Self {
        image: 10 << 20,
        audio: 10 << 20,
        file: 50 << 20,
        per_prompt: 16,
        inline_per_prompt: 25 << 20,
        per_session: 100 << 20,
        chunk: 1 << 20,
    };

    /// The most uploads open on a node at once.
    pub const OPEN_UPLOADS: usize = 16;
    /// The most of those one caller holds, so one device cannot hold
    /// every place for the ten idle minutes an upload is kept.
    pub const OPEN_UPLOADS_PER_CALLER: usize = 4;
    /// The most bytes of attachments one node keeps, all sessions
    /// together.
    pub const PER_NODE: u64 = 4 << 30;
}

/// `attachment.begin`: declare a file to upload to a live structured
/// session. Needs `operate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BeginUpload {
    /// Which session.
    pub session: SessionId,
    /// The file's name.
    pub name: AttachmentName,
    /// Its type.
    pub mime: MimeType,
    /// Its size in bytes.
    pub size: u64,
}

/// Reply to `attachment.begin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Upload {
    /// The upload, for `attachment.write` and `attachment.finish`.
    pub upload: UploadId,
    /// Largest piece a write takes, in bytes.
    pub chunk: u32,
}

/// `attachment.write`: the next piece of an upload. Only the caller that
/// began it may write. Needs `operate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WriteUpload {
    /// Which upload.
    pub upload: UploadId,
    /// Where the piece starts: the bytes received so far.
    pub offset: u64,
    /// The piece, base64 on the wire.
    #[serde(with = "crate::base64_bytes")]
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub data: bytes::Bytes,
}

/// Reply to `attachment.write`: how much of the file the node has. A
/// piece repeated after a lost reply (it ends at or before this) is
/// answered again without being written twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Written {
    /// Bytes received so far; the next piece starts here.
    pub received: u64,
}

/// `attachment.finish`: check an upload and keep it with its session.
/// Needs `operate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FinishUpload {
    /// Which upload.
    pub upload: UploadId,
}

/// `attachment.read`: a piece of a session's attachment, from `offset`
/// (at most `BlobChunk::MAX_LEN` bytes). Needs `read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadAttachment {
    /// Which session.
    pub session: SessionId,
    /// Which attachment.
    pub attachment: AttachmentId,
    /// Where to start.
    pub offset: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kind_follows_the_type() {
        let kind = |mime: &str| AttachmentKind::of(&mime.parse().unwrap());
        assert_eq!(kind("image/png"), AttachmentKind::Image);
        assert_eq!(kind("image/webp"), AttachmentKind::Image);
        assert_eq!(kind("image/svg+xml"), AttachmentKind::File, "never inline");
        assert_eq!(kind("audio/wav"), AttachmentKind::Audio);
        assert_eq!(kind("application/pdf"), AttachmentKind::File);
        let limits = AttachmentLimits::DEFAULT;
        assert_eq!(AttachmentKind::File.max_size(&limits), 50 << 20);
        assert!(AttachmentKind::Image.is_inline());
        assert!(!AttachmentKind::File.is_inline());
    }

    #[test]
    fn a_piece_is_base64() {
        let write = WriteUpload {
            upload: UploadId::random(),
            offset: 0,
            data: bytes::Bytes::from_static(b"hi"),
        };
        let json = serde_json::to_value(&write).unwrap();
        assert_eq!(json["data"], "aGk=");
    }
}
