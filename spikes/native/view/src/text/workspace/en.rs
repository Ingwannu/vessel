//! The workspace's words in English: the page's, word for word.

use vessel_proto::scripts::{ScriptEntry, ScriptKind};
use vessel_proto::workspace::ServicePort;
use vessel_proto::workspace::{
    ChangeStatus, CloneFailure, CloneStage, ConfigKey, DiffScope, GitOperation, GitRefusal,
    GitSupport, KeptReason, ProjectState, RepositoryConfigReason, SetupFailure, SetupState,
    WorktreeRefusal,
};

use super::{AddForm, Labels, Local, WorkspaceWords, labels, lines};
use crate::workspace::branch_name::BranchNameProblem;
use crate::workspace::changes::ScopeKind;
use crate::workspace::command_changes::{Change, CommandLine, Compared, ComparedWith, ScriptLine};
use crate::workspace::git_quick::{Blocked, GitAction};
use crate::workspace::place::PlaceProblem;
use crate::workspace::problem::Problem;
use crate::workspace::save::SaveFailure;

/// English.
#[derive(Debug, Clone, Copy)]
pub struct En;

/// `count` with its noun, singular when one.
fn counted(count: u64, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// [`counted`] for a length.
fn counted_len(count: usize, one: &str, many: &str) -> String {
    counted(u64::try_from(count).unwrap_or(u64::MAX), one, many)
}

/// `text` with its first letter in capitals.
fn sentence(text: &str) -> String {
    let mut characters = text.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(characters).collect()
    })
}

/// Why the repository's own configuration stopped a run (ADR 0030), and
/// what moves it out of the way.
fn config_lines(key: Option<&ConfigKey>, reason: RepositoryConfigReason) -> (String, String) {
    let named = key.map_or_else(
        || "Its git configuration".to_owned(),
        |key| format!("Its setting {key}"),
    );
    let next = "Vessel runs git without programs a repository's own configuration names. Move the setting to your global git configuration (under includeIf to keep it to this repository), or run this in a terminal.";
    match reason {
        RepositoryConfigReason::Program => {
            lines(format!("{named} names a program this would run."), next)
        }
        RepositoryConfigReason::Layout => lines(
            format!("{named} moves the working tree out of the folder."),
            "Remove it from the repository's configuration to use the folder from Vessel.",
        ),
        RepositoryConfigReason::Credential => lines(
            format!("{named} could send your credential elsewhere."),
            next,
        ),
        RepositoryConfigReason::Unnamable => lines(
            format!("{named} names a driver git cannot turn off for one run."),
            "Rename the driver in the repository's configuration, or run this in a terminal.",
        ),
        RepositoryConfigReason::TooMany => lines(
            "The repository's configuration names more drivers than Vessel turns off in one run.",
            "Remove the ones it does not need, or run this in a terminal.",
        ),
        RepositoryConfigReason::TooLarge => lines(
            "The repository's configuration is larger than Vessel reads before running git.",
            "Trim it, or run this in a terminal.",
        ),
        RepositoryConfigReason::Unknown => lines(
            format!("{named} stopped it, for a reason this page does not know."),
            "Update this page's Vessel to read it.",
        ),
    }
}

/// "setup commands: 1 added and 1 changed", or `None` when the list is the
/// same.
fn list_part(name: &str, list: &[CommandLine]) -> Option<String> {
    let count = |wanted: &str| {
        list.iter()
            .filter(|line| match line {
                CommandLine::Line(line) => match line.change {
                    Change::Added => wanted == "added",
                    Change::Removed => wanted == "removed",
                    Change::Same => false,
                },
                CommandLine::Changed { .. } => wanted == "changed",
            })
            .count()
    };
    let said: Vec<String> = ["added", "changed", "removed"]
        .into_iter()
        .filter_map(|change| {
            let count = count(change);
            (count > 0).then(|| format!("{count} {change}"))
        })
        .collect();
    (!said.is_empty()).then(|| format!("{name} commands: {}", said.join(" and ")))
}

