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
            // Every hop decodes the typed inner call. It must know its
            // method and fields too; an older hop could otherwise drop a
            // newer field before the target sees it (ADR 0044), as a
            // protocol-18 home would drop `session.create`'s
            // `side_question_of` and start a plain session (ADR 0045; N5,
            // N3 and N2 each added this rule, merged here as one).
            Self::Forward(request) => Some(request.call.since().get()),
            Self::Relay(request) => Some(request.call.since().get()),
            Self::Deliver(request) => Some(request.call.since().get()),
            Self::JoinFleet(request) => request.grant.iter().map(|scope| scope.since().get()).max(),
            // Options chosen at creation and a title came in 9: an older
            // node would start the session in the agent's default mode (a
            // permissions mode the caller did not ask for) and unnamed.
            Self::CreateSession(request) => {
                let spec = &request.spec;
                let chooses = match &spec.mode {
                    SessionMode::Structured { options, .. } => !options.is_empty(),
                    SessionMode::Terminal { .. } => false,
                };
                let fields = if spec.side_question_of.is_some() {
                    Some(19)
                } else if spec.place.is_some() {
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
                fields
                    .max(added(Some(&spec.harness)))
                    .max((!spec.labels.is_empty()).then_some(crate::lifecycle_version::SINCE))
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
            // Drivers came in 18 (ADR 0057): an older node would keep
            // running the agent over ACP and say nothing.
            Self::SetNodeSettings(request) => request.drivers.map(|_| 18),
            Self::WriteFile(request) => match request.expect {
                crate::workspace::WriteExpect::Content { .. } => Some(11),
                crate::workspace::WriteExpect::Absent | crate::workspace::WriteExpect::Any => None,
            },
            Self::CreatePrompt(request) => template_since(request.session.as_deref()),
            Self::UpdatePrompt(request) => template_since(request.session.as_deref()),
            Self::CreateSchedule(request) => target_since(&request.definition.target),
            Self::UpdateSchedule(request) => target_since(&request.definition.target),
            Self::OpenSideQuestion(_)
            | Self::PromptSideQuestion(_)
            | Self::ResetSideQuestion(_)
            | Self::NodeInfo(_)
            | Self::ListHarnesses(_)
            | Self::ListSessions(_)
            | Self::GetSession(_)
            | Self::KillSession(_)
            | Self::Subscribe(_)
            | Self::Unsubscribe(_)
            | Self::StopNode(_)
            | Self::GetDeviceSelf(_)
            | Self::ListDevices(_)
            | Self::RevokeDevice(_)
            | Self::ReloadProfiles(_)
            | Self::MoveProfile(_)
            | Self::GetManifest(_)
            | Self::ReadBlob(_)
            | Self::WatchFleet(_)
            | Self::ReportApplied(_)
            | Self::ListMembers(_)
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
            | Self::LeaveFleet(_)
            | Self::SyncFleet(_)
            | Self::GetFleetStatus(_)
            | Self::GetPublished(_)
            | Self::PreviewSchedule(_)
            | Self::ListSchedules(_)
            | Self::GetSchedule(_)
            | Self::PauseSchedule(_)
            | Self::ResumeSchedule(_)
            | Self::RunSchedule(_)
            | Self::DeleteSchedule(_)
            | Self::ListRuns(_)
            | Self::QueryUsage(_)
            | Self::ListPrompts(_)
            | Self::DeletePrompt(_)
            | Self::SearchMessages(_)
            | Self::ParkSession(_)
            | Self::WakeSession(_)
            | Self::GetNodeSettings(_)
            | Self::ForkSession(_)
            | Self::RewindSession(_)
            | Self::GetAgentHistory(_) => None,
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

fn template_since(spec: Option<&crate::SessionSpec>) -> Option<u32> {
    spec.and_then(|spec| {
        spec.side_question_of
            .map(|_| 19)
            .max((!spec.labels.is_empty()).then_some(crate::lifecycle_version::SINCE))
    })
}

fn target_since(target: &crate::automation::ScheduleTarget) -> Option<u32> {
    match target {
        crate::automation::ScheduleTarget::NewSession { spec, .. } => template_since(Some(spec)),
        crate::automation::ScheduleTarget::Session { .. }
        | crate::automation::ScheduleTarget::Unknown => None,
    }
}
