//! Initial hooks may arrive before the retained creation task starts persistence.
use super::Sessions;
use crate::persist;
use tokio::time::{Instant, timeout_at};
use vessel_proto::{EventSeq, SessionId, SessionInfo, checkpoint::CheckpointError};

impl Sessions {
    pub(crate) fn checkpoint_root(
        &self,
        session: SessionId,
    ) -> Result<vessel_proto::checkpoint::PhysicalRoot, CheckpointError> {
        self.roots.capture(session, &self.runtime)
    }
    pub(crate) async fn checkpoint_anchor(
        &self,
        session: SessionId,
        anchor: EventSeq,
        deadline: Instant,
    ) -> Result<SessionInfo, CheckpointError> {
        let target = anchor.next().ok_or(CheckpointError::StaleCapture)?;
        timeout_at(deadline, async {
            loop {
                // Register before lookup, so a track between the lookup and
                // await cannot lose its notification. No polling or fresh budget.
                let registered = self.registrations.notified();
                tokio::pin!(registered);
                let _enabled = registered.as_mut().enable();
                let handle = self
                    .runtime
                    .get(session)
                    .ok_or(CheckpointError::StaleCapture)?;
                let durability = self
                    .durability
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get(&session)
                    .cloned();
                if let Some(mut durability) = durability {
                    persist::wait_until(session, &mut durability, target)
                        .await
                        .map_err(|_broken| CheckpointError::StaleCapture)?;
                    return Ok(handle.info());
                }
                if handle.has_ended() {
                    return Err(CheckpointError::StaleCapture);
                }
                registered.await;
            }
        })
        .await
        .map_err(|_deadline| CheckpointError::CaptureTimedOut)?
    }
}

impl Sessions {
    /// Ended-target slice: never start/resume an agent to append file metadata.
    /// Live target pause/admission remains separate until all start seams join.
    pub(crate) async fn checkpoint_ended_target(
        &self,
        session: SessionId,
        root: vessel_proto::checkpoint::PhysicalRoot,
    ) -> Result<(), CheckpointError> {
        match self
            .find(session)
            .await
            .map_err(|_missing| CheckpointError::NotFound)?
        {
            super::Found::Live {
                handle,
                mut durability,
            } => {
                let info = handle.info();
                if info.status.is_live() {
                    return Err(CheckpointError::Busy);
                }
                persist::wait_until(session, &mut durability, info.next_seq)
                    .await
                    .map_err(|_failed| CheckpointError::RecoveryPending)?;
            }
            super::Found::Stored(info) => {
                if info.status.is_live() {
                    return Err(CheckpointError::Busy);
                }
            }
        }
        self.roots.refuse_restore_writers(root, &self.runtime)
    }
}
