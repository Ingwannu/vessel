//! Which protocol version a call needs, past its method's: a field added
//! later makes the call need that later version.

use crate::agent::ContentBlock;
use crate::agents::HarnessRef;
use crate::node::ProtocolVersion;
use crate::session::{SessionInput, SessionMode};

use super::Call;

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
                let fields = if spec.place.is_some() {
                    // A place came in 10: an older node would start the
                    // session in its home folder instead.
                    Some(10)
                } else if chooses || spec.title.is_some() {
                    Some(9)
                } else {
                    spec.profile.as_ref().map(|_| 5)
                };
                // An added agent came in 11 (ADR 0032): an older node
                // cannot decode it.
                fields.max(added(Some(&spec.harness)))
            }
            Self::HarnessOptions(request) => added(Some(&request.harness)),
            Self::ListProfiles(request) => added(request.harness.as_ref()),
            // `scopes` came in 5; a scope newer than that needs its own
            // version, or an older node would refuse the name it lacks.
            Self::InviteDevice(request) => request.scopes.as_ref().map(|scopes| {
                scopes
                    .iter()
                    .map(|scope| scope.since().get())
                    .fold(5, u32::max)
            }),
            Self::ReadEvents(request) => request.wait.map(|_| 6),
            // `notify` came in 11: an older node would deliver the prompt
            // and never tell anyone.
            Self::PromptSession(request) => request.notify.then_some(11),
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
            // A write guarded by its content came in 11: a protocol 10 node
            // cannot decode it (its guard compared a time instead).
            Self::WriteFile(request) => match request.expect {
                crate::workspace::WriteExpect::Content { .. } => Some(11),
                crate::workspace::WriteExpect::Absent | crate::workspace::WriteExpect::Any => None,
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
            | Self::SessionOptions(_)
            | Self::SetSessionOption(_)
            | Self::RenameSession(_)
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
            | Self::CreateDir(_)
            | Self::RenamePath(_)
            | Self::DeletePath(_)
            | Self::FindFiles(_)
            | Self::SearchFiles(_)
            | Self::ForgeStatus(_)
            | Self::ListPullRequests(_)
            | Self::GetPullRequest(_)
            | Self::OpenPullRequest(_)
            | Self::MergePullRequest(_)
            | Self::SetFeedback(_)
            | Self::CreateTerminal(_)
            | Self::ListTerminals(_)
            | Self::GetTerminal(_)
            | Self::ReadTerminal(_)
            | Self::SubscribeTerminal(_)
            | Self::SendTerminal(_)
            | Self::RenameTerminal(_)
            | Self::KillTerminal(_)
            | Self::CloseTerminal(_)
            | Self::ListScripts(_)
            | Self::RunScript(_)
            | Self::RestartScript(_)
            | Self::ListPorts(_)
            | Self::ListAgents(_)
            | Self::AddAgent(_)
            | Self::RemoveAgent(_)
            | Self::GetAgentStatus(_)
            | Self::GetCatalogue(_)
            | Self::InstallAgent(_)
            | Self::ReadInstall(_)
            | Self::ListSchedules(_)
            | Self::GetSchedule(_)
            | Self::CreateSchedule(_)
            | Self::UpdateSchedule(_)
            | Self::PauseSchedule(_)
            | Self::ResumeSchedule(_)
            | Self::RunSchedule(_)
            | Self::DeleteSchedule(_)
            | Self::ListRuns(_)
            | Self::ListPrompts(_)
            | Self::CreatePrompt(_)
            | Self::UpdatePrompt(_)
            | Self::DeletePrompt(_)
            | Self::SearchMessages(_) => None,
        };
        fields.map_or(method, |since| method.max(ProtocolVersion::new(since)))
    }
}

/// The version a call naming `harness` needs for it: 11 for an added agent.
fn added(harness: Option<&HarnessRef>) -> Option<u32> {
    harness
        .filter(|harness| harness.added().is_some())
        .map(|harness| harness.since().get())
}
