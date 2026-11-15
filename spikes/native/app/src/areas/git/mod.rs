//! Git's actions at the head of the code panel's Changes tab: Paseo's
//! repository toolbar (`git/diff-pane.tsx` `ChangesRepositoryToolbar`:
//! the branch switcher, `components/branch-switcher.tsx`, then the git
//! actions split button, `git/actions-split-button.tsx`), with GOAT's
//! commit form (`components/GitActionsControl.tsx`) and the page's
//! branch-name and saved-entry forms for what the node offers beyond
//! Paseo's quick actions. What each action does is decided in
//! `view::workspace::git`; this area keeps state, wires and draws.
//!
//! [Decision Log]
//! - Purpose: commit, push, pull, fetch, switch, create and rename a
//!   branch, stash, all from the panel that shows the changes.
//! - Constraints: the code panel is track F's; it gives one slot at the
//!   head of its Changes tab (`changes::git_head`) and a read of what it
//!   shows (`Shell::code_git_context`). The node's workspace answers echo
//!   the root (reads) or the exact write; they carry no app key.
//! - Alternatives: a keyed git channel in the link (a new command and
//!   event, and their arms in the shell's router and the unsent settler,
//!   all shared files); or matching answers by root alone.
//! - Choice: the existing workspace channel. A read is accepted when its
//!   root is the one shown and a read of it is out (`Reading::outstanding`
//!   gives the key); a write when it equals the one write in flight.
//! - Why: no shared file grows for it, and what the keys protect against
//!   still holds: a different root's late answer is refused, and only
//!   one write per repository is ever out.
//! - Cost: after leaving a repository and coming back while its read is
//!   out, the older read is taken for the newer: the same repository a
//!   moment earlier, read again five seconds later.

mod answers;
mod branch;
pub(crate) mod commands;
mod commit;
mod commit_scope;
mod head;
mod menu;
mod name;
mod origin;
pub(crate) mod panel;
mod retirement;
mod stashes;

use vessel_app_kit::ui::{Context, Window};
use vessel_app_kit::{ComboboxState, DialogState, MenuState, TextField, VirtualList};
use vessel_app_link::workspace::{Ask, GitAsk, GitWrite};
use vessel_app_view::toasts::ToastTone;
use vessel_app_view::workspace::git::{
    BranchMode, DirectWrite, Issue, PanelVisibility, REFRESH_EVERY, Reading, RequestKey,
    RequestKeys, Step, Writing, should_refresh,
};
use vessel_proto::workspace::{BranchList, CommitStage, GitStatus, StashList, WorkspaceRoot};

use crate::shell::Shell;

pub(crate) use head::slot;
pub(crate) use menu::{failure_body, glyph};
pub(crate) use retirement::start;

/// The repository the area follows: the one the Changes tab shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Target {
    pub(crate) machine: String,
    pub(crate) root: WorkspaceRoot,
}

/// What is open over the toolbar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Open {
    /// GOAT's commit form, with the facts it was opened on.
    Commit,
    /// A branch's name: create from here, or rename the current one.
    Branch(BranchMode),
    /// The saved entries.
    Stashes,
}

