//! Typed RPC: the method table, the frames, and the generated dispatcher.
//!
//! [Decision Log]
//! - Purpose: every request must name its response type, and the node must
//!   be unable to compile while any method is unhandled.
//! - Alternatives: REST routes with per-route JSON (no shared method table,
//!   unhandled routes are runtime 404s); Zed's `TypeId`-keyed handler map
//!   with `downcast().unwrap()` (dynamic typing at the core); Paseo's
//!   request id inside each payload with one predicate scan per pending call.
//! - Choice: one table (`rpc_methods!` below) generates `Method`, the `Call`
//!   and `Reply` enums, the `Request` impls binding each request to its
//!   response, a `Handler` trait with one method per call, and `dispatch`,
//!   an exhaustive `match`. Correlation ids live in the frame, not the
//!   payload.
//! - Cost: adding a method means editing this table (deliberate: it is the
//!   one place the protocol is defined) and every `Handler` implementation.

use std::future::Future;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::agent::ContentBlock;
use crate::conversation::{
    AttachmentRef, BeginUpload, ChangeQueue, Conversation, DeleteSession, Delivered, FinishUpload,
    GetConversation, MarkSession, PromptSession, QueueChanged, ReadAttachment, ResumeSession,
    Upload, WriteUpload, Written,
};
use crate::device::{DeviceInfo, Invitation, InviteDevice, ListDevices, RevokeDevice, Scope};
use crate::error::WireError;
use crate::fleet::{
    BlobChunk, FleetMember, Generation, GetManifest, ListMembers, Manifest, ReadBlob,
    ReportApplied, WatchFleet,
};
use crate::forward::{
    Callback, Deliver, FleetNode, ForgetNode, Forward, Forwarded, GetCallback, GrantHome,
    ListNodes, ListPeers, NodeForgotten, RelayCall,
};
use crate::harness::Harness;
use crate::model::{ListModels, ModelRoute};
use crate::name::SecretName;
use crate::node::{NodeInfo, ProtocolVersion};
use crate::options::{
    GetHarnessOptions, GetSessionOptions, HarnessOptions, SessionOption, SetSessionOption,
};
use crate::profile::{ListProfiles, MoveProfile, ProfileInfo, ReloadProfiles};
use crate::secret::{CopySecret, DeleteSecret, ListSecrets, SetSecret};
use crate::session::{SessionInfo, SessionInput, SessionMode};
use crate::transfer::{
    CloseTransfer, ExportMemory, ExportSession, ImportMemory, ImportSession, MemoryImported,
    ReadTransfer, SessionImported, TransferManifest,
};

mod frames;
mod older;
mod requests;

pub use frames::{ClientFrame, ClientInfo, Hello, ServerFrame, SubscriptionEnd, Welcome};
pub use requests::{
    Ack, CreateSession, EventPage, GetNode, GetSession, KillSession, ListHarnesses, ListSessions,
    ReadEvents, RenameSession, SendInput, StopNode, StopWhen, Subscribe, Subscribed, Unsubscribe,
};

/// A request type, bound to its response type and its method.
pub trait Request: Serialize + DeserializeOwned + Send + 'static {
    /// The method this request invokes.
    const METHOD: Method;
    /// What a successful call returns.
    type Response: Serialize + DeserializeOwned + Send + 'static;

    /// Wrap into the generic call enum for sending.
    fn into_call(self) -> Call;

    /// Extract this request's response from a generic reply.
    ///
    /// # Errors
    /// Fails if the reply belongs to a different method, which means the
    /// peer is broken.
    fn response_from(reply: Reply) -> Result<Self::Response, UnexpectedReply>;
}

/// A reply arrived for a different method than the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnexpectedReply {
    /// Method that was called.
    pub expected: Method,
    /// Method the reply belongs to.
    pub got: Method,
}

impl std::fmt::Display for UnexpectedReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "reply for `{}` received for a `{}` request",
            self.got.as_str(),
            self.expected.as_str()
        )
    }
}

impl std::error::Error for UnexpectedReply {}

