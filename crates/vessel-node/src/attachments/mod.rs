//! Attachments (protocol 10, ADR 0025): uploading one to a live
//! structured session (`attachment.begin`, `.write`, `.finish`, with
//! [`uploads`]), reading one back (`attachment.read`), a prompt's
//! references made the node's own records, and the loader sessions ask for
//! a prompt's attachments ([`load`]).
//!
//! ```text
//!  begin ──▶ uploads (staged under uploads/<id>) ◀── write ... ──▶ finish ──▶ store.keep_attachment
//!                                                                        (attachments/<session>/<id>/<name> + row)
//!  session.prompt / queue edit / send ──▶ resolve (the store's record replaces the client's)
//!  a session's actor, sending a prompt ──▶ load ──▶ store.read_attachment (no links, size, SHA-256)
//!  attachment.read ──▶ store.read_attachment, a piece at a time
//! ```
//!
//! [Decision Log]
//! - Purpose: ADR 0025, the contract's "Attachments" and review items 8 to
//!   10. The store owns the files' place and how they are read back (it
//!   owns the data directory); this module decides who may upload what,
//!   when, and how much, and serves the bytes.
//! - An upload belongs to whoever began it ([`Holder`], as a transfer
//!   does): writing or finishing it as anyone else is `not_found`.
//! - The kind follows the type (`AttachmentKind::of`) and must be one the
//!   session's agent said it takes; an agent that has not said yet (still
//!   starting) takes files only, so no image waits for a handshake that
//!   may refuse it.
//! - Finishing checks every byte came and an image's first bytes are its
//!   type's ([`looks_like`]), then that the session still runs (`conflict`
//!   otherwise); the store keeps the file only for a live row.
//! - Read back, a file is hashed once per change ([`Verified`]): a
//!   download's later pieces of an unchanged file skip the hash.

mod error;
pub(crate) mod load;
pub(crate) mod staging;
pub(crate) mod uploads;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use chrono::Utc;
use vessel_proto::conversation::{BeginUpload, MimeType, ReadAttachment, Upload, Written};
use vessel_proto::{
    AttachmentId, AttachmentKind, AttachmentLimits, AttachmentRef, BlobChunk, SessionId,
    SessionMode, UploadId,
};
use vessel_session::SessionError;
use vessel_store::{FileIdentity, KeptAttachment, Store, StoreError};

use crate::error::ServiceError;
use crate::sessions::{Found, Sessions};
use crate::transfers::Holder;
pub(crate) use error::UploadError;
use uploads::{Declared, Uploads};

/// The node's attachments: its open uploads, and the files it verified.
#[derive(Debug)]
pub(crate) struct Attachments {
    store: Store,
    sessions: Arc<Sessions>,
    uploads: Arc<Uploads>,
    verified: Arc<Verified>,
}

impl Attachments {
    pub(crate) const fn new(
        store: Store,
        sessions: Arc<Sessions>,
        uploads: Arc<Uploads>,
        verified: Arc<Verified>,
    ) -> Self {
        Self {
            store,
            sessions,
            uploads,
            verified,
        }
    }

    /// `attachment.begin`.
    pub(crate) async fn begin(
        &self,
        holder: Holder,
        request: BeginUpload,
    ) -> Result<Upload, ServiceError> {
        let BeginUpload {
            session,
            name,
            mime,
            size,
        } = request;
        let handle = self.live(session).await?;
        let kind = AttachmentKind::of(&mime);
        let abilities = handle.conversation()?.abilities;
        let taken = match kind {
            AttachmentKind::Image => abilities.is_some_and(|abilities| abilities.image),
            AttachmentKind::Audio => abilities.is_some_and(|abilities| abilities.audio),
            AttachmentKind::File => true,
        };
        if !taken {
            return Err(UploadError::NotTaken(noun(kind)).into());
        }
        let limit = kind.max_size(&AttachmentLimits::DEFAULT);
        if size > limit {
            return Err(UploadError::OverLimit {
                what: noun_limit(kind),
                size,
                limit,
            }
            .into());
        }
        let declared = Declared {
            session,
            name,
            mime,
            kind,
            size,
        };
        let upload = self.uploads.begin(&self.store, holder, declared).await?;
        Ok(Upload {
            upload,
            chunk: AttachmentLimits::DEFAULT.chunk,
        })
    }

