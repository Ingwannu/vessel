//! The ACP handshake: `initialize`, then `session/new` or `session/load`,
//! then the model and the client's option choices, all within
//! [`AgentSetup::handshake_timeout`].
//!
//! [Decision Log: choices in the handshake] (protocol 9, ADR 0022)
//! - Purpose: a session asked to start in plan mode, or at a low effort,
//!   must not run a turn in another; and the first options a client sees
//!   must be the ones the session will run with.
//! - Choice: the choices are applied here, in order, after the model and
//!   before the session is ready, each checked against what the agent
//!   reported (after the changes before it, since one change can move
//!   another: a model can change the efforts offered). A choice the agent
//!   does not offer, or refuses, fails the handshake as an unknown model
//!   does; a choice already in effect sends nothing. The options as they
//!   stand at the end are the handshake's result, so the session's first
//!   `options_changed` is what it runs with.
//! - The same holds after `session/load`: a resumed session with choices
//!   gets them after the load.
//!
//! [Decision Log]
//! - Purpose: an adapter that never answers `initialize`, `session/new` or
//!   `session/load` (hung, waiting for a login prompt nobody sees) left the
//!   session `Starting` forever, and every input sent to it waited, holding
//!   one of its connection's in-flight call slots on the node.
//! - Choice: one deadline for the whole handshake. When it passes, the
//!   handshake fails like any other (`Disconnect::Handshake`), the session
//!   stops the process and ends `Failed`, and waiting input is refused.
//! - Why the whole handshake rather than each request: the steps are
//!   sequential and none of them is useful alone; one budget is what a user
//!   waiting for "ready" experiences.
//! - Why 60 s by default: `session/load` replays the whole conversation
//!   before it answers, and a long one through a slow store can take tens of
//!   seconds; a healthy adapter starts in a few.

use std::time::Duration;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::v1::ConnectionTo;
use agent_client_protocol::v1::schema as acp;
use agent_client_protocol::{Agent, Error};
use vessel_proto::{AgentAbilities, AgentMessageId, HarnessSessionId, OptionId};

use crate::introduction::{self, Introduction, Refusal, Refused};
use crate::model::ModelError;
use crate::options::{Applied, Change, OptionError, Options};
use crate::{abilities, mcp};

/// The default for [`AgentSetup::handshake_timeout`].
pub const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(60);

pub use crate::setup::AgentSetup;

/// A conversation to fork, and where (protocol 14, ADR 0052).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkFrom {
    /// The agent's conversation forked.
    pub session: HarnessSessionId,
    /// The agent message the fork ends with; `None` for the whole
    /// conversation. A fork at a point is checked: the fork is loaded on
    /// the same connection and its replay must end with this message.
    pub point: Option<AgentMessageId>,
}

/// Why a fork was refused, for the caller to answer `unsupported` rather
/// than a failure to start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForkRefusal {
    /// The agent does not advertise `session/fork`.
    CannotFork,
    /// The agent did not fork at the point: it did not know the message,
    /// or its fork's replay ends elsewhere (it ignored the anchor).
    NotAtPoint,
}

