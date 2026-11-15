//! Typed restart state operations through the existing writer and read pool.
use super::Store;
use crate::{
    RestartCandidate, StoreError,
    restart::RestartWrite,
    writer::{RestartOp, Write},
};
use std::sync::Arc;
use tokio::sync::oneshot;
use vessel_proto::restart::{RestartCheckpoint, RestartGeneration};
use vessel_proto::{EventRecord, SessionId};

impl Store {
    /// Commit preceding records and an internal checkpoint in one operation.
    ///
    /// # Errors
    /// Event ordering, encoding or storage failed. False is a stale checkpoint.
    pub async fn checkpoint_restart(
        &self,
        session: SessionId,
        records: Vec<Arc<EventRecord>>,
        change: RestartCheckpoint,
    ) -> Result<bool, StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(Write::Restart(RestartOp::Checkpoint {
            session,
            records,
            change,
            done,
        }))
        .await?;
        committed.await.map_err(|_closed| StoreError::Closed)?
    }
    /// Update durable startup policy; Off consumes pending unstarted intents.
    ///
    /// # Errors
    /// The writer or commit failed.
    pub async fn set_restart_policy(&self, enabled: bool) -> Result<(), StoreError> {
        let _changed = self.restart_write(RestartWrite::Policy { enabled }).await?;
        Ok(())
    }
    /// Read the oldest pending turn, with a deterministic session-id tie break.
    ///
    /// # Errors
    /// The projection could not be read or decoded.
    pub async fn next_restart(&self) -> Result<Option<RestartCandidate>, StoreError> {
        self.read(|reader| reader.with(crate::restart::next)).await
    }
    /// Consume exactly this observed intent before launching anything.
    ///
    /// # Errors
    /// The writer failed. False means current eligibility no longer matches.
    pub async fn claim_restart(&self, candidate: RestartCandidate) -> Result<bool, StoreError> {
        self.restart_write(RestartWrite::Claim(candidate)).await
    }
    /// Release this boot's completed attempt protection, without replaying it.
    ///
    /// # Errors
    /// The writer failed.
    pub async fn finish_restart(&self, session: SessionId) -> Result<(), StoreError> {
        let _changed = self.restart_write(RestartWrite::Finish { session }).await?;
        Ok(())
    }
    /// Stop generation installed before a restored actor becomes visible.
    ///
    /// # Errors
    /// The projection could not be read or decoded.
    pub async fn restart_generation(
        &self,
        session: SessionId,
    ) -> Result<RestartGeneration, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| crate::restart::generation(connection, session))
        })
        .await
    }
    /// Suppress an ended row only while it remains the observed run.
    ///
    /// # Errors
    /// Storage failed; false means a newer run or log won the race.
    pub async fn suppress_ended_restart(
        &self,
        session: SessionId,
        next: vessel_proto::EventSeq,
    ) -> Result<bool, StoreError> {
        self.restart_write(RestartWrite::SuppressEnded { session, next })
            .await
    }
    async fn restart_write(&self, write: RestartWrite) -> Result<bool, StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(Write::Restart(RestartOp::Change { write, done }))
            .await?;
        committed.await.map_err(|_closed| StoreError::Closed)?
    }
}
