//! A structured session's agent: started for a new session or one
//! started again, its command resolved from the harness and profile,
//! probed before any session of it exists, and moved to another profile.

use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot, watch};
use vessel_agent::{AgentProcess, AgentSetup, CommandSpec, Driver};
use vessel_harness::{ModelProxy, Profile};
use vessel_proto::{
    HarnessRef, ResolvedMcpServers, SessionId, SessionInfo, SessionMode, SessionSpec,
};

use super::parts::{ActorStart, Begin};
use super::{Created, SessionRuntime};
use crate::error::SessionError;
use crate::handle::Input;
use crate::moving::{Control, Launch, MoveError, MoveOrder, ProfileMove, Restart};
use crate::parking::{Parking, Route};
use crate::structured::{MoveState, StructuredActor};

#[derive(Debug, Clone, Copy)]
enum ProbePurpose {
    Options,
    Catalog,
}

/// Only model and options change a draft probe; it can never resume or send a turn.
#[derive(Debug, Clone, Default)]
pub struct ProbeChoices {
    /// The requested provider model, validated by its handshake.
    pub model: Option<vessel_proto::ModelId>,
    /// Mode, effort and feature selections, applied before Ready.
    pub options: Vec<vessel_proto::OptionChoice>,
}

/// A structured session's agent command and the directory ACP is given.
pub(crate) struct AgentCommand {
    pub(crate) command: CommandSpec,
    pub(crate) cwd: PathBuf,
    /// The protocol `command` speaks (`drivers`).
    pub(crate) driver: Driver,
    /// Why it is not the one the node's setting asks for, for the log.
    pub(crate) notices: Vec<String>,
}

impl SessionRuntime {
    /// Spawn a structured session's agent and its actor (a new session's,
    /// or one started again, `again`), and register it.
    pub(crate) fn run_structured(
        &self,
        launch: Launch,
        begin: Begin,
        notices: Vec<String>,
        actor: ActorStart,
    ) -> Result<Created, SessionError> {
        let admission = self.launch_admission();
        admission.check()?;
        // `fork`/`exec` on this thread, as `tokio::process` does.
        let (agent, input, outputs) =
            AgentProcess::spawn_with(launch.driver, &launch.command, launch.setup.clone())
                .map_err(SessionError::agent_not_started)?;
        if admission.check().is_err() {
            // Dropping the agent kills its process group.
            drop(agent);
            return Err(SessionError::ShuttingDown);
        }

        let profile = begin.info.profile.clone();
        let (route, routed) = watch::channel(Route::Live(input.clone()));
        // One move at a time; a second waits for the first's answer.
        let (control, orders) = mpsc::channel(1);
        let process = agent.lifetime();
        let parts = self.parts(
            begin,
            notices,
            (Input::Structured(routed), process),
            (Some(control), launch.driver),
        );
        let ActorStart {
            desk,
            replaying,
            gate,
            opening,
        } = actor;
        let park_setting = self.park_setting();
        let parking = Parking::new(
            *park_setting.borrow(),
            launch.setup.options.clone(),
            launch.setup.resume.clone(),
        );
        let actor = StructuredActor {
            agent,
            #[cfg(test)]
            stop_signal_error: None,
            outputs,
            outputs_open: true,
            input,
            route,
            control: orders,
            launch,
            profile,
            life: parts.life,
            ready: false,
            disconnect: None,
            failed_before_kill: false,
            move_state: MoveState {
                replaying,
                ..MoveState::default()
            },
            desk,
            gate,
            loader: self.attachments.clone(),
            parking,
            park_setting,
            opening,
            unrecoverable: None,
        };
        Ok(self.start(parts.shared, actor.run(), parts.durable))
    }

    /// The command that starts `spec`'s agent under `profile`, and the
    /// directory ACP is given.
    pub(crate) async fn agent_command(
        &self,
        spec: SessionSpec,
        profile: Profile,
        proxy: Option<ModelProxy>,
    ) -> Result<AgentCommand, SessionError> {
        let chosen = self.choose_driver(&spec, &profile).await;
        let driver = chosen.driver();
        let notices = chosen.notices;
        let lookup = self.config.harness_lookup.clone();
        let env = self.config.harness_env.clone();
        #[cfg(test)]
        let gate = self.lookup_gate.take();
        // Lookup and directory checks block.
        tokio::task::spawn_blocking(move || {
            let command = launch_for(driver, &lookup, &env, &spec, &profile, proxy.as_ref())?;
            #[cfg(test)]
            if let Some(gate) = gate {
                gate.pause()?;
            }
            // ACP needs an explicit directory; the launch defaults it to
            // the home directory, and the node's own is the last resort.
            let cwd = command
                .cwd
                .clone()
                .or_else(|| std::env::current_dir().ok())
                .ok_or(SessionError::NoWorkingDirectory)?;
            Ok::<_, SessionError>(AgentCommand {
                command,
                cwd,
                driver,
                notices,
            })
        })
        .await
        .map_err(SessionError::Internal)?
    }

