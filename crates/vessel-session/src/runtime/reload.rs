//! `SessionRuntime::reload` (TRACKER 803, ADR 0110): the fresh launch a
//! live session's reload runs with, built as a move builds its target's
//! (same driver, current profile), handed to the session's actor.

use tokio::sync::oneshot;
use vessel_agent::AgentSetup;
use vessel_harness::Profile;
use vessel_proto::{SessionId, SessionInfo, SessionMode};

use super::SessionRuntime;
use super::agent::launch_for;
use crate::error::SessionError;
use crate::moving::{Control, Launch};
use crate::reload::{NotReloadable, Reload, ReloadOrder};

impl SessionRuntime {
    /// Reload the live structured session `id`'s agent: its running turn
    /// cancelled, its process stopped and started again with `session/load`
    /// under `reload`'s launch (a parked one only started), and what the
    /// agent's history has beyond the log added after `reloaded`. Answered
    /// once the agent is ready.
    ///
    /// # Errors
    /// [`SessionError::NotFound`], [`SessionError::Ended`],
    /// [`SessionError::NotReloadable`] (nothing stopped, or the turn kept),
    /// [`SessionError::ReloadFailed`] (the session ends `failed`, or a
    /// parked one stays parked), and the launch's own failures.
    pub async fn reload(&self, id: SessionId, reload: Reload) -> Result<SessionInfo, SessionError> {
        let handle = self.get(id).ok_or(SessionError::NotFound(id))?;
        let info = handle.info();
        if !info.status.is_live() {
            return Err(SessionError::Ended);
        }
        let SessionMode::Structured { model, .. } = &info.spec.mode else {
            return Err(NotReloadable::Terminal.into());
        };
        if info.abilities.is_some_and(|abilities| !abilities.resume) {
            return Err(NotReloadable::AgentCannot.into());
        }
        let control = handle.0.control.clone().ok_or(NotReloadable::Terminal)?;
        let Reload { launch, before } = reload;
        let profile = launch
            .profile
            .map_or_else(|| Profile::default_of(&info.spec.harness), Ok)?;
        // The session keeps its driver: its options are that driver's.
        let driver = handle.driver();
        let spec = info.spec.clone().with_profile(profile.name().clone());
        let lookup = self.config.harness_lookup.clone();
        let env = self.config.harness_env.clone();
        let proxy = launch.model_proxy;
        let (command, cwd) = tokio::task::spawn_blocking(move || {
            let command = launch_for(driver, &lookup, &env, &spec, &profile, proxy.as_ref())?;
            let cwd = command
                .cwd
                .clone()
                .or_else(|| std::env::current_dir().ok())
                .ok_or(SessionError::NoWorkingDirectory)?;
            Ok::<_, SessionError>((command, cwd))
        })
        .await
        .map_err(SessionError::Internal)??;
        let setup = AgentSetup {
            cwd,
            model: model.clone(),
            // The actor fills in its conversation and its choices as they
            // stand when the order runs.
            resume: info.harness_session.clone(),
            options: handle.choices(),
            mcp_servers: launch.mcp_servers,
            handshake_timeout: self.config.agent_handshake_timeout,
            // The session's own, which the actor keeps.
            questions: false,
            fork: None,
            system_prompt: None,
            private_dir: self.private_dir(),
        };
        let (reply, answer) = oneshot::channel();
        let order = ReloadOrder {
            launch: Launch {
                limits: launch.limits,
                command,
                setup,
                driver,
            },
            before,
            notices: launch.notices,
            reply,
        };
        if control
            .send(Control::Reload(Box::new(order)))
            .await
            .is_err()
        {
            return Err(SessionError::Ended);
        }
        answer.await.map_err(|_ended| SessionError::Ended)??;
        Ok(handle.info())
    }
}
