//! The client against a fake node: over the in-memory transport for stream
//! logic, and over a loopback websocket for the connection and the token.

mod calls;
mod diagnostics;
mod memory;
mod pinned;
mod resume;
mod secure;
mod stream;
mod websocket;

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use chrono::DateTime;
use tokio_util::sync::CancellationToken;
use vessel_proto::{
    Ack, ClientInfo, CreateSession, DeviceInfo, ErrorCode, EventPage, EventRecord, EventSeq,
    GetNode, GetSession, Handler, Harness, HarnessKind, Invitation, InviteDevice, KillSession,
    ListDevices, ListHarnesses, ListSessions, NodeId, NodeInfo, NodeState, ProtocolRange,
    ReadEvents, RevokeDevice, SendInput, SessionEvent, SessionId, SessionInfo, SessionMode,
    SessionSpec, SessionStatus, StopNode, StopWhen, Subscribe, Subscribed, SubscriptionEnd,
    TerminalSize, Unsubscribe, WireError,
};
use vessel_proto::{
    BlobChunk, FleetMember, Generation, GetManifest, ListMembers, ListProfiles, Manifest,
    MoveProfile, ProfileInfo, ReadBlob, ReloadProfiles, ReportApplied, WatchFleet,
};
use vessel_rpc::memory::Disconnected;
use vessel_rpc::{ClientConfig, Outbound, OutboundError, Server, ServerConfig};

use crate::NodeClient;

/// Records in the fake session's log.
const LOG_LEN: u64 = 20;
/// The first subscription delivers this many records, then lags.
const LAG_AFTER: u64 = 5;
/// Records `session.read` can see; the rest are "not persisted yet", so
/// catching up must switch back to a subscription to get them.
const STORED: u64 = 12;
/// `session.read` pages are at most this long.
const PAGE: u64 = 4;

fn session_id() -> SessionId {
    "00000000-0000-4000-8000-000000000001".parse().unwrap()
}

fn record(seq: u64) -> EventRecord {
    EventRecord {
        seq: EventSeq::new(seq),
        at: DateTime::UNIX_EPOCH,
        event: SessionEvent::Status {
            status: SessionStatus::Running,
        },
    }
}

fn node() -> NodeInfo {
    NodeInfo {
        id: NodeId::random(),
        name: "fake".to_owned(),
        hostname: "fake".to_owned(),
        os: "linux".to_owned(),
        arch: "x86_64".to_owned(),
        version: "0".to_owned(),
        protocol: ProtocolRange::CURRENT,
        started_at: DateTime::UNIX_EPOCH,
        state: NodeState::Serving,
        public_key: None,
        timezone: None,
    }
}

fn info() -> SessionInfo {
    {
        let mut info = SessionInfo::new(
            session_id(),
            SessionSpec::new(
                HarnessKind::OpenCode,
                SessionMode::Terminal {
                    args: Vec::new(),
                    size: TerminalSize::default(),
                },
            )
            .with_cwd(PathBuf::from("/work")),
            DateTime::UNIX_EPOCH,
        );
        info.status = SessionStatus::Running;
        info.next_seq = EventSeq::new(LOG_LEN);
        info
    }
}

fn client_info() -> ClientInfo {
    ClientInfo {
        name: "vessel-client-tests".to_owned(),
        version: "0".to_owned(),
    }
}

/// A node with one session whose log is `oldest..LOG_LEN`.
struct FakeNode {
    /// The id new connections are welcomed with (change it to play a
    /// reinstalled machine).
    id: Mutex<NodeId>,
    /// Records below this were pruned.
    oldest: u64,
    subscribes: AtomicUsize,
    reads: AtomicUsize,
    /// `session.kill` calls received; each one hangs (never answers).
    kills: AtomicUsize,
    /// Set: the session is gone (`NotFound`), as after a wiped data dir.
    forgotten: AtomicBool,
    /// The last `node.stop`.
    stopped: Mutex<Option<StopWhen>>,
}

