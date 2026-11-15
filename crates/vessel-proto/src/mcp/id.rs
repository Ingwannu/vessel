//! The tool identifiers shared by launch policy and MCP dispatch.
//! String decoding belongs here so policy cannot maintain a second list.

/// One tool this build advertises.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolId {
    /// Browser tools use the browser producer's one closed name catalogue.
    Browser(crate::browser::BrowserTool),
    /// `list_nodes`.
    ListNodes,
    /// `list_sessions`.
    ListSessions,
    /// `create_session`.
    CreateSession,
    /// `send_input`.
    SendInput,
    /// `answer_permission`.
    AnswerPermission,
    /// `read_events`.
    ReadEvents,
    /// `wait_turn`.
    WaitTurn,
    /// `kill_session`.
    KillSession,
    /// `read_conversation`.
    ReadConversation,
    /// `change_queue`.
    ChangeQueue,
    /// `answer_question`.
    AnswerQuestion,
    /// `resume_session`.
    ResumeSession,
    /// `archive_session`.
    ArchiveSession,
    /// `search_sessions`.
    SearchSessions,
    /// `fork_session`.
    ForkSession,
    /// `rewind_session`.
    RewindSession,
    /// `list_projects`.
    ListProjects,
    /// `list_worktrees`.
    ListWorktrees,
    /// `create_worktree`.
    CreateWorktree,
    /// `get_pull_request`.
    GetPullRequest,
    /// `open_pull_request`.
    OpenPullRequest,
    /// `create_terminal`.
    CreateTerminal,
    /// `list_terminals`.
    ListTerminals,
    /// `read_terminal`.
    ReadTerminal,
    /// `write_terminal`.
    WriteTerminal,
    /// `close_terminal`.
    CloseTerminal,
    /// `list_ports`.
    ListPorts,
    /// `create_schedule`.
    CreateSchedule,
    /// `create_heartbeat`.
    CreateHeartbeat,
    /// `list_schedules`.
    ListSchedules,
    /// `schedule_runs`.
    ScheduleRuns,
    /// `pause_schedule`.
    PauseSchedule,
    /// `resume_schedule`.
    ResumeSchedule,
    /// `delete_schedule`.
    DeleteSchedule,
    /// `list_saved_prompts`.
    ListSavedPrompts,
    /// `vessel_lsp_workspace_symbols`.
    LspWorkspaceSymbols,
    /// `vessel_lsp_document_symbols`.
    LspDocumentSymbols,
    /// `vessel_lsp_definition`.
    LspDefinition,
    /// `vessel_lsp_references`.
    LspReferences,
    /// `vessel_lsp_hover`.
    LspHover,
    /// `vessel_lsp_implementations`.
    LspImplementations,
    /// `vessel_lsp_type_definition`.
    LspTypeDefinition,
    /// `vessel_lsp_incoming_calls`.
    LspIncomingCalls,
    /// `vessel_lsp_outgoing_calls`.
    LspOutgoingCalls,
    /// `vessel_lsp_diagnostics`.
    LspDiagnostics,
    /// A tool added for MCP parity. Nested so the catalogue match grows
    /// by one arm, not one arm per tool.
    Added(AddedId),
}

/// A tool this build adds on top of the catalogue it already had.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddedId {
    /// `get_session`.
    GetSession,
    /// `cancel_turn`.
    CancelTurn,
    /// `rename_session`.
    RenameSession,
    /// `set_session_option`.
    SetSessionOption,
    /// `get_schedule`.
    GetSchedule,
    /// `update_schedule`.
    UpdateSchedule,
    /// `list_harnesses`.
    ListHarnesses,
    /// `list_profiles`.
    ListProfiles,
    /// `harness_options`.
    HarnessOptions,
    /// `list_pending_permissions`.
    ListPendingPermissions,
}

impl AddedId {
    /// The name [`ToolId::parse`] accepts for this tool.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GetSession => "get_session",
            Self::CancelTurn => "cancel_turn",
            Self::RenameSession => "rename_session",
            Self::SetSessionOption => "set_session_option",
            Self::GetSchedule => "get_schedule",
            Self::UpdateSchedule => "update_schedule",
            Self::ListHarnesses => "list_harnesses",
            Self::ListProfiles => "list_profiles",
            Self::HarnessOptions => "harness_options",
            Self::ListPendingPermissions => "list_pending_permissions",
        }
    }
}

