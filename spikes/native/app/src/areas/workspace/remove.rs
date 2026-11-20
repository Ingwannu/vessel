//! Removing a worktree that would lose work (Paseo's archive warning,
//! GOAT's snapshot before cleanup; the page's `remove-worktree-dialog.tsx`):
//! the node first answers `kept` with every reason that applies; this panel
//! says each, then offers to keep a snapshot and remove, or to discard,
//! saying first what discarding destroys. In place under the worktree's
//! row until the kit has its dialog (track C's).
//!
//! From protocol 40 (ADR 0121, ruling 2) a worktree kept only by its live
//! terminals can be removed with them: the panel counts the live terminals
//! at or under its folder, whatever their place (what the node's removal
//! ends), and says Paseo's bulk close line
//! (`workspace.confirmations.bulk.terminals`, en.ts 727-728) with that
//! count, as the kit's small warning notice above the usual buttons. Those
//! buttons then offer what the other reasons allow. A press reads the
//! count again and removes with `end_terminals: true` only if it is
//! unchanged. A changed count stops the press and the notice says the new
//! one. With a live session in the worktree it is as before: the node
//! keeps the worktree whatever is asked. The page does the same (TRACKER
//! 829; the coordinator's ruling on 829 Q3: no button of its own).

use std::path::Path;

