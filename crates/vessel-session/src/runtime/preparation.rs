//! Physical launch preparation retains the caller's existing admission and the
//! runtime's drain token. It never grants quota or launches a process itself.

use std::sync::Arc;

use vessel_harness::{ModelProxy, Profile};
use vessel_proto::SessionSpec;

use super::agent::launch_for;
use super::{AgentCommand, SessionRuntime};
use crate::SessionError;

impl SessionRuntime {
    /// Every actual blocking closure retains both owners even if its async
    /// waiter is dropped. The same runtime tracker supplies hard shutdown drain.
    pub(super) async fn prepare_blocking<T, R, F>(
        &self,
        ownership: Arc<T>,
        work: F,
    ) -> Result<R, SessionError>
    where
        T: Send + Sync + 'static,
        R: Send + 'static,
        F: FnOnce() -> Result<R, SessionError> + Send + 'static,
    {
        let tracked = self.tasks_token();
        if self.is_shutting_down() {
            return Err(SessionError::ShuttingDown);
        }
        tokio::task::spawn_blocking(move || {
            let _tracked = tracked;
            let _ownership = ownership;
            work()
        })
        .await
        .map_err(SessionError::Internal)?
    }

    /// Native-version lookup and command resolution share this owner. Awaiting
    /// normally returns only after physical settlement; cancellation releases
    /// only the waiter, never the closure's token, claim or caller admission.
    pub(crate) async fn agent_command_owned<T: Send + Sync + 'static>(
        &self,
        spec: SessionSpec,
        profile: Profile,
        proxy: Option<ModelProxy>,
        ownership: Arc<T>,
    ) -> Result<AgentCommand, SessionError> {
        // Keep normal preparation tracked between blocking phases too (the
        // native version process may be awaited between its two lookups).
        // Each closure additionally owns its token if this waiter is dropped.
        let _preparing = self.tasks_token();
        if self.is_shutting_down() {
            return Err(SessionError::ShuttingDown);
        }
        let chosen = self
            .choose_driver(&spec, &profile, Arc::clone(&ownership))
            .await;
        let driver = chosen.driver();
        let notices = chosen.notices;
        let lookup = self.config.harness_lookup.clone();
        let env = self.config.harness_env.clone();
        self.prepare_blocking(ownership, move || {
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
    }
}
