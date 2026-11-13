//! The scope check (ADR 0010): every call is checked against the scope its
//! method declares, before it runs.
//!
//! [Decision Log]
//! - Purpose: a paired device may only call what its scopes allow, and no
//!   method may be added without the check.
//! - Constraint: `vessel-proto`'s `Handler` and `dispatch` are frozen for
//!   milestone 3, so the check cannot live in the generated dispatch, and
//!   `vessel-rpc` knows nothing of principals.
//! - Alternatives: a check at the top of every `NodeService` method (one
//!   forgotten line is a hole); a check in `vessel-rpc` before dispatch (a
//!   principal concept in the transport layer).
//! - Choice: [`Guarded`] wraps the service and implements `Handler` itself,
//!   generated from one row per method. Every generated method runs the
//!   same check first, with the method taken from the request type
//!   (`Request::METHOD`), so a row cannot check the wrong scope. A method
//!   added to the protocol is a new `Handler` method, which does not compile
//!   here until it gets its row, and the row cannot leave the check out.
//! - `NodeService`'s calls are inherent methods, not a `Handler`, so the
//!   service cannot be handed to a `Server` without this wrapper.
//! - Cost: the rows repeat the protocol table's handler names and types;
//!   the compiler keeps them in step.
//!
//! [Decision Log: agents and workspaces] (protocol 10, ADR 0027)
//! - Agents get, of the workspace calls, only what orchestration needs
//!   (`workspace::policy`); the rest are refused right after the scope,
//!   whatever the agent's scopes, by the same generated method, so a new
//!   workspace call is held to the list from its first row.
//!
//! [Decision Log: ownership] (protocol 5, ADR 0013)
//! - Agents may `operate` only on sessions their own session started (or
//!   that those started). Every request type declares which session it
//!   operates on ([`Target`]); the generated method checks ownership right
//!   after the scope, so, as with scopes, a new method does not compile
//!   until it says. Reads (`session.get`, `session.read`,
//!   `session.subscribe`) name a session too but are not ownership-checked:
//!   an agent sees the fleet's sessions, as ADR 0013 grants.

use vessel_proto::{
    Ack, BlobChunk, Callback, CloseTransfer, CopySecret, CreateSession, DeleteSecret, Deliver,
    DeviceInfo, EventPage, ExportMemory, ExportSession, FleetMember, FleetNode, ForgetNode,
    Forward, Forwarded, Generation, GetCallback, GetManifest, GetNode, GetSession, GrantHome,
    Handler, Harness, ImportMemory, ImportSession, Invitation, InviteDevice, KillSession,
    ListDevices, ListHarnesses, ListMembers, ListModels, ListNodes, ListPeers, ListProfiles,
    ListSecrets, ListSessions, Manifest, MemoryImported, ModelRoute, MoveProfile, NodeForgotten,
    NodeInfo, ProfileInfo, ReadBlob, ReadEvents, ReadTransfer, RelayCall, ReloadProfiles,
    RenameSession, ReportApplied, Request, RevokeDevice, SecretName, SendInput, SessionId,
    SessionImported, SessionInfo, SessionOption, SetSecret, SetSessionOption, StopNode, Subscribe,
    Subscribed, TransferManifest, Unsubscribe, WatchFleet, WireError,
};

use crate::service::{Connection, NodeService};

/// The node's service behind the scope check. This is the handler every
/// transport serves.
#[derive(Debug)]
pub(crate) struct Guarded(pub(crate) NodeService);

macro_rules! guarded {
    (
        $( $handler:ident : $request:ty => $response:ty; )*
        // The envelope's rows: their handlers run a call here through this
        // guard, so they are handed it (protocol 6, `envelope`).
        @through_the_guard {
            $( $through:ident : $through_request:ty => $through_response:ty; )*
        }
    ) => {
        impl Handler for Guarded {
            type Context = Connection;

            $(
                async fn $handler(
                    &self,
                    connection: &Connection,
                    request: $request,
                ) -> Result<$response, WireError> {
                    connection.principal().permit(<$request as Request>::METHOD)?;
                    crate::workspace::refuse_agents(
                        connection.principal(),
                        <$request as Request>::METHOD,
                    )?;
                    if let Some(session) = Target::target(&request) {
                        self.0.check_owner(connection.principal(), session).await?;
                        self.0.refuse_older(connection, session).await?;
                    }
                    self.0.$handler(connection, request).await
                }
            )*

            $(
                async fn $through(
                    &self,
                    connection: &Connection,
                    request: $through_request,
                ) -> Result<$through_response, WireError> {
                    connection.principal().permit(<$through_request as Request>::METHOD)?;
                    if let Some(session) = Target::target(&request) {
                        self.0.check_owner(connection.principal(), session).await?;
                        self.0.refuse_older(connection, session).await?;
                    }
                    self.0.$through(connection, request, self).await
                }
            )*
        }
    };
}