use vessel_app_kit::theme::radius;
use vessel_app_kit::ui::{
    Context, Div, ElementId, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::alert::{Content, Size, Variant, alert};
use vessel_app_kit::{Tokens, Type, TypeExt as _, button};
use vessel_app_link::Command;
use vessel_app_link::terminals::{Ask as TerminalAsk, Request as TerminalRequest};
use vessel_app_link::workspace::{Ask, WorktreeAsk};
use vessel_app_view::terminal_tabs::workspace_places;
use vessel_app_view::workspace::worktrees::{choices_ending_terminals, in_use, removal_choices};
use vessel_proto::WorktreeId;
use vessel_proto::terminals::TerminalList;
use vessel_proto::workspace::{IfLost, KeptReason, RemoveWorktree, WorktreeInfo};

use super::widgets::banner;
use crate::shell::Shell;

/// A removal the node answered `kept`.
pub(crate) struct Removing {
    /// The worktree.
    pub(super) worktree: WorktreeInfo,
    /// What the node said would be lost.
    pub(super) reasons: Vec<KeptReason>,
    /// The removal asked, while out.
    pub(super) busy: Option<IfLost>,
    /// Why the last try removed nothing.
    pub(super) error: Option<String>,
    /// The live terminals at or under its folder, counted from protocol 40
    /// when the node said it is in use; `None` until counted.
    pub(super) terminals: Option<usize>,
    /// The count the person saw when pressing a removal that ends them,
    /// while it is read again.
    pub(super) recount: Option<usize>,
}

impl Removing {
    /// A removal the node kept, its terminals not counted yet.
    pub(super) const fn new(
        worktree: WorktreeInfo,
        reasons: Vec<KeptReason>,
        error: Option<String>,
    ) -> Self {
        Self {
            worktree,
            reasons,
            busy: None,
            error,
            terminals: None,
            recount: None,
        }
    }
}

/// A worktree's branch, in words.
pub(super) fn branch_name(worktree: &WorktreeInfo, detached: &str) -> String {
    worktree
        .branch
        .as_ref()
        .or(worktree.created_branch.as_ref())
        .map_or_else(|| detached.to_owned(), ToString::to_string)
}

impl Shell {
    /// A removal button pressed. When live terminals alone keep the
    /// worktree, their count is read again first and the removal waits
    /// for it (`removal_terminals_listed`).
    fn remove_with(&mut self, if_lost: IfLost, cx: &mut Context<'_, Self>) {
        let Some(machine) = self.projects_machine() else {
            return;
        };
        let closing = self
            .areas
            .workspace
            .page
            .detail
            .as_ref()
            .and_then(|detail| detail.removing.as_ref())
            .and_then(|removing| closing_terminals(self, removing));
        let Some(removing) = self.removing_mut() else {
            return;
        };
        if removing.busy.is_some() {
            return;
        }
        removing.busy = Some(if_lost);
        removing.error = None;
        if closing.is_some() {
            removing.recount = closing;
            self.command(Command::Terminals(TerminalAsk {
                machine,
                request: TerminalRequest::ListAll,
            }));
        } else {
            let worktree = removing.worktree.id;
            self.ask(
                &machine,
                Ask::Worktree(remove_ask(worktree, if_lost, false)),
            );
        }
        cx.notify();
    }

    /// After a removal's answer: when the node kept the worktree in use and
    /// its terminals are not counted yet, count the live terminals in its
    /// folder, from a node at protocol 40 or later.
    pub(super) fn removal_count_terminals(&mut self, machine: &str) {
        let uncounted = self.removing_mut().is_some_and(|removing| {
            in_use(&removing.reasons) && removing.terminals.is_none() && removing.recount.is_none()
        });
        if !uncounted {
            return;
        }
        let protocol = self
            .fleet
            .machines
            .iter()
            .find(|on| on.name == machine)
            .and_then(|on| on.protocol);
        if workspace_places(protocol) {
            self.command(Command::Terminals(TerminalAsk {
                machine: machine.to_owned(),
                request: TerminalRequest::ListAll,
            }));
        }
    }

    /// Every terminal answered: the panel's count, and a press waiting for
    /// it goes on only when the count is unchanged.
    pub(crate) fn removal_terminals_listed(
        &mut self,
        machine: &str,
        list: Option<&TerminalList>,
        cx: &mut Context<'_, Self>,
    ) {
        if self.projects_machine().as_deref() != Some(machine) {
            return;
        }
        let Some(removing) = self.removing_mut() else {
            return;
        };
        let seen = removing.recount.take();
        let Some(list) = list else {
            if seen.is_some() {
                removing.busy = None;
            }
            return;
        };
        let count = live_in_folder(list, &removing.worktree.path);
        removing.terminals = Some(count);
        let go = seen.is_some_and(|seen| seen == count && count > 0);
        if seen.is_some() && !go {
            removing.busy = None;
        }
        let worktree = removing.worktree.id;
        if go && let Some(if_lost) = removing.busy {
            self.ask(
                machine,
                Ask::Worktree(remove_ask(worktree, if_lost, true)),
            );
        }
        cx.notify();
    }

    fn removing_mut(&mut self) -> Option<&mut Removing> {
        self.areas
            .workspace
            .page
            .detail
            .as_mut()
            .and_then(|detail| detail.removing.as_mut())
    }

    /// Whether a live session runs in `worktree`: by its workspace, or by a
    /// folder at or under the worktree's (the node's own test).
    fn live_session_in(&self, worktree: &WorktreeInfo) -> bool {
        let Some(machine) = self.projects_machine() else {
            return false;
        };
        self.fleet
            .machines
            .iter()
            .filter(|on| on.name == machine)
            .flat_map(|on| &on.sessions)
            .any(|card| {
                card.live
                    && (card.detail.worktree == Some(worktree.id)
                        || card
                            .detail
                            .cwd
                            .as_deref()
                            .is_some_and(|cwd| Path::new(cwd).starts_with(&worktree.path)))
            })
    }
}

/// The live terminals in `list` at or under `folder`, whatever their place.
pub(super) fn live_in_folder(list: &TerminalList, folder: &Path) -> usize {
    list.terminals
        .iter()
        .filter(|info| info.is_live() && info.folder.starts_with(folder))
        .count()
}

/// `worktree.remove` as this app asks it, from the row's Remove and from
/// this panel alike.
///
/// `end_terminals` (ADR 0121) is true only from this panel while live
/// terminals alone keep the worktree, after the person saw how many and
/// the count was read again. Without it a live terminal refuses the
/// removal, as before protocol 40.
pub(super) fn remove_ask(
    worktree: WorktreeId,
    if_lost: IfLost,
    end_terminals: bool,
) -> WorktreeAsk {
    WorktreeAsk::Remove(RemoveWorktree {
        worktree,
        if_lost,
        end_terminals,
    })
}

/// The panel.
pub(super) fn render(
    shell: &Shell,
    removing: &Removing,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words();
    let labels = words.labels();
    let branch = branch_name(&removing.worktree, labels.detached);
    let closing = closing_terminals(shell, removing);
    let choices = if closing.is_some() {
        choices_ending_terminals(&removing.reasons)
    } else {
        removal_choices(&removing.reasons)
    };
    let snapshotable = choices.contains(&IfLost::Snapshot);
    let mut reasons = div().flex().flex_col().gap(px(2.0));
    for reason in &removing.reasons {
        reasons = reasons.child(
            div()
                .type_(Type::Body)
                .child(format!("\u{2022} {}", words.kept_reason(*reason))),
        );
    }
    let explain = if closing.is_none() && in_use(&removing.reasons) {
        labels.end_session_first.to_owned()
    } else {
        let snapshot = if snapshotable {
            labels.snapshot_keeps
        } else {
            labels.snapshot_cannot
        };
        let created = removing
            .worktree
            .created_branch
            .as_ref()
            .map_or_else(String::new, ToString::to_string);
        format!(
            "{snapshot} {}",
            words.discard_says(&removing.worktree.path.display().to_string(), &created)
        )
    };
    let mut card = div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .p(px(14.0))
        .rounded(px(radius::R4))
        .border_1()
        .border_color(tokens.neutral.border)
        .bg(tokens.neutral.surface1)
        .child(div().type_(Type::Title).child(words.remove_title(&branch)))
        .child(div().type_(Type::Body).child(labels.would_lose))
        .child(reasons)
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .child(explain),
        );
    if let Some(count) = closing {
        // Above the buttons, as Paseo's confirm says the cost before its
        // button: the kit's notice, as the page's Alert (sm warning).
        let line = labels
            .close_terminals_line
            .replace("{terminals}", &count.to_string());
        let content = Content {
            title: None,
            description: Some(div().child(line).into_any_element()),
            action: None,
        };
        card = card.child(alert(content, Variant::Warning, Size::Small, tokens));
    }
    if let Some(error) = &removing.error {
        card = card.child(banner(error.clone(), true, None, tokens));
    }
    card.child(buttons(shell, removing, snapshotable, &choices, tokens, cx))
}

