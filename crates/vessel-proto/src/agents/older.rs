//! What a peer of protocol 2 to 10 is handed where a reply would carry an
//! added agent (protocol 11, ADR 0032).
//!
//! [Decision Log]
//! - Purpose: an older peer decodes an agent as one of four strings, so a
//!   list holding `{"added": ...}` would fail to decode whole and drop its
//!   connection.
//! - Constraint: there is no honest stand-in. Reporting an added agent's
//!   session as one of the four would show a person the wrong agent, and a
//!   client acting on it (moving it to a Claude profile, say) would act on
//!   a guess.
//! - Choice: lists (`harness.list`, `profile.list`, `profile.reload`,
//!   `session.list`) leave such entries out; a reply that is about one
//!   (`session.get`, a session's change, a subscription) is withheld and
//!   the peer gets `unsupported`, saying a newer client sees it. The calls
//!   that would change such a session or subscribe to it are refused by
//!   the node before they run (`vessel-node`'s guard), so withholding a
//!   reply never hides a change that happened.
//! - Both walk into the fleet's envelope, whose reply nests the inner
//!   call's.

use crate::error::{ErrorCode, WireError};
use crate::forward::Forwarded;
use crate::harness::Harness;
use crate::node::ProtocolVersion;
use crate::profile::ProfileInfo;
use crate::rpc::Reply;
use crate::session::SessionInfo;

/// The first protocol whose peers read an added agent.
pub const READS_ADDED_AGENTS: ProtocolVersion = ProtocolVersion::new(11);

/// Whether a peer of `protocol` can be handed `session`.
#[must_use]
pub fn readable(session: &SessionInfo, protocol: ProtocolVersion) -> bool {
    protocol >= READS_ADDED_AGENTS || session.spec.harness.added().is_none()
}

/// The error a peer of an older protocol gets in place of an added agent's
/// session.
#[must_use]
pub fn unreadable() -> WireError {
    WireError::new(
        ErrorCode::Unsupported,
        "this session runs an agent added on this machine; a client of protocol 11 or later sees it",
    )
}

/// `harnesses` without the added agents, when a peer of `protocol` cannot
/// read them; `None` when nothing changes.
pub(crate) fn harnesses(harnesses: &[Harness], protocol: ProtocolVersion) -> Option<Vec<Harness>> {
    let hidden = |harness: &Harness| harness.kind.added().is_some();
    (protocol < READS_ADDED_AGENTS && harnesses.iter().any(hidden)).then(|| {
        harnesses
            .iter()
            .filter(|harness| !hidden(harness))
            .cloned()
            .collect()
    })
}

/// `profiles` without the added agents' ones, when a peer of `protocol`
/// cannot read them; `None` when nothing changes.
pub(crate) fn profiles(
    profiles: &[ProfileInfo],
    protocol: ProtocolVersion,
) -> Option<Vec<ProfileInfo>> {
    let hidden = |profile: &ProfileInfo| profile.harness.added().is_some();
    (protocol < READS_ADDED_AGENTS && profiles.iter().any(hidden)).then(|| {
        profiles
            .iter()
            .filter(|profile| !hidden(profile))
            .cloned()
            .collect()
    })
}

impl Reply {
    /// The error a peer of `protocol` gets in place of this reply, when the
    /// reply is about one added agent's session it could not decode; `None`
    /// when it may be handed (lists are filtered by
    /// [`Reply::for_protocol`] instead).
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per reply, so a new reply must decide whether it can carry an added \
                  agent; the list is the point"
    )]
    pub fn withheld(&self, protocol: ProtocolVersion) -> Option<WireError> {
        if protocol >= READS_ADDED_AGENTS {
            return None;
        }
        let one = |session: &SessionInfo| (!readable(session, protocol)).then(unreadable);
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
            Self::DetachSession(detached) => one(&detached.session),
            Self::ImportSession(imported) => imported.session.as_deref().and_then(one),
            Self::Forward(forwarded) | Self::Relay(forwarded) | Self::Deliver(forwarded) => {
                nested(forwarded, protocol)
            }
            // Lists (filtered by `for_protocol`), and replies that name no
            // agent or only one of the four.
            Self::ListSessions(_)
            | Self::ListHarnesses(_)
            | Self::ListProfiles(_)
            | Self::ReloadProfiles(_)
            | Self::NodeInfo(_)
            | Self::CheckPromptTracking(_)
            | Self::TrackPrompt(_)
            | Self::ReadPromptReceipt(_)
            | Self::ClosePromptReceipt(_)
            | Self::GetSkillsStatus(_)
            | Self::ReconcileSkills(_)
            | Self::UninstallSkills(_)
            | Self::SaveSkillsSelection(_)
            | Self::ImportSkillsSelection(_)
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
            | Self::GetRegistry(_)
            | Self::ListWorkspaces(_)
            | Self::GetWorkspace(_)
            | Self::RenameWorkspace(_)
            | Self::PinWorkspace(_)
            | Self::ReleaseWorkspaceAutomation(_)
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
            | Self::GitCompare(_)
            | Self::GitCompareFile(_)
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
            // Protocol 11's schedules and saved prompts: a schedule's new
            // session and a saved prompt carry a `SessionSpec`, whose agent
            // may be an added one, but every one of these is a protocol 11
            // method, which a node refuses on an older connection before it
            // runs (the client does not send it either), so an older peer
            // is never handed one. A run names its session by id.
            | Self::JoinFleet(_)
            | Self::LeaveFleet(_)
            | Self::SyncFleet(_)
            | Self::GetFleetStatus(_)
            | Self::GetPublished(_)
            | Self::PreviewSchedule(_)
            | Self::ListSchedules(_)
            | Self::GetSchedule(_)
            | Self::CreateSchedule(_)
            | Self::UpdateSchedule(_)
            | Self::PauseSchedule(_)
            | Self::ResumeSchedule(_)
            | Self::RunSchedule(_)
            | Self::DeleteSchedule(_)
            | Self::ListRuns(_)
            | Self::GetUsageLimits(_)
            | Self::SetUsageLookup(_)
            | Self::QueryUsage(_)
            | Self::ListPrompts(_)
            | Self::CreatePrompt(_)
            | Self::UpdatePrompt(_)
            | Self::DeletePrompt(_)
            | Self::ListInbox(_)
            | Self::GetInboxEntry(_)
            | Self::MarkInbox(_)
            | Self::GetPushKey(_)
            | Self::SubscribePush(_)
            | Self::UnsubscribePush(_)
            | Self::ReportPushPresence(_)
            // A hit names a session; the session itself comes from a list.
            | Self::Diagnostics(_)
            | Self::SearchMessages(_) => None,
        }
    }
}

