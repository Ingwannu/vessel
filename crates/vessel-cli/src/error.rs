//! What a command can fail with; printed once, with its causes, by `main`.

use std::path::PathBuf;

use vessel_client::book::BookError;
use vessel_client::{CallError, ConnectError, InvalidAddress, NodeUrl, StreamError};
use vessel_node::NodeError;
use vessel_proto::NodeId;
use vessel_remote::{
    BuildError, InstallError, PlatformError, ProvisionError, ReleaseError, SshConnectError,
    SshError,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum CliError {
    /// Canonical pending-request reads suppress arbitrary peer error details.
    #[error(transparent)]
    Permit(#[from] crate::permit::ListError),
    /// File observations refuse unsafe reads before a shutdown request.
    #[error(transparent)]
    ServiceFile(#[from] crate::init::ServiceFileError),
    #[error(
        "this command does not support explicit output selection; omit --json/-o/-q/--no-headers"
    )]
    UnsupportedPresentation,
    #[error("cannot encode command output")]
    ParityEncode(#[source] serde_json::Error),
    /// Shareable diagnostic output must not print the private source chain.
    #[error(transparent)]
    Diagnostics(#[from] crate::diagnostics::CommandError),
    /// Service input and reads must never expose unit environment contents.
    #[error(transparent)]
    Service(#[from] crate::init::ServiceError),
    /// Bounded comparison input or command ownership.
    #[error(transparent)]
    ComparisonInput(#[from] vessel_client::comparison::InputError),
    #[error(transparent)]
    ComparisonJob(#[from] vessel_client::comparison::JobError),
    #[error(transparent)]
    Comparison(#[from] crate::conversation::comparison::CompareError),
    /// Formatting a usage report failed.
    #[error("could not format the usage report")]
    UsageFormat(#[from] std::fmt::Error),
    /// Directory authority must come from identity equality, not loopback alone.
    #[error("cannot inspect the local node identity")]
    LocalIdentity(#[source] vessel_node::StoreError),
    /// A disappeared waiter must not be reported as completed shutdown.
    #[error("the node lock waiter ended without a completion")]
    LockWaiter(#[source] tokio::sync::oneshot::error::RecvError),
    /// A lost preflight result cannot authorize a shutdown.
    #[error("node preflight reader exited before reporting its result")]
    PreflightReader(#[source] tokio::sync::oneshot::error::RecvError),
    /// A lost identity result cannot bind local observations.
    #[error("node identity reader exited before reporting its result")]
    IdentityReader(#[source] tokio::sync::oneshot::error::RecvError),
    /// A lost service observation cannot be treated as a missing file.
    #[error("node service observer exited before reporting its result")]
    ServiceReader(#[source] tokio::sync::oneshot::error::RecvError),
    /// Local lifecycle checks preserve the store's original I/O cause.
    #[error("cannot inspect the node lock")]
    NodeLock(#[source] vessel_node::StoreError),
    /// A stale owner record is never evidence of a running node.
    #[error("the node is not running")]
    NodeNotRunning,
    /// A held lock without a valid owner must not be mistaken for a stale record.
    #[error("the node lock is held but its owner record is unavailable")]
    NodeOwnerUnavailable,
    /// A graceful stop exceeded its complete command deadline.
    #[error("the node did not finish the graceful stop within {limit} seconds (--timeout)")]
    StopTimedOut {
        /// The same parsed budget passed to the command's deadline.
        limit: crate::args::StopTimeout,
        /// Keep expiry's typed cause rather than replacing it with text.
        #[source]
        source: tokio::time::error::Elapsed,
    },
    /// Older peers must refuse before a lifecycle method is sent.
    #[error("the running node cannot be stopped over the wire; stop it on its machine")]
    StopUnsupported(#[source] CallError),
    /// Existing sessions require an explicit interruption or drain choice.
    #[error("not stopped: live sessions remain; see the list above")]
    StopHasLiveSessions,
    /// The interactive interruption confirmation was declined.
    #[error("not confirmed; nothing was stopped")]
    StopNotConfirmed,
    /// Explicit operator LSP configuration failed.
    #[error(transparent)]
    LspConfiguration(#[from] vessel_node::LspConfigError),
    /// No `--token`, and the local token file could not be read.
    #[error(
        "no token: pass --token (or VESSEL_TOKEN), or run on the node's machine ({} could not be read)",
        path.display()
    )]
    Token {
        path: PathBuf,
        #[source]
        source: vessel_node::StoreError,
    },
    /// A node on another machine and no `--token`: the local token file is
    /// only ever sent to this machine's node (see `connect`).
    #[error(
        "{0} is not on this machine: pass its token with --token or VESSEL_TOKEN \
         (the local token file is only sent to a node on this machine)"
    )]
    TokenRequired(NodeUrl),
    /// A relative `--cwd` for a node on another machine, where it would mean
    /// nothing.
    #[error(
        "--cwd {} is relative, but {node} is not on this machine: give an absolute path on that machine",
        cwd.display()
    )]
    RelativeCwd { cwd: PathBuf, node: String },
    /// A book entry's token file could not be read or holds no token.
    #[error("cannot use the token file {}", path.display())]
    TokenFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The node book or a project file.
    #[error(transparent)]
    Book(#[from] BookError),
    /// `vessel ui` was asked for a node the book does not have (the UI
    /// reaches book nodes and this machine's only, never an address a
    /// page made up).
    #[error("no node named `{0}` in the book")]
    NotInBook(String),
    /// Text that is not a node name.
    #[error(transparent)]
    NodeName(#[from] vessel_client::book::InvalidNodeName),
    /// Text that names no node.
    #[error("`{text}` is not a node address")]
    Address {
        text: String,
        #[source]
        source: InvalidAddress,
    },
    /// A different node answered than the one the book pinned.
    #[error(
        "a different node answered (id {found}, expected {expected}); if the machine was \
         reinstalled on purpose, remove the node's `id` from your book ({book})"
    )]
    Changed {
        expected: NodeId,
        found: NodeId,
        book: String,
    },
    /// Connecting over SSH failed.
    #[error(transparent)]
    Remote(#[from] SshConnectError),
    /// Installing vesseld failed.
    #[error(transparent)]
    Provision(#[from] ProvisionError),
    /// `vessel node update` found live sessions and no `--when-idle` or
    /// `--now` (they are listed on stdout).
    #[error("not updated: {0} live session(s); see the list above")]
    LiveSessions(usize),
    /// The node speaks protocol 2: it cannot be stopped remotely.
    #[error(
        "the running node (vesseld {0}) is too old to be stopped remotely; stop it on its \
         machine, and the next connection starts the new version"
    )]
    TooOldToStop(String),
    /// A paired node's book entry without its credential file.
    #[error("{node} is a paired node without a `credential` file in the book; pair it again")]
    NoCredential { node: String },
    /// A paired node's credential file is unreadable or damaged.
    #[error(transparent)]
    Credential(vessel_client::CredentialError),
    /// The credential file belongs to another node than the book entry.
    #[error(transparent)]
    CredentialMismatch(#[from] vessel_client::CredentialMismatch),
    /// No route reached the paired node, or it refused this device.
    #[error(transparent)]
    Reach(vessel_client::secure::ReachError<vessel_client::secure::DialError>),
    /// `wss://` routes, and no TLS roots to verify them.
    #[error("cannot verify wss:// routes")]
    Tls(#[source] vessel_client::secure::TlsError),
    /// `--pair` and nothing on standard input.
    #[error("no pairing link on standard input")]
    NoPairingLink,
    /// The pasted text is not a pairing link.
    #[error(transparent)]
    PairingLink(vessel_proto::LinkError),
    /// The decoded invitation could not be encoded for its typed RPC.
    #[error("could not encode a pairing invitation")]
    PairingEncode(#[source] serde_json::Error),
    /// The person did not confirm the node.
    #[error("not paired")]
    NotPaired,
    /// `vessel node devices revoke` with an id that matches no device.
    #[error("no device matches `{0}`; `vessel node devices` lists them")]
    NoSuchDevice(String),
    /// ... or several.
    #[error("`{prefix}` matches {count} devices; give more of the id")]
    AmbiguousDevice { prefix: String, count: usize },
    /// A command that needs an SSH node got another kind.
    #[error("{node} is not an SSH node; {what} works only over SSH")]
    NotSsh { node: String, what: &'static str },
    /// Arguments that do not fit together.
    #[error("{0}")]
    Usage(&'static str),
    /// An install on the node did not succeed (`vessel agents install`).
    #[error("{0}")]
    AgentInstall(String),
    /// An agent's text breaks its rule (`vessel agents add`).
    #[error(transparent)]
    AgentText(#[from] vessel_proto::agents::InvalidAgentText),
    /// A search's words break its rule (`vessel search`).
    #[error(transparent)]
    Search(#[from] vessel_proto::search::InvalidSearch),
    #[error(transparent)]
    Ssh(#[from] SshError),
    #[error(transparent)]
    Install(#[from] InstallError),
    #[error(transparent)]
    Platform(#[from] PlatformError),
    #[error(transparent)]
    Release(#[from] ReleaseError),
    #[error(transparent)]
    Build(#[from] BuildError),
    /// `vessel init`: the data directory or `profiles.toml` could not be
    /// prepared, read or written.
    #[error("cannot prepare the data directory")]
    Prepare(#[source] vessel_node::StoreError),
    /// `vessel init`: a found profile could not be added to `profiles.toml`.
    #[error("cannot add the profiles")]
    AddProfile(#[source] vessel_harness::AddProfileError),
    /// `vessel init`: the profiles to add would not load; nothing was written.
    #[error("the profiles to add would not load ({0}); nothing was written")]
    ProfilesAfterAdd(String),
    /// `vessel mcp` stopped (ADR 0013).
    #[error(transparent)]
    Mcp(#[from] vessel_mcp::McpError),

    /// The local token file holds something a client cannot send.
    #[error("the token in {} cannot be sent as a bearer token", .0.display())]
    UnusableToken(PathBuf),
    /// No home directory to find the default data directory in.
    #[error("no home directory; pass --data-dir")]
    NoHome,
    #[error(transparent)]
    Connect(#[from] ConnectError),
    #[error(transparent)]
    Call(#[from] CallError),
    #[error(transparent)]
    Stream(#[from] StreamError),
    #[error("no session matches `{0}`")]
    NoSuchSession(String),
    /// A project or worktree selector that names none, or more than one
    /// (protocol 10).
    #[error("{0}")]
    Workspace(String),
    /// A schedule or saved prompt selector that names none, or more than
    /// one, or an argument that does not read (protocol 11).
    #[error("{0}")]
    Automation(String),
    #[error("`{prefix}` matches {matches} sessions; type more of the id")]
    AmbiguousSession { prefix: String, matches: usize },
    /// An id prefix naming none of a session's queued prompts or waiting
    /// requests.
    #[error("no {what} matches `{prefix}`")]
    NoMatch { what: &'static str, prefix: String },
    /// ... or several.
    #[error("`{prefix}` matches {matches} of the {what}s; type more of the id")]
    AmbiguousMatch {
        what: &'static str,
        prefix: String,
        matches: usize,
    },
    /// A file `vessel send --attach` could not read.
    #[error("cannot attach {}", path.display())]
    Attach {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// A file whose name (or type) cannot be an attachment's.
    #[error("cannot attach {}", path.display())]
    NotAttachable {
        path: PathBuf,
        #[source]
        source: vessel_proto::conversation::InvalidText,
    },
    /// A file that changed size while it was sent.
    #[error("{} changed while it was sent; nothing was attached", .0.display())]
    AttachmentChanged(PathBuf),
    /// Values that do not answer the question.
    #[error(transparent)]
    Answer(#[from] vessel_client::compose::FillError),
    #[error(transparent)]
    Node(#[from] NodeError),
    /// `vessel fleet init` did not make this node a home.
    #[error(transparent)]
    FleetInit(#[from] vessel_node::fleet::InitError),
    /// `vessel fleet join` did not join.
    #[error(transparent)]
    FleetJoin(#[from] vessel_node::fleet::JoinError),
    /// `vessel fleet restore` did not restore.
    #[error(transparent)]
    FleetRestore(#[from] vessel_node::fleet::RestoreError),
    /// `secrets.toml` could not be used (the error names no value).
    #[error(transparent)]
    FleetSecrets(#[from] vessel_node::fleet::SecretsError),
    /// A fleet file could not be read or written.
    #[error(transparent)]
    FleetFile(#[from] vessel_node::fleet::FileError),
    #[error("terminal I/O failed")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    UiToken(#[from] vessel_web::MintError),
    #[error(transparent)]
    Ui(#[from] vessel_web::ServeError),
    #[error("an input task failed")]
    Task(#[from] tokio::task::JoinError),
}
