//! Replies as a peer speaking an older protocol reads them.
//!
//! [Decision Log]
//! - Purpose: a node answers peers of every version it accepts. A reply
//!   holding something newer than the peer's version (a record of a later
//!   event, a scope it lacks, a board placement whose meaning changed) must
//!   reach it in a form it decodes and reads correctly, or its connection
//!   fails, or its page places a card where it never placed one before.
//! - Constraint: until protocol 10 only top-level `session.read` pages and
//!   `device.list` were rewritten. A page passed back through the fleet's
//!   envelope (`fleet.forward`, `fleet.relay`, `fleet.deliver`, whose reply
//!   nests the inner call's) reached an older home unchanged, and its
//!   client failed to decode the record and dropped the connection.
//! - Choice: one function per reply ([`Reply::for_protocol`]) that walks
//!   into the envelope's nested reply too, applied where frames are
//!   encoded for a connection (`ServerFrame::for_protocol`). It returns
//!   `None` when nothing changes, so a current peer costs no copy.
//! - Protocol 10 changes what an ended session's board place means: an
//!   ended session that is not archived is in building as `exited` (ADR
//!   0026), where protocol 8 and 9 put every ended session in the archive.
//!   A version 8-9 peer gets the old placement, archived as of its end, so
//!   cards it cannot start again or archive do not pile up in its building
//!   lane.

use super::Reply;
use crate::agents::{harnesses_for, profiles_for, readable};
use crate::board::{KanbanColumn, SessionBoard};
use crate::device::Scope;
use crate::forward::Forwarded;
use crate::node::ProtocolVersion;
use crate::session::SessionInfo;

/// The first version whose ended sessions may sit outside the archive.
const ENDED_OUTSIDE_THE_ARCHIVE: ProtocolVersion = ProtocolVersion::new(10);

mod templates;

