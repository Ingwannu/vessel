//! Client side: one connection to a node, shared by any number of callers.
//!
//! [`Connection::connect`] performs the handshake and spawns a driver task
//! that owns the transport (see `driver`). Callers talk to the driver
//! through a bounded command queue; answers come back on one-shot channels,
//! and each subscription gets its own bounded record channel.
//!
//! [Decision Log]
//! - Purpose: typed calls (`call::<R>()` returns `R::Response`) and
//!   subscriptions that never lose their first events and never block the
//!   connection.
//! - Plan deviation: `docs/plans/backend-m1.md` sketched `connect` returning
//!   one `Events` stream for all subscriptions. That shape has a race: the
//!   driver reads `Subscribed{S}` and then `Event{S}` back to back, and a
//!   router fed by a separate stream cannot know about `S` before the caller
//!   who received the reply registers it, so the first events are dropped.
//!   Here the driver itself creates a subscription's channel in the step that
//!   reads its reply, and [`Connection::subscribe`] returns that channel as a
//!   [`Subscription`].
//! - A slow subscriber never blocks the connection: when its channel is full
//!   the driver ends that subscription locally with `Lagged{resume_from}`
//!   (the same promise the node makes: nothing skipped, resume from there)
//!   and unsubscribes on the node. Blocking instead would stall every call
//!   on the connection behind it, including the `session.read` the
//!   subscriber needs to catch up: a deadlock.
//! - `session.subscribe` is not callable through [`Connection::call`]: its
//!   reply alone would leave the subscription's events with nowhere to go.
//!   The [`Unary`] marker makes that a compile error.

mod driver;
mod subscription;

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::{SinkExt as _, StreamExt as _};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use vessel_proto::{
    Call, ClientFrame, ClientInfo, CloseTransfer, CopySecret, CreateSession, DeleteSecret, Deliver,
    ExportMemory, ExportSession, ForgetNode, Forward, GetCallback, GetManifest, GetNode,
    GetSession, GrantHome, Hello, ImportMemory, ImportSession, InviteDevice, KillSession,
    ListDevices, ListHarnesses, ListMembers, ListModels, ListNodes, ListPeers, ListProfiles,
    ListSecrets, ListSessions, Method, MoveProfile, Outcome, ProtocolRange, ReadBlob, ReadEvents,
    ReadTransfer, RelayCall, ReloadProfiles, RenameSession, Reply, ReportApplied, Request,
    RequestId, RevokeDevice, SendInput, ServerFrame, SetSecret, StopNode, Subscribe, Unsubscribe,
    WatchFleet, Welcome, WireError,
};

pub use subscription::{Subscription, SubscriptionItem, TerminalSubscription};

use self::driver::{Command, Subject, Waiter};
use crate::error::{CallError, ConnectError, ProtocolViolation};
use crate::transport::FrameTransport;

/// A request answered by exactly one reply, callable through
/// [`Connection::call`]. Every method except `session.subscribe` (use
/// [`Connection::subscribe`]).
pub trait Unary: Request {}