/// The envelope's inner reply, withheld.
fn nested(forwarded: &Forwarded, protocol: ProtocolVersion) -> Option<WireError> {
    forwarded.reply.withheld(protocol)
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;
    use crate::agents::{HarnessRef, TerminalSupport};
    use crate::harness::{HarnessKind, StructuredSupport};
    use crate::id::SessionId;
    use crate::rpc::{Outcome, ServerFrame};
    use crate::session::{SessionMode, SessionSpec};

    const V10: ProtocolVersion = ProtocolVersion::new(10);
    const V11: ProtocolVersion = ProtocolVersion::new(11);

    fn session(harness: impl Into<HarnessRef>) -> SessionInfo {
        let mode = SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        };
        SessionInfo::new(
            SessionId::random(),
            SessionSpec::new(harness, mode),
            DateTime::UNIX_EPOCH,
        )
    }

    fn goose() -> HarnessRef {
        HarnessRef::Added("goose".parse().unwrap())
    }

    #[test]
    fn lists_leave_added_agents_out_for_an_older_peer() {
        let known = session(HarnessKind::Codex);
        let added = session(goose());
        let listed = Reply::ListSessions(vec![known.clone(), added]);
        assert_eq!(listed.for_protocol(V11), None);
        assert_eq!(
            listed.for_protocol(V10),
            Some(Reply::ListSessions(vec![known]))
        );

        let harness = |kind: HarnessRef| Harness {
            kind,
            path: "/bin/agent".into(),
            version: None,
            structured: StructuredSupport::Available,
            terminal: TerminalSupport::Available,
        };
        let found =
            Reply::ListHarnesses(vec![harness(HarnessKind::Gemini.into()), harness(goose())]);
        assert_eq!(found.for_protocol(V11), None);
        assert_eq!(
            found.for_protocol(V10),
            Some(Reply::ListHarnesses(vec![harness(
                HarnessKind::Gemini.into()
            )]))
        );

        let profile = |harness: HarnessRef, name: &str| ProfileInfo {
            name: name.parse().unwrap(),
            label: None,
            harness,
            sets: Vec::new(),
            model: None,
        };
        let profiles = vec![
            profile(HarnessKind::Codex.into(), "codex"),
            profile(goose(), "goose"),
        ];
        assert_eq!(
            Reply::ReloadProfiles(profiles.clone()).for_protocol(V10),
            Some(Reply::ReloadProfiles(vec![profile(
                HarnessKind::Codex.into(),
                "codex"
            )]))
        );
        assert_eq!(Reply::ListProfiles(profiles).for_protocol(V11), None);
    }

    #[test]
    fn a_call_naming_an_added_agent_needs_protocol_11() {
        use crate::options::GetHarnessOptions;
        use crate::profile::ListProfiles;
        use crate::rpc::{Call, CreateSession};

        let mode = || SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        };
        let added = Call::CreateSession(CreateSession {
            spec: SessionSpec::new(goose(), mode()),
        });
        assert_eq!(added.since(), Some(V11));
        let known = Call::CreateSession(CreateSession {
            spec: SessionSpec::new(HarnessKind::Codex, mode()),
        });
        assert_eq!(known.since(), Some(ProtocolVersion::new(2)));
        let options = |harness| {
            Call::HarnessOptions(GetHarnessOptions {
                harness,
                profile: None,
            })
        };
        assert_eq!(options(goose()).since(), Some(V11));
        assert_eq!(
            options(HarnessKind::Gemini.into()).since(),
            Some(ProtocolVersion::new(9))
        );
        let profiles = |harness| Call::ListProfiles(ListProfiles { harness });
        assert_eq!(profiles(Some(goose())).since(), Some(V11));
        assert_eq!(profiles(None).since(), Some(ProtocolVersion::new(5)));
    }

    #[test]
    fn one_added_agents_session_is_withheld_from_an_older_peer() {
        let added = Reply::GetSession(session(goose()));
        assert_eq!(added.withheld(V11), None);
        assert_eq!(added.withheld(V10), Some(unreadable()));
        assert_eq!(
            Reply::GetSession(session(HarnessKind::Codex)).withheld(V10),
            None
        );
        let forwarded = Reply::Forward(Forwarded {
            reply: Box::new(added),
        });
        assert_eq!(forwarded.withheld(V10), Some(unreadable()));

        // As the node writes it: the reply becomes the error.
        let frame = ServerFrame::Response {
            id: crate::id::RequestId::new(7),
            outcome: Outcome::Ok(forwarded),
        };
        assert_eq!(frame.for_protocol(V11), None);
        assert_eq!(
            frame.for_protocol(V10),
            Some(ServerFrame::Response {
                id: crate::id::RequestId::new(7),
                outcome: Outcome::Err(unreadable()),
            })
        );
    }
}