    /// `attachment.write`.
    pub(crate) async fn write(
        &self,
        holder: Holder,
        request: vessel_proto::conversation::WriteUpload,
    ) -> Result<Written, ServiceError> {
        let received = self
            .uploads
            .write(holder, request.upload, request.offset, request.data)
            .await?;
        Ok(Written { received })
    }

    /// `attachment.finish`: every byte there, an image's type checked, the
    /// session still running; then the store keeps it.
    pub(crate) async fn finish(
        &self,
        holder: Holder,
        id: UploadId,
    ) -> Result<AttachmentRef, ServiceError> {
        let upload = self.uploads.held(holder, id)?;
        // Held to the end, so no piece lands meanwhile and a second
        // `finish` finds it finished.
        let mut progress = upload.progress.lock().await;
        let declared = &upload.declared;
        let Some(received) = progress.as_ref().map(|progress| progress.received) else {
            return Err(UploadError::Unknown(id).into());
        };
        if received != declared.size {
            return Err(UploadError::Incomplete {
                received,
                size: declared.size,
            }
            .into());
        }
        let looks = progress
            .as_ref()
            .is_some_and(|progress| looks_like(declared.kind, &declared.mime, progress.head()));
        if !looks {
            *progress = None;
            drop(progress);
            self.uploads.drop_upload(id).await;
            return Err(UploadError::NotAnImage(declared.mime.clone()).into());
        }
        if let Err(ended) = self.live(declared.session).await {
            *progress = None;
            drop(progress);
            self.uploads.drop_upload(id).await;
            return Err(ended);
        }
        let Some(finished) = progress.take() else {
            return Err(UploadError::Unknown(id).into());
        };
        let kept = KeptAttachment {
            session: declared.session,
            reference: AttachmentRef {
                id: AttachmentId::random(),
                name: declared.name.clone(),
                kind: declared.kind,
                mime: declared.mime.clone(),
                size: declared.size,
            },
            sha256: finished.finish(),
        };
        let reference = kept.reference.clone();
        let result = self
            .store
            .keep_attachment(kept, upload.path.clone(), Utc::now())
            .await;
        drop(progress);
        match result {
            Ok(()) => {
                self.uploads.forget(id);
                Ok(reference)
            }
            Err(error) => {
                self.uploads.drop_upload(id).await;
                // Ended, or removed, since the check above.
                if let StoreError::SessionEnded(session) | StoreError::UnknownSession(session) =
                    error
                {
                    return Err(ServiceError::Ended(session));
                }
                Err(UploadError::Store(error).into())
            }
        }
    }

    /// `attachment.read`: a piece of a session's attachment, read back
    /// only as the file kept.
    pub(crate) async fn read(&self, request: ReadAttachment) -> Result<BlobChunk, ServiceError> {
        let ReadAttachment {
            session,
            attachment,
            offset,
        } = request;
        let mode = match self.sessions.find(session).await? {
            Found::Live { handle, .. } => handle.info().spec.mode,
            Found::Stored(info) => info.spec.mode,
        };
        if let SessionMode::Terminal { .. } = mode {
            return Err(SessionError::Terminal("attachments").into());
        }
        let kept =
            self.store
                .attachment(session, attachment)
                .await?
                .ok_or(UploadError::NoAttachment {
                    session,
                    id: attachment,
                })?;
        let size = kept.reference.size;
        if offset > size {
            return Err(UploadError::PastEnd { offset, size }.into());
        }
        let read = self
            .store
            .read_attachment(
                kept,
                offset,
                BlobChunk::MAX_LEN,
                self.verified.get(attachment),
            )
            .await
            .map_err(UploadError::Store)?;
        self.verified.put(attachment, read.identity);
        Ok(BlobChunk {
            data: read.data.into(),
            total: size,
        })
    }

