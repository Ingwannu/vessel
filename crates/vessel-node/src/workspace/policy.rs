//! Who may call what, and which folders may be roots.
//!
//! [Decision Log]
//! - Agents (ADR 0013) already read, write and run git in their own folder
//!   with their own tools; through the node they get only what
//!   orchestration needs: the projects, what a project's file would run,
//!   and a worktree (with its setup output) for a session they start. The
//!   rest refuses them here, before the call runs, whatever their scopes.
//!   An agent never approves commands, so a worktree it makes runs setup
//!   only when a person approved that set.
//! - Checked by method in one exhaustive `match`, so the rule reads as a
//!   list and a new method does not compile until it says whether agents
//!   get it (it was a `matches!`, whose other methods were open to agents
//!   by default).
//! - Scripts (protocol 11, ADR 0031): a repository's commands run where a
//!   person approved them and a person asked, so `script.*` refuse agents
//!   too; an agent runs commands with its own tools. The terminal calls
//!   and `port.list` are open to them here, and held to their own rule
//!   (`service::terminals`: a profile that allows it, four at once, only
//!   their own).
//! - Pull requests (protocol 11, ADR 0029): `pr.merge`, `pr.feedback`,
//!   `pr.list` and `forge.status` refuse agents (a merge publishes to a
//!   shared branch; what is sent into a session, the node's sign-in and
//!   every session's pull request are a person's); `pr.get` and `pr.open`
//!   are open here and held to the forge's own rule (`refresh` and opening
//!   only on a session the agent may operate; an agent's open never
//!   pushes).
//! - Schedules and saved prompts (protocol 11, ADR 0033) are open here
//!   and held to their own rules (`automation::schedules::rules`: at most
//!   8 per agent session, a target that is its own child or its own
//!   session, only its own to manage, never run now, never through
//!   `fleet.forward`; `service::prompts`: saved prompts read, never
//!   written). A run acts as the agent through the checks a call of its
//!   own would meet (`service::scheduled`: the scope, ownership, the
//!   agent's limits and its place's), so a schedule gives an agent nothing
//!   it could not get directly: no terminal (a terminal session is refused
//!   as a target, and a scheduled session's agent gets terminals only as
//!   any session's would), no merge and no approval (no run opens or
//!   merges a pull request, makes a worktree, runs setup or a script).
//! - This `match` is the one list of what an agent may call through the
//!   node: every method of every track is in it, either refused here or
//!   named with the rule that holds it instead.
//! - Roots: a project's root is everything a reader of it can read. A
//!   file system's root, the user's home folder and any folder holding the
//!   node's data folder are refused, for every caller.

use std::path::Path;

use vessel_proto::{ErrorCode, Method, WireError};

use super::{Places, WorkspaceError};
use crate::principal::Principal;

/// `forbidden` for an agent calling a workspace method agents may not.
pub(crate) fn refuse_agents(principal: &Principal, method: Method) -> Result<(), WireError> {
    if principal.agent().is_none() || !refused(method) {
        return Ok(());
    }
    // The reason by family; `refused` decides, exhaustively, which methods.
    let why = if matches!(
        method,
        Method::ListScripts | Method::RunScript | Method::RestartScript
    ) {
        "a repository's scripts run where a person approved them and a person asked; an agent \
         runs commands with its own tools"
    } else if matches!(method, Method::DetachSession) {
        "only a person may make a managed child independent"
    } else if matches!(method, Method::GetAgentHistory) {
        "an agent's own history holds a person's conversations outside Vessel; an agent works \
         with the node's sessions"
    } else if matches!(
        method,
        Method::ForgeStatus
            | Method::ListPullRequests
            | Method::MergePullRequest
            | Method::SetFeedback
    ) {
        "a merge publishes to a shared branch, and what is sent into a session, the node's \
         sign-in and every session's pull request are a person's; an agent reads a pull request \
         and opens one for a session it may operate"
    } else {
        "an agent uses its own file and git tools; through the node it may list projects and \
         start a session in a worktree"
    };
    Err(WireError::new(
        ErrorCode::Forbidden,
        format!("agents may not call `{}`: {why}", method.as_str()),
    ))
}