impl Unary for GetNode {}
impl Unary for ListHarnesses {}
impl Unary for ListSessions {}
impl Unary for GetSession {}
impl Unary for CreateSession {}
impl Unary for SendInput {}
impl Unary for KillSession {}
impl Unary for ReadEvents {}
impl Unary for Unsubscribe {}
impl Unary for StopNode {}
impl Unary for InviteDevice {}
impl Unary for ListDevices {}
impl Unary for RevokeDevice {}
impl Unary for ListProfiles {}
impl Unary for ReloadProfiles {}
impl Unary for MoveProfile {}
impl Unary for GetManifest {}
impl Unary for ReadBlob {}
// A long poll: one answer, however long the home waits before giving it.
impl Unary for WatchFleet {}
impl Unary for ReportApplied {}
impl Unary for ListMembers {}
// Protocol 6. The envelope's reply is its inner call's one reply (only
// one-reply calls can be forwarded), and `session.read` with `wait` is a
// long poll like `fleet.watch`.
impl Unary for Forward {}
impl Unary for RelayCall {}
impl Unary for Deliver {}
impl Unary for GrantHome {}
impl Unary for ForgetNode {}
impl Unary for GetCallback {}
impl Unary for ListPeers {}
impl Unary for ListNodes {}
impl Unary for ExportSession {}
impl Unary for ImportSession {}
impl Unary for ExportMemory {}
impl Unary for ImportMemory {}
impl Unary for ReadTransfer {}
impl Unary for CloseTransfer {}
// Protocol 7.
impl Unary for ListModels {}
impl Unary for SetSecret {}
impl Unary for ListSecrets {}
impl Unary for DeleteSecret {}
impl Unary for CopySecret {}
impl Unary for vessel_proto::lsp::WorkspaceSymbols {}
impl Unary for vessel_proto::lsp::DocumentSymbols {}
impl Unary for vessel_proto::lsp::Definition {}
impl Unary for vessel_proto::lsp::References {}
impl Unary for vessel_proto::lsp::GetHover {}
impl Unary for vessel_proto::lsp::Implementations {}
impl Unary for vessel_proto::lsp::TypeDefinition {}
impl Unary for vessel_proto::lsp::IncomingCalls {}
impl Unary for vessel_proto::lsp::OutgoingCalls {}
impl Unary for vessel_proto::lsp::Diagnostics {}
// Protocol 9. `harness.options` may start an agent to ask it, and still
// answers once.
impl Unary for vessel_proto::GetHarnessOptions {}
impl Unary for vessel_proto::GetSessionOptions {}
impl Unary for vessel_proto::SetSessionOption {}
impl Unary for RenameSession {}
// Protocol 10 (Track A, the conversation).
impl Unary for vessel_proto::conversation::PromptSession {}
impl Unary for vessel_proto::conversation::ChangeQueue {}
impl Unary for vessel_proto::conversation::GetConversation {}
impl Unary for vessel_proto::conversation::ResumeSession {}
impl Unary for vessel_proto::conversation::MarkSession {}
impl Unary for vessel_proto::conversation::DeleteSession {}
impl Unary for vessel_proto::conversation::BeginUpload {}
impl Unary for vessel_proto::conversation::WriteUpload {}
impl Unary for vessel_proto::conversation::FinishUpload {}
impl Unary for vessel_proto::conversation::ReadAttachment {}
// Protocol 10 (Track B): every workspace call answers once; long polls
// (`project.get`, `worktree.output`) are bounded waits, not streams.
impl Unary for vessel_proto::workspace::ListProjects {}
impl Unary for vessel_proto::workspace::GetProject {}
impl Unary for vessel_proto::workspace::AddProject {}
impl Unary for vessel_proto::workspace::CloneProject {}
impl Unary for vessel_proto::workspace::InitProject {}
impl Unary for vessel_proto::workspace::RemoveProject {}
impl Unary for vessel_proto::workspace::GetProjectCommands {}
impl Unary for vessel_proto::workspace::ApproveCommands {}
impl Unary for vessel_proto::workspace::ListWorktrees {}
impl Unary for vessel_proto::workspace::CreateWorktree {}
impl Unary for vessel_proto::workspace::ReadSetupOutput {}
impl Unary for vessel_proto::workspace::RunSetup {}
impl Unary for vessel_proto::workspace::RemoveWorktree {}
impl Unary for vessel_proto::workspace::ListSnapshots {}
impl Unary for vessel_proto::workspace::DropSnapshot {}
impl Unary for vessel_proto::workspace::GetStatus {}
impl Unary for vessel_proto::workspace::GetChanges {}
impl Unary for vessel_proto::workspace::GetFileDiff {}
impl Unary for vessel_proto::workspace::ShowFile {}
impl Unary for vessel_proto::workspace::ListBranches {}
impl Unary for vessel_proto::workspace::ReadLog {}
impl Unary for vessel_proto::workspace::ListStashes {}
impl Unary for vessel_proto::workspace::Stage {}
impl Unary for vessel_proto::workspace::Unstage {}
impl Unary for vessel_proto::workspace::Discard {}
impl Unary for vessel_proto::workspace::Commit {}
impl Unary for vessel_proto::workspace::Fetch {}
impl Unary for vessel_proto::workspace::Push {}
impl Unary for vessel_proto::workspace::Pull {}
impl Unary for vessel_proto::workspace::Switch {}
impl Unary for vessel_proto::workspace::RenameBranch {}
impl Unary for vessel_proto::workspace::StashChanges {}
impl Unary for vessel_proto::workspace::ListDir {}
impl Unary for vessel_proto::workspace::ReadFile {}
impl Unary for vessel_proto::workspace::WriteFile {}
impl Unary for vessel_proto::workspace::CreateDir {}
impl Unary for vessel_proto::workspace::RenamePath {}
impl Unary for vessel_proto::workspace::DeletePath {}
impl Unary for vessel_proto::workspace::FindFiles {}
impl Unary for vessel_proto::workspace::SearchFiles {}
impl Unary for vessel_proto::forge::GetForgeStatus {}
impl Unary for vessel_proto::forge::ListPullRequests {}
impl Unary for vessel_proto::forge::GetPullRequest {}
impl Unary for vessel_proto::forge::OpenPullRequest {}
impl Unary for vessel_proto::forge::MergePullRequest {}
impl Unary for vessel_proto::forge::SetFeedback {}
// Protocol 11 (Track C): every call but `terminal.subscribe` answers once;
// `terminal.get`, `terminal.read` and `port.list` wait at most 30 s.
impl Unary for vessel_proto::terminals::CreateTerminal {}
impl Unary for vessel_proto::terminals::ListTerminals {}
impl Unary for vessel_proto::terminals::GetTerminal {}
impl Unary for vessel_proto::terminals::ReadTerminal {}
impl Unary for vessel_proto::terminals::SendTerminal {}
impl Unary for vessel_proto::terminals::RenameTerminal {}
impl Unary for vessel_proto::terminals::KillTerminal {}
impl Unary for vessel_proto::terminals::CloseTerminal {}
impl Unary for vessel_proto::scripts::ListScripts {}
impl Unary for vessel_proto::scripts::RunScript {}
impl Unary for vessel_proto::scripts::RestartScript {}
impl Unary for vessel_proto::ports::ListPorts {}
impl Unary for vessel_proto::agents::ListAgents {}
impl Unary for vessel_proto::agents::AddAgent {}
impl Unary for vessel_proto::agents::RemoveAgent {}
impl Unary for vessel_proto::agents::GetAgentStatus {}
impl Unary for vessel_proto::agents::GetCatalogue {}
impl Unary for vessel_proto::agents::InstallAgent {}
impl Unary for vessel_proto::agents::ReadInstall {}
// Protocol 11 (Track D): schedules and saved prompts answer once.
impl Unary for vessel_proto::automation::PreviewSchedule {}
impl Unary for vessel_proto::automation::ListSchedules {}
impl Unary for vessel_proto::automation::GetSchedule {}
impl Unary for vessel_proto::automation::CreateSchedule {}
impl Unary for vessel_proto::automation::UpdateSchedule {}
impl Unary for vessel_proto::automation::PauseSchedule {}
impl Unary for vessel_proto::automation::ResumeSchedule {}
impl Unary for vessel_proto::automation::RunSchedule {}
impl Unary for vessel_proto::automation::DeleteSchedule {}
impl Unary for vessel_proto::automation::ListRuns {}
impl Unary for vessel_proto::automation::ListPrompts {}
impl Unary for vessel_proto::automation::CreatePrompt {}
impl Unary for vessel_proto::automation::UpdatePrompt {}
impl Unary for vessel_proto::automation::DeletePrompt {}
impl Unary for vessel_proto::search::SearchMessages {}
impl Unary for vessel_proto::parking::ParkSession {}
impl Unary for vessel_proto::parking::WakeSession {}
impl Unary for vessel_proto::settings::GetNodeSettings {}
impl Unary for vessel_proto::settings::SetNodeSettings {}
impl Unary for vessel_proto::forking::ForkSession {}
impl Unary for vessel_proto::forking::RewindSession {}
impl Unary for vessel_proto::history::GetAgentHistory {}

