//! Per-method fleet admission, separate from the envelope wire values.
//! The exhaustive policy stays on Method; no second registry is introduced.

use crate::rpc::Method;

impl Method {
    /// Whether a call of this method can travel in `fleet.forward`,
    /// `fleet.relay` and `fleet.deliver`. Deny by default: the `match` names
    /// every method, so a new row does not compile until someone decides.
    /// A forwarded call still runs with the target's grant to its home
    /// intersected with the origin's scopes.
    ///
    /// - Forwardable, the work agents and people do on another node: node
    ///   and harness info, sessions (list, read, create, send, kill, move
    ///   between profiles), profiles to choose from, and the transfers F4
    ///   pulls through the home (export, import, read, close), with
    ///   `secret.set`, which `secret.copy` sends to its target, and
    ///   `secret.list` (names only), which it asks the target first, so a
    ///   target that would refuse the value never receives it, and
    ///   `model.list` (protocol 7: route names, providers and model ids,
    ///   never a URL or a secret name), which a picker of profiles for a
    ///   session there needs as much as the profiles, and protocol 9's
    ///   options and titles (`harness.options`, `session.options`,
    ///   `session.set_option`, `session.rename`), which starting and
    ///   steering a session there needs, and protocol 10's conversation
    ///   (`session.prompt`, `session.queue`, `session.conversation`,
    ///   `session.resume`, `session.mark`, `session.delete`, and the
    ///   `attachment.*` calls), which talking to a session there needs as
    ///   much as `session.send` does, and protocol 10's projects,
    ///   worktrees, git and files (`project.*`, `worktree.*`, `git.*`,
    ///   `file.*`), which working in a project there needs; the roots that
    ///   would widen a reader's reach are refused for every caller instead
    ///   (ADR 0027), and protocol 11's `agent.list`, read as `harness.list`
    ///   is.
    /// - Not forwardable, administration of a node done there: stopping it,
    ///   its devices, re-reading its profiles, deleting or copying its
    ///   secrets, adding or removing its agents (ADR 0032).
    /// - Not forwardable, the fleet's own plumbing: an envelope is never
    ///   wrapped in another (at most two hops, no loops), and the bundle,
    ///   grant and membership calls belong to the hop between a member and
    ///   its home.
    /// - Not forwardable, streams: a subscription's events would need a
    ///   stream across hops (`session.read` with `wait` serves instead, and
    ///   `terminal.read` with `wait` for a terminal).
    /// - Forwardable, protocol 11's terminals, scripts and ports
    ///   (`terminal.*` but its subscription, `script.*`, `port.list`): an
    ///   agent opens a shell beside a session it started on another node,
    ///   and a person runs a project's service there (ADR 0031).
    /// - Forwardable, protocol 12's and 14's conversation calls
    ///   (`session.search`, `session.park`, `session.wake`, `session.fork`,
    ///   `session.rewind`) and protocol 17's `agent.history` (ADR 0053),
    ///   which an import there needs as `session.create` does; not
    ///   `node.settings` or `node.set_settings`, a node's administration.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per method, deny by default, so a new method does not compile until \
                  someone decides; the list is the point"
    )]
    pub const fn forwardable(self) -> bool {
        match self {
            Self::NodeInfo
            | Self::ListHarnesses
            | Self::ListSessions
            | Self::GetSession
            | Self::OpenSideQuestion
            | Self::PromptSideQuestion
            | Self::ResetSideQuestion
            | Self::CreateSession
            | Self::DetachSession
            | Self::SendInput
            | Self::KillSession
            | Self::ReadEvents
            | Self::MoveProfile
            | Self::ListProfiles
            | Self::ExportSession
            | Self::ImportSession
            | Self::ExportMemory
            | Self::ImportMemory
            | Self::ReadTransfer
            | Self::CloseTransfer
            | Self::SetSecret
            | Self::LspWorkspaceSymbols
            | Self::LspDocumentSymbols
            | Self::LspDefinition
            | Self::LspReferences
            | Self::LspHover
            | Self::LspImplementations
            | Self::LspTypeDefinition
            | Self::LspIncomingCalls
            | Self::LspOutgoingCalls
            | Self::LspDiagnostics
            | Self::ListSecrets
            | Self::ListModels
            | Self::HarnessOptions
            | Self::SessionOptions
            | Self::SetSessionOption
            | Self::RenameSession
            | Self::PromptSession
            | Self::ChangeQueue
            | Self::GetConversation
            | Self::ResumeSession
            | Self::MarkSession
            | Self::DeleteSession
            | Self::BeginUpload
            | Self::WriteUpload
            | Self::FinishUpload
            | Self::ReadAttachment
            | Self::GetRegistry
            | Self::ListWorkspaces
            | Self::GetWorkspace
            | Self::RenameWorkspace
            | Self::PinWorkspace
            | Self::ReleaseWorkspaceAutomation
            | Self::ListProjects
            | Self::GetProject
            | Self::AddProject
            | Self::CloneProject
            | Self::InitProject
            | Self::RemoveProject
            | Self::ProjectCommands
            | Self::ApproveCommands
            | Self::ListWorktrees
            | Self::CreateWorktree
            | Self::SetupOutput
            | Self::RunSetup
            | Self::RemoveWorktree
            | Self::ListSnapshots
            | Self::DropSnapshot
            | Self::GitStatus
            | Self::GitCompare
            | Self::GitCompareFile
            | Self::GitChanges
            | Self::GitDiff
            | Self::GitShow
            | Self::GitBranches
            | Self::GitLog
            | Self::GitStashes
            | Self::GitStage
            | Self::GitUnstage
            | Self::GitDiscard
            | Self::GitCommit
            | Self::GitFetch
            | Self::GitPush
            | Self::GitPull
            | Self::GitSwitch
            | Self::GitRenameBranch
            | Self::GitStash
            | Self::ListDir
            | Self::InspectDirectory
            | Self::ReadFileBytes
            | Self::DuplicateFileEntry
            | Self::ObserveFile
            | Self::SuggestEntries
            | Self::ReadFile
            | Self::WriteFile
            | Self::CreateDir
            | Self::RenamePath
            | Self::DeletePath
            | Self::FindFiles
            | Self::SearchFiles
            // Protocol 11: a session's pull request, on the machine the
            // session runs on.
            | Self::ForgeStatus
            | Self::ListPullRequests
            | Self::GetPullRequest
            | Self::OpenPullRequest
            | Self::MergePullRequest
            | Self::SetFeedback
            // Protocol 11: terminals, scripts and ports, on the machine
            // they run on (a terminal's stream is not forwarded: it is
            // read with `terminal.read` from another node).
            | Self::CreateTerminal
            | Self::ListTerminals
            | Self::GetTerminal
            | Self::ReadTerminal
            | Self::SendTerminal
            | Self::RenameTerminal
            | Self::KillTerminal
            | Self::CloseTerminal
            | Self::ListScripts
            | Self::RunScript
            | Self::RestartScript
            | Self::ListPorts
            // Protocol 11: what a node can run is read like its harnesses.
            | Self::ListAgents
            | Self::GetAgentStatus
            // Schedules (protocol 11): forwardable; who may do what from
            // another node is the target's scheduler's rule (ADR 0033).
            | Self::GetDeviceSelf
            | Self::GetFleetStatus
            | Self::PreviewSchedule
            | Self::ListSchedules
            | Self::GetSchedule
            | Self::CreateSchedule
            | Self::UpdateSchedule
            | Self::PauseSchedule
            | Self::ResumeSchedule
            | Self::RunSchedule
            | Self::DeleteSchedule
            | Self::ListRuns
            // Saved prompts: readable from another node; written only on
            // their own (the service refuses a delegated writer).
            // Lookup policy carries only a choice; credentials stay at the
            // destination, whose admin/operate scope intersection is checked.
            | Self::ListUsageReports
            | Self::InspectUsageReport
            | Self::SetUsageSourceLookup
            | Self::GetUsageLimits
            | Self::SetUsageLookup
            | Self::QueryUsage
            | Self::ListPrompts
            | Self::CreatePrompt
            | Self::UpdatePrompt
            | Self::DeletePrompt
            | Self::ListInbox
            | Self::GetInboxEntry
            | Self::MarkInbox
            // Protocol 12: a search reads, as `session.read` does.
            | Self::SearchMessages
            // Parking acts on a session as a prompt does.
            | Self::ParkSession
            | Self::WakeSession
            // Forking and rewinding act on a session as a prompt does.
            | Self::ForkSession
            | Self::RewindSession
            // An agent's history there, to import from it there (ADR
            // 0053): it needs `operate`, as the import does.
            | Self::GetAgentHistory => true,
            // Protocol 11: adding and removing an agent is the node's own
            // administration, by its owner or its own devices (ADR 0032).
            Self::Diagnostics
            | Self::GetPushKey
            | Self::SubscribePush
            | Self::UnsubscribePush
            | Self::ReportPushPresence
            | Self::AddAgent
            | Self::GetCatalogue
            | Self::InstallAgent
            | Self::ReadInstall
            | Self::RemoveAgent
            | Self::StopNode
            // The node's settings (protocol 12): its own administration,
            // read and changed by its owner and its own devices.
            | Self::GetSkillsStatus
            | Self::ReconcileSkills
            | Self::UninstallSkills
            | Self::SaveSkillsSelection
            | Self::ImportSkillsSelection
            | Self::GetNodeSettings
            | Self::SetNodeSettings
            | Self::InviteDevice
            | Self::ListDevices
            | Self::RevokeDevice
            | Self::ReloadProfiles
            | Self::DeleteSecret
            | Self::CopySecret
            | Self::Forward
            | Self::Relay
            | Self::Deliver
            | Self::GrantHome
            | Self::GetCallback
            | Self::ForgetNode
            | Self::ListPeers
            | Self::ListNodes
            | Self::GetManifest
            | Self::ReadBlob
            | Self::WatchFleet
            | Self::ReportApplied
            | Self::ListMembers
            | Self::JoinFleet
            | Self::LeaveFleet
            | Self::SyncFleet
            // Each machine is read over its own connection (ADR 0056).
            | Self::GetPublished
            | Self::Subscribe
            | Self::SubscribeTerminal
            | Self::Unsubscribe => false,
        }
    }
}
