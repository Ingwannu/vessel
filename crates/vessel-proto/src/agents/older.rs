//! What a peer is handed where a reply would carry an agent newer than its
//! protocol: an added agent (protocol 11, ADR 0032) for a peer of 2 to 10,
//! one of the seven known agents of ADR 0058 for a peer older than
//! [`READS_TEN_AGENTS`].
//!
//! [Decision Log]
//! - Purpose: an older peer decodes an agent as one of the kinds it knows,
//!   so a list holding `{"added": ...}` or `"grok"` would fail to decode
//!   whole and drop its connection.
//! - Constraint: there is no honest stand-in. Reporting such a session as
//!   one of the agents the peer knows would show a person the wrong agent,
//!   and a client acting on it (moving it to a Claude profile, say) would
//!   act on a guess.
//! - Choice: lists (`harness.list`, `profile.list`, `profile.reload`,
//!   `session.list`, and from protocol 11 `agent.list`, `agent.status`,
//!   `agent.catalogue`, `schedule.list`, `prompt.list`, and from protocol
//!   21 `usage.query`'s agent groups) leave such entries
//!   out (`lists`); a reply that is about one (`session.get`, a session's
//!   change, a subscription, one schedule or saved prompt) is withheld and
//!   the peer gets `unsupported`, saying which protocol sees it. The calls
//!   that would change such a session or subscribe to it are refused by
//!   the node before they run (`vessel-node`'s guard), so withholding a
//!   reply never hides a change that happened. A schedule or saved prompt
//!   is named by an id the peer only learns from a list that leaves it
//!   out.
//! - One rule for both: an agent is readable from [`HarnessRef::since`] on.
//! - Both walk into the fleet's envelope, whose reply nests the inner
//!   call's.

mod lists;

use crate::agents::HarnessRef;
use crate::automation::{SavedPrompt, ScheduleInfo, ScheduleTarget};
use crate::error::{ErrorCode, WireError};
use crate::forward::Forwarded;
use crate::node::ProtocolVersion;
use crate::rpc::Reply;
use crate::session::{SessionInfo, SessionSpec};

pub(crate) use lists::{
    agents, catalogue, harnesses, profiles, prompts, readiness, schedules, usage,
};

/// The first protocol whose peers read an added agent.
pub const READS_ADDED_AGENTS: ProtocolVersion = ProtocolVersion::new(11);

/// The first protocol whose peers read the seven known agents of ADR 0058
/// (Grok, Pi, OMP, Antigravity, Muse Code, Amp, Cursor). Provisional: the
/// number is confirmed when the line merges.
pub const READS_TEN_AGENTS: ProtocolVersion = ProtocolVersion::new(22);

/// The first protocol whose peers read every agent: nothing is withheld
/// from them.
pub const READS_EVERY_AGENT: ProtocolVersion = READS_TEN_AGENTS;

/// Whether a peer of `protocol` can read `harness`.
#[must_use]
pub const fn reads(harness: &HarnessRef, protocol: ProtocolVersion) -> bool {
    harness.since().get() <= protocol.get()
}

/// Whether a peer of `protocol` can be handed `session`.
#[must_use]
pub const fn readable(session: &SessionInfo, protocol: ProtocolVersion) -> bool {
    reads(&session.spec.harness, protocol)
}

/// The error a peer of an older protocol gets in place of a session, a
/// schedule or a saved prompt running `harness`.
#[must_use]
pub fn unreadable(harness: &HarnessRef) -> WireError {
    let message = match harness {
        HarnessRef::Added(_) => format!(
            "this runs an agent added on this machine; a client of protocol {READS_ADDED_AGENTS} or later sees it"
        ),
        HarnessRef::Known(kind) => format!(
            "this runs {kind}, which a client of protocol {} or later knows",
            kind.since()
        ),
    };
    WireError::new(ErrorCode::Unsupported, message)
}

/// `spec`'s agent, when a peer of `protocol` cannot read it.
fn newer(spec: &SessionSpec, protocol: ProtocolVersion) -> Option<WireError> {
    (!reads(&spec.harness, protocol)).then(|| unreadable(&spec.harness))
}

/// A schedule's new session, when a peer of `protocol` cannot read its
/// agent.
fn newer_schedule(schedule: &ScheduleInfo, protocol: ProtocolVersion) -> Option<WireError> {
    match &schedule.definition.target {
        ScheduleTarget::NewSession { spec, .. } => newer(spec, protocol),
        ScheduleTarget::Session { .. } | ScheduleTarget::Unknown => None,
    }
}

/// A saved prompt's session, when a peer of `protocol` cannot read its
/// agent.
fn newer_prompt(prompt: &SavedPrompt, protocol: ProtocolVersion) -> Option<WireError> {
    prompt
        .session
        .as_deref()
        .and_then(|spec| newer(spec, protocol))
}