/// A call answered by exactly one reply, built already: every method but
/// `session.subscribe` (whose answer opens a stream, [`Connection::subscribe`]).
/// For callers that receive calls rather than build them, such as `vessel
/// ui`'s bridge relaying a browser's (ADR 0017): checked once, here, then
/// sent as it is with [`Connection::call_any`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnaryCall(Call);

/// A call that is not one-reply: it was refused before sending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("`{}` is not a one-reply call", .0.as_str())]
pub struct NotUnary(pub Method);

impl TryFrom<Call> for UnaryCall {
    type Error = NotUnary;

    fn try_from(call: Call) -> Result<Self, NotUnary> {
        let method = call.method();
        if is_unary(method) {
            Ok(Self(call))
        } else {
            Err(NotUnary(method))
        }
    }
}

impl UnaryCall {
    /// The method.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.0.method()
    }
}

/// Whether `method` is answered by exactly one reply. Exhaustive, so a new
/// method is decided here, as it is by the [`Unary`] impls above.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per method, so a new method does not compile until someone decides; the \
              list is the point"
)]
const fn is_unary(method: Method) -> bool {
    match method {
        Method::Subscribe | Method::SubscribeTerminal => false,
        Method::NodeInfo
        | Method::ListHarnesses
        | Method::ListSessions
        | Method::GetSession
        | Method::CreateSession
        | Method::SendInput
        | Method::KillSession
        | Method::ReadEvents
        | Method::Unsubscribe
        | Method::StopNode
        | Method::InviteDevice
        | Method::ListDevices
        | Method::RevokeDevice
        | Method::ListProfiles
        | Method::ReloadProfiles
        | Method::MoveProfile
        | Method::GetManifest
        | Method::ReadBlob
        | Method::WatchFleet
        | Method::ReportApplied
        | Method::ListMembers
        | Method::Forward
        | Method::Relay
        | Method::Deliver
        | Method::GrantHome
        | Method::GetCallback
        | Method::ListPeers
        | Method::ListNodes
        | Method::ForgetNode
        | Method::ExportSession
        | Method::ImportSession
        | Method::ExportMemory
        | Method::ImportMemory
        | Method::ReadTransfer
        | Method::CloseTransfer
        | Method::SetSecret
        | Method::ListSecrets
        | Method::DeleteSecret
        | Method::CopySecret
        | Method::LspWorkspaceSymbols
        | Method::LspDocumentSymbols
        | Method::LspDefinition
        | Method::LspReferences
        | Method::LspHover
        | Method::LspImplementations
        | Method::LspTypeDefinition
        | Method::LspIncomingCalls
        | Method::LspOutgoingCalls
        | Method::LspDiagnostics
        | Method::ListModels
        | Method::HarnessOptions
        | Method::SessionOptions
        | Method::SetSessionOption
        | Method::RenameSession
        | Method::PromptSession
        | Method::ChangeQueue
        | Method::GetConversation
        | Method::ResumeSession
        | Method::MarkSession
        | Method::DeleteSession
        | Method::BeginUpload
        | Method::WriteUpload
        | Method::FinishUpload
        | Method::ReadAttachment
        | Method::ListProjects
        | Method::GetProject
        | Method::AddProject
        | Method::CloneProject
        | Method::InitProject
        | Method::RemoveProject
        | Method::ProjectCommands
        | Method::ApproveCommands
        | Method::ListWorktrees
        | Method::CreateWorktree
        | Method::SetupOutput
        | Method::RunSetup
        | Method::RemoveWorktree
        | Method::ListSnapshots
        | Method::DropSnapshot
        | Method::GitStatus
        | Method::GitChanges
        | Method::GitDiff
        | Method::GitShow
        | Method::GitBranches
        | Method::GitLog
        | Method::GitStashes
        | Method::GitStage
        | Method::GitUnstage
        | Method::GitDiscard
        | Method::GitCommit
        | Method::GitFetch
        | Method::GitPush
        | Method::GitPull
        | Method::GitSwitch
        | Method::GitRenameBranch
        | Method::GitStash
        | Method::ListDir
        | Method::ReadFile
        | Method::WriteFile
        | Method::CreateDir
        | Method::RenamePath
        | Method::DeletePath
        | Method::FindFiles
        | Method::SearchFiles
        | Method::ForgeStatus
        | Method::ListPullRequests
        | Method::GetPullRequest
        | Method::OpenPullRequest
        | Method::MergePullRequest
        | Method::SetFeedback
        | Method::CreateTerminal
        | Method::ListTerminals
        | Method::GetTerminal
        | Method::ReadTerminal
        | Method::SendTerminal
        | Method::RenameTerminal
        | Method::KillTerminal
        | Method::CloseTerminal
        | Method::ListScripts
        | Method::RunScript
        | Method::RestartScript
        | Method::ListPorts
        | Method::ListAgents
        | Method::AddAgent
        | Method::RemoveAgent
        | Method::GetAgentStatus
        | Method::GetCatalogue
        | Method::InstallAgent
        | Method::ReadInstall
        | Method::PreviewSchedule
        | Method::ListSchedules
        | Method::GetSchedule
        | Method::CreateSchedule
        | Method::UpdateSchedule
        | Method::PauseSchedule
        | Method::ResumeSchedule
        | Method::RunSchedule
        | Method::DeleteSchedule
        | Method::ListRuns
        | Method::ListPrompts
        | Method::CreatePrompt
        | Method::UpdatePrompt
        | Method::DeletePrompt
        | Method::SearchMessages
        | Method::ParkSession
        | Method::WakeSession
        | Method::GetNodeSettings
        | Method::SetNodeSettings
        | Method::ForkSession
        | Method::RewindSession
        | Method::GetAgentHistory => true,
    }
}

