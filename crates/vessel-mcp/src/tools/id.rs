//! The MCP tool name, decoded once from the text a client sends.
//!
//! The name is external text. [`decode`] is that edge: [`ToolId::parse`]
//! matches a known name, and a `vessel_lsp_` prefix that is not one of
//! those names stays the LSP module's to refuse, as the old prefix arm
//! did. After that the server matches the enum.

pub(super) use vessel_proto::mcp::{AddedId, ToolId};

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
        for tool in every() {
            assert_eq!(ToolId::parse(tool.as_str()), Some(tool));
        }
        assert_eq!(ToolId::parse("speak"), None);
        assert_eq!(ToolId::parse("vessel_lsp_"), None);
    }

    fn every() -> Vec<ToolId> {
        vec![
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
            ToolId::Added(super::AddedId::GetSession),
            ToolId::Added(super::AddedId::CancelTurn),
            ToolId::Added(super::AddedId::RenameSession),
            ToolId::Added(super::AddedId::SetSessionOption),
            ToolId::Added(super::AddedId::GetSchedule),
            ToolId::Added(super::AddedId::UpdateSchedule),
            ToolId::Added(super::AddedId::ListHarnesses),
            ToolId::Added(super::AddedId::ListProfiles),
            ToolId::Added(super::AddedId::HarnessOptions),
        ]
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