impl FakeNode {
    fn new(oldest: u64) -> Self {
        Self {
            id: Mutex::new(NodeId::random()),
            oldest,
            subscribes: AtomicUsize::new(0),
            reads: AtomicUsize::new(0),
            kills: AtomicUsize::new(0),
            forgotten: AtomicBool::new(false),
            stopped: Mutex::new(None),
        }
    }
}

struct Context {
    outbound: Outbound,
}

fn unsupported() -> WireError {
    WireError::new(ErrorCode::Unsupported, "not in the fake")
}

/// Methods the fake does not serve.
macro_rules! unserved {
    ($( $handler:ident : $request:ty => $response:ty; )*) => {
        $(
            async fn $handler(
                &self,
                _context: &Context,
                _request: $request,
            ) -> Result<$response, WireError> {
                Err(unsupported())
            }
        )*
    };
}

impl Handler for FakeNode {
    type Context = Context;

    async fn node_info(&self, _: &Context, _: GetNode) -> Result<NodeInfo, WireError> {
        Ok(node())
    }

    async fn list_harnesses(
        &self,
        _: &Context,
        _: ListHarnesses,
    ) -> Result<Vec<Harness>, WireError> {
        Ok(Vec::new())
    }

    async fn list_sessions(
        &self,
        _: &Context,
        _: ListSessions,
    ) -> Result<Vec<SessionInfo>, WireError> {
        Ok(vec![info()])
    }

    async fn get_session(
        &self,
        _: &Context,
        request: GetSession,
    ) -> Result<SessionInfo, WireError> {
        if request.session == session_id() {
            Ok(info())
        } else {
            Err(WireError::new(ErrorCode::NotFound, "no such session"))
        }
    }

    async fn create_session(
        &self,
        _: &Context,
        _: CreateSession,
    ) -> Result<SessionInfo, WireError> {
        Err(unsupported())
    }

    async fn send_input(&self, _: &Context, _: SendInput) -> Result<Ack, WireError> {
        Err(unsupported())
    }

    async fn kill_session(&self, _: &Context, _: KillSession) -> Result<Ack, WireError> {
        let _kills = self.kills.fetch_add(1, Ordering::Relaxed);
        std::future::pending().await
    }

    async fn read_events(&self, _: &Context, request: ReadEvents) -> Result<EventPage, WireError> {
        let _reads = self.reads.fetch_add(1, Ordering::Relaxed);
        let start = request.from.get().max(self.oldest).min(STORED);
        let end = start.saturating_add(PAGE).min(STORED);
        Ok(EventPage {
            records: (start..end).map(record).collect(),
            next: EventSeq::new(end),
            at_end: end == STORED,
        })
    }

    async fn subscribe(
        &self,
        context: &Context,
        request: Subscribe,
    ) -> Result<Subscribed, WireError> {
        if request.session != session_id() || self.forgotten.load(Ordering::Relaxed) {
            return Err(WireError::new(ErrorCode::NotFound, "no such session"));
        }
        let first = self.subscribes.fetch_add(1, Ordering::Relaxed) == 0;
        let mut sink = context
            .outbound
            .open_subscription(request.from)
            .await
            .map_err(OutboundError::to_wire)?;
        let subscription = sink.id();
        let start = request.from.get().max(self.oldest);
        drop(tokio::spawn(async move {
            let (end, reason) = if first {
                let lag_at = start.saturating_add(LAG_AFTER);
                let resume_from = EventSeq::new(lag_at);
                (lag_at, SubscriptionEnd::Lagged { resume_from })
            } else {
                (LOG_LEN, SubscriptionEnd::SessionEnded)
            };
            for seq in start..end {
                if sink.send(Arc::new(record(seq))).await.is_err() {
                    return;
                }
            }
            sink.end(reason).await;
        }));
        Ok(Subscribed {
            subscription,
            info: info(),
        })
    }