/// Declares every RPC method in one table.
///
/// Each row: `Variant / handler_fn / "wire.name" [Scope, since]: Request => Response;`
/// The scope column (protocol 4, ADR 0010) is what a paired device must hold
/// to call the method; adding a method without one does not compile. `since`
/// (protocol 5) is the first protocol version with the method: the client
/// refuses to send it on an older connection and the node refuses to run it
/// there, so neither side relies on the other ignoring what it cannot know.
macro_rules! rpc_methods {
    ($(
        $(#[$doc:meta])*
        $variant:ident / $handler:ident / $wire:literal [$scope:ident, $since:literal] : $request:ty => $response:ty;
    )*) => {
        /// Every RPC method.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        pub enum Method {
            $(
                $(#[$doc])*
                #[serde(rename = $wire)]
                $variant,
            )*
        }

        impl Method {
            /// Wire name.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( Self::$variant => $wire, )*
                }
            }

            /// The scope a paired device needs to call it.
            #[must_use]
            pub const fn scope(self) -> Scope {
                match self {
                    $( Self::$variant => Scope::$scope, )*
                }
            }

            /// The first protocol version with this method.
            #[must_use]
            pub const fn since(self) -> ProtocolVersion {
                match self {
                    $( Self::$variant => ProtocolVersion::new($since), )*
                }
            }

            /// Every method, in table order.
            pub const ALL: &'static [Self] = &[$( Self::$variant, )*];
        }

        /// A request of any method, as sent on the wire.
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(tag = "method", content = "params")]
        pub enum Call {
            $(
                $(#[$doc])*
                #[serde(rename = $wire)]
                $variant($request),
            )*
        }

        impl Call {
            /// Which method this is.
            #[must_use]
            pub const fn method(&self) -> Method {
                match self {
                    $( Self::$variant(_) => Method::$variant, )*
                }
            }
        }

        /// A successful response of any method, as sent on the wire.
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(tag = "method", content = "result")]
        pub enum Reply {
            $(
                $(#[$doc])*
                #[serde(rename = $wire)]
                $variant($response),
            )*
        }

        impl Reply {
            /// Which method this answers.
            #[must_use]
            pub const fn method(&self) -> Method {
                match self {
                    $( Self::$variant(_) => Method::$variant, )*
                }
            }
        }

        $(
            impl Request for $request {
                const METHOD: Method = Method::$variant;
                type Response = $response;

                fn into_call(self) -> Call {
                    Call::$variant(self)
                }

                fn response_from(reply: Reply) -> Result<$response, UnexpectedReply> {
                    let got = reply.method();
                    if let Reply::$variant(response) = reply {
                        Ok(response)
                    } else {
                        Err(UnexpectedReply { expected: Method::$variant, got })
                    }
                }
            }
        )*

        /// Server side of the protocol: one method per RPC.
        ///
        /// `Context` is per-connection state (for example the connection's
        /// subscriptions), so connection-scoped methods such as `subscribe`
        /// go through the same exhaustive table as everything else.
        pub trait Handler: Sync {
            /// Per-connection state passed to every call.
            type Context: Sync;

            $(
                $(#[$doc])*
                fn $handler(
                    &self,
                    context: &Self::Context,
                    request: $request,
                ) -> impl Future<Output = Result<$response, WireError>> + Send;
            )*
        }

        /// Route a call to its handler method. Exhaustive: a method without
        /// a handler does not compile.
        pub async fn dispatch<H: Handler>(handler: &H, context: &H::Context, call: Call) -> Outcome {
            match call {
                $(
                    Call::$variant(request) => match handler.$handler(context, request).await {
                        Ok(response) => Outcome::Ok(Reply::$variant(response)),
                        Err(error) => Outcome::Err(error),
                    },
                )*
            }
        }
    };
}