/// `n` as a `NonZeroUsize` for constants.
const fn at_least_one(n: usize) -> NonZeroUsize {
    match NonZeroUsize::new(n) {
        Some(n) => n,
        None => NonZeroUsize::MIN,
    }
}

/// Limits for a client connection.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Commands (calls, cancels) queued for the driver; callers wait when
    /// it is full.
    pub command_capacity: NonZeroUsize,
    /// Records buffered per subscription. When a subscriber falls this far
    /// behind, its subscription ends with `Lagged` instead of blocking the
    /// connection.
    pub subscription_capacity: NonZeroUsize,
    /// How long to wait for the node's answer to `Hello`.
    pub handshake_timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            command_capacity: at_least_one(64),
            subscription_capacity: at_least_one(1024),
            handshake_timeout: Duration::from_secs(10),
        }
    }
}

/// A connection to a node. Cheap to clone; the connection closes when every
/// clone and every [`Subscription`] is dropped, or when the node leaves.
#[derive(Debug, Clone)]
pub struct Connection {
    shared: Arc<Shared>,
}

#[derive(Debug)]
struct Shared {
    commands: mpsc::Sender<Command>,
    ids: Arc<RequestIds>,
    welcome: Welcome,
    closed: CancellationToken,
}

/// Request ids for one connection, shared by callers and the driver.
#[derive(Debug, Default)]
pub(crate) struct RequestIds(AtomicU64);