/// The area's state.
pub(crate) struct GitArea {
    pub(crate) target: Option<Target>,
    keys: RequestKeys,
    panels: panel::Panels,
    commands_generation: Option<RequestKey>,
    pub(crate) status: Reading<GitStatus>,
    pub(crate) branches: Reading<BranchList>,
    pub(crate) stashes: Reading<StashList>,
    pub(crate) writing: Writing,
    /// The one write out, as sent: its answer echoes it.
    pending: Option<(RequestKey, GitWrite)>,
    /// What went wrong last, shown where it was asked from.
    pub(crate) issue: Option<Issue>,
    pub(crate) open: Option<Open>,
    pub(crate) menu: MenuState,
    pub(crate) switcher: ComboboxState,
    /// The commit form's scope list and where its select was drawn.
    pub(crate) scope_list: ComboboxState,
    pub(crate) scope_anchor: vessel_app_kit::select::Anchor,
    pub(crate) dialog: DialogState,
    pub(crate) message: TextField,
    pub(crate) name: TextField,
    pub(crate) stage: CommitStage,
    pub(crate) files: VirtualList,
    /// A switch refused for local changes waits for "Stash & Switch".
    pub(crate) confirm_switch: Option<vessel_proto::workspace::Switch>,
    /// A switch waits for the save made for it to succeed.
    then_switch: Option<vessel_proto::workspace::Switch>,
    /// A dropped entry waits for its confirmation.
    pub(crate) dropping: Option<vessel_app_view::workspace::git::DropStash>,
    /// The refresh loop runs (one per area, for its life).
    polling: bool,
    /// The toolbar's tooltips and where each trigger was drawn.
    pub(crate) tips: vessel_app_kit::Tooltips,
    pub(crate) branch_tip: vessel_app_kit::tooltip::Place,
    /// The branch name's own box in the header and in the commit form, as
    /// last drawn: the room its one line is cut to.
    pub(crate) branch_room: vessel_app_kit::tooltip::Place,
    pub(crate) commit_branch_room: vessel_app_kit::tooltip::Place,
    pub(crate) commit_branch_tip: vessel_app_kit::tooltip::Place,
    /// The saved-entry list's text column, as last drawn.
    pub(crate) stash_room: vessel_app_kit::tooltip::Place,
    pub(crate) actions_tip: vessel_app_kit::tooltip::Place,
    /// The split button's bounds: the menu opens under its right edge
    /// from either half.
    pub(crate) actions_anchor: vessel_app_kit::tooltip::Place,
}

impl GitArea {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            target: None,
            keys: RequestKeys::default(),
            panels: panel::Panels::default(),
            commands_generation: None,
            status: Reading::Empty,
            branches: Reading::Empty,
            stashes: Reading::Empty,
            writing: Writing::default(),
            pending: None,
            issue: None,
            open: None,
            menu: MenuState::new(cx),
            switcher: ComboboxState::new(window, cx),
            scope_list: ComboboxState::new(window, cx),
            scope_anchor: vessel_app_kit::select::Anchor::default(),
            dialog: DialogState::new(cx),
            message: TextField::form_multiline(
                "",
                |shell: &mut Shell, text, window, cx| shell.git_commit_submitted(&text, window, cx),
                window,
                cx,
            ),
            name: TextField::new(
                "",
                |shell: &mut Shell, text, window, cx| shell.git_branch_submitted(&text, window, cx),
                window,
                cx,
            ),
            stage: CommitStage::Everything,
            files: VirtualList::new(false),
            confirm_switch: None,
            then_switch: None,
            dropping: None,
            polling: false,
            tips: vessel_app_kit::Tooltips::new(),
            branch_tip: vessel_app_kit::tooltip::Place::default(),
            branch_room: vessel_app_kit::tooltip::Place::default(),
            commit_branch_room: vessel_app_kit::tooltip::Place::default(),
            commit_branch_tip: vessel_app_kit::tooltip::Place::default(),
            stash_room: vessel_app_kit::tooltip::Place::default(),
            actions_tip: vessel_app_kit::tooltip::Place::default(),
            actions_anchor: vessel_app_kit::tooltip::Place::default(),
        }
    }
}

impl Shell {
    /// The Changes tab shows `target` now: follow it, reading its status
    /// once; what belonged to another repository is let go.
    fn git_follow(&mut self, target: Target, cx: &mut Context<'_, Self>) {
        if self.areas.git.panels.owner_problem().is_some() {
            return;
        }
        let git = &mut self.areas.git;
        if git.target.as_ref() == Some(&target) {
            return;
        }
        git.target = Some(target);
        git.status = Reading::Empty;
        git.branches = Reading::Empty;
        git.stashes = Reading::Empty;
        git.writing = Writing::default();
        git.pending = None;
        git.issue = None;
        git.commands_generation = match git.keys.issue() {
            Ok(key) => Some(key),
            Err(_exhausted) => {
                git.issue = Some(Issue::Keys);
                None
            }
        };
        git.open = None;
        git.dropping = None;
        git.confirm_switch = None;
        git.then_switch = None;
        git.menu.close();
        git.switcher.close();
        self.git_read_status();
        if !self.areas.git.polling {
            self.areas.git.polling = true;
            Self::git_poll(cx);
        }
    }