    /// What `harness`'s agent offers before any session of it exists
    /// (protocol 9's `harness.options`, `vessel_agent::probe`): started as
    /// a structured session would be, under `profile` (the harness's
    /// default without one) and `proxy` (a routed profile's), in `cwd`,
    /// without MCP servers (they change nothing an agent offers, and
    /// starting them would only slow the probe), then stopped. No prompt
    /// is sent. Bounded by the handshake timeout.
    ///
    /// # Errors
    /// [`SessionError::Launch`] when the harness or its adapter is missing
    /// or `cwd` is unusable, [`SessionError::Probe`] when the agent did not
    /// start or complete its handshake (the message says why: not signed
    /// in, for example), [`SessionError::ShuttingDown`] after shutdown
    /// began.
    pub async fn probe(
        &self,
        harness: HarnessRef,
        profile: Option<Profile>,
        proxy: Option<ModelProxy>,
        cwd: PathBuf,
    ) -> Result<vessel_agent::Probed, SessionError> {
        self.probe_selected(
            harness,
            profile,
            proxy,
            cwd,
            ProbeChoices::default(),
            ProbePurpose::Options,
        )
        .await
    }

    /// Probe exactly a draft's model and option choices without sending a prompt.
    /// Directory authorization belongs to the node before it calls this low-level path.
    ///
    /// # Errors
    /// As [`Self::probe`]; unsupported choices fail the prompt-free handshake.
    pub async fn probe_with_choices(
        &self,
        harness: HarnessRef,
        profile: Option<Profile>,
        proxy: Option<ModelProxy>,
        cwd: PathBuf,
        choices: ProbeChoices,
    ) -> Result<vessel_agent::Probed, SessionError> {
        self.probe_selected(harness, profile, proxy, cwd, choices, ProbePurpose::Catalog)
            .await
    }

    async fn probe_selected(
        &self,
        harness: HarnessRef,
        profile: Option<Profile>,
        proxy: Option<ModelProxy>,
        cwd: PathBuf,
        choices: ProbeChoices,
        purpose: ProbePurpose,
    ) -> Result<vessel_agent::Probed, SessionError> {
        self.launch_admission().check()?;
        let mode = SessionMode::Structured {
            model: choices.model.clone(),
            resume: None,
            options: choices.options.clone(),
        };
        let profile = profile.map_or_else(|| Profile::default_of(&harness), Ok)?;
        let mut spec = SessionSpec::new(harness.clone(), mode).with_cwd(cwd);
        if !profile.is_default() {
            spec = spec.with_profile(profile.name().clone());
        }
        let AgentCommand {
            command,
            cwd,
            driver,
            ..
        } = self.agent_command(spec, profile, proxy).await?;
        self.launch_admission().check()?;
        let mut setup = probe_setup(cwd, self.config.agent_handshake_timeout);
        setup.model = choices.model;
        setup.options = choices.options;
        let mut probed = match purpose {
            ProbePurpose::Options => vessel_agent::probe_with(driver, &command, setup).await?,
            ProbePurpose::Catalog => {
                vessel_agent::probe_catalog_with(driver, &command, setup).await?
            }
        };
        if let Some(commands) = probed.commands.as_mut() {
            vessel_harness::classify_commands(&harness, commands);
        }
        Ok(probed)
    }

