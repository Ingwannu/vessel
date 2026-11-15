//! Initial hooks may arrive before the retained creation task starts persistence.
use super::Sessions;
use crate::persist;
use tokio::time::{Instant, timeout_at};
use vessel_proto::{EventSeq, SessionId, SessionInfo, checkpoint::CheckpointError};

impl Sessions {
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
