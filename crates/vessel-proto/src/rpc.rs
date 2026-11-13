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

use crate::conversation::{
    AttachmentRef, BeginUpload, ChangeQueue, Conversation, DeleteSession, Delivered, FinishUpload,
    GetConversation, MarkSession, PromptSession, QueueChanged, ReadAttachment, ResumeSession,
    SessionDeleted, Upload, WriteUpload, Written,
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
use crate::session::SessionInfo;
use crate::transfer::{
    CloseTransfer, ExportMemory, ExportSession, ImportMemory, ImportSession, MemoryImported,
    ReadTransfer, SessionImported, TransferManifest,
};

mod frames;
mod older;
mod requests;
mod since;

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

#[macro_use]
mod methods;

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
    DeleteSession / delete_session / "session.delete" [Operate, 10]: DeleteSession => SessionDeleted;
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
    // Protocol 11, track B2 (ADR 0029): pull requests on a forge.
    /// How the node is signed in to each forge.
    ForgeStatus / forge_status / "forge.status" [Read, 11]: crate::forge::GetForgeStatus => crate::forge::ForgeStatus;
    /// Every watched session's pull request.
    ListPullRequests / list_pull_requests / "pr.list" [Read, 11]: crate::forge::ListPullRequests => crate::forge::PullRequestList;
    /// A session's pull request, whole.
    GetPullRequest / get_pull_request / "pr.get" [Read, 11]: crate::forge::GetPullRequest => crate::forge::PullRequestView;
    /// Open a pull request from a session's branch.
    OpenPullRequest / open_pull_request / "pr.open" [Operate, 11]: crate::forge::OpenPullRequest => crate::forge::OpenOutcome;
    /// Merge a session's pull request, at the head the caller saw.
    MergePullRequest / merge_pull_request / "pr.merge" [Operate, 11]: crate::forge::MergePullRequest => crate::forge::MergeOutcome;
    /// Turn feedback to the session on or off.
    SetFeedback / set_feedback / "pr.feedback" [Operate, 11]: crate::forge::SetFeedback => crate::forge::SessionPullRequest;
    /// Open a shell beside a session or in a project (protocol 11).
    CreateTerminal / create_terminal / "terminal.create" [Operate, 11]: crate::terminals::CreateTerminal => crate::terminals::TerminalInfo;
    /// The terminals (protocol 11).
    ListTerminals / list_terminals / "terminal.list" [Read, 11]: crate::terminals::ListTerminals => crate::terminals::TerminalList;
    /// One terminal, or its next change (protocol 11).
    GetTerminal / get_terminal / "terminal.get" [Read, 11]: crate::terminals::GetTerminal => crate::terminals::TerminalInfo;
    /// A page of a terminal's log (protocol 11).
    ReadTerminal / read_terminal / "terminal.read" [Read, 11]: crate::terminals::ReadTerminal => EventPage;
    /// Subscribe to a terminal's log (protocol 11).
    SubscribeTerminal / subscribe_terminal / "terminal.subscribe" [Read, 11]: crate::terminals::SubscribeTerminal => crate::terminals::TerminalSubscribed;
    /// Keystrokes or a resize, for a shell (protocol 11).
    SendTerminal / send_terminal / "terminal.send" [Operate, 11]: crate::terminals::SendTerminal => Ack;
    /// Give a terminal a title (protocol 11).
    RenameTerminal / rename_terminal / "terminal.rename" [Operate, 11]: crate::terminals::RenameTerminal => crate::terminals::TerminalInfo;
    /// End a terminal's process, keeping its output (protocol 11).
    KillTerminal / kill_terminal / "terminal.kill" [Operate, 11]: crate::terminals::KillTerminal => Ack;
    /// End a terminal and forget it (protocol 11).
    CloseTerminal / close_terminal / "terminal.close" [Operate, 11]: crate::terminals::CloseTerminal => Ack;
    /// A place's scripts, their approval and their runs (protocol 11).
    ListScripts / list_scripts / "script.list" [Read, 11]: crate::scripts::ListScripts => crate::scripts::ScriptList;
    /// Run a project's script or start a service (protocol 11).
    RunScript / run_script / "script.run" [Operate, 11]: crate::scripts::RunScript => crate::scripts::ScriptRunOutcome;
    /// Run a script's run again (protocol 11).
    RestartScript / restart_script / "script.restart" [Operate, 11]: crate::scripts::RestartScript => crate::scripts::ScriptRunOutcome;
    /// The ports the node's sessions and terminals listen on (protocol 11).
    ListPorts / list_ports / "port.list" [Read, 11]: crate::ports::ListPorts => crate::ports::PortList;
    /// The agents the node can run (protocol 11, ADR 0032).
    ListAgents / list_agents / "agent.list" [Read, 11]: crate::agents::ListAgents => Vec<crate::agents::AgentInfo>;
    /// Add an agent by its command (protocol 11).
    AddAgent / add_agent / "agent.add" [Admin, 11]: crate::agents::AddAgent => crate::agents::AgentInfo;
    /// Forget an added agent (protocol 11).
    RemoveAgent / remove_agent / "agent.remove" [Admin, 11]: crate::agents::RemoveAgent => Ack;
    /// Whether agents are installed and signed in (protocol 11).
    GetAgentStatus / agent_status / "agent.status" [Read, 11]: crate::agents::GetAgentStatus => Vec<crate::agents::AgentReadiness>;
    /// The ACP registry, as the node's copy (protocol 11).
    GetCatalogue / agent_catalogue / "agent.catalogue" [Read, 11]: crate::agents::GetCatalogue => crate::agents::Catalogue;
    /// Run one of the four's documented installers (protocol 11).
    InstallAgent / install_agent / "agent.install" [Admin, 11]: crate::agents::InstallAgent => crate::agents::InstallJob;
    /// A page of an install's output (protocol 11).
    ReadInstall / install_output / "agent.install_output" [Admin, 11]: crate::agents::ReadInstall => crate::agents::InstallOutput;
    // Protocol 11, track D (ADR 0033): schedules and saved prompts.
    /// Compute a bounded schedule preview without saving or starting work.
    PreviewSchedule / preview_schedule / "schedule.preview" [Read, 12]: crate::automation::PreviewSchedule => crate::automation::SchedulePreview;
    /// Every schedule on the node (protocol 11).
    ListSchedules / list_schedules / "schedule.list" [Read, 11]: crate::automation::ListSchedules => crate::automation::ScheduleList;
    /// One schedule (protocol 11).
    GetSchedule / get_schedule / "schedule.get" [Read, 11]: crate::automation::GetSchedule => crate::automation::ScheduleInfo;
    /// Make a schedule (protocol 11).
    CreateSchedule / create_schedule / "schedule.create" [Operate, 11]: crate::automation::CreateSchedule => crate::automation::ScheduleInfo;
    /// Replace a schedule's definition (protocol 11).
    UpdateSchedule / update_schedule / "schedule.update" [Operate, 11]: crate::automation::UpdateSchedule => crate::automation::ScheduleInfo;
    /// Stop a schedule's runs (protocol 11).
    PauseSchedule / pause_schedule / "schedule.pause" [Operate, 11]: crate::automation::PauseSchedule => crate::automation::ScheduleInfo;
    /// Let a paused schedule run again (protocol 11).
    ResumeSchedule / resume_schedule / "schedule.resume" [Operate, 11]: crate::automation::ResumeSchedule => crate::automation::ScheduleInfo;
    /// Run a schedule once now (protocol 11).
    RunSchedule / run_schedule / "schedule.run" [Operate, 11]: crate::automation::RunSchedule => crate::automation::ScheduleRun;
    /// Remove a schedule and its runs (protocol 11).
    DeleteSchedule / delete_schedule / "schedule.delete" [Operate, 11]: crate::automation::DeleteSchedule => Ack;
    /// A schedule's newest runs (protocol 11).
    ListRuns / list_runs / "schedule.runs" [Read, 11]: crate::automation::ListRuns => crate::automation::RunList;
    /// The saved prompts (protocol 11).
    ListPrompts / list_prompts / "prompt.list" [Read, 11]: crate::automation::ListPrompts => crate::automation::PromptList;
    /// Save a prompt (protocol 11).
    CreatePrompt / create_prompt / "prompt.create" [Operate, 11]: crate::automation::CreatePrompt => crate::automation::SavedPrompt;
    /// Change a saved prompt (protocol 11).
    UpdatePrompt / update_prompt / "prompt.update" [Operate, 11]: crate::automation::UpdatePrompt => crate::automation::SavedPrompt;
    /// Remove a saved prompt (protocol 11).
    DeletePrompt / delete_prompt / "prompt.delete" [Operate, 11]: crate::automation::DeletePrompt => Ack;
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
