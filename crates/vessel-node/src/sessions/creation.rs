//! Cancellation-safe creation and the empty-row-before-log persistence boundary.
//! Handover and reads remain the session collection's responsibility.

use std::sync::Arc;
use vessel_proto::{EventSeq, SessionInfo, SessionSpec};
use vessel_session::{Created, LaunchOptions};

use super::Sessions;
use crate::error::ServiceError;
use crate::persist::{Durability, Persisted};

impl Sessions {
    /// Start a session and its persistence.
    ///
    /// Runs as its own task: once the process is started, storing the
    /// session and starting its persistence must finish even if the client
    /// cancels the call, or the session would run unrecorded.
    pub(crate) async fn create(
        self: &Arc<Self>,
        spec: SessionSpec,
        options: LaunchOptions,
        lease: Option<tokio::sync::OwnedMutexGuard<()>>,
    ) -> Result<SessionInfo, ServiceError> {
        let sessions = Arc::clone(self);
        self.tasks
            .spawn(async move {
                let _lease = lease;
                sessions.start(spec, options).await
            })
            .await
            .map_err(ServiceError::Task)?
    }

    async fn start(
        self: &Arc<Self>,
        spec: SessionSpec,
        options: LaunchOptions,
    ) -> Result<SessionInfo, ServiceError> {
        let Created {
            initial,
            handle,
            durable,
        } = self.runtime.create_with(spec, options).await?;
        let id = handle.id();
        // Visible from here on (see the module docs). Readers that need the
        // log wait on the watch until persistence starts below.
        let durability = self.track(&handle, EventSeq::ZERO);
        if let Err(error) = self
            .store
            .create_session_committed(*initial)
            .await
            .map(|committed| handle.acknowledge_time(committed.updated_at()))
        {
            // Without a stored row its records cannot be kept. Fail anyone
            // already waiting for its log, stop it, and forget it; dropping
            // `durable` lets it finish without waiting for us.
            tracing::error!(session = %id, %error, "cannot store a new session; killing it");
            durability.send_modify(|durability| *durability = Durability::Broken);
            handle.kill();
            self.forget(id);
            drop(durable);
            // Remove it from the runtime once it has ended, without holding
            // up this call's answer.
            let sessions = Arc::clone(self);
            drop(self.tasks.spawn(async move {
                sessions.hand_over(handle, Persisted::Incomplete).await;
            }));
            return Err(error.into());
        }
        let info = handle.info();
        self.persist(handle, durable, durability);
        Ok(info)
    }
}
