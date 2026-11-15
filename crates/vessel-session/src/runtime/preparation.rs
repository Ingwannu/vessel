//! Physical preparation retains original admission/claim owners. Native version
//! probing is a separately admitted process effect; no session starts here.

use std::sync::Arc;

use vessel_harness::{ModelProxy, Profile};
use vessel_proto::SessionSpec;

use super::agent::launch_for;
use super::preparation_gate::{
    PreparationAdmission, PreparationCancellation, PreparationEffect, PreparationGate,
    PreparationRefusal,
};
use super::{AgentCommand, SessionRuntime};
use crate::SessionError;

/// Keep admission failure distinct from an unreadable version lookup: only the
/// latter may use the legacy ACP fallback.
pub(super) enum PreparationError {
    Refused(PreparationRefusal),
    Failed(SessionError),
}

impl PreparationError {
    pub(super) fn session(self) -> SessionError {
        match self {
            Self::Refused(reason) => reason.into(),
            Self::Failed(error) => error,
        }
    }
}

impl SessionRuntime {
    pub(crate) fn preparation_gate(
        &self,
    ) -> Arc<PreparationGate<super::preparation_gate::RuntimeAdmission>> {
        Arc::new(super::preparation_gate::runtime_gate(self.shutdown.clone()))
    }

    /// Waits only for a temporary host pause. The immutable gate retains the
    /// original owner/generation; a wakeup never recaptures newer authority.
    pub(super) async fn admit_preparation<F>(
        &self,
        gate: &PreparationGate<F>,
        effect: PreparationEffect,
    ) -> Result<(), PreparationRefusal>
    where
        F: for<'a> Fn(PreparationEffect, PreparationCancellation<'a>) -> PreparationAdmission
            + Send
            + Sync
            + 'static,
    {
        loop {
            let cancellation = PreparationCancellation {
                node: &self.shutdown,
                caller: &gate.cancelled,
            };
            cancellation.check()?;
            match (gate.admit)(effect, cancellation) {
                PreparationAdmission::Admitted => return Ok(()),
                PreparationAdmission::Refused(reason) => return Err(reason),
                PreparationAdmission::Wait(mut changed) => {
                    tokio::select! {
                        () = self.shutdown.cancelled() => return Err(PreparationRefusal::Cancelled),
                        () = gate.cancelled.cancelled() => return Err(PreparationRefusal::Cancelled),
                        result = changed.changed() => result.map_err(|_closed| PreparationRefusal::Changed)?,
                    }
                }
            }
        }
    }

    /// The actual closure, not its waiter, retains admission and hard drain.
    pub(super) async fn prepare_blocking<T, R, F, W>(
        &self,
        ownership: Arc<T>,
        gate: Arc<PreparationGate<F>>,
        work: W,
    ) -> Result<R, PreparationError>
    where
        T: Send + Sync + 'static,
        R: Send + 'static,
        F: for<'a> Fn(PreparationEffect, PreparationCancellation<'a>) -> PreparationAdmission
            + Send
            + Sync
            + 'static,
        W: FnOnce() -> Result<R, SessionError> + Send + 'static,
    {
        let tracked = self.tasks_token();
        self.admit_preparation(&gate, PreparationEffect::Lookup)
            .await
            .map_err(PreparationError::Refused)?;
        // No await from logical admission to dispatch of this retained owner.
        tokio::task::spawn_blocking(move || {
            let _tracked = tracked;
            let _ownership = ownership;
            let _gate = gate;
            work()
        })
        .await
        .map_err(|error| PreparationError::Failed(SessionError::Internal(error)))?
        .map_err(PreparationError::Failed)
    }

    /// All phases use the same original host gate; refusals cannot become a
    /// native-version miss or an ACP retry. Caller drop does not end real work.
    pub(crate) async fn agent_command_owned<T, F>(
        &self,
        spec: SessionSpec,
        profile: Profile,
        proxy: Option<ModelProxy>,
        ownership: Arc<T>,
        gate: Arc<PreparationGate<F>>,
    ) -> Result<AgentCommand, SessionError>
    where
        T: Send + Sync + 'static,
        F: for<'a> Fn(PreparationEffect, PreparationCancellation<'a>) -> PreparationAdmission
            + Send
            + Sync
            + 'static,
    {
        let _preparing = self.tasks_token();
        let chosen = self
            .choose_driver(&spec, &profile, Arc::clone(&ownership), Arc::clone(&gate))
            .await?;
        let driver = chosen.driver();
        let notices = chosen.notices;
        let lookup = self.config.harness_lookup.clone();
        let env = self.config.harness_env.clone();
        self.prepare_blocking(ownership, gate, move || {
            let command = launch_for(driver, &lookup, &env, &spec, &profile, proxy.as_ref())?;
            let cwd = command
                .cwd
                .clone()
                .or_else(|| std::env::current_dir().ok())
                .ok_or(SessionError::NoWorkingDirectory)?;
            Ok(AgentCommand {
                command,
                cwd,
                driver,
                notices,
            })
        })
        .await
        .map_err(PreparationError::session)
    }
}
