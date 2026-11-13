//! Starting a new session from its spec: the launch options the node
//! decides, what the session records about its start, and the two modes'
//! own starts (a terminal's program in a PTY, a structured session's
//! agent through `agent`).

use std::path::PathBuf;

use chrono::Utc;
use vessel_agent::AgentSetup;
use vessel_harness::{McpFile, ModelProxy, Profile};
use vessel_proto::{
    Creator, HarnessSessionId, ModelId, NodeId, OptionChoice, ResolvedMcpServers, SessionId,
    SessionInfo, SessionMode, SessionModel, SessionOrigin, SessionSpec, TerminalSize,
};
use vessel_pty::{PtyProcess, WindowSize};

use super::agent::AgentCommand;
use super::parts::{ActorStart, Begin};
use super::{Created, SessionRuntime};
use crate::error::SessionError;
use crate::handle::Input;
use crate::moving::Launch;
use crate::terminal::TerminalActor;

/// How a session is launched beyond its spec: the node decides these
/// (ADR 0011, ADR 0012), clients only name them.
///
/// [Decision Log]
/// - Purpose: profiles and MCP servers reach `vessel_harness::launch` and
///   the ACP handshake from the node, which resolves the profile name and
///   the servers' secrets.
/// - Constraint: `create(spec)` is the node's call today and `SessionSpec`
///   gains its `profile` field only in contract step 1b, so this is an
///   additive `create_with`; `create` is `create_with` with the defaults.
/// - Debug shows names only: `Profile` and `ResolvedValue` redact values.
#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    /// The profile `spec.profile` names, resolved by the caller; `None` is
    /// the spec's harness's default profile (the launch from before
    /// profiles), which is right only when the spec names no profile.
    pub profile: Option<Profile>,
    /// MCP servers, values resolved. Structured sessions get them over ACP;
    /// terminal ones in the harness's launch-time form, when it has one.
    pub mcp_servers: ResolvedMcpServers,
    /// Where a terminal session's MCP configuration file goes: an
    /// owner-only directory of the node's (ADR 0012). There is no shared
    /// fallback: without one, a terminal session whose harness reads its
    /// servers from a file is refused.
    pub mcp_dir: Option<PathBuf>,
    /// Who asks for the session (ADR 0013), recorded in its
    /// `SessionInfo::created_by`; the owner by default.
    pub created_by: Creator,
    /// Lines Vessel adds to the session's log right after
    /// `Status{Starting}`, each a `HarnessLog` record (in either mode),
    /// starting with `vessel:` by convention: for example MCP servers left
    /// out because a secret is missing (ADR 0012).
    pub notices: Vec<String>,
    /// The session's id, when the caller needs it before the session starts
    /// (an agent credential is bound to it, ADR 0013); a fresh random one
    /// otherwise. An id the runtime already has is refused.
    pub id: Option<SessionId>,
    /// Where the session came from when another node of the fleet had a
    /// hand in it (protocol 6): a call forwarded from it, or an import of
    /// one of its sessions. Recorded in `SessionInfo::origin`; `None` for
    /// this node's own.
    pub origin: Option<SessionOrigin>,
    /// The home a creating call came through (protocol 6), recorded in
    /// `SessionInfo::via`.
    pub via: Option<NodeId>,
    /// The model route the session's harness is pointed at through the
    /// node's proxy (protocol 7), recorded in `SessionInfo::model`; `None`
    /// for a session on its harness's own provider.
    pub model: Option<SessionModel>,
    /// Where the harness of a profile with a model route sends its calls,
    /// with the session's token (ADR 0015); given exactly when `profile`
    /// has a route, or the launch is refused. Its `Debug` hides the token.
    pub model_proxy: Option<ModelProxy>,
    /// Whether a structured session's agent may ask questions (protocol
    /// 10, `vessel_agent::AgentSetup::questions`): the node's choice, true
    /// for a session someone watches. Off by default. Kept for the
    /// session's life: an agent a move starts again asks as the first did.
    pub questions: bool,
}

/// What a session records about its start besides its spec.
struct Record {
    id: SessionId,
    created_by: Creator,
    origin: Option<SessionOrigin>,
    via: Option<NodeId>,
    model: Option<SessionModel>,
    notices: Vec<String>,
}

impl Record {
    /// A new session of `spec`'s first info, created now, and its notices.
    fn begin(self, spec: SessionSpec) -> (Begin, Vec<String>) {
        let Self {
            id,
            created_by,
            origin,
            via,
            model,
            notices,
        } = self;
        let mut info = SessionInfo::new(id, spec, Utc::now()).with_creator(created_by);
        info.origin = origin;
        info.via = via;
        info.model = model;
        let since = info.created_at;
        (Begin::new(info, since), notices)
    }
}

/// A terminal session's MCP servers and where their file goes.
struct TerminalMcp {
    servers: ResolvedMcpServers,
    dir: Option<PathBuf>,
}

/// What a structured session starts with besides its process.
struct AgentStart {
    model: Option<ModelId>,
    resume: Option<HarnessSessionId>,
    options: Vec<OptionChoice>,
    mcp_servers: ResolvedMcpServers,
    questions: bool,
}