impl RequestIds {
    /// The next unused id, or `None` once all 2^64 are used.
    pub(crate) fn next(&self) -> Option<RequestId> {
        self.0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .ok()
            .map(RequestId::new)
    }
}

/// The node's `Rejected` error, if that is what the handshake's answer is.
fn refusal<E>(
    answer: &Result<Option<Result<String, E>>, tokio::time::error::Elapsed>,
) -> Option<WireError> {
    let Ok(Some(Ok(text))) = answer else {
        return None;
    };
    match serde_json::from_str::<ServerFrame>(text) {
        Ok(ServerFrame::Rejected { error }) => Some(error),
        Ok(
            ServerFrame::Welcome(_)
            | ServerFrame::Response { .. }
            | ServerFrame::Event { .. }
            | ServerFrame::SubscriptionEnded { .. },
        )
        | Err(_) => None,
    }
}

impl Connection {
    /// Say hello over `transport` and start the connection's driver task.
    ///
    /// # Errors
    /// The transport failed, the node did not answer in time, refused the
    /// connection (for example no common protocol version), or broke the
    /// protocol.
    pub async fn connect<T: FrameTransport>(
        transport: T,
        client: ClientInfo,
        config: &ClientConfig,
    ) -> Result<Self, ConnectError<T::TransportError>> {
        let (mut sink, mut stream) = transport.split();
        let hello = ClientFrame::Hello(Hello {
            protocol: ProtocolRange::CURRENT,
            client,
        });
        let text = serde_json::to_string(&hello).map_err(ConnectError::Encode)?;
        let sent = sink.send(text).await;
        let answer = tokio::time::timeout(config.handshake_timeout, stream.next()).await;
        if let Err(error) = sent {
            // A node may refuse before reading `Hello` and close at once (a
            // wrong token on the local socket, ADR 0007), which breaks the
            // send. Its refusal can still be waiting to be read, and says
            // why; otherwise the send's own error does.
            return Err(
                refusal(&answer).map_or(ConnectError::Transport(error), ConnectError::Rejected)
            );
        }
        let answer = answer
            .map_err(|_elapsed| ConnectError::Timeout)?
            .ok_or(ConnectError::Closed)?
            .map_err(ConnectError::Transport)?;
        let welcome = match serde_json::from_str::<ServerFrame>(&answer) {
            Ok(ServerFrame::Welcome(welcome)) => welcome,
            Ok(ServerFrame::Rejected { error }) => return Err(ConnectError::Rejected(error)),
            Ok(
                ServerFrame::Response { .. }
                | ServerFrame::Event { .. }
                | ServerFrame::SubscriptionEnded { .. },
            ) => {
                return Err(ConnectError::Violation(ProtocolViolation::ExpectedWelcome));
            }
            Err(error) => return Err(ConnectError::Decode(error)),
        };

        let ids = Arc::new(RequestIds::default());
        let closed = CancellationToken::new();
        let (commands, commands_rx) = mpsc::channel(config.command_capacity.get());
        let driver = driver::Driver {
            commands: commands_rx,
            unsubscribe: commands.downgrade(),
            ids: Arc::clone(&ids),
            subscription_capacity: config.subscription_capacity,
            closed: closed.clone(),
        };
        drop(tokio::spawn(driver.run(sink, stream)));
        Ok(Self {
            shared: Arc::new(Shared {
                commands,
                ids,
                welcome,
                closed,
            }),
        })
    }