impl ToolId {
    /// `name` as a tool this build has.
    ///
    /// `None` is an unknown name. It is refused, not ignored.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "list_nodes" => Self::ListNodes,
            "list_sessions" => Self::ListSessions,
            "create_session" => Self::CreateSession,
            "send_input" => Self::SendInput,
            "answer_permission" => Self::AnswerPermission,
            "read_events" => Self::ReadEvents,
            "wait_turn" => Self::WaitTurn,
            "kill_session" => Self::KillSession,
            "read_conversation" => Self::ReadConversation,
            "change_queue" => Self::ChangeQueue,
            "answer_question" => Self::AnswerQuestion,
            "resume_session" => Self::ResumeSession,
            "archive_session" => Self::ArchiveSession,
            "search_sessions" => Self::SearchSessions,
            "fork_session" => Self::ForkSession,
            "rewind_session" => Self::RewindSession,
            "list_projects" => Self::ListProjects,
            "list_worktrees" => Self::ListWorktrees,
            "create_worktree" => Self::CreateWorktree,
            "get_pull_request" => Self::GetPullRequest,
            "open_pull_request" => Self::OpenPullRequest,
            "create_terminal" => Self::CreateTerminal,
            "list_terminals" => Self::ListTerminals,
            "read_terminal" => Self::ReadTerminal,
            "write_terminal" => Self::WriteTerminal,
            "close_terminal" => Self::CloseTerminal,
            "list_ports" => Self::ListPorts,
            "create_schedule" => Self::CreateSchedule,
            "create_heartbeat" => Self::CreateHeartbeat,
            "list_schedules" => Self::ListSchedules,
            "schedule_runs" => Self::ScheduleRuns,
            "pause_schedule" => Self::PauseSchedule,
            "resume_schedule" => Self::ResumeSchedule,
            "delete_schedule" => Self::DeleteSchedule,
            "list_saved_prompts" => Self::ListSavedPrompts,
            "get_session" => Self::Added(AddedId::GetSession),
            "cancel_turn" => Self::Added(AddedId::CancelTurn),
            "rename_session" => Self::Added(AddedId::RenameSession),
            "set_session_option" => Self::Added(AddedId::SetSessionOption),
            "get_schedule" => Self::Added(AddedId::GetSchedule),
            "update_schedule" => Self::Added(AddedId::UpdateSchedule),
            "list_harnesses" => Self::Added(AddedId::ListHarnesses),
            "list_profiles" => Self::Added(AddedId::ListProfiles),
            "harness_options" => Self::Added(AddedId::HarnessOptions),
            "list_pending_permissions" => Self::Added(AddedId::ListPendingPermissions),
            "vessel_lsp_workspace_symbols" => Self::LspWorkspaceSymbols,
            "vessel_lsp_document_symbols" => Self::LspDocumentSymbols,
            "vessel_lsp_definition" => Self::LspDefinition,
            "vessel_lsp_references" => Self::LspReferences,
            "vessel_lsp_hover" => Self::LspHover,
            "vessel_lsp_implementations" => Self::LspImplementations,
            "vessel_lsp_type_definition" => Self::LspTypeDefinition,
            "vessel_lsp_incoming_calls" => Self::LspIncomingCalls,
            "vessel_lsp_outgoing_calls" => Self::LspOutgoingCalls,
            "vessel_lsp_diagnostics" => Self::LspDiagnostics,
            _ => return crate::browser::BrowserTool::from_name(name).map(Self::Browser),
        })
    }

    /// The name [`parse`](Self::parse) accepts.
    ///
    /// Modules that still match the text take this, until that match moves
    /// onto the enum. It is not a second decode.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Browser(tool) => tool.name(),
            Self::ListNodes => "list_nodes",
            Self::ListSessions => "list_sessions",
            Self::CreateSession => "create_session",
            Self::SendInput => "send_input",
            Self::AnswerPermission => "answer_permission",
            Self::ReadEvents => "read_events",
            Self::WaitTurn => "wait_turn",
            Self::KillSession => "kill_session",
            Self::ReadConversation => "read_conversation",
            Self::ChangeQueue => "change_queue",
            Self::AnswerQuestion => "answer_question",
            Self::ResumeSession => "resume_session",
            Self::ArchiveSession => "archive_session",
            Self::SearchSessions => "search_sessions",
            Self::ForkSession => "fork_session",
            Self::RewindSession => "rewind_session",
            Self::ListProjects => "list_projects",
            Self::ListWorktrees => "list_worktrees",
            Self::CreateWorktree => "create_worktree",
            Self::GetPullRequest => "get_pull_request",
            Self::OpenPullRequest => "open_pull_request",
            Self::CreateTerminal => "create_terminal",
            Self::ListTerminals => "list_terminals",
            Self::ReadTerminal => "read_terminal",
            Self::WriteTerminal => "write_terminal",
            Self::CloseTerminal => "close_terminal",
            Self::ListPorts => "list_ports",
            Self::CreateSchedule => "create_schedule",
            Self::CreateHeartbeat => "create_heartbeat",
            Self::ListSchedules => "list_schedules",
            Self::ScheduleRuns => "schedule_runs",
            Self::PauseSchedule => "pause_schedule",
            Self::ResumeSchedule => "resume_schedule",
            Self::DeleteSchedule => "delete_schedule",
            Self::ListSavedPrompts => "list_saved_prompts",
            Self::Added(tool) => tool.as_str(),
            Self::LspWorkspaceSymbols => "vessel_lsp_workspace_symbols",
            Self::LspDocumentSymbols => "vessel_lsp_document_symbols",
            Self::LspDefinition => "vessel_lsp_definition",
            Self::LspReferences => "vessel_lsp_references",
            Self::LspHover => "vessel_lsp_hover",
            Self::LspImplementations => "vessel_lsp_implementations",
            Self::LspTypeDefinition => "vessel_lsp_type_definition",
            Self::LspIncomingCalls => "vessel_lsp_incoming_calls",
            Self::LspOutgoingCalls => "vessel_lsp_outgoing_calls",
            Self::LspDiagnostics => "vessel_lsp_diagnostics",
        }
    }
}
