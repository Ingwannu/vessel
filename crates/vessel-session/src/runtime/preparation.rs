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
use super::prepared::RetainedPreparation;
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

    pub(super) fn default_preparation<T>(
        &self,
        owner: Arc<T>,
    ) -> RetainedPreparation<T, super::preparation_gate::RuntimeAdmission> {
        RetainedPreparation {
            owner,
            gate: self.preparation_gate(),
            admission: self.launch_admission(),
        }
    }

    /// Waits only for a temporary host pause. The immutable gate retains the
    /// original owner/generation; a wakeup never recaptures newer authority.
    pub(super) async fn admit_preparation<T, F>(
        preparation: &RetainedPreparation<T, F>,
        effect: PreparationEffect,
    ) -> Result<(), PreparationRefusal>
    where
        F: for<'a> Fn(PreparationEffect, PreparationCancellation<'a>) -> PreparationAdmission
            + Send
            + Sync
            + 'static,
    {
        let gate = &preparation.gate;
        loop {
            let cancellation = PreparationCancellation {
                admission: &preparation.admission,
                caller: &gate.cancelled,
            };
            cancellation.check()?;
            match (gate.admit)(effect, cancellation) {
                PreparationAdmission::Admitted => return Ok(()),
                PreparationAdmission::Refused(reason) => return Err(reason),
                PreparationAdmission::Wait(mut changed) => {
                    tokio::select! {
                        () = preparation.admission.cancelled() => return Err(PreparationRefusal::Cancelled),
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
        preparation: RetainedPreparation<T, F>,
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
        Self::admit_preparation(&preparation, PreparationEffect::Lookup)
            .await
            .map_err(PreparationError::Refused)?;
        // No await from logical admission to dispatch of this retained owner.
        tokio::task::spawn_blocking(move || {
            let _tracked = tracked;
            let _preparation = preparation;
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
        preparation: RetainedPreparation<T, F>,
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
            .choose_driver(&spec, &profile, preparation.clone())
            .await?;
        let driver = chosen.driver();
        let notices = chosen.notices;
        let lookup = self.config.harness_lookup.clone();
        let env = self.config.harness_env.clone();
        #[cfg(test)]
        let gate = self.lookup_gate.take();
        self.prepare_blocking(preparation, move || {
            let command = launch_for(driver, &lookup, &env, &spec, &profile, proxy.as_ref())?;
            #[cfg(test)]
            if let Some(gate) = gate {
                gate.pause()?;
            }
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