    /// The node's `Welcome`: negotiated protocol and node description.
    #[must_use]
    pub fn welcome(&self) -> &Welcome {
        &self.shared.welcome
    }

    /// Call a method and wait for its typed answer. Dropping the future
    /// cancels the call on the node.
    ///
    /// # Errors
    /// The node answered with an error, or the connection closed first.
    pub async fn call<R: Unary>(&self, request: R) -> Result<R::Response, CallError> {
        let (waiter, answer) = oneshot::channel();
        let pending = self
            .start(request.into_call(), Waiter::Unary(waiter))
            .await?;
        match pending.wait(answer).await? {
            Outcome::Ok(reply) => R::response_from(reply).map_err(CallError::UnexpectedReply),
            Outcome::Err(error) => Err(CallError::Remote(error)),
        }
    }

    /// Send an already-built one-reply call and return its reply as it
    /// came, for callers that relay calls they did not build (`vessel
    /// ui`'s bridge). The protocol-version gate is the typed calls' own
    /// ([`CallError::NotInProtocol`]); the node checks scopes as for any
    /// call. Dropping the future cancels the call on the node.
    ///
    /// # Errors
    /// The node answered with an error, or the connection closed first.
    pub async fn call_any(&self, call: UnaryCall) -> Result<Reply, CallError> {
        let (waiter, answer) = oneshot::channel();
        let pending = self.start(call.0, Waiter::Unary(waiter)).await?;
        match pending.wait(answer).await? {
            Outcome::Ok(reply) => Ok(reply),
            Outcome::Err(error) => Err(CallError::Remote(error)),
        }
    }

