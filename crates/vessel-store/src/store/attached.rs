//! The store's side of attachments (protocol 10, ADR 0025): keeping a
//! finished upload, reading its record and its bytes back, and what a
//! session's attachments weigh.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use vessel_proto::{AttachmentId, SessionId};

use super::Store;
use crate::attachments::files::{FileIdentity, ReadBack};
use crate::attachments::{self, AttachmentUsage, KeptAttachment};
use crate::error::StoreError;
use crate::writer::Write;

impl Store {
    /// Keep a finished upload as `kept` (protocol 10, ADR 0025): `staged`,
    /// the upload's received bytes, is moved to the attachment's place
    /// under the data directory and checked against `kept`'s size and
    /// SHA-256, then its row is recorded at `at`. A refused row takes the
    /// file back; `staged` is gone either way once the move was made.
    ///
    /// # Errors
    /// [`StoreError::UnknownSession`], [`StoreError::SessionEnded`] once
    /// its session ended (or was removed), [`StoreError::Attachment`] when
    /// the file could not be put in place or is not what was received, or
    /// the write failed.
    pub async fn keep_attachment(
        &self,
        kept: KeptAttachment,
        staged: PathBuf,
        at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let root = Arc::clone(&self.attachments);
        let placing = kept.clone();
        let placed = tokio::task::spawn_blocking(move || {
            attachments::files::place(&root, &placing, &staged)
        })
        .await
        .map_err(StoreError::Task)??;
        let recorded = self
            .write(|done| Write::Attach {
                kept: Box::new(kept),
                at,
                done,
            })
            .await;
        if recorded.is_err() {
            let removed = tokio::task::spawn_blocking(move || placed.remove()).await;
            if let Err(error) = removed {
                tracing::warn!(%error, "could not take back an attachment not kept");
            }
        }
        recorded
    }

    /// `session`'s attachment `id`, if the store keeps it (protocol 10).
    ///
    /// # Errors
    /// The read failed, or the row is corrupt.
    pub async fn attachment(
        &self,
        session: SessionId,
        id: AttachmentId,
    ) -> Result<Option<KeptAttachment>, StoreError> {
        self.read(move |reader| reader.attachment(session, id))
            .await
    }

    /// How many bytes of attachments the node keeps, and `session` (protocol
    /// 10's limits).
    ///
    /// # Errors
    /// The read failed.
    pub async fn attachment_usage(
        &self,
        session: SessionId,
    ) -> Result<AttachmentUsage, StoreError> {
        self.read(move |reader| reader.attachment_usage(session))
            .await
    }

    /// Up to `max` bytes of `kept` from `from`, read back only from the
    /// regular file of its recorded size and SHA-256, never through a link
    /// (ADR 0025); `max` 0 only checks it. A file still of identity
    /// `verified` is not hashed again.
    ///
    /// # Errors
    /// [`StoreError::Attachment`]: the file is missing, was changed, or
    /// could not be read.
    pub async fn read_attachment(
        &self,
        kept: KeptAttachment,
        from: u64,
        max: usize,
        verified: Option<FileIdentity>,
    ) -> Result<ReadBack, StoreError> {
        let root = Arc::clone(&self.attachments);
        tokio::task::spawn_blocking(move || {
            attachments::files::read(&root, &kept, from, max, verified)
        })
        .await
        .map_err(StoreError::Task)?
        .map_err(StoreError::from)
    }

    /// Where `kept`'s bytes are: what an agent is given to read a file
    /// attachment with its own tools.
    #[must_use]
    pub fn attachment_path(&self, kept: &KeptAttachment) -> PathBuf {
        attachments::files::path(&self.attachments, kept)
    }
}