    async fn unsubscribe(&self, context: &Context, request: Unsubscribe) -> Result<Ack, WireError> {
        context
            .outbound
            .close_subscription(request.subscription)
            .await
            .map_err(OutboundError::to_wire)?;
        Ok(Ack)
    }

    async fn stop_node(&self, _context: &Context, request: StopNode) -> Result<Ack, WireError> {
        *self.stopped.lock().unwrap() = Some(request.when);
        Ok(Ack)
    }

    async fn invite_device(
        &self,
        _context: &Context,
        _request: InviteDevice,
    ) -> Result<Invitation, WireError> {
        Err(WireError::new(
            ErrorCode::Unsupported,
            "no devices in this fake",
        ))
    }

    async fn device_self(
        &self,
        _context: &Self::Context,
        _request: vessel_proto::GetDeviceSelf,
    ) -> Result<vessel_proto::DeviceSelf, WireError> {
        Err(unsupported())
    }

    async fn list_devices(
        &self,
        _context: &Context,
        _request: ListDevices,
    ) -> Result<Vec<DeviceInfo>, WireError> {
        Ok(Vec::new())
    }

    async fn revoke_device(
        &self,
        _context: &Context,
        _request: RevokeDevice,
    ) -> Result<Ack, WireError> {
        Ok(Ack)
    }

    async fn list_profiles(
        &self,
        _context: &Context,
        _request: ListProfiles,
    ) -> Result<Vec<ProfileInfo>, WireError> {
        Err(unsupported())
    }

    async fn reload_profiles(
        &self,
        _context: &Context,
        _request: ReloadProfiles,
    ) -> Result<Vec<ProfileInfo>, WireError> {
        Err(unsupported())
    }

    async fn list_agents(
        &self,
        _context: &Context,
        _request: vessel_proto::agents::ListAgents,
    ) -> Result<Vec<vessel_proto::agents::AgentInfo>, WireError> {
        Err(unsupported())
    }

    async fn add_agent(
        &self,
        _context: &Context,
        _request: vessel_proto::agents::AddAgent,
    ) -> Result<vessel_proto::agents::AgentInfo, WireError> {
        Err(unsupported())
    }

    async fn remove_agent(
        &self,
        _context: &Context,
        _request: vessel_proto::agents::RemoveAgent,
    ) -> Result<Ack, WireError> {
        Err(unsupported())
    }

    async fn agent_status(
        &self,
        _context: &Context,
        _request: vessel_proto::agents::GetAgentStatus,
    ) -> Result<Vec<vessel_proto::agents::AgentReadiness>, WireError> {
        Err(unsupported())
    }

    async fn agent_catalogue(
        &self,
        _context: &Context,
        _request: vessel_proto::agents::GetCatalogue,
    ) -> Result<vessel_proto::agents::Catalogue, WireError> {
        Err(unsupported())
    }

    async fn install_agent(
        &self,
        _context: &Context,
        _request: vessel_proto::agents::InstallAgent,
    ) -> Result<vessel_proto::agents::InstallJob, WireError> {
        Err(unsupported())
    }

    async fn install_output(
        &self,
        _context: &Context,
        _request: vessel_proto::agents::ReadInstall,
    ) -> Result<vessel_proto::agents::InstallOutput, WireError> {
        Err(unsupported())
    }

    async fn move_profile(
        &self,
        _context: &Context,
        _request: MoveProfile,
    ) -> Result<SessionInfo, WireError> {
        Err(unsupported())
    }

    async fn get_manifest(
        &self,
        _context: &Context,
        _request: GetManifest,
    ) -> Result<Manifest, WireError> {
        Err(unsupported())
    }

    async fn read_blob(
        &self,
        _context: &Context,
        _request: ReadBlob,
    ) -> Result<BlobChunk, WireError> {
        Err(unsupported())
    }

    async fn watch_fleet(
        &self,
        _context: &Context,
        _request: WatchFleet,
    ) -> Result<Generation, WireError> {
        Err(unsupported())
    }