impl Reply {
    /// The error a peer of `protocol` gets in place of this reply, when the
    /// reply is about one session, schedule or saved prompt whose agent it
    /// could not decode; `None` when it may be handed (lists are filtered
    /// by [`Reply::for_protocol`] instead).
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per reply, so a new reply must decide whether it can carry an agent \
                  newer than a peer; the list is the point"
    )]
    pub fn withheld(&self, protocol: ProtocolVersion) -> Option<WireError> {
        if protocol >= READS_EVERY_AGENT {
            return None;
        }
        let one = |session: &SessionInfo| newer(&session.spec, protocol);
        match self {
            Self::GetSession(info)
            | Self::OpenSideQuestion(info)
            | Self::ResetSideQuestion(info)
            | Self::CreateSession(info)
            | Self::MoveProfile(info)
            | Self::RenameSession(info)
            | Self::ResumeSession(info)
            | Self::MarkSession(info)
            | Self::ParkSession(info)
            | Self::WakeSession(info)
            | Self::ForkSession(info)
            | Self::RewindSession(info) => one(info),
            Self::Subscribe(subscribed) => one(&subscribed.info),
            Self::ImportSession(imported) => imported.session.as_deref().and_then(one),
            Self::GetSchedule(schedule)
            | Self::CreateSchedule(schedule)
            | Self::UpdateSchedule(schedule)
            | Self::PauseSchedule(schedule)
            | Self::ResumeSchedule(schedule) => newer_schedule(schedule, protocol),
            Self::CreatePrompt(prompt) | Self::UpdatePrompt(prompt) => {
                newer_prompt(prompt, protocol)
            }
            Self::Forward(forwarded) | Self::Relay(forwarded) | Self::Deliver(forwarded) => {
                nested(forwarded, protocol)
            }
            // Lists (filtered by `for_protocol`), and replies that name no
            // agent or only one the asking call named (which needs the
            // agent's protocol, `Call::since`).
            Self::ListSessions(_)
            | Self::ListHarnesses(_)
            | Self::ListProfiles(_)
            | Self::ReloadProfiles(_)
            | Self::NodeInfo(_)
            | Self::GetNodeSettings(_)
            | Self::SetNodeSettings(_)
            | Self::GetAgentHistory(_)
            | Self::SendInput(_)
            | Self::KillSession(_)
            | Self::ReadEvents(_)
            | Self::Unsubscribe(_)
            | Self::StopNode(_)
            | Self::InviteDevice(_)
            | Self::GetDeviceSelf(_)
            | Self::ListDevices(_)
            | Self::RevokeDevice(_)
            | Self::GetManifest(_)
            | Self::ReadBlob(_)
            | Self::WatchFleet(_)
            | Self::ReportApplied(_)
            | Self::ListMembers(_)
            | Self::GrantHome(_)
            | Self::GetCallback(_)
            | Self::ListPeers(_)
            | Self::ListNodes(_)
            | Self::ForgetNode(_)
            | Self::ExportSession(_)
            | Self::ExportMemory(_)
            | Self::ImportMemory(_)
            | Self::ReadTransfer(_)
            | Self::CloseTransfer(_)
            | Self::SetSecret(_)
            | Self::ListSecrets(_)
            | Self::DeleteSecret(_)
            | Self::CopySecret(_)
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
            | Self::ListModels(_)
            | Self::HarnessOptions(_)
            | Self::SessionOptions(_)
            | Self::SetSessionOption(_)
            | Self::PromptSession(_)
            | Self::ChangeQueue(_)
            | Self::GetConversation(_)
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
            | Self::SearchFiles(_)
            // Protocol 11's pull requests, terminals, scripts and ports: a
            // pull request names its session by id, a terminal the session
            // it is beside by id; none carries a session's agent.
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
            | Self::PromptSideQuestion(_)
            | Self::ReadInstall(_)
            // Protocol 11's fleet calls, and the schedules' runs and
            // previews: none carries an agent. A run names its session by
            // id.
            | Self::JoinFleet(_)
            | Self::LeaveFleet(_)
            | Self::SyncFleet(_)
            | Self::GetFleetStatus(_)
            | Self::GetPublished(_)
            | Self::PreviewSchedule(_)
            | Self::ListSchedules(_)
            | Self::RunSchedule(_)
            | Self::DeleteSchedule(_)
            | Self::ListRuns(_)
            | Self::QueryUsage(_)
            | Self::ListPrompts(_)
            | Self::DeletePrompt(_)
            // A hit names a session; the session itself comes from a list.
            | Self::SearchMessages(_) => None,
        }
    }
}

/// The envelope's inner reply, withheld.
fn nested(forwarded: &Forwarded, protocol: ProtocolVersion) -> Option<WireError> {
    forwarded.reply.withheld(protocol)
}

#[cfg(test)]
mod tests;
