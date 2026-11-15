//! Calls: what the browser asks a node, decoded into the protocol's typed
//! `Call`, checked against what a browser may call ([`forwardable`]) and
//! sent over the node's shared connection with `NodeClient::call_any`,
//! whose protocol-version gate is the typed calls'. What a browser may do
//! on a node is the node's to decide, as for the CLI. A call in flight
//! when the connection drops fails as `disconnected` and is never repeated
//! (a `session.send` must not run twice).

use std::sync::Arc;

use vessel_client::{CallError, UnaryCall};
use vessel_proto::{Call, Method, RawJson};

use crate::registry::Registry;
use crate::source::NodeSource;
use crate::wire::{CallOutcome, Failure, NodeKey};

/// A call refused because the tab has too many in flight.
pub(crate) fn busy() -> Failure {
    Failure::Refused {
        message: "too many calls at once".to_owned(),
    }
}

/// Decode `call`, pass it to `node`, and say how it went.
pub(crate) async fn forward<S: NodeSource>(
    registry: &Arc<Registry<S>>,
    node: &NodeKey,
    call: RawJson,
) -> CallOutcome {
    let call = match decode(&call) {
        Ok(call) => call,
        Err(failure) => return CallOutcome::Err(failure),
    };
    let client = match registry.client(node).await {
        Ok(client) => client,
        Err(failure) => return CallOutcome::Err(failure),
    };
    match client.call_any(call).await {
        Ok(reply) => CallOutcome::Ok(Box::new(reply)),
        Err(error) => CallOutcome::Err(failure(error)),
    }
}

/// The browser's call, typed and checked.
fn decode(call: &RawJson) -> Result<UnaryCall, Failure> {
    let call: Call = serde_json::from_str(call.as_text()).map_err(|error| Failure::Refused {
        message: format!("not a call this vessel knows: {error}"),
    })?;
    let method = call.method();
    if !forwardable(method) {
        return Err(Failure::Refused {
            message: format!("`{}` is not for browsers", method.as_str()),
        });
    }
    UnaryCall::try_from(call).map_err(|error| Failure::Refused {
        message: error.to_string(),
    })
}

