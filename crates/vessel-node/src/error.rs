//! Errors: why a node could not start ([`NodeError`]), and why a call
//! failed (`ServiceError`, mapped to the wire in one place).

use std::net::SocketAddr;

use vessel_proto::{
    AgentName, DeviceInviteError, ErrorCode, ErrorDetail, EventSeq, HarnessKind, HarnessRef,
    ModelRouteName, ProfileName, SecretName, SessionId, WireError,
};
use vessel_rpc::OutboundError;
use vessel_session::SessionError;
use vessel_store::StoreError;

use crate::persist::Unpersisted;
use crate::profiles::ProfilesFileError;

/// An error and its causes on one line, for a log: a device connection
/// that "ended abnormally" with "transport failed" says nothing until its
/// causes (the channel's, the socket's) follow.
pub(crate) fn chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

/// A node failed to start, or its server failed.
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    /// The data directory, node id or token could not be prepared.
    #[error("the data directory is unusable")]
    DataDir(#[source] StoreError),
    /// The event log could not be opened.
    #[error("the event log could not be opened")]
    Store(#[source] StoreError),
    /// Another node already uses this data directory.
    #[error("another node already uses this data directory")]
    AlreadyRunning(#[source] StoreError),
    /// A client address header is set without a trusted proxy
    /// (`--client-address-header` needs `--trusted-proxy`).
    #[error(transparent)]
    NoTrustedProxy(#[from] vessel_relay::address::NoTrustedProxy),
    /// Relays are configured without `--relay-token-file`.
    #[error("a relay needs its registration token: pass --relay-token-file")]
    NoRelayToken,
    /// The relays' registration token file is unreadable or empty.
    #[error(transparent)]
    RelayToken(#[from] vessel_relay::TokenFileError),
    /// A `wss://` relay is configured, and no TLS root certificate could
    /// be loaded to verify it.
    #[error("cannot verify wss:// relays")]
    RelayTls(#[source] vessel_relay::TlsError),
    /// The local socket (or named pipe) could not be bound.
    #[error("cannot listen on the local socket {}", path.display())]
    LocalSocket {
        /// The socket path or pipe name.
        path: std::path::PathBuf,
        /// Why.
        #[source]
        source: std::io::Error,
    },
    /// The listen address could not be bound.
    #[error("cannot listen on {address}")]
    Bind {
        /// The address.
        address: SocketAddr,
        /// Why.
        #[source]
        source: std::io::Error,
    },
    /// `profiles.toml` cannot be read or does not parse; the node does not
    /// start rather than run sessions under a guess (ADR 0011).
    #[error("profiles.toml cannot be used")]
    Profiles(#[source] ProfilesFileError),
    /// `models.toml` cannot be read, is readable by other users, or does
    /// not parse; the node does not start rather than route calls under a
    /// guess (ADR 0015).
    #[error("models.toml cannot be used")]
    Models(#[source] crate::models::ModelsError),
    /// The client for model calls cannot be built.
    #[error("cannot build the client for model calls")]
    ModelClient(#[source] vessel_llm::client::ClientError),
    /// The model proxy's loopback port cannot be bound.
    #[error("cannot listen for model calls on loopback")]
    ModelProxy(#[source] std::io::Error),
    /// The data directory's MCP directory cannot be prepared.
    #[error("the MCP configuration directory is unusable")]
    McpDir(#[source] StoreError),
    /// The directory agents are probed in (protocol 9) cannot be created.
    #[error("the probe directory is unusable")]
    ProbeDir(#[source] StoreError),
    /// The folder of worktrees and setup logs (protocol 10) cannot be
    /// prepared.
    #[error("the workspace directory is unusable")]
    WorkspaceDir(#[source] StoreError),
    /// The HTTP server failed.
    #[error("the server failed")]
    Serve(#[source] std::io::Error),
    /// A startup or shutdown task failed unexpectedly.
    #[error("a node task failed")]
    Task(#[source] tokio::task::JoinError),
}

/// Why a call failed inside the node. Mapped to the wire once, by
/// [`ServiceError::to_wire`].
#[derive(Debug, thiserror::Error)]
pub(crate) enum ServiceError {
    /// No session with this id, live or stored.
    #[error("no session {0}")]
    NotFound(SessionId),
    /// A side-question boundary refused the action (ADR 0045).
    #[error(transparent)]
    SideQuestion(#[from] crate::side_questions::SideError),
    /// Side creation inherits a spec or returns an existing side; it cannot apply overrides.
    #[error("creation labels cannot override a side question's inherited spec")]
    SideCreationLabels,
    /// Side conversations inherit their existing spec instead of creating a new one.
    #[error("a listing override cannot replace a side question's inherited spec")]
    SideCreationListing,
    /// No protocol has published the label metadata yet.
    #[error("session labels have no assigned protocol version")]
    LabelsUnassigned,
    /// A manifest must not install unassigned discovery metadata before starting.
    #[error("session listing has no assigned protocol version")]
    ListingUnassigned,
    /// Export must preserve current metadata across every peer boundary.
    #[error(transparent)]
    MetadataProtocol(vessel_rpc::CallError),
    /// The session has ended and is served from the store (it ended in an
    /// earlier run, or in this one and was handed over); it can be read but
    /// not driven.
    #[error("session {0} has ended; its log can still be read")]
    Ended(SessionId),
    /// A `read` mark's `through` is past the end of the session's log
    /// (protocol 10).
    #[error("session {session} has {end} records; cannot mark it read through {through}")]
    ReadAhead {
        /// The session.
        session: SessionId,
        /// The `read`'s point.
        through: EventSeq,
        /// The log's length.
        end: EventSeq,
    },
    /// `session.delete` of a session that runs (protocol 10).
    #[error("session {0} is live; archive it first, which stops it")]
    Live(SessionId),
    /// `session.delete` or `session.resume` of a session that has ended
    /// while its log is still being written to the store (protocol 10).
    #[error("session {0} has just ended and its log is still being written; try again")]
    LogWriting(SessionId),
    /// An agent asked to mark a session read or unread (protocol 10).
    #[error("an agent may not mark a session read or unread: only a person sees it")]
    AgentMarksRead,
    /// `session.resume` of a session that runs (protocol 10).
    #[error("session {0} is live already")]
    AlreadyLive(SessionId),
    /// `session.resume` of a session another call is starting again
    /// (protocol 10).
    #[error("session {0} is being started again already")]
    StartingAgain(SessionId),
    /// `session.resume` of a session placed in a worktree that setup,
    /// teardown or a removal holds now (protocol 10).
    #[error("worktree {0} is being set up or removed; start the session again once that has ended")]
    WorktreeBusy(vessel_proto::WorktreeId),
    /// `session.resume` of a session whose folder is gone (protocol 10): a
    /// worktree removed by a person, or tidied when the session it was made
    /// for went, or a folder removed by hand. A conflict, not a refusal of
    /// the session itself: the folder put back, it starts again.
    #[error(
        "session {session} ran in {folder}, which is gone (a worktree removed, or a folder \
         deleted); it cannot start again there"
    )]
    FolderGone {
        /// Which session.
        session: SessionId,
        /// The folder it ran in.
        folder: std::path::PathBuf,
    },
    /// `session.resume` of a session whose profile `profiles.toml` no
    /// longer defines (protocol 10).
    #[error(
        "session {session} ran under profile `{profile}`, which this node no longer defines \
         (see `profile.list`)"
    )]
    ProfileRemoved {
        /// The session.
        session: SessionId,
        /// Its profile.
        profile: ProfileName,
    },
    /// `session.resume`: the agent did not start, or `session/load` failed
    /// (protocol 10); the attempt is in the session's log, which ends with
    /// it, and the session stays ended.
    #[error("session {session} did not start again: {why}")]
    DidNotStart {
        /// The session.
        session: SessionId,
        /// Why, as its log's final status says.
        why: String,
    },
    /// `from` is past the end of the session's log.
    #[error("session {session} has {next} records; cannot start at {from}")]
    Ahead {
        /// The session.
        session: SessionId,
        /// Requested start.
        from: EventSeq,
        /// The log's length.
        next: EventSeq,
    },
    /// The node is draining for an update and starts no new sessions.
    #[error("the node is draining for an update and starts no new sessions")]
    Draining,
    /// The session runtime refused or failed.
    #[error(transparent)]
    Session(#[from] SessionError),
    /// Physical startup admission refused before an agent could start.
    #[error(transparent)]
    StartRoot(#[from] crate::workspace::WorkspaceError),
    /// The event log failed.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// The session's records could not all be persisted.
    #[error(transparent)]
    Unpersisted(#[from] Unpersisted),
    /// The connection cannot take the subscription.
    #[error(transparent)]
    Outbound(#[from] OutboundError),
    /// A task the call depended on failed.
    #[error("a node task failed")]
    Task(#[source] tokio::task::JoinError),
    /// `device.invite` with a lifetime of zero.
    #[error("an invitation must be valid for at least one second")]
    InvalidTtl,
    /// `device.invite` naming an empty list of scopes.
    #[error("an invitation must grant at least one scope")]
    NoScopes,
    /// No effective advertised direct or relay route, including callbacks.
    #[error(
        "this node has no pairing route; configure --public with a reachable address or --relay, then create a new invitation"
    )]
    NoPairingRoutes,
    /// Too many invitations are waiting (`devices::invitations`).
    #[error("too many pairing invitations are waiting; let some be used or expire")]
    TooManyInvitations,
    /// The operating system's random source failed.
    #[error("no randomness available")]
    Random(#[source] getrandom::Error),
    /// The pairing link could not be encoded.
    #[error("could not encode the pairing link")]
    Link(#[source] serde_json::Error),
    /// `spec.profile` names no profile on this node.
    #[error("this node has no profile `{0}` (see `profile.list`)")]
    UnknownProfile(ProfileName),
    /// A call names an agent this node has not added (protocol 11, ADR
    /// 0032).
    #[error("this node has no agent `{0}` (see `agent.list`)")]
    UnknownAgent(AgentName),
    /// An install of this agent is running (`agent.install`, ADR 0032):
    /// it replaces the program a session would start.
    #[error("an install of {0} is running on this node; start it once the install ends")]
    Installing(HarnessKind),
    /// An added agent names a secret this node has not set (ADR 0032).
    #[error(
        "agent `{agent}` needs secret {secret}, which is not set on this node (vessel fleet secret set {secret})"
    )]
    AgentSecret {
        /// The agent.
        agent: AgentName,
        /// The secret its environment names.
        secret: SecretName,
    },
    /// `spec.profile` names a profile of another harness.
    #[error("profile `{profile}` is for {profile_harness}, not {harness}")]
    ProfileHarness {
        /// The profile.
        profile: ProfileName,
        /// The harness it extends.
        profile_harness: HarnessRef,
        /// The session's harness.
        harness: HarnessRef,
    },
    /// A session runs under a profile `profiles.toml` no longer defines, so
    /// its files cannot be found to move.
    #[error("the session's profile `{0}` is no longer defined")]
    ProfileGone(ProfileName),
    /// A profile's harness home is a relative path, which the harness
    /// resolves per session.
    #[error("profile `{0}`'s harness home cannot be known (a relative path)")]
    HomeUnknown(ProfileName),
    /// An agent's session is gone while it asks for a new one.
    #[error("the calling agent's session {0} has ended")]
    AgentGone(SessionId),
    /// Agents may start agents only so deep (ADR 0013).
    #[error("agents may start sessions at most {limit} levels deep")]
    AgentDepth {
        /// The limit.
        limit: u8,
    },
    /// An agent owns as many live sessions as it may (ADR 0013).
    #[error("an agent may own at most {limit} live sessions; end one first")]
    AgentLimit {
        /// The limit.
        limit: usize,
    },
    /// An agent asked to operate on a session it did not start (ADR 0013).
    #[error("an agent may only operate on sessions it started, or that those started")]
    NotOwned(SessionId),
    /// The `vessel` server's definition cannot be built (a name the
    /// protocol refuses: a bug).
    #[error("cannot define the `vessel` MCP server")]
    AgentTools(#[source] vessel_proto::InvalidName),
    /// The node's local endpoint is not Unicode, so `vessel mcp` cannot be
    /// told where it is.
    #[error("the node's local socket path is not Unicode")]
    AgentToolsPath,
    /// A profile move of this session is running already.
    #[error("session {0} is already moving to another profile")]
    MoveRunning(SessionId),
    /// The target profile's account is not logged in (its harness's check
    /// failed, or did not answer in time).
    #[error("profile `{0}` is not logged in; log it in first")]
    NotLoggedIn(ProfileName),
    /// `profile.reload` found a file it cannot use; the profiles in effect
    /// stay.
    #[error("the profiles in effect stay")]
    Profiles(#[source] ProfilesFileError),
    /// `agent.add` or `agent.remove` was refused or failed (protocol 11,
    /// ADR 0032); it says why.
    #[error(transparent)]
    Added(crate::added::AddedError),
    /// `profile.reload` found a `models.toml` it cannot use; the routes in
    /// effect stay.
    #[error("the model routes in effect stay")]
    Models(#[source] crate::models::ModelsError),
    /// A transfer could not be opened, read, pulled or placed (protocol
    /// 6).
    #[error(transparent)]
    Transfer(#[from] crate::transfers::TransferError),
    /// An upload or a read of an attachment was refused (protocol 10).
    #[error(transparent)]
    Upload(#[from] crate::attachments::UploadError),
    /// A profile names a model route this node's `models.toml` does not
    /// have (ADR 0015).
    #[error("{}model route `{route}` is not in this node's models.toml (see `model.list`)", profile.as_ref().map_or_else(String::new, |profile| format!("profile `{profile}`: ")))]
    UnknownRoute {
        /// The profile, if one names it.
        profile: Option<ProfileName>,
        /// The route.
        route: ModelRouteName,
    },
    /// A move would change the session's model route (ADR 0015).
    #[error("a session keeps its model route: it is on {}, the profile on {}", route_text(from.as_ref()), route_text(to.as_ref()))]
    MoveRoute {
        /// The session's route.
        from: Option<ModelRouteName>,
        /// The target profile's.
        to: Option<ModelRouteName>,
    },
    /// A routed session has no model token (a bug: one is minted at
    /// launch).
    #[error("session {0} is routed but has no model token")]
    Unrouted(SessionId),
    /// The model proxy is not listening (a bug: it listens before sessions
    /// start).
    #[error("the node's model proxy is not listening")]
    ModelProxyDown,
    /// A fork or a rewind names no record of the session (protocol 14).
    #[error("session {session} has no record {}", seq.get())]
    NoRecord {
        /// The session.
        session: SessionId,
        /// The record asked for.
        seq: EventSeq,
    },
    /// A fork or a rewind cannot start there (protocol 14, ADR 0052).
    #[error("session {session} cannot be forked there: {why}")]
    NotAPoint {
        /// The session.
        session: SessionId,
        /// Why, for a person.
        why: &'static str,
    },
    /// A fork of a whole conversation while a turn writes it (protocol 14).
    #[error("session {0} is running a turn; fork it once the turn ends, or from an earlier one")]
    ForkRunning(SessionId),
}

/// A route as a sentence names it.
fn route_text(route: Option<&ModelRouteName>) -> String {
    route.map_or_else(
        || "its harness's own provider".to_owned(),
        |route| format!("route `{route}`"),
    )
}

impl ServiceError {
    /// The error as the client sees it: a code to branch on and the full
    /// chain of causes as text (none of which contains secrets).
    pub(crate) fn to_wire(&self) -> WireError {
        let code = match self {
            Self::NotFound(_) | Self::UnknownAgent(_) | Self::NoRecord { .. } => {
                ErrorCode::NotFound
            }
            // `Profiles(Parse)`: the call is fine; the node's file is not
            // (and stays unused).
            Self::Ended(_)
            | Self::Profiles(
                ProfilesFileError::Parse(_)
                | ProfilesFileError::ParseAgents(_)
                | ProfilesFileError::ReadAgents(vessel_store::AgentsFileError::Unsafe { .. }),
            )
            | Self::ProfileGone(_)
            | Self::HomeUnknown(_)
            | Self::NotLoggedIn(_)
            | Self::AgentGone(_)
            | Self::MoveRunning(_)
            | Self::ForkRunning(_)
            | Self::Live(_)
            | Self::LogWriting(_)
            | Self::AlreadyLive(_)
            | Self::StartingAgain(_)
            | Self::FolderGone { .. }
            | Self::WorktreeBusy(_)
            | Self::AgentSecret { .. }
            | Self::Installing(_)
            | Self::Models(_)
            | Self::NoPairingRoutes => ErrorCode::Conflict,
            Self::AgentDepth { .. }
            | Self::AgentLimit { .. }
            | Self::NotOwned(_)
            | Self::AgentMarksRead => ErrorCode::Forbidden,
            Self::Draining | Self::TooManyInvitations => ErrorCode::Unavailable,
            Self::Ahead { .. }
            | Self::ReadAhead { .. }
            | Self::InvalidTtl
            | Self::SideCreationLabels
            | Self::SideCreationListing
            | Self::NoScopes
            | Self::UnknownProfile(_)
            | Self::ProfileRemoved { .. }
            | Self::UnknownRoute { .. }
            | Self::ProfileHarness { .. } => ErrorCode::InvalidRequest,
            Self::DidNotStart { .. } => ErrorCode::SpawnFailed,
            Self::MoveRoute { .. }
            | Self::NotAPoint { .. }
            | Self::LabelsUnassigned
            | Self::ListingUnassigned => ErrorCode::Unsupported,
            Self::MetadataProtocol(error) => error.code().unwrap_or(ErrorCode::Internal),
            Self::SideQuestion(error) => error.code(),
            Self::Session(error) => error.code(),
            Self::StartRoot(error) => return error.wire(),
            Self::Transfer(error) => error.code(),
            Self::Upload(error) => error.code(),
            Self::Store(error)
            | Self::Profiles(
                ProfilesFileError::Read(error)
                | ProfilesFileError::ReadAgents(vessel_store::AgentsFileError::Read(error)),
            ) => error.code(),
            Self::Added(error) => error.code(),
            Self::Outbound(error) => error.code(),
            Self::Unpersisted(_)
            | Self::Task(_)
            | Self::Random(_)
            | Self::Link(_)
            | Self::AgentTools(_)
            | Self::AgentToolsPath
            | Self::Unrouted(_)
            | Self::ModelProxyDown => ErrorCode::Internal,
        };
        let mut message = self.to_string();
        let mut source = std::error::Error::source(self);
        while let Some(cause) = source {
            message.push_str(": ");
            message.push_str(&cause.to_string());
            source = cause.source();
        }
        let mut error = WireError::new(code, message);
        if matches!(self, Self::NoPairingRoutes) {
            error.detail = Some(ErrorDetail::DeviceInvite {
                reason: DeviceInviteError::NoRoutes,
            });
        }
        error
    }
}