/// The live terminals the removal would close, when only they keep the
/// worktree: the node said it is in use, the count is above none, and no
/// live session runs in it.
fn closing_terminals(shell: &Shell, removing: &Removing) -> Option<usize> {
    if !in_use(&removing.reasons) || shell.live_session_in(&removing.worktree) {
        return None;
    }
    removing.terminals.filter(|count| *count > 0)
}

/// Cancel, discard, and keep a snapshot: each only when the node's reasons
/// allow it and nothing is out.
fn buttons(
    shell: &Shell,
    removing: &Removing,
    snapshotable: bool,
    choices: &[IfLost],
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let labels = shell.words().labels();
    let busy = removing.busy.is_some();
    let discard = choices.contains(&IfLost::Discard) && !busy;
    let snapshot = snapshotable && !busy;
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .justify_end()
        .gap(px(8.0))
        .child(
            button("remove-cancel", labels.cancel, false, tokens).on_click(cx.listener(
                |this, _event, _window, cx| {
                    if let Some(detail) = this.areas.workspace.page.detail.as_mut() {
                        detail.removing = None;
                    }
                    cx.notify();
                },
            )),
        )
        .child(
            button(
                ElementId::Name("remove-discard".into()),
                labels.discard_and_remove,
                false,
                tokens,
            )
            .text_color(tokens.neutral.destructive)
            .opacity(if discard { 1.0 } else { 0.5 })
            .on_click(cx.listener(move |this, _event, _window, cx| {
                if discard {
                    this.remove_with(IfLost::Discard, cx);
                }
            })),
        )
        .child(
            button(
                "remove-snapshot",
                labels.keep_snapshot_and_remove,
                true,
                tokens,
            )
            .opacity(if snapshot { 1.0 } else { 0.5 })
            .on_click(cx.listener(move |this, _event, _window, cx| {
                if snapshot {
                    this.remove_with(IfLost::Snapshot, cx);
                }
            })),
        )
}

#[cfg(test)]
#[path = "remove_tests.rs"]
mod tests;