impl Reply {
    /// This reply as a peer speaking `protocol` reads it, or `None` when it
    /// needs no change.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per reply, so a new reply must decide whether an older peer reads it \
                  differently; the list is the point"
    )]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        let info = |info: &SessionInfo| info.for_protocol(protocol);
        match self {
            Self::GetRegistry(snapshot) => snapshot.for_protocol(protocol).map(Self::GetRegistry),
            Self::OpenSideQuestion(session) => info(session).map(Self::OpenSideQuestion),
            Self::ListWorkspaceScripts(reply) => reply.for_protocol(protocol).map(Self::ListWorkspaceScripts),
            Self::StartWorkspaceScript(reply) => reply.for_protocol(protocol).map(Self::StartWorkspaceScript),
            Self::StopWorkspaceScript(reply) => reply.for_protocol(protocol).map(Self::StopWorkspaceScript),
            Self::RestartWorkspaceScript(reply) => reply.for_protocol(protocol).map(Self::RestartWorkspaceScript),
            Self::ResetSideQuestion(session) => info(session).map(Self::ResetSideQuestion),
            Self::NodeInfo(node) => node.for_protocol(protocol).map(Self::NodeInfo),
            Self::GetNodeSettings(settings) => settings.for_protocol(protocol).map(Self::GetNodeSettings),
            Self::SetNodeSettings(settings) => settings.for_protocol(protocol).map(Self::SetNodeSettings),
            Self::JoinFleet(joined) => joined.home.for_protocol(protocol).map(|home| {
                Self::JoinFleet(crate::JoinedFleet { home })
            }),
            Self::ReadEvents(page) => page.for_protocol(protocol).map(Self::ReadEvents),
            Self::GetDeviceSelf(authority) => {
                let newer = |scope: &Scope| scope.since() > protocol;
                authority.scopes.iter().any(newer).then(|| {
                    let mut authority = authority.clone();
                    authority.scopes.retain(|scope| !newer(scope));
                    Self::GetDeviceSelf(authority)
                })
            }
            Self::ListDevices(devices) => {
                let newer = |scope: &Scope| scope.since() > protocol;
                let changed = devices.iter().any(|device| device.scopes.iter().any(newer));
                changed.then(|| {
                    Self::ListDevices(
                        devices
                            .iter()
                            .map(|device| {
                                let mut device = device.clone();
                                device.scopes.retain(|scope| !newer(scope));
                                device
                            })
                            .collect(),
                    )
                })
            }
            Self::DetachSession(detached) => info(&detached.session).map(|info| {
                let mut detached = detached.clone();
                detached.session = Box::new(info);
                Self::DetachSession(detached)
            }),
            Self::ListSessions(sessions) => {
                // An added agent's session is left out of a list an older
                // peer could not decode with it (protocol 11, ADR 0032).
                let shown = |session: &SessionInfo| readable(session, protocol);
                let changed = sessions
                    .iter()
                    .any(|session| !shown(session) || info(session).is_some());
                changed.then(|| {
                    Self::ListSessions(
                        sessions
                            .iter()
                            .filter(|session| shown(session))
                            .map(|session| info(session).unwrap_or_else(|| session.clone()))
                            .collect(),
                    )
                })
            }
            Self::ListHarnesses(harnesses) => {
                harnesses_for(harnesses, protocol).map(Self::ListHarnesses)
            }
            Self::ListProfiles(profiles) => {
                profiles_for(profiles, protocol).map(Self::ListProfiles)
            }
            Self::ReloadProfiles(profiles) => {
                profiles_for(profiles, protocol).map(Self::ReloadProfiles)
            }
            Self::GetSession(session) => info(session).map(Self::GetSession),
            Self::GetConversation(conversation) => conversation
                .composer_for_protocol(protocol)
                .map(Self::GetConversation),
            Self::CreateSession(session) => info(session).map(Self::CreateSession),
            Self::MoveProfile(session) => info(session).map(Self::MoveProfile),
            Self::RenameSession(session) => info(session).map(Self::RenameSession),
            Self::Subscribe(subscribed) => info(&subscribed.info).map(|info| {
                let mut subscribed = subscribed.clone();
                subscribed.info = info;
                Self::Subscribe(subscribed)
            }),
            // A set naming scripts reaches an older client as one it cannot
            // approve (protocol 11, ADR 0031).
            Self::ProjectCommands(commands) => {
                commands.for_protocol(protocol).map(Self::ProjectCommands)
            }
            Self::ApproveCommands(commands) => {
                commands.for_protocol(protocol).map(Self::ApproveCommands)
            }
            Self::CreateWorktree(outcome) => {
                outcome.for_protocol(protocol).map(Self::CreateWorktree)
            }
            Self::ListScripts(list) => list.commands.for_protocol(protocol).map(|commands| {
                let mut list = list.clone();
                list.commands = commands;
                Self::ListScripts(list)
            }),
            Self::RunScript(outcome) => outcome.for_protocol(protocol).map(Self::RunScript),
            Self::RestartScript(outcome) => outcome.for_protocol(protocol).map(Self::RestartScript),
            Self::Forward(forwarded) => nested(forwarded, protocol).map(Self::Forward),
            Self::Relay(forwarded) => nested(forwarded, protocol).map(Self::Relay),
            Self::Deliver(forwarded) => nested(forwarded, protocol).map(Self::Deliver),
            Self::ResumeSession(session) => info(session).map(Self::ResumeSession),
            Self::MarkSession(session) => info(session).map(Self::MarkSession),
            Self::ParkSession(session) => info(session).map(Self::ParkSession),
            Self::WakeSession(session) => info(session).map(Self::WakeSession),
            Self::ForkSession(session) => info(session).map(Self::ForkSession),
            Self::RewindSession(session) => info(session).map(Self::RewindSession),
            Self::ExportSession(value) => templates::exported(value, protocol).map(Self::ExportSession),
            Self::ExportMemory(value) => templates::exported(value, protocol).map(Self::ExportMemory),
            Self::ImportSession(value) => templates::imported(value, protocol).map(Self::ImportSession),
            Self::ListPrompts(value) => templates::prompts(value, protocol).map(Self::ListPrompts),
            Self::CreatePrompt(value) => templates::prompt(value, protocol).map(Self::CreatePrompt),
            Self::UpdatePrompt(value) => templates::prompt(value, protocol).map(Self::UpdatePrompt),
            Self::ListSchedules(value) => templates::schedules(value, protocol).map(Self::ListSchedules),
            Self::GetSchedule(value) => templates::schedule(value, protocol).map(Self::GetSchedule),
            Self::CreateSchedule(value) => templates::schedule(value, protocol).map(Self::CreateSchedule),
            Self::UpdateSchedule(value) => templates::schedule(value, protocol).map(Self::UpdateSchedule),
            Self::PauseSchedule(value) => templates::schedule(value, protocol).map(Self::PauseSchedule),
            Self::ResumeSchedule(value) => templates::schedule(value, protocol).map(Self::ResumeSchedule),
            // Replies of calls newer than every peer that can ask them, or
            // holding nothing that changed meaning.
            Self::PromptSideQuestion(_)
            | Self::SendInput(_)
            | Self::KillSession(_)
            | Self::Unsubscribe(_)
            | Self::StopNode(_)
            | Self::InviteDevice(_)
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
            | Self::DeleteSession(_)
            | Self::BeginUpload(_)
            | Self::WriteUpload(_)
            | Self::FinishUpload(_)
            | Self::ReadAttachment(_)
            | Self::ListWorkspaceLabels(_)
            | Self::AssignWorkspaceLabel(_)
            | Self::UpdateWorkspaceLabel(_)
            | Self::InspectWorkspaceLabelDelete(_)
            | Self::DeleteWorkspaceLabel(_)
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
            | Self::ListWorktrees(_)
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
            | Self::InspectDirectory(_)
            | Self::ReadFileBytes(_)
            | Self::DuplicateFileEntry(_)
            | Self::ObserveFile(_)
            | Self::SuggestEntries(_)
            | Self::ReadFile(_)
            | Self::WriteFile(_)
            | Self::CreateDir(_)
            | Self::RenamePath(_)
            | Self::DeletePath(_)
            | Self::FindFiles(_)
            | Self::SearchFiles(_)
            // Protocol 11's are newer than every older peer.
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
            | Self::ListPorts(_)
            // Protocol 11's: asked only by peers that read them.
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
            | Self::GetSkillsStatus(_)
            | Self::ReconcileSkills(_)
            | Self::UninstallSkills(_)
            | Self::SaveSkillsSelection(_)
            | Self::ImportSkillsSelection(_)
            | Self::PreviewSchedule(_)
            | Self::RunSchedule(_)
            | Self::DeleteSchedule(_)
            | Self::ListRuns(_)
            | Self::ListUsageReports(_)
            | Self::InspectUsageReport(_)
            | Self::SetUsageSourceLookup(_)
            | Self::GetUsageLimits(_)
            | Self::SetUsageLookup(_)
            | Self::QueryUsage(_)
            | Self::DeletePrompt(_)
            | Self::ListInbox(_)
            | Self::GetInboxEntry(_)
            | Self::MarkInbox(_)
            | Self::Diagnostics(_)
            | Self::GetWorkspaceServiceEnv(_)
            | Self::GetServiceSettings(_)
            | Self::SetServiceGateway(_)
            | Self::SetServiceExposure(_)
            | Self::GetPushKey(_)
            | Self::SubscribePush(_)
            | Self::UnsubscribePush(_)
            | Self::ReportPushPresence(_)
            | Self::SearchMessages(_)
            // Protocol 12's, 14's and 17's: asked only by peers that read them.
            | Self::GetAgentHistory(_) => None,
        }
    }
}