/// The session an `operate` call acts on, for the ownership rule.
trait Target {
    fn target(&self) -> Option<SessionId>;
}

macro_rules! targets {
    ($( $request:ty ),* $(,)?) => {
        $(
            impl Target for $request {
                fn target(&self) -> Option<SessionId> {
                    Some(self.session)
                }
            }
        )*
    };
}

macro_rules! untargeted {
    ($( $request:ty ),* $(,)?) => {
        $(
            impl Target for $request {
                fn target(&self) -> Option<SessionId> {
                    None
                }
            }
        )*
    };
}

// `session.export` copies a session's conversation away: an agent may do
// that only to a session it started, like any other `operate` call on one.
// Protocol 9: changing a session's options or its title steers it, like
// sending it input.
// Protocol 10: prompting, the queue, starting again, marks, deleting and
// uploading to a session all act on it, as sending it input does. An
// upload's later pieces name the upload, whose holder check keeps them to
// the caller that began it (`uploads`).
targets!(
    SendInput,
    KillSession,
    MoveProfile,
    ExportSession,
    SetSessionOption,
    RenameSession,
    vessel_proto::conversation::PromptSession,
    vessel_proto::conversation::ChangeQueue,
    vessel_proto::conversation::ResumeSession,
    vessel_proto::conversation::MarkSession,
    vessel_proto::conversation::DeleteSession,
    vessel_proto::conversation::BeginUpload,
    // Protocol 11: opening, merging and the feedback switch act on a
    // session's branch and its conversation.
    vessel_proto::forge::OpenPullRequest,
    vessel_proto::forge::MergePullRequest,
    vessel_proto::forge::SetFeedback,
);

