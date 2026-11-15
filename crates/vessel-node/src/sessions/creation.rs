//! A created actor is stored with explicit provenance before its journal drains.
//! Creation is node-owned after launch; an RPC cancellation cannot leave an
//! unrecorded process or promote an unattended origin to a startup candidate.
use super::Sessions;
use crate::error::ServiceError;
use crate::persist::{Durability, Persisted};
use std::sync::Arc;
use vessel_proto::{EventSeq, SessionInfo, SessionSpec};
use vessel_session::{Created, LaunchOptions};

impl Sessions {
    /// Ordinary interactive creation, retaining the existing internal seam.
    pub(crate) async fn create(
        self: &Arc<Self>,
        spec: SessionSpec,
        options: LaunchOptions,
        lease: Option<tokio::sync::OwnedMutexGuard<()>>,
    ) -> Result<SessionInfo, ServiceError> {
        self.create_eligible(
            spec,
            options,
            lease,
            vessel_store::RestartEligibility::Person,
        )
        .await
    }

    /// Agent, unattended or forwarded creation remains manual at restart.
    pub(crate) async fn create_without_recovery(
        self: &Arc<Self>,
        spec: SessionSpec,
        options: LaunchOptions,
        lease: Option<tokio::sync::OwnedMutexGuard<()>>,
    ) -> Result<SessionInfo, ServiceError> {
        self.create_eligible(
            spec,
            options,
            lease,
            vessel_store::RestartEligibility::Never,
        )
        .await
    }

    /// Start a session and its persistence.
    ///
    /// Runs as its own task: once the process is started, storing the
    /// session and starting its persistence must finish even if the client
    /// cancels the call, or the session would run unrecorded.
    async fn create_eligible(
        self: &Arc<Self>,
        spec: SessionSpec,
        options: LaunchOptions,
        lease: Option<tokio::sync::OwnedMutexGuard<()>>,
        restart_eligible: vessel_store::RestartEligibility,
    ) -> Result<SessionInfo, ServiceError> {
        let sessions = Arc::clone(self);
        self.tasks
            .spawn(async move {
                let _lease = lease;
                sessions.start(spec, options, restart_eligible).await
            })
            .await
            .map_err(ServiceError::Task)?
    }

    async fn start(
        self: &Arc<Self>,
        spec: SessionSpec,
        options: LaunchOptions,
        restart_eligible: vessel_store::RestartEligibility,
    ) -> Result<SessionInfo, ServiceError> {
        let Created { handle, durable } = self.runtime.create_with(spec, options).await?;
        let id = handle.id();
        // Visible from here on (see the module docs). Readers that need the
        // log wait on the watch until persistence starts below.
        let durability = self.track(&handle, EventSeq::ZERO);
        let kept = self
            .store
            .create_session_with_restart(handle.info(), restart_eligible)
            .await;
        if let Err(error) = kept {
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