    /// A page of `harness`'s own conversations (protocol 17's
    /// `agent.history`, `vessel_agent::history`), those in `folder` or all
    /// of them, from `from`: the agent started as [`probe`](Self::probe)
    /// starts it (under `profile`, `proxy`, in `cwd`), asked
    /// `session/list`, and stopped. No session is started or loaded.
    /// Bounded by the handshake timeout.
    ///
    /// # Errors
    /// [`SessionError::Launch`] as for a probe, [`SessionError::History`]
    /// when the agent does not list or did not answer,
    /// [`SessionError::ShuttingDown`] after shutdown began.
    pub async fn history(
        &self,
        harness: HarnessRef,
        (profile, proxy): (Option<Profile>, Option<ModelProxy>),
        cwd: PathBuf,
        folder: Option<&std::path::Path>,
        from: vessel_agent::HistoryFrom,
    ) -> Result<vessel_agent::HistoryPage, SessionError> {
        self.launch_admission().check()?;
        let mode = SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        };
        let profile = profile.map_or_else(|| Profile::default_of(&harness), Ok)?;
        let mut spec = SessionSpec::new(harness, mode).with_cwd(cwd);
        if !profile.is_default() {
            spec = spec.with_profile(profile.name().clone());
        }
        let AgentCommand {
            command, driver, ..
        } = self.agent_command(spec, profile, proxy).await?;
        let limit = self.config.agent_handshake_timeout;
        self.launch_admission().check()?;
        Ok(vessel_agent::history_with(driver, &command, folder, from, limit).await?)
    }

    /// Move a live structured session to another profile (ADR 0011): link
    /// its files into the target home, restart its agent there with
    /// `session/load`, and publish `ProfileChanged`. The session keeps its
    /// id, its log and its subscribers. See `moving` for the order and what
    /// a failure leaves.
    ///
    /// # Errors
    /// [`SessionError::NotFound`], [`SessionError::Ended`],
    /// [`SessionError::Launch`] (the profile is for another harness, or the
    /// adapter is missing), and [`SessionError::Move`] for the rest.
    pub async fn move_profile(
        &self,
        id: SessionId,
        request: ProfileMove,
    ) -> Result<SessionInfo, SessionError> {
        let handle = self.get(id).ok_or(SessionError::NotFound(id))?;
        let info = handle.info();
        if !info.status.is_live() {
            return Err(SessionError::Ended);
        }
        let SessionMode::Structured { model, .. } = &info.spec.mode else {
            return Err(MoveError::Terminal.into());
        };
        let model = model.clone();
        // The new agent starts in the agent's defaults; the session keeps
        // what it runs with (its permissions mode above all) instead.
        let options = handle.choices();
        let harness_session = info.harness_session.clone().ok_or(MoveError::NotReady)?;
        let control = handle.0.control.clone().ok_or(MoveError::Terminal)?;
        let ProfileMove {
            profile,
            limits,
            from_home,
            to_home,
            mcp_servers,
            notices,
            force,
            model_proxy,
        } = request;
        let to = (!profile.is_default()).then(|| profile.name().clone());
        // The session keeps its driver: its options are that driver's.
        let driver = handle.driver();
        // One agent per conversation (`claims`): from the link on, its
        // files are under the target profile too, so the move claims it
        // there before it starts, and the session holds both after.
        let mut profiles = self.held(id, info.profile.as_ref());
        let _target = profiles.insert(to.clone());
        let claim = self.claim(id, &info.spec.harness, profiles, &harness_session)?;
        let spec = info.spec.with_profile(profile.name().clone());
        let lookup = self.config.harness_lookup.clone();
        let env = self.config.harness_env.clone();
        let session = harness_session.clone();
        let from = from_home.clone();
        let (command, cwd, files) = tokio::task::spawn_blocking(move || {
            let command = launch_for(driver, &lookup, &env, &spec, &profile, model_proxy.as_ref())?;
            let cwd = command
                .cwd
                .clone()
                .or_else(|| std::env::current_dir().ok())
                .ok_or(SessionError::NoWorkingDirectory)?;
            let files = vessel_harness::session_files(&spec.harness, &from, &cwd, &session)
                .map_err(MoveError::Files)?;
            Ok::<_, SessionError>((command, cwd, files))
        })
        .await
        .map_err(SessionError::Internal)??;
        let setup = AgentSetup {
            cwd,
            model,
            resume: Some(harness_session),
            options,
            mcp_servers,
            handshake_timeout: self.config.agent_handshake_timeout,
            // The session's own, which the actor keeps (`moving`): a move
            // never changes whether the agent may ask.
            questions: false,
            fork: None,
            system_prompt: None,
            private_dir: self.private_dir(),
        };
        let (reply, answer) = oneshot::channel();
        let order = MoveOrder {
            limits,
            command,
            setup,
            to,
            files,
            from_home,
            to_home,
            notices,
            force,
            reply,
        };
        if control.send(Control::Move(Box::new(order))).await.is_err() {
            return Err(SessionError::Ended);
        }
        let moved = answer.await.map_err(|_ended| SessionError::Ended)?;
        // A move whose new agent did not start leaves its links in the
        // target home (`moving`), so the session holds that profile too. A
        // parked session that did not wake never linked.
        if matches!(
            moved,
            Ok(())
                | Err(MoveError::Restart(
                    Restart::Spawn(_) | Restart::Handshake(_) | Restart::Exited(_)
                ))
        ) {
            claim.keep();
        }
        moved?;
        Ok(handle.info())
    }
}

/// A probe's setup: no session, no MCP servers, no questions, and no
/// private directory, so its launch writes no file
/// (`AgentSetup::private_dir`).
pub(super) fn probe_setup(cwd: PathBuf, handshake_timeout: Duration) -> AgentSetup {
    AgentSetup {
        cwd,
        model: None,
        resume: None,
        options: Vec::new(),
        mcp_servers: ResolvedMcpServers::new(),
        handshake_timeout,
        questions: false,
        fork: None,
        system_prompt: None,
        private_dir: None,
    }
}

/// `spec`'s command under `profile` for `driver`: an ACP launch (MCP
/// servers go over ACP, not here), or the agent's own protocol's
/// (`vessel_harness::launch_direct`).
fn launch_for(
    driver: Driver,
    lookup: &vessel_harness::Lookup,
    env: &vessel_harness::HarnessEnv,
    spec: &SessionSpec,
    profile: &Profile,
    proxy: Option<&ModelProxy>,
) -> Result<CommandSpec, SessionError> {
    let launch = match driver {
        Driver::Acp => {
            let none = ResolvedMcpServers::new();
            vessel_harness::launch(lookup, env, spec, profile, proxy, &none, None)?
        }
        Driver::CodexAppServer | Driver::ClaudeStream | Driver::OpencodeHttp => {
            vessel_harness::launch_direct(lookup, env, spec, profile, proxy)?
        }
    };
    Ok(launch.command)
}