// `session.create` is limited by `agents` instead (count and depth), and so
// is `session.import`, which creates one; the reads are open to every
// agent; the envelope's rows carry a call whose own row is checked where it
// runs (protocol 6, Track X); the rest are not `operate` calls.
// Schedules name schedules, not sessions: who may change one is the
// scheduler's rule (ADR 0033), and a run is checked as its definer.
// Saved prompts name none either: agents may only read them.
untargeted!(
    vessel_proto::automation::ListPrompts,
    vessel_proto::automation::CreatePrompt,
    vessel_proto::automation::UpdatePrompt,
    vessel_proto::automation::DeletePrompt,
    vessel_proto::automation::PreviewSchedule,
    vessel_proto::automation::ListSchedules,
    vessel_proto::automation::GetSchedule,
    vessel_proto::automation::CreateSchedule,
    vessel_proto::automation::UpdateSchedule,
    vessel_proto::automation::PauseSchedule,
    vessel_proto::automation::ResumeSchedule,
    vessel_proto::automation::RunSchedule,
    vessel_proto::automation::DeleteSchedule,
    vessel_proto::automation::ListRuns,
    vessel_proto::conversation::GetConversation,
    vessel_proto::conversation::ReadAttachment,
    vessel_proto::conversation::WriteUpload,
    vessel_proto::conversation::FinishUpload,
    ListModels,
    vessel_proto::GetHarnessOptions,
    vessel_proto::GetSessionOptions,
    GetNode,
    ListHarnesses,
    ListSessions,
    GetSession,
    CreateSession,
    ReadEvents,
    Subscribe,
    Unsubscribe,
    StopNode,
    InviteDevice,
    ListDevices,
    RevokeDevice,
    ListProfiles,
    ReloadProfiles,
    GetManifest,
    ReadBlob,
    WatchFleet,
    ReportApplied,
    ListMembers,
    Forward,
    RelayCall,
    Deliver,
    GrantHome,
    GetCallback,
    ForgetNode,
    ListPeers,
    ListNodes,
    ImportSession,
    ExportMemory,
    ImportMemory,
    ReadTransfer,
    CloseTransfer,
    SetSecret,
    ListSecrets,
    DeleteSecret,
    CopySecret,
    vessel_proto::lsp::WorkspaceSymbols,
    vessel_proto::lsp::DocumentSymbols,
    vessel_proto::lsp::Definition,
    vessel_proto::lsp::References,
    vessel_proto::lsp::GetHover,
    vessel_proto::lsp::Implementations,
    vessel_proto::lsp::TypeDefinition,
    vessel_proto::lsp::IncomingCalls,
    vessel_proto::lsp::OutgoingCalls,
    vessel_proto::lsp::Diagnostics,
    vessel_proto::workspace::ListProjects,
    vessel_proto::workspace::GetProject,
    vessel_proto::workspace::AddProject,
    vessel_proto::workspace::CloneProject,
    vessel_proto::workspace::InitProject,
    vessel_proto::workspace::RemoveProject,
    vessel_proto::workspace::GetProjectCommands,
    vessel_proto::workspace::ApproveCommands,
    vessel_proto::workspace::ListWorktrees,
    vessel_proto::workspace::CreateWorktree,
    vessel_proto::workspace::ReadSetupOutput,
    vessel_proto::workspace::RunSetup,
    vessel_proto::workspace::RemoveWorktree,
    vessel_proto::workspace::ListSnapshots,
    vessel_proto::workspace::DropSnapshot,
    vessel_proto::workspace::GetStatus,
    vessel_proto::workspace::GetChanges,
    vessel_proto::workspace::GetFileDiff,
    vessel_proto::workspace::ShowFile,
    vessel_proto::workspace::ListBranches,
    vessel_proto::workspace::ReadLog,
    vessel_proto::workspace::ListStashes,
    vessel_proto::workspace::Stage,
    vessel_proto::workspace::Unstage,
    vessel_proto::workspace::Discard,
    vessel_proto::workspace::Commit,
    vessel_proto::workspace::Fetch,
    vessel_proto::workspace::Push,
    vessel_proto::workspace::Pull,
    vessel_proto::workspace::Switch,
    vessel_proto::workspace::RenameBranch,
    vessel_proto::workspace::StashChanges,
    vessel_proto::workspace::ListDir,
    vessel_proto::workspace::ReadFile,
    vessel_proto::workspace::WriteFile,
    vessel_proto::workspace::CreateDir,
    vessel_proto::workspace::RenamePath,
    vessel_proto::workspace::DeletePath,
    vessel_proto::workspace::FindFiles,
    vessel_proto::workspace::SearchFiles,
    // Protocol 11: reads; `pr.get`'s refresh is held to ownership by the
    // service (ADR 0029).
    vessel_proto::forge::GetForgeStatus,
    vessel_proto::forge::ListPullRequests,
    vessel_proto::forge::GetPullRequest,
    // Protocol 11 (ADR 0031): a terminal names a session it is beside, not
    // one it operates; who may open one and use it is the terminals' rule
    // (`service::terminals`), which checks an agent's ownership of the
    // session it opens one beside.
    vessel_proto::terminals::CreateTerminal,
    vessel_proto::terminals::ListTerminals,
    vessel_proto::terminals::GetTerminal,
    vessel_proto::terminals::ReadTerminal,
    vessel_proto::terminals::SubscribeTerminal,
    vessel_proto::terminals::SendTerminal,
    vessel_proto::terminals::RenameTerminal,
    vessel_proto::terminals::KillTerminal,
    vessel_proto::terminals::CloseTerminal,
    vessel_proto::scripts::ListScripts,
    vessel_proto::scripts::RunScript,
    vessel_proto::scripts::RestartScript,
    vessel_proto::ports::ListPorts,
    // Protocol 11's agents act on no session; adding and removing one is
    // refused to agents by the service itself (ADR 0032).
    vessel_proto::agents::ListAgents,
    vessel_proto::agents::AddAgent,
    vessel_proto::agents::RemoveAgent,
    vessel_proto::agents::GetAgentStatus,
    vessel_proto::agents::GetCatalogue,
    vessel_proto::agents::InstallAgent,
    vessel_proto::agents::ReadInstall,
);