impl SessionRuntime {
    /// Start a session. Status events are part of the stream: the first
    /// record is `Status{Starting}`.
    ///
    /// A structured session's handshake failing is not an error here: the
    /// session exists and ends with `Status{Failed}`, with the agent's log.
    ///
    /// # Errors
    /// [`SessionError::Launch`] when the harness, its adapter or the working
    /// directory is missing, [`SessionError::Spawn`] or
    /// [`SessionError::StartAgent`] when the program cannot be started, and
    /// [`SessionError::ShuttingDown`] after shutdown began.
    pub async fn create(&self, spec: SessionSpec) -> Result<Created, SessionError> {
        self.create_with(spec, LaunchOptions::default()).await
    }

    /// [`create`](Self::create) with a profile and MCP servers.
    ///
    /// A terminal session's MCP configuration file, if its harness takes
    /// one, is written right before the program starts and removed when the
    /// session ends. Servers a terminal harness cannot be given are logged
    /// by name; the session starts without them.
    ///
    /// # Errors
    /// As [`create`](Self::create), and [`SessionError::Launch`] for a
    /// profile of another harness or servers the harness cannot take,
    /// [`SessionError::McpFile`] when the MCP file cannot be written.
    pub async fn create_with(
        &self,
        spec: SessionSpec,
        options: LaunchOptions,
    ) -> Result<Created, SessionError> {
        // Counts as a running task, so a concurrent `shutdown` waits until
        // this session is either refused or tracked.
        let _in_flight = self.tasks.token();
        if self.shutdown.is_cancelled() {
            return Err(SessionError::ShuttingDown);
        }
        let LaunchOptions {
            profile,
            mcp_servers,
            mcp_dir,
            created_by,
            notices,
            id,
            origin,
            via,
            model,
            model_proxy,
            questions,
        } = options;
        let id = id.unwrap_or_else(SessionId::random);
        if self.get(id).is_some() {
            return Err(SessionError::DuplicateId(id));
        }
        let record = Record {
            id,
            created_by,
            origin,
            via,
            model,
            notices,
        };
        let profile = profile.map_or_else(|| Profile::default_of(&spec.harness), Ok)?;
        match spec.mode.clone() {
            SessionMode::Terminal { size, .. } => {
                let mcp = TerminalMcp {
                    servers: mcp_servers,
                    dir: mcp_dir,
                };
                self.create_terminal(spec, size, (profile, model_proxy), mcp, record)
                    .await
            }
            SessionMode::Structured {
                model,
                resume,
                options,
            } => {
                let start = AgentStart {
                    model,
                    resume,
                    options,
                    mcp_servers,
                    questions,
                };
                self.create_structured(spec, (profile, model_proxy), start, record)
                    .await
            }
        }
    }

    async fn create_terminal(
        &self,
        spec: SessionSpec,
        size: TerminalSize,
        (profile, proxy): (Profile, Option<ModelProxy>),
        mcp: TerminalMcp,
        record: Record,
    ) -> Result<Created, SessionError> {
        let lookup = self.config.harness_lookup.clone();
        let env = self.config.harness_env.clone();
        let launch_spec = spec.clone();
        let window = WindowSize {
            cols: size.cols,
            rows: size.rows,
        };
        // Lookup, file writes, `fork`/`exec` and thread creation all block.
        let (pty, output, mcp_file) = tokio::task::spawn_blocking(move || {
            let launch = vessel_harness::launch(
                &lookup,
                &env,
                &launch_spec,
                &profile,
                proxy.as_ref(),
                &mcp.servers,
                mcp.dir.as_deref(),
            )?;
            if !launch.unattached.is_empty() {
                tracing::warn!(
                    servers = ?launch.unattached,
                    harness = %launch_spec.harness,
                    "this harness takes no MCP servers at launch in terminal mode; the session starts without them"
                );
            }
            // Written last and dropped (removed) if the program fails to
            // start.
            let mcp_file = launch
                .mcp_file
                .map(McpFile::write)
                .transpose()
                .map_err(SessionError::McpFile)?;
            let (pty, output) =
                PtyProcess::spawn(&launch.command, window).map_err(SessionError::Spawn)?;
            Ok::<_, SessionError>((pty, output, mcp_file))
        })
        .await
        .map_err(SessionError::Internal)??;
        if self.shutdown.is_cancelled() {
            // Dropping the PTY kills its program.
            drop(pty);
            return Err(SessionError::ShuttingDown);
        }

        let (begin, notices) = record.begin(spec);
        let parts = self.parts(
            begin,
            notices,
            (Input::Terminal(pty.input()), pty.pid()),
            None,
        );
        let actor = TerminalActor {
            pty,
            output,
            life: parts.life,
            _mcp_file: mcp_file,
        };
        Ok(self.start(parts.shared, actor.run(), parts.durable))
    }

    async fn create_structured(
        &self,
        spec: SessionSpec,
        (profile, proxy): (Profile, Option<ModelProxy>),
        start: AgentStart,
        record: Record,
    ) -> Result<Created, SessionError> {
        let AgentCommand { command, cwd } =
            self.agent_command(spec.clone(), profile, proxy).await?;
        let setup = AgentSetup {
            cwd,
            model: start.model,
            resume: start.resume,
            options: start.options,
            mcp_servers: start.mcp_servers,
            handshake_timeout: self.config.agent_handshake_timeout,
            questions: start.questions,
        };
        let (begin, notices) = record.begin(spec);
        self.run_structured(
            Launch { command, setup },
            begin,
            notices,
            ActorStart::default(),
        )
    }
}