rpc_methods! {
    /// Describe the node.
    NodeInfo / node_info / "node.info" [Read, 2]: GetNode => NodeInfo;
    /// List installed harnesses.
    ListHarnesses / list_harnesses / "harness.list" [Read, 2]: ListHarnesses => Vec<Harness>;
    /// List sessions.
    ListSessions / list_sessions / "session.list" [Read, 2]: ListSessions => Vec<SessionInfo>;
    /// Get one session.
    GetSession / get_session / "session.get" [Read, 2]: GetSession => SessionInfo;
    /// Start a session.
    CreateSession / create_session / "session.create" [Operate, 2]: CreateSession => SessionInfo;
    /// Send input to a session.
    SendInput / send_input / "session.send" [Operate, 2]: SendInput => Ack;
    /// Kill a session.
    KillSession / kill_session / "session.kill" [Operate, 2]: KillSession => Ack;
    /// Read a page of a session's log.
    ReadEvents / read_events / "session.read" [Read, 2]: ReadEvents => EventPage;
    /// Subscribe to a session's events.
    Subscribe / subscribe / "session.subscribe" [Read, 2]: Subscribe => Subscribed;
    /// End a subscription.
    Unsubscribe / unsubscribe / "session.unsubscribe" [Read, 2]: Unsubscribe => Ack;
    /// Stop the node (protocol 3).
    StopNode / stop_node / "node.stop" [Admin, 3]: StopNode => Ack;
    /// Make a pairing invitation (protocol 4).
    InviteDevice / invite_device / "device.invite" [Admin, 4]: InviteDevice => Invitation;
    /// List paired devices (protocol 4).
    ListDevices / list_devices / "device.list" [Admin, 4]: ListDevices => Vec<DeviceInfo>;
    /// Revoke a device and close its connections (protocol 4).
    RevokeDevice / revoke_device / "device.revoke" [Admin, 4]: RevokeDevice => Ack;
    /// List profiles (protocol 5).
    ListProfiles / list_profiles / "profile.list" [Read, 5]: ListProfiles => Vec<ProfileInfo>;
    /// Read `profiles.toml` again (protocol 5).
    ReloadProfiles / reload_profiles / "profile.reload" [Admin, 5]: ReloadProfiles => Vec<ProfileInfo>;
    /// Move a structured session to another profile (protocol 5).
    MoveProfile / move_profile / "session.move_profile" [Operate, 5]: MoveProfile => SessionInfo;
    /// The fleet's current bundle (protocol 5).
    GetManifest / get_manifest / "fleet.manifest" [Sync, 5]: GetManifest => Manifest;
    /// Part of a bundle blob (protocol 5).
    ReadBlob / read_blob / "fleet.blob" [Sync, 5]: ReadBlob => BlobChunk;
    /// Wait for a newer bundle generation (protocol 5).
    WatchFleet / watch_fleet / "fleet.watch" [Sync, 5]: WatchFleet => Generation;
    /// Report an applied generation (protocol 5).
    ReportApplied / report_applied / "fleet.report" [Sync, 5]: ReportApplied => Ack;
    /// List fleet members (protocol 5).
    ListMembers / list_members / "fleet.members" [Admin, 5]: ListMembers => Vec<FleetMember>;
    /// Run a call on another node of the fleet (protocol 6).
    Forward / forward / "fleet.forward" [Read, 6]: Forward => Forwarded;
    /// A member asks its home to pass a call on (protocol 6).
    Relay / relay / "fleet.relay" [Relay, 6]: RelayCall => Forwarded;
    /// The home has a member run a call (protocol 6).
    Deliver / deliver / "fleet.deliver" [Read, 6]: Deliver => Forwarded;
    /// A member grants its home a credential on itself (protocol 6).
    GrantHome / grant_home / "fleet.grant" [Sync, 6]: GrantHome => Ack;
    /// The home's relays, for a member to be called back through (protocol 6).
    GetCallback / get_callback / "fleet.callback" [Sync, 6]: GetCallback => Callback;
    /// The home's view of the fleet, for members (protocol 6).
    ListPeers / list_peers / "fleet.peers" [Sync, 6]: ListPeers => Vec<FleetNode>;
    /// The fleet as this node knows it (protocol 6).
    ListNodes / list_nodes / "fleet.nodes" [Read, 6]: ListNodes => Vec<FleetNode>;
    /// The home forgets a member node's binding (protocol 6).
    ForgetNode / forget_node / "fleet.forget" [Admin, 6]: ForgetNode => NodeForgotten;
    /// Snapshot a session's harness files (protocol 6).
    ExportSession / export_session / "session.export" [Operate, 6]: ExportSession => TransferManifest;
    /// Copy a session here from a node of the fleet (protocol 6).
    ImportSession / import_session / "session.import" [Operate, 6]: ImportSession => SessionImported;
    /// Snapshot a profile's memory (protocol 6).
    ExportMemory / export_memory / "memory.export" [Admin, 6]: ExportMemory => TransferManifest;
    /// Copy memory here from a node of the fleet (protocol 6).
    ImportMemory / import_memory / "memory.import" [Admin, 6]: ImportMemory => MemoryImported;
    /// Part of a file of an open transfer (protocol 6).
    ReadTransfer / read_transfer / "transfer.read" [Read, 6]: ReadTransfer => BlobChunk;
    /// Drop an open transfer (protocol 6).
    CloseTransfer / close_transfer / "transfer.close" [Read, 6]: CloseTransfer => Ack;
    /// Store a secret (protocol 6).
    SetSecret / set_secret / "secret.set" [Admin, 6]: SetSecret => Ack;
    /// List secrets by name (protocol 6).
    ListSecrets / list_secrets / "secret.list" [Admin, 6]: ListSecrets => Vec<SecretName>;
    /// Remove a secret (protocol 6).
    DeleteSecret / delete_secret / "secret.delete" [Admin, 6]: DeleteSecret => Ack;
    /// Copy a secret to a node of the fleet (protocol 6).
    CopySecret / copy_secret / "secret.copy" [Admin, 6]: CopySecret => Ack;
    /// Saved-file workspace symbols (protocol 7).
    LspWorkspaceSymbols / lsp_workspace_symbols / "lsp.workspace_symbols" [Read, 7]: crate::lsp::WorkspaceSymbols => crate::lsp::Page<crate::lsp::Symbol>;
    /// Saved-file document symbols (protocol 7).
    LspDocumentSymbols / lsp_document_symbols / "lsp.document_symbols" [Read, 7]: crate::lsp::DocumentSymbols => crate::lsp::Page<crate::lsp::Symbol>;
    /// Saved-file definition (protocol 7).
    LspDefinition / lsp_definition / "lsp.definition" [Read, 7]: crate::lsp::Definition => crate::lsp::Page<crate::lsp::Location>;
    /// Saved-file references (protocol 7).
    LspReferences / lsp_references / "lsp.references" [Read, 7]: crate::lsp::References => crate::lsp::Page<crate::lsp::Location>;
    /// Saved-file hover (protocol 7).
    LspHover / lsp_hover / "lsp.hover" [Read, 7]: crate::lsp::GetHover => crate::lsp::Hover;
    /// Saved-file implementations (protocol 7).
    LspImplementations / lsp_implementations / "lsp.implementations" [Read, 7]: crate::lsp::Implementations => crate::lsp::Page<crate::lsp::Location>;
    /// Saved-file type definition (protocol 7).
    LspTypeDefinition / lsp_type_definition / "lsp.type_definition" [Read, 7]: crate::lsp::TypeDefinition => crate::lsp::Page<crate::lsp::Location>;
    /// Saved-file incoming calls (protocol 7).
    LspIncomingCalls / lsp_incoming_calls / "lsp.incoming_calls" [Read, 7]: crate::lsp::IncomingCalls => crate::lsp::Page<crate::lsp::Call>;
    /// Saved-file outgoing calls (protocol 7).
    LspOutgoingCalls / lsp_outgoing_calls / "lsp.outgoing_calls" [Read, 7]: crate::lsp::OutgoingCalls => crate::lsp::Page<crate::lsp::Call>;
    /// Saved-file diagnostics (protocol 7).
    LspDiagnostics / lsp_diagnostics / "lsp.diagnostics" [Read, 7]: crate::lsp::Diagnostics => crate::lsp::Page<crate::lsp::Diagnostic>;
    /// The model routes this node offers (protocol 7).
    ListModels / list_models / "model.list" [Read, 7]: ListModels => Vec<ModelRoute>;
    /// What a harness's agent offers before a session exists (protocol 9).
    HarnessOptions / harness_options / "harness.options" [Read, 9]: GetHarnessOptions => HarnessOptions;
    /// A structured session's config options (protocol 9).
    SessionOptions / session_options / "session.options" [Read, 9]: GetSessionOptions => Vec<SessionOption>;
    /// Change one of a running session's config options (protocol 9).
    SetSessionOption / set_session_option / "session.set_option" [Operate, 9]: SetSessionOption => Vec<SessionOption>;
    /// Give a session a title (protocol 9).
    RenameSession / rename_session / "session.rename" [Operate, 9]: RenameSession => SessionInfo;
    /// Send a prompt, delivered as asked (protocol 10).
    PromptSession / prompt_session / "session.prompt" [Operate, 10]: PromptSession => Delivered;
    /// Change a session's queue (protocol 10).
    ChangeQueue / change_queue / "session.queue" [Operate, 10]: ChangeQueue => QueueChanged;
    /// A session's conversation as it stands (protocol 10).
    GetConversation / get_conversation / "session.conversation" [Read, 10]: GetConversation => Conversation;
    /// Start an ended session again with its conversation (protocol 10).
    ResumeSession / resume_session / "session.resume" [Operate, 10]: ResumeSession => SessionInfo;
    /// Pin, archive or mark a session read (protocol 10).
    MarkSession / mark_session / "session.mark" [Operate, 10]: MarkSession => SessionInfo;
    /// Remove an ended session and everything kept with it (protocol 10).
    DeleteSession / delete_session / "session.delete" [Operate, 10]: DeleteSession => Ack;
    /// Declare a file to upload to a session (protocol 10).
    BeginUpload / begin_upload / "attachment.begin" [Operate, 10]: BeginUpload => Upload;
    /// The next piece of an upload (protocol 10).
    WriteUpload / write_upload / "attachment.write" [Operate, 10]: WriteUpload => Written;
    /// Check an upload and keep it with its session (protocol 10).
    FinishUpload / finish_upload / "attachment.finish" [Operate, 10]: FinishUpload => AttachmentRef;
    /// A piece of a session's attachment (protocol 10).
    ReadAttachment / read_attachment / "attachment.read" [Read, 10]: ReadAttachment => BlobChunk;
    /// The projects, and the git the node found (protocol 10).
    ListProjects / list_projects / "project.list" [Read, 10]: crate::workspace::ListProjects => crate::workspace::ProjectList;
    /// One project, or its next change (protocol 10).
    GetProject / get_project / "project.get" [Read, 10]: crate::workspace::GetProject => crate::workspace::ProjectInfo;
    /// Add a folder as a project (protocol 10).
    AddProject / add_project / "project.add" [Operate, 10]: crate::workspace::AddProject => crate::workspace::ProjectInfo;
    /// Clone a repository as a project (protocol 10).
    CloneProject / clone_project / "project.clone" [Operate, 10]: crate::workspace::CloneProject => crate::workspace::ProjectInfo;
    /// Start a new repository as a project (protocol 10).
    InitProject / init_project / "project.init" [Operate, 10]: crate::workspace::InitProject => crate::workspace::ProjectInfo;
    /// Forget a project (protocol 10).
    RemoveProject / remove_project / "project.remove" [Operate, 10]: crate::workspace::RemoveProject => Ack;
    /// What a project's file would run (protocol 10).
    ProjectCommands / project_commands / "project.commands" [Read, 10]: crate::workspace::GetProjectCommands => crate::workspace::ProjectCommands;
    /// Approve a project's commands (protocol 10).
    ApproveCommands / approve_commands / "project.approve" [Operate, 10]: crate::workspace::ApproveCommands => crate::workspace::ProjectCommands;
    /// The worktrees (protocol 10).
    ListWorktrees / list_worktrees / "worktree.list" [Read, 10]: crate::workspace::ListWorktrees => crate::workspace::WorktreeList;
    /// Make a worktree (protocol 10).
    CreateWorktree / create_worktree / "worktree.create" [Operate, 10]: crate::workspace::CreateWorktree => crate::workspace::CreateWorktreeOutcome;
    /// A page of a worktree's setup output (protocol 10).
    SetupOutput / setup_output / "worktree.output" [Read, 10]: crate::workspace::ReadSetupOutput => crate::workspace::SetupOutput;
    /// Run a worktree's setup again (protocol 10).
    RunSetup / run_setup / "worktree.setup" [Operate, 10]: crate::workspace::RunSetup => crate::workspace::WorktreeInfo;
    /// Remove a worktree (protocol 10).
    RemoveWorktree / remove_worktree / "worktree.remove" [Operate, 10]: crate::workspace::RemoveWorktree => crate::workspace::RemoveWorktreeOutcome;
    /// A project's worktree snapshots (protocol 10).
    ListSnapshots / list_snapshots / "worktree.snapshots" [Read, 10]: crate::workspace::ListSnapshots => crate::workspace::SnapshotList;
    /// Delete a worktree snapshot (protocol 10).
    DropSnapshot / drop_snapshot / "worktree.drop_snapshot" [Operate, 10]: crate::workspace::DropSnapshot => Ack;
    /// A root's git status (protocol 10).
    GitStatus / git_status / "git.status" [Read, 10]: crate::workspace::GetStatus => crate::workspace::GitStatus;
    /// The changed files of a diff scope (protocol 10).
    GitChanges / git_changes / "git.changes" [Read, 10]: crate::workspace::GetChanges => crate::workspace::DiffSummary;
    /// One file's diff (protocol 10).
    GitDiff / git_diff / "git.diff" [Read, 10]: crate::workspace::GetFileDiff => crate::workspace::FileDiff;
    /// A page of a file at a commit (protocol 10).
    GitShow / git_show / "git.show" [Read, 10]: crate::workspace::ShowFile => crate::workspace::FileRead;
    /// A root's branches (protocol 10).
    GitBranches / git_branches / "git.branches" [Read, 10]: crate::workspace::ListBranches => crate::workspace::BranchList;
    /// A page of history (protocol 10).
    GitLog / git_log / "git.log" [Read, 10]: crate::workspace::ReadLog => crate::workspace::CommitPage;
    /// The stash (protocol 10).
    GitStashes / git_stashes / "git.stashes" [Read, 10]: crate::workspace::ListStashes => crate::workspace::StashList;
    /// Stage files (protocol 10).
    GitStage / git_stage / "git.stage" [Operate, 10]: crate::workspace::Stage => crate::workspace::GitStatus;
    /// Unstage files (protocol 10).
    GitUnstage / git_unstage / "git.unstage" [Operate, 10]: crate::workspace::Unstage => crate::workspace::GitStatus;
    /// Drop local changes (protocol 10).
    GitDiscard / git_discard / "git.discard" [Operate, 10]: crate::workspace::Discard => crate::workspace::GitStatus;
    /// Commit (protocol 10).
    GitCommit / git_commit / "git.commit" [Operate, 10]: crate::workspace::Commit => crate::workspace::CommitOutcome;
    /// Fetch from a remote (protocol 10).
    GitFetch / git_fetch / "git.fetch" [Operate, 10]: crate::workspace::Fetch => crate::workspace::FetchOutcome;
    /// Push the current branch (protocol 10).
    GitPush / git_push / "git.push" [Operate, 10]: crate::workspace::Push => crate::workspace::PushOutcome;
    /// Fast-forward from the upstream (protocol 10).
    GitPull / git_pull / "git.pull" [Operate, 10]: crate::workspace::Pull => crate::workspace::PullOutcome;
    /// Switch to a branch (protocol 10).
    GitSwitch / git_switch / "git.switch" [Operate, 10]: crate::workspace::Switch => crate::workspace::SwitchOutcome;
    /// Rename a branch (protocol 10).
    GitRenameBranch / git_rename_branch / "git.rename_branch" [Operate, 10]: crate::workspace::RenameBranch => crate::workspace::RenameOutcome;
    /// Save, apply or drop a stash entry (protocol 10).
    GitStash / git_stash / "git.stash" [Operate, 10]: crate::workspace::StashChanges => crate::workspace::StashOutcome;
    /// A folder's entries (protocol 10).
    ListDir / list_dir / "file.list" [Read, 10]: crate::workspace::ListDir => crate::workspace::DirListing;
    /// A page of a file (protocol 10).
    ReadFile / read_file / "file.read" [Read, 10]: crate::workspace::ReadFile => crate::workspace::FileRead;
    /// Write a text file (protocol 10).
    WriteFile / write_file / "file.write" [Operate, 10]: crate::workspace::WriteFile => crate::workspace::FileVersion;
    /// Create a folder (protocol 10).
    CreateDir / create_dir / "file.create_dir" [Operate, 10]: crate::workspace::CreateDir => Ack;
    /// Move a file or folder (protocol 10).
    RenamePath / rename_path / "file.rename" [Operate, 10]: crate::workspace::RenamePath => Ack;
    /// Delete a file or folder (protocol 10).
    DeletePath / delete_path / "file.delete" [Operate, 10]: crate::workspace::DeletePath => Ack;
    /// Find files by name (protocol 10).
    FindFiles / find_files / "file.find" [Read, 10]: crate::workspace::FindFiles => crate::workspace::FoundFiles;
    /// Search inside files (protocol 10).
    SearchFiles / search_files / "file.grep" [Read, 10]: crate::workspace::SearchFiles => crate::workspace::GrepResult;
}

