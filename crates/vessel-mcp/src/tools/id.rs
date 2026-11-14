//! The MCP tool name, decoded once from the text a client sends.
//!
//! The name is external text. [`decode`] is that edge: [`ToolId::parse`]
//! matches a known name, and a `vessel_lsp_` prefix that is not one of
//! those names stays the LSP module's to refuse, as the old prefix arm
//! did. After that the server matches the enum.

/// One tool this build advertises.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolId {
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
}

impl ToolId {
    /// `name` as a tool this build has.
    ///
    /// `None` is an unknown name. It is refused, not ignored.
    pub(super) fn parse(name: &str) -> Option<Self> {
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
            _ => return None,
        })
    }

    /// The name [`parse`](Self::parse) accepts.
    ///
    /// Modules that still match the text take this, until that match moves
    /// onto the enum. It is not a second decode.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
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

/// A client's tool name, once decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Decoded {
    /// One tool this build advertises.
    Tool(ToolId),
    /// `vessel_lsp_` and then not one of the ten.
    LspPrefix,
    /// Not a tool.
    Unknown,
}

/// Decode `name`. This, and [`ToolId::parse`], are the only reads of it.
pub(super) fn decode(name: &str) -> Decoded {
    if let Some(tool) = ToolId::parse(name) {
        return Decoded::Tool(tool);
    }
    if name.starts_with("vessel_lsp_") {
        return Decoded::LspPrefix;
    }
    Decoded::Unknown
}

#[cfg(test)]
mod tests {
    use super::{Decoded, ToolId, decode};

    #[test]
    fn every_tool_parses_back_to_itself_and_an_unknown_name_does_not() {
        let all = [
            ToolId::ListNodes,
            ToolId::ListSessions,
            ToolId::CreateSession,
            ToolId::SendInput,
            ToolId::AnswerPermission,
            ToolId::ReadEvents,
            ToolId::WaitTurn,
            ToolId::KillSession,
            ToolId::ReadConversation,
            ToolId::ChangeQueue,
            ToolId::AnswerQuestion,
            ToolId::ResumeSession,
            ToolId::ArchiveSession,
            ToolId::SearchSessions,
            ToolId::ForkSession,
            ToolId::RewindSession,
            ToolId::ListProjects,
            ToolId::ListWorktrees,
            ToolId::CreateWorktree,
            ToolId::GetPullRequest,
            ToolId::OpenPullRequest,
            ToolId::CreateTerminal,
            ToolId::ListTerminals,
            ToolId::ReadTerminal,
            ToolId::WriteTerminal,
            ToolId::CloseTerminal,
            ToolId::ListPorts,
            ToolId::CreateSchedule,
            ToolId::CreateHeartbeat,
            ToolId::ListSchedules,
            ToolId::ScheduleRuns,
            ToolId::PauseSchedule,
            ToolId::ResumeSchedule,
            ToolId::DeleteSchedule,
            ToolId::ListSavedPrompts,
            ToolId::LspWorkspaceSymbols,
            ToolId::LspDocumentSymbols,
            ToolId::LspDefinition,
            ToolId::LspReferences,
            ToolId::LspHover,
            ToolId::LspImplementations,
            ToolId::LspTypeDefinition,
            ToolId::LspIncomingCalls,
            ToolId::LspOutgoingCalls,
            ToolId::LspDiagnostics,
        ];
        for tool in all {
            assert_eq!(ToolId::parse(tool.as_str()), Some(tool));
        }
        assert_eq!(ToolId::parse("speak"), None);
        assert_eq!(ToolId::parse("vessel_lsp_"), None);
    }

    #[test]
    fn a_vessel_lsp_prefix_that_is_not_a_tool_stays_with_the_lsp_module() {
        assert_eq!(
            decode("vessel_lsp_"),
            Decoded::LspPrefix,
            "the old prefix arm refused this inside the LSP module"
        );
        assert_eq!(decode("vessel_lsp_nope"), Decoded::LspPrefix);
        assert_eq!(
            decode("vessel_lsp_diagnostics"),
            Decoded::Tool(ToolId::LspDiagnostics)
        );
        assert_eq!(decode("speak"), Decoded::Unknown);
    }
}