    /// A live structured session's handle: `not_found` for an unknown
    /// one, `unsupported` for a terminal one, `conflict` once it ended.
    async fn live(
        &self,
        session: SessionId,
    ) -> Result<vessel_session::SessionHandle, ServiceError> {
        match self.sessions.find(session).await? {
            Found::Live { handle, .. } => {
                if let SessionMode::Terminal { .. } = handle.info().spec.mode {
                    return Err(SessionError::Terminal("attachments").into());
                }
                if handle.has_ended() {
                    return Err(ServiceError::Ended(session));
                }
                Ok(handle)
            }
            Found::Stored(info) => match info.spec.mode {
                SessionMode::Terminal { .. } => Err(SessionError::Terminal("attachments").into()),
                SessionMode::Structured { .. } => Err(ServiceError::Ended(session)),
            },
        }
    }
}

/// Whether `head` (an upload's first bytes) starts as a file of type
/// `mime` does, for an image; anything else is not checked (the agent
/// reads a file as what it is).
fn looks_like(kind: AttachmentKind, mime: &MimeType, head: &[u8]) -> bool {
    match kind {
        AttachmentKind::Audio | AttachmentKind::File => true,
        AttachmentKind::Image => match mime.as_str() {
            "image/png" => head.starts_with(b"\x89PNG\r\n\x1a\n"),
            "image/jpeg" => head.starts_with(&[0xFF, 0xD8, 0xFF]),
            "image/gif" => head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a"),
            "image/webp" => {
                head.get(..4) == Some(b"RIFF".as_slice())
                    && head.get(8..12) == Some(b"WEBP".as_slice())
            }
            // `AttachmentKind::of` makes only these four images.
            _ => false,
        },
    }
}

/// An attachment kind in a sentence.
const fn noun(kind: AttachmentKind) -> &'static str {
    match kind {
        AttachmentKind::Image => "image",
        AttachmentKind::Audio => "audio",
        AttachmentKind::File => "file",
    }
}

const fn noun_limit(kind: AttachmentKind) -> &'static str {
    match kind {
        AttachmentKind::Image => "an image",
        AttachmentKind::Audio => "an audio file",
        AttachmentKind::File => "a file",
    }
}

/// Files read back and found to be what was kept, by attachment: a later
/// read of one whose identity has not changed is not hashed again
/// (`vessel_store`'s read-back). At most [`Verified::MAX`] are remembered;
/// past that the memory starts over, which costs only a hash each.
#[derive(Debug, Default)]
pub(crate) struct Verified(Mutex<HashMap<AttachmentId, FileIdentity>>);

impl Verified {
    const MAX: usize = 256;

    pub(crate) fn get(&self, id: AttachmentId) -> Option<FileIdentity> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&id)
            .copied()
    }

    pub(crate) fn put(&self, id: AttachmentId, identity: Option<FileIdentity>) {
        let mut known = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(identity) = identity else {
            let _forgotten = known.remove(&id);
            return;
        };
        if known.len() >= Self::MAX && !known.contains_key(&id) {
            known.clear();
        }
        let _replaced = known.insert(id, identity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_image_must_start_as_its_type_does() {
        let png: MimeType = "image/png".parse().unwrap();
        let webp: MimeType = "image/webp".parse().unwrap();
        let gif: MimeType = "image/gif".parse().unwrap();
        let jpeg: MimeType = "image/jpeg".parse().unwrap();
        let image = AttachmentKind::Image;
        assert!(looks_like(image, &png, b"\x89PNG\r\n\x1a\n\0\0\0\r"));
        assert!(!looks_like(image, &png, b"GIF89a"), "a gif called a png");
        assert!(!looks_like(image, &png, b"\x89PN"), "too short");
        assert!(looks_like(image, &jpeg, &[0xFF, 0xD8, 0xFF, 0xE0]));
        assert!(looks_like(image, &gif, b"GIF87a..."));
        assert!(looks_like(image, &webp, b"RIFF\x10\0\0\0WEBPVP8 "));
        assert!(!looks_like(image, &webp, b"RIFF\x10\0\0\0WAVEfmt "));
        let text: MimeType = "text/plain".parse().unwrap();
        assert!(looks_like(AttachmentKind::File, &text, b"anything"));
    }
}