impl En {
    /// "script lint added", "service web changed (its command)".
    fn script_part(self, line: &ScriptLine) -> Option<String> {
        let script = line.script();
        let named = format!(
            "{} {}",
            self.script_kind(script.kind()),
            script.name.as_str()
        );
        match line {
            ScriptLine::Line {
                change: Change::Same,
                ..
            } => None,
            ScriptLine::Line {
                change: Change::Added,
                ..
            } => Some(format!("{named} added")),
            ScriptLine::Line {
                change: Change::Removed,
                ..
            } => Some(format!("{named} removed")),
            ScriptLine::Changed { script, was } => Some(format!(
                "{named} changed ({})",
                self.script_change(was, script)
            )),
        }
    }
}

impl WorkspaceWords for En {
    fn branch_name(&self, problem: BranchNameProblem) -> String {
        match problem {
            BranchNameProblem::Empty => "A branch needs a name.".to_owned(),
            BranchNameProblem::TooLong => "That name is too long.".to_owned(),
            BranchNameProblem::Reserved(name) => {
                format!("\u{201c}{name}\u{201d} is not a branch name git takes.")
            }
            BranchNameProblem::LeadingDash => "A branch name cannot start with a dash.".to_owned(),
            BranchNameProblem::SpaceOrControl => {
                "A branch name cannot hold spaces or control characters.".to_owned()
            }
            BranchNameProblem::Character(character) => {
                format!("A branch name cannot hold \u{201c}{character}\u{201d}.")
            }
            BranchNameProblem::Sequence(sequence) => {
                format!("A branch name cannot hold \u{201c}{sequence}\u{201d}.")
            }
            BranchNameProblem::Slashes => {
                "A branch name cannot start or end with \u{201c}/\u{201d}, or hold two in a row."
                    .to_owned()
            }
            BranchNameProblem::TrailingDot => "A branch name cannot end with a dot.".to_owned(),
            BranchNameProblem::DotPart => "No part of a branch name can start with a dot.".into(),
            BranchNameProblem::LockPart => {
                "No part of a branch name can end with \u{201c}.lock\u{201d}.".to_owned()
            }
        }
    }

    fn place(&self, problem: PlaceProblem) -> String {
        match problem {
            PlaceProblem::NotOnMachine => "That project is not on this machine.".to_owned(),
            PlaceProblem::StillCloning => "The project is still being cloned.".to_owned(),
            PlaceProblem::CloneFailed => {
                "The project's clone failed; clone it again from Settings \u{203a} Projects."
                    .to_owned()
            }
            PlaceProblem::FolderGone => "The project's folder is gone.".to_owned(),
            PlaceProblem::UnknownState => {
                "The project is in a state this page does not know.".to_owned()
            }
            PlaceProblem::NotGit => {
                "This project is not a git repository: run in its folder.".to_owned()
            }
            PlaceProblem::ChooseBranch => "Choose the branch to run on.".to_owned(),
            PlaceProblem::BranchName(problem) => self.branch_name(problem),
        }
    }

    fn files(&self, count: u64) -> String {
        counted(count, "file", "files")
    }