/// The envelope's inner reply, rewritten.
fn nested(forwarded: &Forwarded, protocol: ProtocolVersion) -> Option<Forwarded> {
    forwarded
        .reply
        .for_protocol(protocol)
        .map(|reply| Forwarded {
            reply: Box::new(reply),
        })
}

impl SessionInfo {
    /// This session as a peer speaking `protocol` reads it, or `None` when
    /// it needs no change. Unassigned or older peers receive neither label map; version
    /// 8-9 peers also keep an ended session in their archive placement.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        self.for_protocol_with_mutation_time(protocol, crate::SESSION_MUTATIONS)
    }

    /// One projection rule also exercised with a synthetic assigned threshold.
    pub(crate) fn for_protocol_with_metadata(
        &self,
        protocol: ProtocolVersion,
        assigned: Option<ProtocolVersion>,
    ) -> Option<Self> {
        let strip = !crate::lifecycle_version::visible(assigned, protocol)
            && (!self.labels.is_empty()
                || !self.spec.labels.is_empty()
                || !self.listing.is_shown()
                || self.spec.listing.is_some());
        let archive = protocol < ENDED_OUTSIDE_THE_ARCHIVE && !self.status.is_live();
        if !strip && !archive {
            return None;
        }
        let mut info = self.clone();
        if strip {
            info.labels.clear();
            info.spec.labels.clear();
            info.listing = crate::SessionListing::Shown;
            info.spec.listing = None;
        }
        if !archive {
            return Some(info);
        }
        let Some(board) = self.board.as_ref() else {
            return strip.then_some(info);
        };
        if board.column == KanbanColumn::Archive {
            return strip.then_some(info);
        }
        info.board = Some(SessionBoard::archived(
            self.ended_at.unwrap_or(board.last_activity_at),
        ));
        Some(info)
    }

    /// Apply provenance after the head's label and archive projection, so
    /// hiding one feature never bypasses another field's compatibility.
    pub(crate) fn for_protocol_with_names(
        &self,
        protocol: ProtocolVersion,
        assigned: Option<ProtocolVersion>,
    ) -> Option<Self> {
        let projected = self.for_protocol_with_metadata(protocol, crate::lifecycle_version::SINCE);
        let source = projected.as_ref().unwrap_or(self);
        let origin = source.title_origin.for_protocol(protocol, assigned);
        if origin == source.title_origin {
            return projected;
        }
        let mut info = projected.unwrap_or_else(|| self.clone());
        info.title_origin = origin;
        Some(info)
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;
    use crate::agent::AgentEvent;
    use crate::board::{Activity, DisplayStatus};
    use crate::event::{EventRecord, SessionEvent};
    use crate::harness::HarnessKind;
    use crate::id::{EventSeq, SessionId};
    use crate::rpc::EventPage;
    use crate::session::{SessionMode, SessionSpec, SessionStatus};

    const V9: ProtocolVersion = ProtocolVersion::new(9);
    const V10: ProtocolVersion = ProtocolVersion::new(10);

    pub(super) fn exited() -> SessionInfo {
        let spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        );
        let mut info = SessionInfo::new(SessionId::random(), spec, DateTime::UNIX_EPOCH);
        info.status = SessionStatus::Exited { code: Some(0) };
        info.ended_at = Some(DateTime::UNIX_EPOCH);
        info.board = Some(SessionBoard::new(
            KanbanColumn::Building,
            DisplayStatus::Exited,
            Activity::Exited,
            DateTime::UNIX_EPOCH,
        ));
        info
    }

    #[test]
    fn an_older_peer_keeps_ended_sessions_in_the_archive() {
        let info = exited();
        assert_eq!(info.for_protocol(V10), None);
        let older = info.for_protocol(V9).unwrap();
        assert_eq!(
            older.board,
            Some(SessionBoard::archived(DateTime::UNIX_EPOCH))
        );
        let listed = Reply::ListSessions(vec![info.clone()]);
        assert_eq!(
            listed.for_protocol(V9),
            Some(Reply::ListSessions(vec![older]))
        );
        let mut live = info;
        live.status = SessionStatus::Running;
        assert_eq!(live.for_protocol(V9), None);
    }

    #[test]
    fn hiding_origin_still_adapts_archive_placement() {
        let mut info = exited();
        info.title_origin = crate::TitleOrigin::Given;
        let old = info.for_protocol_with_names(V9, Some(V10)).unwrap();
        assert_eq!(old.title_origin, crate::TitleOrigin::Unknown);
        assert_eq!(
            old.board,
            Some(SessionBoard::archived(DateTime::UNIX_EPOCH))
        );
        let current = info.for_protocol_with_names(V10, Some(V10));
        assert_eq!(current, None);
    }

    #[test]
    fn a_page_passed_through_the_envelope_is_rewritten_too() {
        let page = EventPage {
            records: vec![EventRecord {
                seq: EventSeq::new(4),
                at: DateTime::UNIX_EPOCH,
                event: SessionEvent::UsageChanged {
                    usage: crate::conversation::ContextUsage {
                        observation: crate::conversation::UsageObservation::Current,
                        used: crate::model::TokenCount::new(1),
                        size: crate::model::TokenCount::new(2),
                        cost: None,
                    },
                },
            }],
            next: EventSeq::new(5),
            at_end: true,
        };
        let delivered = Reply::Deliver(Forwarded {
            reply: Box::new(Reply::ReadEvents(page)),
        });
        assert_eq!(delivered.for_protocol(V10), None);
        let Some(Reply::Deliver(Forwarded { reply })) = delivered.for_protocol(V9) else {
            panic!("an envelope stays an envelope");
        };
        let Reply::ReadEvents(older) = *reply else {
            panic!("its reply stays a page");
        };
        assert_eq!(older.records[0].seq, EventSeq::new(4));
        assert!(matches!(
            older.records[0].event,
            SessionEvent::Agent {
                event: AgentEvent::HarnessLog { .. }
            }
        ));
    }
    #[test]
    fn an_assigned_label_threshold_keeps_both_maps_only_for_supported_peers() {
        let labels = serde_json::from_str(r#"{"env":"prod"}"#).unwrap();
        let mut info = exited();
        info.labels = labels;
        info.spec.labels = info.labels.clone();
        let peer = crate::PROTOCOL_VERSION;
        let hidden = info.for_protocol(peer).unwrap();
        assert!(hidden.labels.is_empty() && hidden.spec.labels.is_empty());
        info.listing = crate::SessionListing::Background;
        info.spec.listing = Some(crate::SessionListing::Background);
        assert!(info.for_protocol_with_metadata(peer, Some(peer)).is_none());
        let older = ProtocolVersion::new(peer.get().saturating_sub(1));
        let hidden = info.for_protocol_with_metadata(older, Some(peer)).unwrap();
        assert!(hidden.listing.is_shown());
        assert_eq!(hidden.spec.listing, None);
        assert!(hidden.labels.is_empty() && hidden.spec.labels.is_empty());
        assert!(!info.labels.is_empty() && !info.spec.labels.is_empty());
    }
}

#[cfg(test)]
mod origins;

#[cfg(test)]
mod restart_tests;
