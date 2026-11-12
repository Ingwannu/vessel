//! The store's side of pull request feedback (protocol 11, schema 10, ADR
//! 0029): whether a session has it on, and what was sent to it.

use chrono::{DateTime, Utc};
use vessel_proto::SessionId;

use super::Store;
use crate::error::StoreError;
use crate::forge::{ForgeWrite, StoredFeedback};
use crate::reader::Reader;
use crate::writer::Write;

impl Store {
    /// `session`'s feedback: off with nothing sent when never turned on.
    ///
    /// # Errors
    /// The read failed.
    pub async fn feedback(&self, session: SessionId) -> Result<StoredFeedback, StoreError> {
        self.read(move |reader: &Reader| reader.feedback(session))
            .await
    }

    /// Turn `session`'s feedback on or off; what was sent stays recorded,
    /// so turning it on again sends nothing twice.
    ///
    /// # Errors
    /// The write failed (a session the store does not have, too).
    pub async fn set_feedback(&self, session: SessionId, on: bool) -> Result<(), StoreError> {
        self.forge_write(ForgeWrite::Feedback { session, on }).await
    }

    /// Record that the events with `signatures` were sent to `session` at
    /// `at`. Past [`crate::MAX_FEEDBACK_SENT`] the oldest are dropped.
    ///
    /// # Errors
    /// The write failed.
    pub async fn feedback_sent(
        &self,
        session: SessionId,
        signatures: Vec<String>,
        at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        if signatures.is_empty() {
            return Ok(());
        }
        self.forge_write(ForgeWrite::Sent {
            session,
            signatures,
            at,
        })
        .await
    }

    /// Forget signatures sent to `session` (events its pull request no
    /// longer has).
    ///
    /// # Errors
    /// The write failed.
    pub async fn forget_feedback_sent(
        &self,
        session: SessionId,
        signatures: Vec<String>,
    ) -> Result<(), StoreError> {
        if signatures.is_empty() {
            return Ok(());
        }
        self.forge_write(ForgeWrite::Forget {
            session,
            signatures,
        })
        .await
    }

    async fn forge_write(&self, write: ForgeWrite) -> Result<(), StoreError> {
        self.write(|done| Write::Forge { write, done }).await
    }
}