/// Whether a browser may call `method` through the bridge (ADR 0017: one
/// line per method, decided here when the method is added).
///
/// Refused: the connection-scoped subscription methods (streams have their
/// own frames), and the fleet's node-to-node machinery, which only nodes
/// and relays call. Desktop voice also remains on its direct connection
/// (ADR 0080); this bridge owns no microphone/playback lease. Everything
/// else passes, and the node checks scopes as for any client.
#[expect(
    clippy::too_many_lines,
    reason = "one line per method (ADR 0017), so a new method does not compile until someone \
              decides; the list is the point"
)]
const fn forwardable(method: Method) -> bool {
    match method {
        Method::GetUsageLimits
        | Method::SetUsageLookup
        | Method::QueryUsage
        | Method::NodeInfo
        | Method::ListHarnesses
        | Method::ListSessions
        | Method::GetSession
        | Method::OpenSideQuestion
        | Method::PromptSideQuestion
        | Method::ResetSideQuestion
        | Method::CreateSession
        | Method::SendInput
        | Method::KillSession
        | Method::ReadEvents
        | Method::StopNode
        | Method::InviteDevice
        | Method::GetDeviceSelf
        | Method::ListDevices
        | Method::RevokeDevice
        | Method::ListProfiles
        | Method::ReloadProfiles
        | Method::MoveProfile
        | Method::GetManifest
        | Method::ReadBlob
        | Method::WatchFleet
        | Method::ListMembers
        | Method::Forward
        | Method::ListNodes
        | Method::ForgetNode
        | Method::ExportSession
        | Method::ImportSession
        | Method::ExportMemory
        | Method::ImportMemory
        | Method::ReadTransfer
        | Method::CloseTransfer
        | Method::SetSecret
        | Method::ListSecrets
        | Method::DeleteSecret
        | Method::CopySecret
        | Method::LspWorkspaceSymbols
        | Method::LspDocumentSymbols
        | Method::LspDefinition
        | Method::LspReferences
        | Method::LspHover
        | Method::LspImplementations
        | Method::LspTypeDefinition
        | Method::LspIncomingCalls
        | Method::LspOutgoingCalls
        | Method::LspDiagnostics
        | Method::ListModels
        | Method::HarnessOptions
        | Method::SessionOptions
        | Method::SetSessionOption
        | Method::RenameSession
        | Method::PromptSession
        | Method::ChangeQueue
        | Method::GetConversation
        | Method::ResumeSession
        | Method::MarkSession
        | Method::DeleteSession
        | Method::BeginUpload
        | Method::WriteUpload
        | Method::FinishUpload
        | Method::ReadAttachment
        | Method::ReleaseWorkspaceAutomation | Method::GetRegistry | Method::ListWorkspaces | Method::GetWorkspace | Method::RenameWorkspace | Method::PinWorkspace
        | Method::ListProjects
        | Method::GetProject
        | Method::AddProject
        | Method::CloneProject
        | Method::InitProject
        | Method::RemoveProject
        | Method::ProjectCommands
        | Method::ApproveCommands
        | Method::ListWorktrees
        | Method::CreateWorktree
        | Method::SetupOutput
        | Method::RunSetup
        | Method::RemoveWorktree
        | Method::ListSnapshots
        | Method::DropSnapshot
        | Method::GitStatus
        | Method::GitChanges
        | Method::GitDiff
        | Method::GitShow
        | Method::GitBranches
        | Method::GitLog
        | Method::GitStashes
        | Method::GitStage
        | Method::GitUnstage
        | Method::GitDiscard
        | Method::GitCommit
        | Method::GitFetch
        | Method::GitPush
        | Method::GitPull
        | Method::GitSwitch
        | Method::GitRenameBranch
        | Method::GitStash
        | Method::ListDir
        | Method::ReadFile
        // ADR0083: unary workspace calls remain subject to node scope and
        // pending protocol admission; no stream or browser authority is added.
        | Method::InspectDirectory
        | Method::ReadFileBytes
        | Method::DuplicateFileEntry
        | Method::ObserveFile
        | Method::SuggestEntries
        | Method::WriteFile
        | Method::CreateDir
        | Method::RenamePath
        | Method::DeletePath
        | Method::FindFiles
        | Method::SearchFiles
        | Method::ForgeStatus
        | Method::ListPullRequests
        | Method::GetPullRequest
        | Method::OpenPullRequest
        | Method::MergePullRequest
        | Method::SetFeedback
        // Protocol 11 (ADR 0031): a terminal's calls, a project's scripts
        // and the ports; its records follow through the bridge's own
        // frames (`follow_terminal`), as a session's do.
        | Method::CreateTerminal
        | Method::ListTerminals
        | Method::GetTerminal
        | Method::ReadTerminal
        | Method::SendTerminal
        | Method::RenameTerminal
        | Method::KillTerminal
        | Method::CloseTerminal
        | Method::ListScripts
        | Method::RunScript
        | Method::RestartScript
        | Method::ListPorts
        | Method::ListAgents
        | Method::AddAgent
        | Method::RemoveAgent
        | Method::GetAgentStatus
        | Method::GetCatalogue
        | Method::InstallAgent
        | Method::ReadInstall
        | Method::JoinFleet
        | Method::LeaveFleet
        | Method::SyncFleet
        | Method::GetFleetStatus
        // A person's read of what the machine shares (ADR 0056).
        | Method::GetPublished
        | Method::PreviewSchedule
        | Method::ListSchedules
        | Method::GetSchedule
        | Method::CreateSchedule
        | Method::UpdateSchedule
        | Method::PauseSchedule
        | Method::ResumeSchedule
        | Method::RunSchedule
        | Method::DeleteSchedule
        | Method::ListRuns
        | Method::ListPrompts
        | Method::CreatePrompt
        | Method::UpdatePrompt
        | Method::DeletePrompt
        | Method::ListInbox
        | Method::GetInboxEntry
        | Method::MarkInbox
        | Method::Diagnostics
        | Method::GetPushKey
        | Method::SubscribePush
        | Method::UnsubscribePush
        | Method::ReportPushPresence
        // Protocol 12: searching every session's messages.
        | Method::SearchMessages
        // Protocol 12: parking and waking a session's agent.
        | Method::ParkSession
        | Method::WakeSession
        // Protocol 12: the node's settings (the change needs `admin`).
        | Method::GetNodeSettings
        | Method::SetNodeSettings
        // Protocol 14: forking and rewinding a session (ADR 0052).
        | Method::ForkSession
        | Method::RewindSession
        // Protocol 17: an agent's own history (ADR 0053).
        | Method::GetAgentHistory
        | Method::DetachSession
        // Staged git comparison reads (ADR 0087), like the other git reads.
        | Method::GitCompare
        | Method::GitCompareFile => true,
        Method::Subscribe
        | Method::SubscribeTerminal
        | Method::Unsubscribe
        | Method::ReportApplied
        | Method::Relay
        | Method::Deliver
        | Method::GrantHome
        | Method::GetCallback
        | Method::ListPeers
        | Method::GetSkillsStatus
        | Method::ReconcileSkills
        | Method::UninstallSkills
        | Method::SaveSkillsSelection
        | Method::ImportSkillsSelection
        | Method::StartVoice
        | Method::CaptureVoice
        | Method::NextVoice
        | Method::PlayedVoice
        | Method::AbortVoice
        | Method::StopVoice
        | Method::SpeakVoice => false,
    }
}