    /// Subscribe to a session's events. Dropping the returned
    /// [`Subscription`] unsubscribes.
    ///
    /// # Errors
    /// The node answered with an error, or the connection closed first.
    pub async fn subscribe(&self, request: Subscribe) -> Result<Subscription, CallError> {
        let (waiter, answer) = oneshot::channel();
        let pending = self
            .start(request.into_call(), Waiter::Subscribe(waiter))
            .await?;
        let opened = pending.wait(answer).await??;
        Subscription::new(
            opened,
            self.shared.commands.clone(),
            |subject| match subject {
                Subject::Session(info) => Some(*info),
                Subject::Terminal(_) => None,
            },
        )
        .ok_or(CallError::UnexpectedReply(vessel_proto::UnexpectedReply {
            expected: Method::Subscribe,
            got: Method::SubscribeTerminal,
        }))
    }

    /// Follow a terminal's records (protocol 11, ADR 0031): as
    /// [`subscribe`](Self::subscribe) does a session's. Dropping the
    /// returned subscription unsubscribes.
    ///
    /// # Errors
    /// The node answered with an error, or the connection closed first.
    pub async fn subscribe_terminal(
        &self,
        request: vessel_proto::terminals::SubscribeTerminal,
    ) -> Result<TerminalSubscription, CallError> {
        let (waiter, answer) = oneshot::channel();
        let pending = self
            .start(request.into_call(), Waiter::Subscribe(waiter))
            .await?;
        let opened = pending.wait(answer).await??;
        Subscription::new(
            opened,
            self.shared.commands.clone(),
            |subject| match subject {
                Subject::Terminal(info) => Some(*info),
                Subject::Session(_) => None,
            },
        )
        .ok_or(CallError::UnexpectedReply(vessel_proto::UnexpectedReply {
            expected: Method::SubscribeTerminal,
            got: Method::Subscribe,
        }))
    }

    /// Whether the connection has closed.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.shared.closed.is_cancelled()
    }

    /// Resolves once the connection has closed.
    pub async fn closed(&self) {
        self.shared.closed.cancelled().await;
    }

    async fn start(&self, call: Call, waiter: Waiter) -> Result<PendingCall<'_>, CallError> {
        let negotiated = self.shared.welcome.protocol;
        let needs = call.since();
        if needs > negotiated {
            return Err(CallError::NotInProtocol {
                method: call.method(),
                needs,
                negotiated,
            });
        }
        let id = self
            .shared
            .ids
            .next()
            .ok_or(CallError::RequestIdsExhausted)?;
        let command = Command::Call { id, call, waiter };
        // The queue is bounded; while waiting for room, also watch for the
        // connection closing, so a caller never waits on a dead driver.
        let queued = tokio::select! {
            biased;
            () = self.shared.closed.cancelled() => Err(CallError::Disconnected),
            queued = self.shared.commands.send(command) => {
                queued.map_err(|_closed| CallError::Disconnected)
            }
        };
        queued?;
        Ok(PendingCall {
            id,
            commands: &self.shared.commands,
            answered: false,
        })
    }
}

/// A call that was sent and not answered yet; cancels it on the node when
/// dropped unanswered.
struct PendingCall<'a> {
    id: RequestId,
    commands: &'a mpsc::Sender<Command>,
    answered: bool,
}

impl PendingCall<'_> {
    async fn wait<T>(mut self, answer: oneshot::Receiver<T>) -> Result<T, CallError> {
        let answer = answer.await;
        self.answered = true;
        answer.map_err(|_driver_gone| CallError::Disconnected)
    }
}

impl Drop for PendingCall<'_> {
    fn drop(&mut self) {
        if self.answered {
            return;
        }
        // Best effort: if the queue is full, the node finishes the call and
        // its answer is discarded when it arrives.
        if self.commands.try_send(Command::Cancel(self.id)).is_err() {
            tracing::trace!(id = %self.id, "could not send cancel");
        }
    }
}