/// Whether an agent is refused `method` by this rule. Exhaustive over
/// every method, so a new one does not compile until it says whether
/// agents get it.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per method, so a new method does not compile until someone decides; the \
              list is the point"
)]
const fn refused(method: Method) -> bool {
    match method {
        // The workspace but orchestration: an agent uses its own file and
        // git tools in its folder.
        Method::ListWorkspaceLabels | Method::AssignWorkspaceLabel | Method::UpdateWorkspaceLabel | Method::InspectWorkspaceLabelDelete | Method::DeleteWorkspaceLabel | Method::ReleaseWorkspaceAutomation | Method::GetRegistry | Method::ListWorkspaces | Method::GetWorkspace | Method::RenameWorkspace | Method::PinWorkspace
        | Method::AddProject
        | Method::GetSkillsStatus
        | Method::ReconcileSkills
        | Method::UninstallSkills
        | Method::SaveSkillsSelection
        | Method::ImportSkillsSelection
        | Method::CloneProject
        | Method::InitProject
        | Method::RemoveProject
        | Method::ApproveCommands
        | Method::RunSetup
        | Method::RemoveWorktree
        | Method::ListSnapshots
        | Method::DropSnapshot
        | Method::GitStatus
        | Method::GitChanges
        | Method::GitCompare
        | Method::GitCompareFile
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
        | Method::InspectDirectory
        | Method::ReadFileBytes
        | Method::DuplicateFileEntry
        | Method::ObserveFile
        | Method::SuggestEntries
        | Method::ReadFile
        | Method::WriteFile
        | Method::CreateDir
        | Method::RenamePath
        | Method::DeletePath
        | Method::FindFiles
        | Method::SearchFiles
        // Protocol 11 (ADR 0029): a merge publishes to a shared branch and
        // is a person's to decide, as is what is sent into a session; the
        // node's sign-in and the whole list are a person's view.
        | Method::ForgeStatus
        | Method::ListPullRequests
        | Method::MergePullRequest
        | Method::SetFeedback
        // A repository's scripts (protocol 11): a person's.
        | Method::ListScripts
        | Method::RunScript
        | Method::RestartScript
        | Method::JoinFleet
        | Method::LeaveFleet
        | Method::SyncFleet
        // What the machine shares (ADR 0056): a person's read, `admin`.
        | Method::GetPublished
        // An agent's own history (protocol 17, ADR 0053): an account's
        // other conversations are a person's, not the fleet's sessions.
        | Method::GetAgentHistory
        | Method::DetachSession => true,
        // Orchestration: the projects, and a worktree with its setup's
        // output for a session the agent starts.
        Method::ListProjects
        // Side-question policy refuses agents before reading a parent;
        // its typed reason belongs to that domain, not file/git policy.
        | Method::OpenSideQuestion
        | Method::PromptSideQuestion
        | Method::ResetSideQuestion
        | Method::GetProject
        | Method::ProjectCommands
        | Method::ListWorktrees
        | Method::CreateWorktree
        | Method::SetupOutput
        // Protocol 11: an agent reads a session's pull request, and opens
        // one for a session it may operate (the guard's ownership), never
        // pushing (`forge::open`).
        | Method::GetPullRequest
        | Method::OpenPullRequest
        // Not workspace calls: held to the scopes, ownership and limits
        // every agent is (ADR 0013, `guard`), not to this rule.
        // Schedules (protocol 11) are held to theirs (ADR 0033,
        // `automation::schedules::rules`).
        | Method::GetDeviceSelf
        | Method::GetFleetStatus
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
        // Session-scoped reads are held to their own service rule;
        // lookup policy needs admin, which no agent holds.
        | Method::ListUsageReports
        | Method::InspectUsageReport
        | Method::SetUsageSourceLookup
        | Method::GetUsageLimits
        | Method::SetUsageLookup
        | Method::QueryUsage
        | Method::ListPrompts
        | Method::CreatePrompt
        | Method::UpdatePrompt
        | Method::DeletePrompt
        | Method::SearchMessages
        | Method::ParkSession
        | Method::WakeSession
        // The node's settings (protocol 12): the read as `node.info`; the
        // change needs `admin`, which no agent holds.
        | Method::GetNodeSettings
        | Method::SetNodeSettings
        // Forking and rewinding a session it may operate (ADR 0052).
        | Method::ForkSession
        | Method::RewindSession
        | Method::NodeInfo
        | Method::ListHarnesses
        | Method::ListSessions
        | Method::GetSession
        | Method::CreateSession
        | Method::SendInput
        | Method::KillSession
        | Method::ReadEvents
        | Method::Subscribe
        | Method::Unsubscribe
        | Method::StopNode
        | Method::InviteDevice
        | Method::ListDevices
        | Method::RevokeDevice
        | Method::ListProfiles
        | Method::ReloadProfiles
        // `agent.add` and `agent.remove` refuse every agent themselves
        // (`crate::added::person`); `agent.list` reads.
        | Method::ListAgents
        | Method::AddAgent
        | Method::RemoveAgent
        | Method::GetAgentStatus
        | Method::GetCatalogue
        | Method::InstallAgent
        | Method::ReadInstall
        | Method::MoveProfile
        | Method::GetManifest
        | Method::ReadBlob
        | Method::WatchFleet
        | Method::ReportApplied
        | Method::ListMembers
        | Method::Forward
        | Method::Relay
        | Method::Deliver
        | Method::GrantHome
        | Method::GetCallback
        | Method::ListPeers
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
        // Terminals and ports (protocol 11): held to their own rule.
        | Method::CreateTerminal
        | Method::ListTerminals
        | Method::GetTerminal
        | Method::ReadTerminal
        | Method::SubscribeTerminal
        | Method::SendTerminal
        | Method::RenameTerminal
        | Method::KillTerminal
        | Method::CloseTerminal
        | Method::ListPorts
        | Method::ListInbox
        | Method::GetInboxEntry
        | Method::MarkInbox
        | Method::Diagnostics
        | Method::GetPushKey
        | Method::SubscribePush
        | Method::UnsubscribePush
        | Method::ReportPushPresence => false,
    }
}