impl Call {
    /// The protocol version this call needs: its method's, or a later one
    /// when it uses a field added since. A node that predates a field would
    /// ignore it silently (and, say, start a session with the wrong profile
    /// or invite a device with more scopes than asked), so the client checks
    /// this before sending. A request that gains a field adds a line here.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per method, listed so that a new method must decide whether a \
                  field of it needs a later version; the list is the point"
    )]
    pub fn since(&self) -> ProtocolVersion {
        let method = self.method().since();
        let fields = match self {
            // Options chosen at creation and a title came in 9: an older
            // node would start the session in the agent's default mode (a
            // permissions mode the caller did not ask for) and unnamed.
            Self::CreateSession(request) => {
                let spec = &request.spec;
                let chooses = match &spec.mode {
                    SessionMode::Structured { options, .. } => !options.is_empty(),
                    SessionMode::Terminal { .. } => false,
                };
                if spec.place.is_some() {
                    // A place came in 10: an older node would start the
                    // session in its home folder instead.
                    Some(10)
                } else if chooses || spec.title.is_some() {
                    Some(9)
                } else {
                    spec.profile.as_ref().map(|_| 5)
                }
            }
            // `scopes` came in 5; a scope newer than that needs its own
            // version, or an older node would refuse the name it lacks.
            Self::InviteDevice(request) => request.scopes.as_ref().map(|scopes| {
                scopes
                    .iter()
                    .map(|scope| scope.since().get())
                    .fold(5, u32::max)
            }),
            Self::ReadEvents(request) => request.wait.map(|_| 6),
            // An answer to a question, and a prompt naming an attachment,
            // came in 10: an older node cannot decode either.
            Self::SendInput(request) => match &request.input {
                SessionInput::AnswerQuestion { .. } => Some(10),
                SessionInput::Prompt { content } => content
                    .iter()
                    .any(|block| matches!(block, ContentBlock::Attachment { .. }))
                    .then_some(10),
                // A cancel naming its turn: an older node would cancel
                // whatever runs, the next queued prompt included.
                SessionInput::CancelTurn { turn } => turn.map(|_| 10),
                SessionInput::Terminal { .. } | SessionInput::AnswerPermission { .. } => None,
            },
            Self::NodeInfo(_)
            | Self::ListHarnesses(_)
            | Self::ListSessions(_)
            | Self::GetSession(_)
            | Self::KillSession(_)
            | Self::Subscribe(_)
            | Self::Unsubscribe(_)
            | Self::StopNode(_)
            | Self::ListDevices(_)
            | Self::RevokeDevice(_)
            | Self::ListProfiles(_)
            | Self::ReloadProfiles(_)
            | Self::MoveProfile(_)
            | Self::GetManifest(_)
            | Self::ReadBlob(_)
            | Self::WatchFleet(_)
            | Self::ReportApplied(_)
            | Self::ListMembers(_)
            // The inner call of an envelope is checked against the target's
            // protocol where it is delivered, not against this connection.
            | Self::Forward(_)
            | Self::Relay(_)
            | Self::Deliver(_)
            | Self::GrantHome(_)
            | Self::GetCallback(_)
            | Self::ForgetNode(_)
            | Self::ListPeers(_)
            | Self::ListNodes(_)
            | Self::ExportSession(_)
            | Self::ImportSession(_)
            | Self::ExportMemory(_)
            | Self::ImportMemory(_)
            | Self::ReadTransfer(_)
            | Self::CloseTransfer(_)
            | Self::SetSecret(_)
            | Self::ListSecrets(_)
            | Self::DeleteSecret(_)
            | Self::LspWorkspaceSymbols(_)
            | Self::LspDocumentSymbols(_)
            | Self::LspDefinition(_)
            | Self::LspReferences(_)
            | Self::LspHover(_)
            | Self::LspImplementations(_)
            | Self::LspTypeDefinition(_)
            | Self::LspIncomingCalls(_)
            | Self::LspOutgoingCalls(_)
            | Self::LspDiagnostics(_)
            | Self::CopySecret(_)
            | Self::ListModels(_)
            | Self::HarnessOptions(_)
            | Self::SessionOptions(_)
            | Self::SetSessionOption(_)
            | Self::RenameSession(_)
            | Self::PromptSession(_)
            | Self::ChangeQueue(_)
            | Self::GetConversation(_)
            | Self::ResumeSession(_)
            | Self::MarkSession(_)
            | Self::DeleteSession(_)
            | Self::BeginUpload(_)
            | Self::WriteUpload(_)
            | Self::FinishUpload(_)
            | Self::ReadAttachment(_)
            | Self::ListProjects(_)
            | Self::GetProject(_)
            | Self::AddProject(_)
            | Self::CloneProject(_)
            | Self::InitProject(_)
            | Self::RemoveProject(_)
            | Self::ProjectCommands(_)
            | Self::ApproveCommands(_)
            | Self::ListWorktrees(_)
            | Self::CreateWorktree(_)
            | Self::SetupOutput(_)
            | Self::RunSetup(_)
            | Self::RemoveWorktree(_)
            | Self::ListSnapshots(_)
            | Self::DropSnapshot(_)
            | Self::GitStatus(_)
            | Self::GitChanges(_)
            | Self::GitDiff(_)
            | Self::GitShow(_)
            | Self::GitBranches(_)
            | Self::GitLog(_)
            | Self::GitStashes(_)
            | Self::GitStage(_)
            | Self::GitUnstage(_)
            | Self::GitDiscard(_)
            | Self::GitCommit(_)
            | Self::GitFetch(_)
            | Self::GitPush(_)
            | Self::GitPull(_)
            | Self::GitSwitch(_)
            | Self::GitRenameBranch(_)
            | Self::GitStash(_)
            | Self::ListDir(_)
            | Self::ReadFile(_)
            | Self::WriteFile(_)
            | Self::CreateDir(_)
            | Self::RenamePath(_)
            | Self::DeletePath(_)
            | Self::FindFiles(_)
            | Self::SearchFiles(_) => None,
        };
        fields.map_or(method, |since| method.max(ProtocolVersion::new(since)))
    }
}

/// Result of a call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
#[expect(
    clippy::large_enum_variant,
    reason = "one Outcome per call, moved once to the writer and never stored in bulk; \
              boxing the reply would add an allocation to every call to save a copy"
)]
pub enum Outcome {
    /// Success.
    Ok(Reply),
    /// Failure.
    Err(WireError),
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod conversation_tests;