fn failure(error: CallError) -> Failure {
    match error {
        CallError::Remote(error) => Failure::Remote { error },
        CallError::NotInProtocol {
            method,
            needs,
            negotiated,
        } => Failure::NotInProtocol {
            method,
            needs,
            negotiated,
        },
        CallError::Disconnected => Failure::Disconnected,
        CallError::UnexpectedReply(_)
        | CallError::RequestIdsExhausted
        | CallError::UnassignedProtocol { .. } => Failure::Failed {
            message: error.to_string(),
        },
    }
}

/// A failure as one line, for a stream notice or an op's answer.
pub(crate) fn failure_text(failure: &Failure) -> String {
    match failure {
        Failure::Remote { error } => error.to_string(),
        Failure::NotInProtocol { method, .. } => {
            format!("`{}` is newer than the node", method.as_str())
        }
        Failure::Disconnected => "the connection closed".to_owned(),
        Failure::Unreachable { message }
        | Failure::Refused { message }
        | Failure::Failed { message } => message.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscriptions_and_node_machinery_are_not_forwarded() {
        let refused: Vec<_> = Method::ALL
            .iter()
            .filter(|method| !forwardable(**method))
            .map(|method| method.as_str())
            .collect();
        assert_eq!(
            refused,
            [
                "session.subscribe",
                "session.unsubscribe",
                "fleet.report",
                "fleet.relay",
                "fleet.deliver",
                "fleet.grant",
                "fleet.callback",
                "fleet.peers",
                // Protocol 11: a terminal's stream has its own frame.
                "terminal.subscribe",
                "skills.status",
                "skills.reconcile",
                "skills.uninstall",
                "skills.save_selection",
                "skills.import_selection",
                // ADR 0080: a desktop voice lease belongs to its direct connection.
                "voice.start",
                "voice.chunk",
                "voice.next",
                "voice.played",
                "voice.abort",
                "voice.stop",
                "voice.speak",
            ]
        );
    }

    #[test]
    fn file_panel_unaries_keep_pending_admission_when_forwarded() {
        for method in [
            Method::InspectDirectory,
            Method::ReadFileBytes,
            Method::DuplicateFileEntry,
            Method::ObserveFile,
            Method::SuggestEntries,
        ] {
            assert!(forwardable(method));
            assert_eq!(method.since(), None);
            assert_eq!(method.shape(), vessel_proto::MethodShape::Unary);
        }
    }

    #[test]
    fn a_call_is_decoded_typed_or_refused() {
        let call = RawJson::from_text(r#"{"method":"session.list","params":null}"#).unwrap();
        assert_eq!(decode(&call).unwrap().method(), Method::ListSessions);
        for refused in [
            r#"{"method":"session.subscribe","params":{"session":"00000000-0000-0000-0000-000000000000","from":0}}"#,
            r#"{"method":"no.such","params":null}"#,
            r#"{"method":"session.get","params":{"nope":1}}"#,
            "[1,2]",
        ] {
            let call = RawJson::from_text(refused).unwrap();
            assert!(
                matches!(decode(&call), Err(Failure::Refused { .. })),
                "{refused}"
            );
        }
    }
}