/// Refuse `root` (resolved) as a project's root when it would widen what
/// its readers can reach.
pub(super) fn check_root(places: &Places, root: &Path) -> Result<(), WorkspaceError> {
    let refuse = |why: &str| {
        Err(WorkspaceError::Invalid(format!(
            "this folder cannot be a project: {why}"
        )))
    };
    if root.parent().is_none() {
        return refuse("it is a file system's root");
    }
    if places.home.starts_with(root) {
        return refuse("it is or holds the home folder, which holds credentials");
    }
    if places.data.starts_with(root) {
        return refuse("it holds the node's data folder and its token");
    }
    if root.starts_with(&places.data) {
        return refuse("it is inside the node's data folder");
    }
    // ADR 0030: git's own folder is not a working tree; its `config` and
    // `hooks` would be files the file calls write.
    // By git's own rules for what names `.git` (any case, trailing dots
    // or spaces, `git~1`, ignored zero-width characters).
    if root.components().any(|part| {
        part.as_os_str()
            .to_str()
            .is_some_and(vessel_proto::workspace::is_git_dir)
    }) {
        return refuse("it is a `.git` folder or inside one");
    }
    Ok(())
}

/// Whether `root` holds what makes a folder git's own (`HEAD`, `objects`
/// and `refs`): a bare repository or a `.git` folder under another name,
/// told apart without git (a node may have none). Blocking: call it on the
/// blocking pool.
pub(super) fn looks_like_git_dir(root: &Path) -> bool {
    root.join("HEAD").is_file() && root.join("objects").is_dir() && root.join("refs").is_dir()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vessel_proto::{NodeId, Scope, SessionId};

    use super::*;

    #[test]
    fn agents_get_orchestration_and_nothing_else() {
        let agent = Principal::Agent {
            node: NodeId::random(),
            session: SessionId::random(),
            scopes: vec![Scope::Read, Scope::Operate],
        };
        for allowed in [
            Method::ListProjects,
            Method::GetProject,
            Method::ProjectCommands,
            Method::ListWorktrees,
            Method::CreateWorktree,
            Method::SetupOutput,
            Method::CreateSession,
            // Not workspace calls: other rules decide (`guard`).
            Method::PromptSession,
            Method::DeleteSession,
            // Terminals: their own rule decides (ADR 0031).
            Method::CreateTerminal,
            Method::ListTerminals,
            Method::GetTerminal,
            Method::ReadTerminal,
            Method::SubscribeTerminal,
            Method::SendTerminal,
            Method::RenameTerminal,
            Method::KillTerminal,
            Method::CloseTerminal,
            Method::ListPorts,
            // A pull request: read, and opened without a push, on a session
            // the agent may operate (the forge's rule, ADR 0029).
            Method::GetPullRequest,
            Method::OpenPullRequest,
        ] {
            assert!(refuse_agents(&agent, allowed).is_ok(), "{allowed:?}");
        }
        for refused in [
            Method::ApproveCommands,
            Method::RunSetup,
            Method::RemoveWorktree,
            Method::ListSnapshots,
            Method::ReadFile,
            Method::GitPush,
            Method::WriteFile,
            Method::ListScripts,
            Method::RunScript,
            Method::RestartScript,
            Method::ForgeStatus,
            Method::ListPullRequests,
            Method::MergePullRequest,
            Method::SetFeedback,
        ] {
            let error = refuse_agents(&agent, refused).unwrap_err();
            assert_eq!(error.code, ErrorCode::Forbidden);
            assert!(!error.message.contains("  "), "{}", error.message);
        }
        assert!(refuse_agents(&Principal::Owner, Method::ApproveCommands).is_ok());
    }

    #[test]
    fn registry_visibility_and_mutation_remain_person_only() {
        let agent = Principal::Agent {
            node: NodeId::random(),
            session: SessionId::random(),
            scopes: vec![Scope::Read, Scope::Operate],
        };
        for method in [
            Method::ReleaseWorkspaceAutomation,
            Method::GetRegistry,
            Method::ListWorkspaceLabels,
            Method::AssignWorkspaceLabel,
            Method::UpdateWorkspaceLabel,
            Method::InspectWorkspaceLabelDelete,
            Method::DeleteWorkspaceLabel,
            Method::ListWorkspaces,
            Method::GetWorkspace,
            Method::RenameWorkspace,
            Method::PinWorkspace,
        ] {
            assert_eq!(
                refuse_agents(&agent, method).unwrap_err().code,
                ErrorCode::Forbidden
            );
            assert!(refuse_agents(&Principal::Owner, method).is_ok());
        }
    }

    #[test]
    fn roots_that_would_reach_the_token_or_the_home_are_refused() {
        let places = Places {
            data: PathBuf::from("/home/me/.vessel"),
            home: PathBuf::from("/home/me"),
        };
        // The data folder elsewhere: the home's parents are refused by the
        // home's own rule.
        let elsewhere = Places {
            data: PathBuf::from("/srv/vessel"),
            home: PathBuf::from("/home/me"),
        };
        for refused in ["/home", "/home/me"] {
            assert!(
                check_root(&elsewhere, Path::new(refused)).is_err(),
                "{refused}"
            );
        }
        assert!(check_root(&elsewhere, Path::new("/home/me/code")).is_ok());
        for refused in [
            "/",
            "/home/me",
            "/home",
            "/home/me/.vessel",
            "/home/me/.vessel/workspace",
        ] {
            assert!(
                check_root(&places, Path::new(refused)).is_err(),
                "{refused}"
            );
        }
        assert!(check_root(&places, Path::new("/home/me/code/app")).is_ok());
        for refused in [
            "/home/me/code/app/.git",
            "/home/me/code/app/.GIT/hooks",
            "/home/me/code/app/.git./x",
            "/home/me/code/app/git~1",
        ] {
            assert!(
                check_root(&places, Path::new(refused)).is_err(),
                "{refused}"
            );
        }
    }
}