guarded! {
    node_info: GetNode => NodeInfo;
    list_harnesses: ListHarnesses => Vec<Harness>;
    list_sessions: ListSessions => Vec<SessionInfo>;
    get_session: GetSession => SessionInfo;
    create_session: CreateSession => SessionInfo;
    send_input: SendInput => Ack;
    kill_session: KillSession => Ack;
    read_events: ReadEvents => EventPage;
    subscribe: Subscribe => Subscribed;
    unsubscribe: Unsubscribe => Ack;
    stop_node: StopNode => Ack;
    invite_device: InviteDevice => Invitation;
    list_devices: ListDevices => Vec<DeviceInfo>;
    revoke_device: RevokeDevice => Ack;
    list_profiles: ListProfiles => Vec<ProfileInfo>;
    reload_profiles: ReloadProfiles => Vec<ProfileInfo>;
    move_profile: MoveProfile => SessionInfo;
    get_manifest: GetManifest => Manifest;
    read_blob: ReadBlob => BlobChunk;
    watch_fleet: WatchFleet => Generation;
    report_applied: ReportApplied => Ack;
    list_members: ListMembers => Vec<FleetMember>;
    grant_home: GrantHome => Ack;
    get_callback: GetCallback => Callback;
    forget_node: ForgetNode => NodeForgotten;
    list_peers: ListPeers => Vec<FleetNode>;
    list_nodes: ListNodes => Vec<FleetNode>;
    export_session: ExportSession => TransferManifest;
    import_session: ImportSession => SessionImported;
    export_memory: ExportMemory => TransferManifest;
    import_memory: ImportMemory => MemoryImported;
    read_transfer: ReadTransfer => BlobChunk;
    close_transfer: CloseTransfer => Ack;
    set_secret: SetSecret => Ack;
    list_secrets: ListSecrets => Vec<SecretName>;
    delete_secret: DeleteSecret => Ack;
    copy_secret: CopySecret => Ack;
    list_models: ListModels => Vec<ModelRoute>;
    harness_options: vessel_proto::GetHarnessOptions => vessel_proto::HarnessOptions;
    session_options: vessel_proto::GetSessionOptions => Vec<SessionOption>;
    set_session_option: SetSessionOption => Vec<SessionOption>;
    rename_session: RenameSession => SessionInfo;
    prompt_session: vessel_proto::conversation::PromptSession => vessel_proto::Delivered;
    change_queue: vessel_proto::conversation::ChangeQueue => vessel_proto::conversation::QueueChanged;
    get_conversation: vessel_proto::conversation::GetConversation => vessel_proto::Conversation;
    resume_session: vessel_proto::conversation::ResumeSession => SessionInfo;
    mark_session: vessel_proto::conversation::MarkSession => SessionInfo;
    delete_session: vessel_proto::conversation::DeleteSession => vessel_proto::conversation::SessionDeleted;
    begin_upload: vessel_proto::conversation::BeginUpload => vessel_proto::conversation::Upload;
    write_upload: vessel_proto::conversation::WriteUpload => vessel_proto::conversation::Written;
    finish_upload: vessel_proto::conversation::FinishUpload => vessel_proto::AttachmentRef;
    read_attachment: vessel_proto::conversation::ReadAttachment => BlobChunk;
    list_projects: vessel_proto::workspace::ListProjects => vessel_proto::workspace::ProjectList;
    get_project: vessel_proto::workspace::GetProject => vessel_proto::workspace::ProjectInfo;
    add_project: vessel_proto::workspace::AddProject => vessel_proto::workspace::ProjectInfo;
    clone_project: vessel_proto::workspace::CloneProject => vessel_proto::workspace::ProjectInfo;
    init_project: vessel_proto::workspace::InitProject => vessel_proto::workspace::ProjectInfo;
    remove_project: vessel_proto::workspace::RemoveProject => Ack;
    project_commands: vessel_proto::workspace::GetProjectCommands => vessel_proto::workspace::ProjectCommands;
    approve_commands: vessel_proto::workspace::ApproveCommands => vessel_proto::workspace::ProjectCommands;
    list_worktrees: vessel_proto::workspace::ListWorktrees => vessel_proto::workspace::WorktreeList;
    create_worktree: vessel_proto::workspace::CreateWorktree => vessel_proto::workspace::CreateWorktreeOutcome;
    setup_output: vessel_proto::workspace::ReadSetupOutput => vessel_proto::workspace::SetupOutput;
    run_setup: vessel_proto::workspace::RunSetup => vessel_proto::workspace::WorktreeInfo;
    remove_worktree: vessel_proto::workspace::RemoveWorktree => vessel_proto::workspace::RemoveWorktreeOutcome;
    list_snapshots: vessel_proto::workspace::ListSnapshots => vessel_proto::workspace::SnapshotList;
    drop_snapshot: vessel_proto::workspace::DropSnapshot => Ack;
    git_status: vessel_proto::workspace::GetStatus => vessel_proto::workspace::GitStatus;
    git_changes: vessel_proto::workspace::GetChanges => vessel_proto::workspace::DiffSummary;
    git_diff: vessel_proto::workspace::GetFileDiff => vessel_proto::workspace::FileDiff;
    git_show: vessel_proto::workspace::ShowFile => vessel_proto::workspace::FileRead;
    git_branches: vessel_proto::workspace::ListBranches => vessel_proto::workspace::BranchList;
    git_log: vessel_proto::workspace::ReadLog => vessel_proto::workspace::CommitPage;
    git_stashes: vessel_proto::workspace::ListStashes => vessel_proto::workspace::StashList;
    git_stage: vessel_proto::workspace::Stage => vessel_proto::workspace::GitStatus;
    git_unstage: vessel_proto::workspace::Unstage => vessel_proto::workspace::GitStatus;
    git_discard: vessel_proto::workspace::Discard => vessel_proto::workspace::GitStatus;
    git_commit: vessel_proto::workspace::Commit => vessel_proto::workspace::CommitOutcome;
    git_fetch: vessel_proto::workspace::Fetch => vessel_proto::workspace::FetchOutcome;
    git_push: vessel_proto::workspace::Push => vessel_proto::workspace::PushOutcome;
    git_pull: vessel_proto::workspace::Pull => vessel_proto::workspace::PullOutcome;
    git_switch: vessel_proto::workspace::Switch => vessel_proto::workspace::SwitchOutcome;
    git_rename_branch: vessel_proto::workspace::RenameBranch => vessel_proto::workspace::RenameOutcome;
    git_stash: vessel_proto::workspace::StashChanges => vessel_proto::workspace::StashOutcome;
    list_dir: vessel_proto::workspace::ListDir => vessel_proto::workspace::DirListing;
    read_file: vessel_proto::workspace::ReadFile => vessel_proto::workspace::FileRead;
    write_file: vessel_proto::workspace::WriteFile => vessel_proto::workspace::FileVersion;
    create_dir: vessel_proto::workspace::CreateDir => Ack;
    rename_path: vessel_proto::workspace::RenamePath => Ack;
    delete_path: vessel_proto::workspace::DeletePath => Ack;
    find_files: vessel_proto::workspace::FindFiles => vessel_proto::workspace::FoundFiles;
    search_files: vessel_proto::workspace::SearchFiles => vessel_proto::workspace::GrepResult;
    forge_status: vessel_proto::forge::GetForgeStatus => vessel_proto::forge::ForgeStatus;
    list_pull_requests: vessel_proto::forge::ListPullRequests => vessel_proto::forge::PullRequestList;
    get_pull_request: vessel_proto::forge::GetPullRequest => vessel_proto::forge::PullRequestView;
    open_pull_request: vessel_proto::forge::OpenPullRequest => vessel_proto::forge::OpenOutcome;
    merge_pull_request: vessel_proto::forge::MergePullRequest => vessel_proto::forge::MergeOutcome;
    set_feedback: vessel_proto::forge::SetFeedback => vessel_proto::forge::SessionPullRequest;
    list_agents: vessel_proto::agents::ListAgents => Vec<vessel_proto::agents::AgentInfo>;
    add_agent: vessel_proto::agents::AddAgent => vessel_proto::agents::AgentInfo;
    remove_agent: vessel_proto::agents::RemoveAgent => Ack;
    agent_status: vessel_proto::agents::GetAgentStatus => Vec<vessel_proto::agents::AgentReadiness>;
    agent_catalogue: vessel_proto::agents::GetCatalogue => vessel_proto::agents::Catalogue;
    install_agent: vessel_proto::agents::InstallAgent => vessel_proto::agents::InstallJob;
    install_output: vessel_proto::agents::ReadInstall => vessel_proto::agents::InstallOutput;
    lsp_workspace_symbols: vessel_proto::lsp::WorkspaceSymbols => vessel_proto::lsp::Page<vessel_proto::lsp::Symbol>;
    lsp_document_symbols: vessel_proto::lsp::DocumentSymbols => vessel_proto::lsp::Page<vessel_proto::lsp::Symbol>;
    lsp_definition: vessel_proto::lsp::Definition => vessel_proto::lsp::Page<vessel_proto::lsp::Location>;
    lsp_references: vessel_proto::lsp::References => vessel_proto::lsp::Page<vessel_proto::lsp::Location>;
    lsp_hover: vessel_proto::lsp::GetHover => vessel_proto::lsp::Hover;
    lsp_implementations: vessel_proto::lsp::Implementations => vessel_proto::lsp::Page<vessel_proto::lsp::Location>;
    lsp_type_definition: vessel_proto::lsp::TypeDefinition => vessel_proto::lsp::Page<vessel_proto::lsp::Location>;
    lsp_incoming_calls: vessel_proto::lsp::IncomingCalls => vessel_proto::lsp::Page<vessel_proto::lsp::Call>;
    lsp_outgoing_calls: vessel_proto::lsp::OutgoingCalls => vessel_proto::lsp::Page<vessel_proto::lsp::Call>;
    lsp_diagnostics: vessel_proto::lsp::Diagnostics => vessel_proto::lsp::Page<vessel_proto::lsp::Diagnostic>;
    create_terminal: vessel_proto::terminals::CreateTerminal => vessel_proto::terminals::TerminalInfo;
    list_terminals: vessel_proto::terminals::ListTerminals => vessel_proto::terminals::TerminalList;
    get_terminal: vessel_proto::terminals::GetTerminal => vessel_proto::terminals::TerminalInfo;
    read_terminal: vessel_proto::terminals::ReadTerminal => EventPage;
    subscribe_terminal: vessel_proto::terminals::SubscribeTerminal => vessel_proto::terminals::TerminalSubscribed;
    send_terminal: vessel_proto::terminals::SendTerminal => Ack;
    rename_terminal: vessel_proto::terminals::RenameTerminal => vessel_proto::terminals::TerminalInfo;
    kill_terminal: vessel_proto::terminals::KillTerminal => Ack;
    close_terminal: vessel_proto::terminals::CloseTerminal => Ack;
    list_scripts: vessel_proto::scripts::ListScripts => vessel_proto::scripts::ScriptList;
    run_script: vessel_proto::scripts::RunScript => vessel_proto::scripts::ScriptRunOutcome;
    restart_script: vessel_proto::scripts::RestartScript => vessel_proto::scripts::ScriptRunOutcome;
    list_ports: vessel_proto::ports::ListPorts => vessel_proto::ports::PortList;
    preview_schedule: vessel_proto::automation::PreviewSchedule => vessel_proto::automation::SchedulePreview;
    list_schedules: vessel_proto::automation::ListSchedules => vessel_proto::automation::ScheduleList;
    get_schedule: vessel_proto::automation::GetSchedule => vessel_proto::automation::ScheduleInfo;
    create_schedule: vessel_proto::automation::CreateSchedule => vessel_proto::automation::ScheduleInfo;
    update_schedule: vessel_proto::automation::UpdateSchedule => vessel_proto::automation::ScheduleInfo;
    pause_schedule: vessel_proto::automation::PauseSchedule => vessel_proto::automation::ScheduleInfo;
    resume_schedule: vessel_proto::automation::ResumeSchedule => vessel_proto::automation::ScheduleInfo;
    run_schedule: vessel_proto::automation::RunSchedule => vessel_proto::automation::ScheduleRun;
    delete_schedule: vessel_proto::automation::DeleteSchedule => Ack;
    list_runs: vessel_proto::automation::ListRuns => vessel_proto::automation::RunList;
    list_prompts: vessel_proto::automation::ListPrompts => vessel_proto::automation::PromptList;
    create_prompt: vessel_proto::automation::CreatePrompt => vessel_proto::automation::SavedPrompt;
    update_prompt: vessel_proto::automation::UpdatePrompt => vessel_proto::automation::SavedPrompt;
    delete_prompt: vessel_proto::automation::DeletePrompt => Ack;

    @through_the_guard {
        forward: Forward => Forwarded;
        relay: RelayCall => Forwarded;
        deliver: Deliver => Forwarded;
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use super::*;

    /// Method resolution on a concrete type picks `IsHandler` when the
    /// type implements `Handler`, and falls back to `NotHandler` (one
    /// autoref further) when it does not: a test of a missing impl that
    /// stable Rust can write.
    struct Probe<T>(PhantomData<T>);

    trait IsHandler {
        fn is_handler(&self) -> bool {
            true
        }
    }

    impl<T: Handler> IsHandler for Probe<T> {}

    trait NotHandler {
        fn is_handler(&self) -> bool {
            false
        }
    }

    impl<T> NotHandler for &Probe<T> {}

    /// Only the guarded service can be handed to a `Server`; the service
    /// itself has no `Handler` impl to serve without the scope check.
    #[test]
    fn only_the_guarded_service_is_a_handler() {
        assert!(Probe::<Guarded>(PhantomData).is_handler());
        assert!(!(&Probe::<NodeService>(PhantomData)).is_handler());
    }
}
