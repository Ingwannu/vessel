//! The node's answers to the area's reads and writes, taken only for the
//! repository followed (`mod.rs` says how they are matched).

use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::{GitWrite, GitWritten};
use vessel_app_view::toasts::ToastTone;
use vessel_app_view::workspace::git::{
    Dialog, Issue, WriteIntent, Written, commit_files, offers_stash, saved,
};
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::workspace::{BranchList, GitStatus, StashList, WorkspaceRoot};

use super::Open;
use crate::shell::Shell;

/// The view's reading of a write's typed outcome.
fn written(outcome: GitWritten) -> Written {
    match outcome {
        GitWritten::Status(value) => value.into(),
        GitWritten::Commit(value) => value.into(),
        GitWritten::Fetch(value) => value.into(),
        GitWritten::Push(value) => value.into(),
        GitWritten::Pull(value) => value.into(),
        GitWritten::Switch(value) => value.into(),
        GitWritten::Rename(value) => value.into(),
        GitWritten::Stash(value) => value.into(),
    }
}

impl Shell {
    fn git_follows(&self, machine: &str, root: &WorkspaceRoot) -> bool {
        self.areas
            .git
            .target
            .as_ref()
            .is_some_and(|target| target.machine == machine && target.root == *root)
    }

    /// `git.status` answered.
    ///
    /// Returns whether what is drawn changed: the five-second refresh of an
    /// unchanged repository must not redraw the window (measured: 0.45 s of
    /// a core per refresh under a software renderer, 9.7 % idle CPU with the
    /// panel open, against 0.6 % closed).
    pub(crate) fn git_status_read(
        &mut self,
        machine: &str,
        root: &WorkspaceRoot,
        result: Result<GitStatus, Problem>,
    ) -> bool {
        if !self.git_follows(machine, root) {
            return false;
        }
        let git = &mut self.areas.git;
        let unchanged = matches!(
            (&result, git.status.shown()),
            (Ok(read), Some(shown)) if read == shown
        );
        if let Some(key) = git.status.outstanding() {
            let _taken = git.status.accept(key, result);
        }
        // A form opened on the earlier facts keeps its draft and shows the
        // new ones (`Dialog::refresh_commit`); its list follows the count.
        if let (Some(Open::Commit), Some(status)) = (&git.open, git.status.ready()) {
            git.files.reset(commit_files(status, git.stage).count());
        }
        !unchanged
    }

    /// `git.branches` answered.
    pub(crate) fn git_branches_read(
        &mut self,
        machine: &str,
        root: &WorkspaceRoot,
        result: &Result<BranchList, Problem>,
    ) {
        if !self.git_follows(machine, root) {
            return;
        }
        let git = &mut self.areas.git;
        if let Some(key) = git.branches.outstanding() {
            let _taken = git.branches.accept(key, result.clone());
        }
    }

    /// `git.stashes` answered.
    pub(crate) fn git_stashes_read(
        &mut self,
        machine: &str,
        root: &WorkspaceRoot,
        result: Result<StashList, Problem>,
    ) {
        if !self.git_follows(machine, root) {
            return;
        }
        let git = &mut self.areas.git;
        if let Some(key) = git.stashes.outstanding() {
            let _taken = git.stashes.accept(key, result);
        }
    }

    /// A git write answered: the one out, or nothing.
    pub(crate) fn git_written(
        &mut self,
        machine: &str,
        write: &GitWrite,
        result: Result<GitWritten, Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.git_follows(machine, write.root()) {
            return;
        }
        let git = &mut self.areas.git;
        let Some(key) = git
            .pending
            .as_ref()
            .filter(|(_, sent)| sent == write)
            .map(|(key, _)| *key)
        else {
            return;
        };
        git.pending = None;
        let intent = git.writing.intent().cloned();
        let Some(result) = git.writing.finish(key, result.map(written)) else {
            return;
        };
        if self.git_switch_follow_up(write, intent.as_ref(), &result, cx) {
            return;
        }
        let lang = self.t.lang;
        let dialog = self.git_dialog();
        let next = dialog
            .as_ref()
            .and_then(|dialog| dialog.after_write(&result));
        let git = &mut self.areas.git;
        match &result {
            Ok(Written::Said(_)) => {
                git.issue = None;
                if let Some(receipt) = git.writing.success() {
                    let said = lang.git_success(receipt).message;
                    let key = receipt.key();
                    self.toast(said, ToastTone::Ok, cx);
                    Self::git_success_ends(key, cx);
                }
            }
            Ok(Written::Status(_)) => self.areas.git.issue = None,
            Ok(Written::Refused(refusal)) => {
                self.git_failed(Issue::Refused(refusal.clone()));
            }
            Err(problem) => self.git_failed(Issue::Call(problem.clone())),
        }
        // The view's rule: a refusal keeps the form it came from; success
        // closes it, or returns a confirmed drop to the list it came from.
        if dialog.is_some() {
            let git = &mut self.areas.git;
            match next {
                None => {
                    git.open = None;
                    git.dropping = None;
                }
                Some(Dialog::Stashes) => git.dropping = None,
                Some(
                    Dialog::Commit(_) | Dialog::Branch(_) | Dialog::Discard(_) | Dialog::Drop(_),
                ) => {}
            }
        }
        // What the write changed is read again: this toolbar's facts, and
        // the panel's comparison under it (track F's seam).
        self.git_read_status();
        if self.areas.git.open == Some(Open::Stashes) {
            self.git_read_stashes();
        }
        if let Some(target) = self.areas.git.target.clone() {
            self.code_git_changed(&target.machine, &target.root, cx);
        }
        cx.notify();
    }

    /// Paseo's "Stash & Switch": a switch refused for local changes asks
    /// to stash them (true: the confirmation now explains it); a save made
    /// for a waiting switch sends the switch when it succeeded.
    fn git_switch_follow_up(
        &mut self,
        write: &GitWrite,
        intent: Option<&WriteIntent>,
        result: &Result<Written, Problem>,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if let (GitWrite::Switch(switch), Some(intent)) = (write, intent)
            && offers_stash(intent, result)
        {
            let git = &mut self.areas.git;
            git.issue = None;
            git.confirm_switch = Some(switch.clone());
            self.git_read_status();
            cx.notify();
            return true;
        }
        if let (GitWrite::Stash(_), Some(waiting)) = (write, self.areas.git.then_switch.take())
            && saved(result)
        {
            self.git_read_status();
            self.git_write(GitWrite::Switch(waiting), WriteIntent::SwitchBranch, cx);
            return true;
        }
        false
    }

    /// A refusal or a failure stays beside what was asked: inside the
    /// form, or under the toolbar for a menu's write. Paseo says these in
    /// an error toast (`toastActionError`); the app's rule keeps an error
    /// beside its control, with git's own output under the explanation.
    fn git_failed(&mut self, issue: Issue) {
        self.areas.git.issue = Some(issue);
    }

    /// The open form as the view's dialog, for its after-write rule.
    fn git_dialog(&self) -> Option<Dialog> {
        let git = &self.areas.git;
        if let Some(drop) = &git.dropping {
            return Some(Dialog::Drop(drop.clone()));
        }
        match git.open.as_ref()? {
            Open::Commit => git.status.shown().map(Dialog::commit),
            Open::Branch(mode) => Some(Dialog::Branch(*mode)),
            Open::Stashes => Some(Dialog::Stashes),
        }
    }
}