    /// Paseo's and the page's five-second refresh while the Changes tab
    /// shows a repository, so an agent's edits turn the menu to Commit.
    fn git_poll(cx: &mut Context<'_, Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(REFRESH_EVERY).await;
                let alive = this.update(cx, |shell, _cx| {
                    let visibility = match (shell.code_git_context(), &shell.areas.git.target) {
                        (Some((machine, root)), Some(target))
                            if machine == target.machine && *root == target.root =>
                        {
                            PanelVisibility::Shown
                        }
                        _ => PanelVisibility::Hidden,
                    };
                    let git = &shell.areas.git;
                    // A form over the panel keeps the facts it opened on:
                    // a refresh would take its Commit button away for the
                    // read's length every five seconds.
                    let visibility = if git.open.is_some()
                        || git.confirm_switch.is_some()
                        || git.dropping.is_some()
                    {
                        PanelVisibility::Hidden
                    } else {
                        visibility
                    };
                    // A background read changes nothing drawn (the facts
                    // shown stay; only a running write greys a row), so it
                    // asks for no frame: an idle panel draws nothing.
                    if should_refresh(&git.status, &git.writing, visibility) {
                        shell.git_read_status();
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    /// Ask for the followed repository's status, what is shown kept until
    /// the answer replaces it.
    pub(crate) fn git_read_status(&mut self) {
        if self.areas.git.panels.owner_problem().is_some() {
            return;
        }
        let Some(target) = self.areas.git.target.clone() else {
            return;
        };
        let git = &mut self.areas.git;
        let Ok(key) = git.keys.issue() else {
            git.issue = Some(Issue::Keys);
            return;
        };
        git.status.start(key);
        self.ask(&target.machine, Ask::Git(GitAsk::Status(target.root)));
    }

    /// Ask for the branch list (the switcher opened).
    pub(crate) fn git_read_branches(&mut self) {
        if self.areas.git.panels.owner_problem().is_some() {
            return;
        }
        let Some(target) = self.areas.git.target.clone() else {
            return;
        };
        let git = &mut self.areas.git;
        let Ok(key) = git.keys.issue() else {
            git.issue = Some(Issue::Keys);
            return;
        };
        git.branches.start(key);
        self.ask(&target.machine, Ask::Git(GitAsk::Branches(target.root)));
    }

    /// Ask for the saved entries (their list opened, or one changed).
    fn git_read_stashes(&mut self) {
        if self.areas.git.panels.owner_problem().is_some() {
            return;
        }
        let Some(target) = self.areas.git.target.clone() else {
            return;
        };
        let git = &mut self.areas.git;
        let Ok(key) = git.keys.issue() else {
            git.issue = Some(Issue::Keys);
            return;
        };
        git.stashes.start(key);
        self.ask(&target.machine, Ask::Git(GitAsk::Stashes(target.root)));
    }

    /// Do what choosing `step` means (the menu, the primary half).
    fn git_step(&mut self, step: Step, window: &mut Window, cx: &mut Context<'_, Self>) {
        match step {
            Step::Explain(reason) => {
                let words = self.t.lang.git_unavailable(reason);
                self.toast(words, ToastTone::Info, cx);
            }
            Step::Commit => self.git_open_commit(window, cx),
            Step::Switch => self.git_open_switcher(window, cx),
            Step::Branch(mode) => self.git_open_branch(mode, window, cx),
            Step::Stashes => self.git_open_stashes(window, cx),
            Step::Write(write, intent) => {
                let write = match write {
                    DirectWrite::Push(ask) => GitWrite::Push(ask),
                    DirectWrite::Pull(ask) => GitWrite::Pull(ask),
                    DirectWrite::Fetch(ask) => GitWrite::Fetch(ask),
                    DirectWrite::Stash(ask) => GitWrite::Stash(ask),
                };
                self.git_write(write, intent, cx);
            }
        }
        cx.notify();
    }

    /// Close whatever is open over the toolbar.
    fn git_close(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let git = &mut self.areas.git;
        git.open = None;
        git.dropping = None;
        git.confirm_switch = None;
        git.issue = None;
        cx.notify();
    }

    /// The success phase of `key` ends (Paseo's one second).
    fn git_success_ends(key: RequestKey, cx: &mut Context<'_, Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(vessel_app_view::workspace::git::SUCCESS_DISPLAY)
                .await;
            let _gone = this.update(cx, |shell, cx| {
                if shell.areas.git.writing.expire_success(key) {
                    cx.notify();
                }
            });
        })
        .detach();
    }
}