/// Why the handshake failed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum HandshakeError {
    #[error("`initialize` failed")]
    Initialize(#[source] Error),
    #[error("the agent speaks ACP version {version:?}; Vessel speaks version 1")]
    Version {
        version: ProtocolVersion,
        introduction: Introduction,
    },
    #[error("the agent cannot resume sessions (it does not support `session/load`)")]
    CannotResume,
    #[error("the agent cannot fork a conversation (it does not offer `session/fork`)")]
    CannotFork,
    #[error("`session/fork` failed")]
    Fork(#[source] Error),
    #[error("the agent did not fork at the chosen point")]
    NotAtPoint,
    #[error("`session/new` failed")]
    NewSession {
        #[source]
        source: Error,
        introduction: Introduction,
    },
    #[error("`session/load` failed")]
    LoadSession(#[source] Error),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error("selecting the model failed")]
    SetModel(#[source] Error),
    #[error(transparent)]
    Option(#[from] OptionError),
    #[error("the agent refused option `{id}`")]
    SetOption {
        id: OptionId,
        #[source]
        source: Error,
    },
    #[error("the agent did not complete the ACP handshake within {} s", .0.as_secs())]
    TimedOut(Duration),
}

impl HandshakeError {
    /// Whether this is a fork refused (rather than an agent that failed).
    pub(crate) const fn fork_refused(&self) -> Option<ForkRefusal> {
        match self {
            Self::CannotFork => Some(ForkRefusal::CannotFork),
            Self::NotAtPoint => Some(ForkRefusal::NotAtPoint),
            Self::Initialize(_)
            | Self::Version { .. }
            | Self::CannotResume
            | Self::Fork(_)
            | Self::NewSession { .. }
            | Self::LoadSession(_)
            | Self::Model(_)
            | Self::SetModel(_)
            | Self::Option(_)
            | Self::SetOption { .. }
            | Self::TimedOut(_) => None,
        }
    }

    /// Why, as `agent.status` reads it; `None` for a step a probe never
    /// takes (a model, an option, a resume).
    pub(crate) fn refused(&self) -> Option<Refused> {
        let (refusal, introduction) = match self {
            Self::Initialize(_) => (Refusal::Initialize, Introduction::default()),
            Self::Version {
                version,
                introduction,
            } => (
                Refusal::Version {
                    speaks: introduction::number(*version),
                },
                introduction.clone(),
            ),
            Self::NewSession {
                source,
                introduction,
            } => (
                if source.code == agent_client_protocol::ErrorCode::AUTH_REQUIRED {
                    Refusal::SignIn
                } else {
                    Refusal::SessionNew
                },
                introduction.clone(),
            ),
            Self::TimedOut(limit) => (Refusal::TimedOut { after: *limit }, Introduction::default()),
            Self::CannotResume
            | Self::CannotFork
            | Self::Fork(_)
            | Self::NotAtPoint
            | Self::LoadSession(_)
            | Self::Model(_)
            | Self::SetModel(_)
            | Self::Option(_)
            | Self::SetOption { .. } => return None,
        };
        Some(Refused {
            refusal,
            introduction,
        })
    }
}

/// A session the agent accepted, its options after the choices, what the
/// agent said it can do, and what it said of itself.
#[derive(Debug)]
pub(crate) struct Started {
    pub(crate) session: acp::SessionId,
    /// The point a fork was asked at, for the driver to check against the
    /// fork's replay (protocol 14).
    pub(crate) fork_point: Option<AgentMessageId>,
    /// The agent deletes a session it keeps (`sessionCapabilities.delete`):
    /// a fork that fails its check is deleted rather than left in the
    /// agent's history (protocol 14).
    pub(crate) deletes: bool,
    pub(crate) options: Options,
    pub(crate) abilities: AgentAbilities,
    pub(crate) introduction: Introduction,
}

/// [`run`], bounded by `setup.handshake_timeout`.
pub(crate) async fn run_within(
    cx: &ConnectionTo<Agent>,
    setup: &AgentSetup,
) -> Result<Started, HandshakeError> {
    let limit = setup.handshake_timeout;
    tokio::time::timeout(limit, run(cx, setup))
        .await
        .unwrap_or(Err(HandshakeError::TimedOut(limit)))
}

/// Run the handshake; returns the ACP session and its options.
///
/// Client capabilities are deliberately small: no `fs` and no `terminal`,
/// so agents use their own tools (Vessel wraps harnesses; it does not
/// re-implement their file and shell access, ADR 0003). Form elicitation
/// only when `setup.questions` says so; never URL elicitation (a link the
/// agent chose, opened from a page on another machine).
pub(crate) async fn run(
    cx: &ConnectionTo<Agent>,
    setup: &AgentSetup,
) -> Result<Started, HandshakeError> {
    let mut capabilities = acp::ClientCapabilities::new();
    if setup.questions {
        capabilities = capabilities.elicitation(
            acp::ElicitationCapabilities::new().form(acp::ElicitationFormCapabilities::new()),
        );
    }
    let initialized = cx
        .send_request(
            acp::InitializeRequest::new(ProtocolVersion::V1)
                .client_capabilities(capabilities)
                .client_info(acp::Implementation::new(
                    "vessel",
                    env!("CARGO_PKG_VERSION"),
                )),
        )
        .block_task()
        .await
        .map_err(HandshakeError::Initialize)?;
    let introduction = introduction::of(&initialized);
    if initialized.protocol_version != ProtocolVersion::V1 {
        return Err(HandshakeError::Version {
            version: initialized.protocol_version,
            introduction,
        });
    }
    let abilities = abilities::of(&initialized);

    let (servers, left_out) = mcp::to_acp(
        &setup.mcp_servers,
        &initialized.agent_capabilities.mcp_capabilities,
    );
    if !left_out.is_empty() {
        tracing::warn!(
            servers = ?left_out,
            "the agent does not take HTTP MCP servers; the session starts without them"
        );
    }
    let (session, mut options, fork_point) = if let Some(fork) = &setup.fork {
        forked(cx, setup, fork, &initialized, servers).await?
    } else if let Some(resume) = &setup.resume {
        if !initialized.agent_capabilities.load_session {
            return Err(HandshakeError::CannotResume);
        }
        let session = acp::SessionId::new(resume.as_str());
        let loaded = cx
            .send_request(
                acp::LoadSessionRequest::new(session.clone(), &setup.cwd).mcp_servers(servers),
            )
            .block_task()
            .await
            .map_err(HandshakeError::LoadSession)?;
        (
            session,
            Options::new(loaded.config_options, loaded.modes),
            None,
        )
    } else {
        let created = cx
            .send_request(acp::NewSessionRequest::new(&setup.cwd).mcp_servers(servers))
            .block_task()
            .await
            .map_err(|source| HandshakeError::NewSession {
                source,
                introduction: introduction.clone(),
            })?;
        let options = Options::new(created.config_options, created.modes);
        (created.session_id, options, None)
    };

    if let Some(wanted) = &setup.model
        && let Some(change) = options.model(wanted)?
    {
        let applied = send(cx, &session, change)
            .await
            .map_err(HandshakeError::SetModel)?;
        options.apply(applied);
    }
    for choice in &setup.options {
        if let Some(change) = options.change(choice)? {
            let applied =
                send(cx, &session, change)
                    .await
                    .map_err(|source| HandshakeError::SetOption {
                        id: choice.id.clone(),
                        source,
                    })?;
            options.apply(applied);
        }
    }
    Ok(Started {
        session,
        fork_point,
        deletes: deletes(&initialized),
        options,
        abilities,
        introduction,
    })
}

/// `session/fork` of `fork.session`, at `fork.point` when there is one
/// (the anchor both installed adapters read, `_meta.jetbrains.air.fork`),
/// then, for a point, `session/load` of the fork on this connection: its
/// replay is how the driver checks where the fork ends (both adapters load
/// a session they just forked: Claude's reads the transcript again, Codex's
/// reattaches and replays).
async fn forked(
    cx: &ConnectionTo<Agent>,
    setup: &AgentSetup,
    fork: &ForkFrom,
    initialized: &acp::InitializeResponse,
    servers: Vec<acp::McpServer>,
) -> Result<(acp::SessionId, Options, Option<AgentMessageId>), HandshakeError> {
    if initialized
        .agent_capabilities
        .session_capabilities
        .fork
        .is_none()
    {
        return Err(HandshakeError::CannotFork);
    }
    // Without `session/load` a fork at a point cannot be checked.
    if fork.point.is_some() && !initialized.agent_capabilities.load_session {
        return Err(HandshakeError::CannotFork);
    }
    let mut request =
        acp::ForkSessionRequest::new(acp::SessionId::new(fork.session.as_str()), &setup.cwd)
            .mcp_servers(servers.clone());
    if let Some(point) = &fork.point {
        request = request.meta(anchor(point));
    }
    let forked = cx
        .send_request(request)
        .block_task()
        .await
        .map_err(|source| {
            // An anchor the agent does not know is `invalid_params`: the
            // point is not one it can fork at.
            if fork.point.is_some()
                && source.code == agent_client_protocol::ErrorCode::INVALID_PARAMS
            {
                HandshakeError::NotAtPoint
            } else {
                HandshakeError::Fork(source)
            }
        })?;
    let session = forked.session_id;
    let Some(point) = &fork.point else {
        return Ok((
            session,
            Options::new(forked.config_options, forked.modes),
            None,
        ));
    };
    let loaded = match cx
        .send_request(
            acp::LoadSessionRequest::new(session.clone(), &setup.cwd).mcp_servers(servers),
        )
        .block_task()
        .await
    {
        Ok(loaded) => loaded,
        Err(error) => {
            // The fork exists but cannot be checked: nobody will use it.
            if deletes(initialized) {
                delete_orphan(cx, &session).await;
            }
            return Err(HandshakeError::LoadSession(error));
        }
    };
    Ok((
        session,
        Options::new(loaded.config_options, loaded.modes),
        Some(point.clone()),
    ))
}

/// Whether the agent deletes a session it keeps (`sessionCapabilities.delete`;
/// both installed adapters do: Claude's deletes the transcript, Codex's
/// archives the thread, either way it leaves the agent's list).
fn deletes(initialized: &acp::InitializeResponse) -> bool {
    initialized
        .agent_capabilities
        .session_capabilities
        .delete
        .is_some()
}

/// How long deleting a refused fork may take before it is given up.
const DELETE_WITHIN: Duration = Duration::from_secs(5);

/// Delete a fork made for a check that refused it (or could not run), so
/// it is not left in the agent's history as a conversation nobody holds
/// (protocol 14, ADR 0052). Best effort: a failure is logged, the refusal
/// stands either way.
pub(crate) async fn delete_orphan(cx: &ConnectionTo<Agent>, session: &acp::SessionId) {
    let deleted = tokio::time::timeout(
        DELETE_WITHIN,
        cx.send_request(acp::DeleteSessionRequest::new(session.clone()))
            .block_task(),
    )
    .await;
    match deleted {
        Ok(Ok(_)) => tracing::info!(%session, "deleted the refused fork from the agent's history"),
        Ok(Err(error)) => {
            tracing::warn!(%session, %error, "the agent did not delete the refused fork");
        }
        Err(_elapsed) => {
            tracing::warn!(%session, "the agent did not delete the refused fork in time");
        }
    }
}

/// The fork point as both installed adapters read it.
fn anchor(point: &AgentMessageId) -> acp::Meta {
    let mut fork = serde_json::Map::new();
    let _version = fork.insert("version".to_owned(), serde_json::Value::from(1));
    let _message = fork.insert(
        "messageId".to_owned(),
        serde_json::Value::from(point.as_str()),
    );
    let mut air = serde_json::Map::new();
    let _fork = air.insert("fork".to_owned(), serde_json::Value::Object(fork));
    let mut jetbrains = serde_json::Map::new();
    let _air = jetbrains.insert("air".to_owned(), serde_json::Value::Object(air));
    let mut meta = acp::Meta::new();
    let _jetbrains = meta.insert("jetbrains".to_owned(), serde_json::Value::Object(jetbrains));
    meta
}

/// Ask the agent to make `change`; its answer says what changed. Awaits
/// the response, so the driver runs it as a task of its own (see
/// `driver`), as the handshake runs.
pub(crate) async fn send(
    cx: &ConnectionTo<Agent>,
    session: &acp::SessionId,
    change: Change,
) -> Result<Applied, Error> {
    match change {
        Change::Config { config, value } => cx
            .send_request(acp::SetSessionConfigOptionRequest::new(
                session.clone(),
                config,
                value,
            ))
            .block_task()
            .await
            .map(|response| Applied::Config(response.config_options)),
        Change::Mode(mode) => cx
            .send_request(acp::SetSessionModeRequest::new(
                session.clone(),
                mode.clone(),
            ))
            .block_task()
            .await
            .map(|_response| Applied::Mode(mode)),
    }
}