    fn clone_stage(&self, stage: CloneStage) -> &'static str {
        match stage {
            CloneStage::Connecting => "Connecting",
            CloneStage::Counting => "Counting objects",
            CloneStage::Compressing => "Compressing objects",
            CloneStage::Receiving => "Receiving objects",
            CloneStage::Resolving => "Resolving deltas",
            CloneStage::CheckingOut => "Checking out files",
            CloneStage::Unknown => "Cloning",
        }
    }

    fn clone_failure(&self, reason: CloneFailure) -> &'static str {
        match reason {
            CloneFailure::NeedsCredential => {
                "The remote asked for a credential. Sign in to it on that machine (a credential helper or an SSH key), then clone again."
            }
            CloneFailure::NotFound => {
                "No repository was found at that address. Check it and clone again."
            }
            CloneFailure::Network => {
                "The machine could not reach the remote. Check its network, then clone again."
            }
            CloneFailure::TimedOut => "The clone took longer than an hour and was stopped.",
            CloneFailure::Other | CloneFailure::Unknown => {
                "Git could not clone it. What it said is below."
            }
        }
    }

    fn project_state(&self, state: &ProjectState) -> Option<String> {
        match state {
            ProjectState::Ready => None,
            ProjectState::Cloning { stage, percent, .. } => Some(match percent {
                None => self.clone_stage(*stage).to_owned(),
                Some(percent) => format!("{} \u{b7} {percent}%", self.clone_stage(*stage)),
            }),
            ProjectState::CloneFailed { .. } => Some("Clone failed".to_owned()),
            ProjectState::Missing => Some("Folder missing".to_owned()),
            ProjectState::Unknown => Some("Unknown state".to_owned()),
        }
    }

    fn git_support(&self, git: &GitSupport) -> Option<String> {
        match git {
            GitSupport::Found { .. } | GitSupport::Unknown => None,
            GitSupport::TooOld { version, minimum, .. } => Some(format!(
                "Git {}.{} is too old on this machine; Vessel needs {}.{} or newer for projects.",
                version.major, version.minor, minimum.major, minimum.minor
            )),
            GitSupport::Missing => Some(
                "Git is not installed on this machine: folders can be added, but not cloned, and they get no worktrees or changes."
                    .to_owned(),
            ),
        }
    }

    fn setup_failure(&self, failure: &SetupFailure, commands: &[&str]) -> String {
        match failure {
            SetupFailure::Exit { command, code, .. } => {
                let how = code.map_or_else(
                    || "was stopped by a signal".to_owned(),
                    |code| format!("exited with {code}"),
                );
                let named = usize::try_from(*command)
                    .ok()
                    .and_then(|index| commands.get(index));
                match named {
                    Some(named) => format!("`{named}` {how}."),
                    None => format!("Setup command {} {how}.", command.saturating_add(1)),
                }
            }
            SetupFailure::TimedOut => "Setup ran for 30 minutes and was stopped.".to_owned(),
            SetupFailure::Spawn => {
                "The machine could not start the setup command (its shell).".to_owned()
            }
            SetupFailure::Unknown => "Setup failed.".to_owned(),
        }
    }

    fn setup_state(&self, setup: &SetupState) -> &'static str {
        match setup {
            SetupState::None => "No setup",
            SetupState::Skipped => "Setup skipped",
            SetupState::Running { .. } => "Setting up",
            SetupState::Succeeded { .. } => "Set up",
            SetupState::Failed { .. } => "Setup failed",
            SetupState::Unknown => "Unknown",
        }
    }

    fn kept_reason(&self, reason: KeptReason) -> String {
        match reason {
            KeptReason::Uncommitted { files, .. } => format!(
                "{} with uncommitted changes",
                counted(u64::from(files), "file", "files")
            ),
            KeptReason::UniqueCommits { commits, .. } => format!(
                "{} on no other branch",
                counted(u64::from(commits), "commit", "commits")
            ),
            KeptReason::Operation { operation, .. } => {
                sentence(&format!("{} in progress", self.operation(operation)))
            }
            KeptReason::Submodules => "Submodules, whose changes are not inspected".to_owned(),
            KeptReason::FlaggedFiles => {
                "Files marked so that git does not show their changes".to_owned()
            }
            KeptReason::WorktreeRefs => "References of this worktree's own".to_owned(),
            KeptReason::InUse => {
                "A live session, a terminal or service, or setup is using it".to_owned()
            }
            KeptReason::InspectionFailed => "Git could not inspect it".to_owned(),
            KeptReason::Unknown => "Something this page does not know".to_owned(),
        }
    }

    fn worktree_refusal(&self, reason: &WorktreeRefusal) -> String {
        match reason {
            WorktreeRefusal::NotGit => "This project is not a git repository, so it has no worktrees. Run the session in its folder instead.".to_owned(),
            WorktreeRefusal::NotReady => "The project is not ready yet (cloning, or its folder is missing).".to_owned(),
            WorktreeRefusal::NoCommit => "The repository has no commit yet; commit once in its folder first.".to_owned(),
            WorktreeRefusal::BranchExists => "A branch with that name exists. Choose another name, or run on the existing branch.".to_owned(),
            WorktreeRefusal::CheckedOutElsewhere { path, .. } => format!(
                "That branch is checked out in another worktree ({}).",
                path.display()
            ),
            WorktreeRefusal::NoSuchBase => "The base branch was not found. Choose another, or fetch first.".to_owned(),
            WorktreeRefusal::NoSuchBranch => "That branch was not found.".to_owned(),
            WorktreeRefusal::NoSuchSnapshot => "That snapshot is gone.".to_owned(),
            WorktreeRefusal::Limit => "This project has as many worktrees as a machine keeps. Remove one first.".to_owned(),
            WorktreeRefusal::Unknown => "The machine refused to make the worktree for a reason this page does not know.".to_owned(),
        }
    }

    fn operation(&self, operation: GitOperation) -> &'static str {
        match operation {
            GitOperation::Merge => "a merge",
            GitOperation::Rebase => "a rebase",
            GitOperation::CherryPick => "a cherry-pick",
            GitOperation::Revert => "a revert",
            GitOperation::Bisect => "a bisect",
        }
    }

    fn refusal_lines(&self, refusal: &GitRefusal) -> (String, String) {
        match refusal {
            GitRefusal::NeedsCredential => lines(
                "The remote asked for a credential.",
                "Sign in to it on that machine (a credential helper or an SSH key that needs no prompt), then try again.",
            ),
            GitRefusal::TimedOut => lines(
                "Git took too long and was stopped.",
                "Check the machine's network or the remote, then try again.",
            ),
            GitRefusal::NonFastForward => lines(
                "The remote has commits this branch does not.",
                "Pull first, then push again. Vessel never forces a push.",
            ),
            GitRefusal::Diverged => lines(
                "This branch and its upstream have both moved on.",
                "Pulling only fast-forwards; merge or rebase in a terminal or ask the agent to.",
            ),
            GitRefusal::WouldOverwrite { .. } => lines(
                "Local changes would be overwritten.",
                "Commit or stash them first.",
            ),
            GitRefusal::Conflicts { .. } => lines(
                "This left conflicts to resolve.",
                "Resolve them in these files, then stage and commit.",
            ),
            GitRefusal::NothingToCommit => lines(
                "There is nothing to commit.",
                "Stage changes first, or commit everything.",
            ),
            GitRefusal::NoIdentity => lines(
                "Git has no name and email for commits on that machine.",
                "Set user.name and user.email in git's configuration there.",
            ),
            GitRefusal::HookFailed { .. } => lines(
                "A hook of the repository refused it.",
                "Read what the hook said, fix it, and try again.",
            ),
            GitRefusal::NoUpstream => lines("This branch has no upstream.", "Push it to set one."),
            GitRefusal::Detached => lines(
                "No branch is checked out.",
                "Switch to a branch or create one first.",
            ),
            GitRefusal::CheckedOutElsewhere { path, .. } => lines(
                "That branch is checked out in another worktree.",
                format!(
                    "Use it there ({}), or choose another branch.",
                    path.display()
                ),
            ),
            GitRefusal::NothingToStash => lines(
                "There is nothing to stash.",
                "Stash keeps uncommitted changes; there are none.",
            ),
            GitRefusal::StashMoved => lines(
                "The stash changed since it was listed.",
                "Look at the list again and choose the entry once more.",
            ),
            GitRefusal::BranchExists => lines(
                "A branch with that name exists.",
                "Choose another name, or switch to it.",
            ),
            GitRefusal::NoSuchRef => lines(
                "No branch, tag or commit has that name.",
                "Check the name, or fetch first.",
            ),
            GitRefusal::Operation { operation, .. } => lines(
                format!(
                    "The repository is in the middle of {}.",
                    self.operation(*operation)
                ),
                "Finish or abort it first.",
            ),
            GitRefusal::RemoteFailed { .. } => {
                lines("The remote refused it.", "Read what it said below.")
            }
            GitRefusal::RepositoryConfig { key, reason, .. } => config_lines(key.as_ref(), *reason),
            GitRefusal::Unknown => lines(
                "Git refused it for a reason this page does not know.",
                "Update this page's Vessel to read it.",
            ),
        }
    }

    fn action(&self, action: GitAction) -> &'static str {
        match action {
            GitAction::Commit => "Commit",
            GitAction::Push => "Push",
            GitAction::Pull => "Pull",
            GitAction::Fetch => "Fetch",
        }
    }

    fn blocked(&self, blocked: Blocked) -> &'static str {
        match blocked {
            Blocked::Clean => "Worktree is clean. Make changes before committing.",
            Blocked::DetachedPush => "Detached HEAD: check out a branch before pushing.",
            Blocked::DetachedPull => "Detached HEAD: check out a branch before pulling.",
            Blocked::Diverged => "Branch has diverged from upstream. Rebase/merge first.",
            Blocked::Behind => "Branch is behind upstream. Pull/rebase before pushing.",
            Blocked::NothingToPush => "No local commits to push.",
            Blocked::NoUpstream => "This branch has no upstream to pull.",
        }
    }

    fn script_kind(&self, kind: ScriptKind) -> &'static str {
        super::script_words::kind(crate::Lang::En, kind)
    }

    fn script_port(&self, port: Option<ServicePort>) -> String {
        super::script_words::port(crate::Lang::En, port)
    }

    fn script_change(&self, was: &ScriptEntry, now: &ScriptEntry) -> String {
        super::script_words::change(crate::Lang::En, was, now)
    }

    fn compared(&self, compared: &Compared, local: &Local) -> String {
        let Compared::Changed {
            against,
            setup,
            teardown,
            scripts,
        } = compared
        else {
            return "No earlier approval of this project's commands is known, so every command is shown as it is.".to_owned();
        };
        let parts: Vec<String> = [list_part("setup", setup), list_part("teardown", teardown)]
            .into_iter()
            .chain(scripts.iter().map(|line| self.script_part(line)))
            .flatten()
            .collect();
        let changed = if parts.is_empty() {
            "nothing that runs reads differently".to_owned()
        } else {
            parts.join("; ")
        };
        match against {
            ComparedWith::LastApproved { at } => {
                let (today, time) = local.when(*at);
                let when = if today { format!("today {time}") } else { time };
                format!("Since the set approved {when}: {changed}.")
            }
            ComparedWith::Folder => format!(
                "Against the set approved for the project's folder (this machine does not say which set was approved last): {changed}."
            ),
        }
    }

    fn scope_label(&self, scope: &DiffScope) -> String {
        match scope {
            DiffScope::Base { .. } => "Changes".to_owned(),
            DiffScope::Uncommitted => "Uncommitted".to_owned(),
            DiffScope::Staged => "Staged".to_owned(),
            DiffScope::Unstaged => "Unstaged".to_owned(),
            DiffScope::Commit { id, .. } => {
                let short: String = id.as_str().chars().take(7).collect();
                format!("Commit {short}")
            }
        }
    }

    fn scope_detail(&self, kind: ScopeKind) -> &'static str {
        match kind {
            ScopeKind::Base => "Everything since the base branch",
            ScopeKind::Uncommitted => "Not committed yet",
            ScopeKind::Staged => "Staged for the next commit",
            ScopeKind::Unstaged => "Not staged yet",
        }
    }

    fn scope_empty(&self, scope: &DiffScope) -> &'static str {
        match scope {
            DiffScope::Base { .. } => "No changes against base.",
            DiffScope::Uncommitted | DiffScope::Unstaged => "No changes against HEAD.",
            DiffScope::Staged => "Nothing is staged.",
            DiffScope::Commit { .. } => "This commit changes nothing.",
        }
    }

    fn status_word(&self, status: ChangeStatus) -> &'static str {
        match status {
            ChangeStatus::Added => "Added",
            ChangeStatus::Deleted => "Deleted",
            ChangeStatus::Modified => "Modified",
            ChangeStatus::Renamed => "Renamed",
            ChangeStatus::Copied => "Copied",
            ChangeStatus::TypeChanged => "Type changed",
            ChangeStatus::Conflicted => "Conflicted",
            ChangeStatus::Unknown => "Changed",
        }
    }

    fn problem(&self, problem: &Problem) -> String {
        match problem {
            Problem::Node { message, .. } => message.clone(),
            Problem::Older => {
                "This machine's Vessel is too old for this; update it to use it here.".to_owned()
            }
            // The transport's text is not shown (it is English and names
            // the endpoint); the sidebar's tooltip keeps it.
            Problem::Gone(_) => {
                "The machine could not be reached. Check that it is on and its node is running."
                    .to_owned()
            }
            Problem::Unsent => "Could not send. Try again.".to_owned(),
        }
    }

    fn save_failure(&self, failure: &SaveFailure) -> String {
        match failure {
            SaveFailure::NoGuard => "This file was too large to be compared when it was read, so it cannot be saved without overwriting. Reload it and edit again.".to_owned(),
            SaveFailure::Call(problem) => self.problem(problem),
        }
    }

    fn labels(&self) -> &'static Labels {
        &labels::EN
    }

    fn projects_on(&self, machine: &str) -> String {
        format!("Projects on {machine}")
    }

    fn no_projects_body(&self, machine: &str) -> String {
        format!("Add a folder of {machine}, clone a repository there, or start one.")
    }

    fn older_machine(&self, machine: &str) -> String {
        format!("{machine} runs a Vessel older than projects.")
    }

    fn forgot(&self, name: &str) -> String {
        format!("Forgot {name}; its files stay")
    }

    fn not_removed(&self, name: &str, why: &str) -> String {
        format!("{name} was not removed: {why}")
    }

    fn live(&self, count: usize) -> String {
        format!("{count} live")
    }

    fn kept(&self, reason: &str) -> String {
        format!("kept: {reason}")
    }

    fn commands_title(&self, setup: usize, teardown: usize, scripts: usize) -> String {
        let lists = format!("{setup} setup, {teardown} teardown");
        match scripts {
            0 => lists,
            1 => format!("{lists}, 1 script or service"),
            many => format!("{lists}, {many} scripts and services"),
        }
    }

    fn invalid_commands(&self, message: &str) -> String {
        format!("Its vessel.toml does not parse, so nothing in it runs: {message}")
    }

    fn commands_not(&self, approved: bool, why: &str) -> String {
        let what = if approved { "approved" } else { "withdrawn" };
        format!("The commands were not {what}: {why}")
    }

    fn setup_not_started(&self, why: &str) -> String {
        format!("Setup did not start: {why}")
    }

    fn removed(&self, branch: &str, snapshot: Option<&str>) -> String {
        match snapshot {
            None => format!("Removed {branch}"),
            Some(commit) => format!("Removed {branch}; its work is kept in snapshot {commit}"),
        }
    }

    fn remove_title(&self, branch: &str) -> String {
        format!("Remove \"{branch}\"?")
    }

    fn discard_says(&self, path: &str, branch: &str) -> String {
        format!(
            "Discarding deletes the folder at {path}, its uncommitted changes and its branch {branch}; commits on no other branch are lost."
        )
    }

    fn snapshot_not_dropped(&self, name: &str, why: &str) -> String {
        format!("The snapshot of {name} was not dropped: {why}")
    }

    fn add_hint(&self, form: AddForm, machine: &str, clone_target: Option<(&str, &str)>) -> String {
        match (form, clone_target) {
            (AddForm::Clone, None) => {
                format!("A folder on {machine}; the repository gets a folder inside it.")
            }
            (AddForm::Clone, Some((folder, name))) => {
                format!("Repository will be created at {folder}/{name}.")
            }
            (AddForm::New, _) => {
                format!("An empty or new folder on {machine}; git starts a repository there.")
            }
            (AddForm::Folder, _) => format!(
                "An absolute folder on {machine}; inside a repository, its top level is the project."
            ),
        }
    }

    fn approval_copy(&self, place: &str, scripts: bool) -> String {
        let started = if scripts {
            "; a script or service runs when someone starts it"
        } else {
            ""
        };
        format!(
            "The repository's vessel.toml names these commands. Setup and teardown run {place}{started}, as you, with your environment. Approving covers this exact set; if any command changes, you are asked again."
        )
    }

    fn approval_place(&self, project: &str, machine: &str, each: bool) -> String {
        if each {
            format!("in each new worktree of {project} on {machine}")
        } else {
            format!("in a new worktree of {project} on {machine}")
        }
    }

    fn add_lines(&self, first: usize, last: usize) -> String {
        if first == last {
            format!("Add line {first} to chat")
        } else {
            format!("Add lines {first}\u{2013}{last} to chat")
        }
    }

    fn editor_lines(&self, count: usize) -> String {
        counted_len(count, "line", "lines")
    }

    fn found_count(&self, files: usize, searched: u32, complete: bool, capped: bool) -> String {
        let found = counted_len(files, "file", "files");
        if capped {
            format!("The best {found}; type more of the name to narrow them.")
        } else if complete {
            found
        } else {
            format!("{found}, from the first {searched} searched")
        }
    }

    fn grepped_count(&self, lines: usize, files: usize, more: bool) -> String {
        let lines = counted_len(lines, "line", "lines");
        let files = counted_len(files, "file", "files");
        if more {
            format!("The first {lines}, in {files}; there are more. Narrow the search to see them.")
        } else {
            format!("{lines} in {files}")
        }
    }

    fn digest(&self, digest: &str) -> String {
        format!("Digest {digest}")
    }

    fn was(&self, command: &str) -> String {
        format!("was {command}")
    }
}