    async fn report_applied(
        &self,
        _context: &Context,
        _request: ReportApplied,
    ) -> Result<Ack, WireError> {
        Err(unsupported())
    }

    async fn list_members(
        &self,
        _context: &Context,
        _request: ListMembers,
    ) -> Result<Vec<FleetMember>, WireError> {
        Err(unsupported())
    }

    unserved! {
        get_registry: vessel_proto::registry::GetRegistry => vessel_proto::registry::RegistrySnapshot;
        list_workspaces: vessel_proto::registry::ListWorkspaces => Vec<vessel_proto::registry::WorkspaceDescriptor>;
        get_workspace: vessel_proto::registry::GetWorkspace => vessel_proto::registry::WorkspaceDescriptor;
        rename_workspace: vessel_proto::registry::RenameWorkspace => vessel_proto::registry::WorkspaceDescriptor;
        pin_workspace: vessel_proto::registry::PinWorkspace => vessel_proto::registry::WorkspaceDescriptor;
        release_workspace_automation: vessel_proto::registry::ReleaseWorkspaceAutomation => vessel_proto::registry::WorkspaceDescriptor;
        diagnostics: vessel_proto::diagnostics::GetDiagnostics => vessel_proto::diagnostics::DiagnosticReport;
        open_side_question: vessel_proto::side_questions::OpenSideQuestion => SessionInfo;
        prompt_side_question: vessel_proto::side_questions::PromptSideQuestion => vessel_proto::Delivered;
        reset_side_question: vessel_proto::side_questions::ResetSideQuestion => SessionInfo;
        forward: vessel_proto::Forward => vessel_proto::Forwarded;
        relay: vessel_proto::RelayCall => vessel_proto::Forwarded;
        deliver: vessel_proto::Deliver => vessel_proto::Forwarded;
        grant_home: vessel_proto::GrantHome => Ack;
        get_callback: vessel_proto::GetCallback => vessel_proto::Callback;
        forget_node: vessel_proto::ForgetNode => vessel_proto::NodeForgotten;
        list_peers: vessel_proto::ListPeers => Vec<vessel_proto::FleetNode>;
        list_nodes: vessel_proto::ListNodes => Vec<vessel_proto::FleetNode>;
        export_session: vessel_proto::ExportSession => vessel_proto::TransferManifest;
        import_session: vessel_proto::ImportSession => vessel_proto::SessionImported;
        export_memory: vessel_proto::ExportMemory => vessel_proto::TransferManifest;
        import_memory: vessel_proto::ImportMemory => vessel_proto::MemoryImported;
        read_transfer: vessel_proto::ReadTransfer => BlobChunk;
        close_transfer: vessel_proto::CloseTransfer => Ack;
        set_secret: vessel_proto::SetSecret => Ack;
        list_secrets: vessel_proto::ListSecrets => Vec<vessel_proto::SecretName>;
        delete_secret: vessel_proto::DeleteSecret => Ack;
        copy_secret: vessel_proto::CopySecret => Ack;
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
        list_models: vessel_proto::ListModels => Vec<vessel_proto::ModelRoute>;
        harness_options: vessel_proto::GetHarnessOptions => vessel_proto::HarnessOptions;
        session_options: vessel_proto::GetSessionOptions => Vec<vessel_proto::SessionOption>;
        set_session_option: vessel_proto::SetSessionOption => Vec<vessel_proto::SessionOption>;
        rename_session: vessel_proto::RenameSession => SessionInfo;
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
        git_compare: vessel_proto::workspace::CompareGit => vessel_proto::workspace::GitCompared<vessel_proto::workspace::DiffSummary>;
        git_compare_file: vessel_proto::workspace::CompareGitFile => vessel_proto::workspace::GitCompared<vessel_proto::workspace::FileDiff>;
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
        join_fleet: vessel_proto::JoinFleet => vessel_proto::JoinedFleet;
        leave_fleet: vessel_proto::LeaveFleet => Ack;
        sync_fleet: vessel_proto::SyncFleet => Ack;
        fleet_status: vessel_proto::GetFleetStatus => vessel_proto::FleetSyncStatus;
        fleet_published: vessel_proto::GetPublished => vessel_proto::FleetPublished;
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
            get_usage_limits: vessel_proto::limits::GetUsageLimits => vessel_proto::limits::UsageLimits;
        set_usage_lookup: vessel_proto::limits::SetUsageLookup => vessel_proto::limits::ExternalLookup;
    query_usage: vessel_proto::usage::QueryUsage => vessel_proto::usage::UsageReport;
        list_prompts: vessel_proto::automation::ListPrompts => vessel_proto::automation::PromptList;
        create_prompt: vessel_proto::automation::CreatePrompt => vessel_proto::automation::SavedPrompt;
        update_prompt: vessel_proto::automation::UpdatePrompt => vessel_proto::automation::SavedPrompt;
        delete_prompt: vessel_proto::automation::DeletePrompt => Ack;
        list_inbox: vessel_proto::inbox::ListInbox => vessel_proto::inbox::InboxPage;
        mark_inbox: vessel_proto::inbox::MarkInbox => Ack;
        get_inbox_entry: vessel_proto::inbox::GetInboxEntry => Option<vessel_proto::inbox::InboxEntry>;
        get_push_key: vessel_proto::push::GetPushKey => vessel_proto::push::PushKey;
        subscribe_push: vessel_proto::push::SubscribePush => vessel_proto::push::PushLease;
        unsubscribe_push: vessel_proto::push::UnsubscribePush => Ack;
        report_push_presence: vessel_proto::push::ReportPushPresence => Ack;
        search_messages: vessel_proto::search::SearchMessages => vessel_proto::search::SearchPage;
        park_session: vessel_proto::parking::ParkSession => SessionInfo;
        wake_session: vessel_proto::parking::WakeSession => SessionInfo;
        node_settings: vessel_proto::settings::GetNodeSettings => vessel_proto::settings::NodeSettings;
        set_node_settings: vessel_proto::settings::SetNodeSettings => vessel_proto::settings::NodeSettings;
        fork_session: vessel_proto::forking::ForkSession => SessionInfo;
        rewind_session: vessel_proto::forking::RewindSession => SessionInfo;
        agent_history: vessel_proto::history::GetAgentHistory => vessel_proto::history::AgentHistoryPage;
        detach_session: vessel_proto::subagents::DetachSession => vessel_proto::subagents::DetachedSession;
        list_provider_children: vessel_proto::subagents::ListProviderChildren => vessel_proto::subagents::ProviderChildren;
        read_provider_child: vessel_proto::subagents::ReadProviderChild => vessel_proto::subagents::ProviderChildPage;
    }
}

fn server(fake: Arc<FakeNode>) -> Arc<Server<FakeNode>> {
    let mut info = node();
    info.id = *fake.id.lock().unwrap();
    Arc::new(Server::new(
        fake,
        tokio::sync::watch::channel(info).1,
        ServerConfig::default(),
        CancellationToken::new(),
    ))
}

/// A client connected to `node` over the in-memory transport.
async fn connect(node: Arc<FakeNode>) -> NodeClient {
    let (client, _serving) = open(node).await.unwrap();
    client
}

/// A connection to `node` over the in-memory transport, and the task
/// serving it: aborting the task drops the node's end, as a lost network
/// would.
async fn open(
    node: Arc<FakeNode>,
) -> Result<(NodeClient, tokio::task::JoinHandle<()>), vessel_rpc::ConnectError<Disconnected>> {
    let (client_end, server_end) = vessel_rpc::memory::pair(NonZeroUsize::new(16).unwrap());
    let server = server(node);
    let serving = tokio::spawn(async move {
        let _served = server
            .serve(server_end, |peer| Context {
                outbound: peer.outbound,
            })
            .await;
    });
    let client = NodeClient::over(client_end, client_info(), &ClientConfig::default()).await?;
    Ok((client, serving))
}
