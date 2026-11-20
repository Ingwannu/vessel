//! Removing a worktree that would lose work (Paseo's archive warning,
//! GOAT's snapshot before cleanup; the page's `remove-worktree-dialog.tsx`):
//! the node first answers `kept` with every reason that applies; this panel
//! says each, then offers to keep a snapshot and remove, or to discard,
//! saying first what discarding destroys. In place under the worktree's
//! row until the kit has its dialog (track C's).

use vessel_app_kit::theme::radius;
use vessel_app_kit::ui::{
    Context, Div, ElementId, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div,
    px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, button};
use vessel_app_link::workspace::{Ask, WorktreeAsk};
use vessel_app_view::workspace::worktrees::{in_use, removal_choices};
use vessel_proto::WorktreeId;
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
    fn remove_with(&mut self, if_lost: IfLost, cx: &mut Context<'_, Self>) {
        let Some(machine) = self.projects_machine() else {
            return;
        };
        let Some(removing) = self
            .areas
            .workspace
            .page
            .detail
            .as_mut()
            .and_then(|detail| detail.removing.as_mut())
        else {
            return;
        };
        if removing.busy.is_some() {
            return;
        }
        removing.busy = Some(if_lost);
        removing.error = None;
        let worktree = removing.worktree.id;
        self.ask(&machine, Ask::Worktree(remove_ask(worktree, if_lost)));
        cx.notify();
    }
}

/// `worktree.remove` as this app asks it, from the row's Remove and from
/// this panel alike.
///
/// `end_terminals` stays false (ADR 0121): ending a worktree's live
/// terminals is the person's choice, made after the app shows how many, and
/// the native app does not ask it yet. Without it a live terminal refuses
/// the removal, as before protocol 40. One place, so the day the app asks,
/// both callers change together.
pub(super) fn remove_ask(worktree: WorktreeId, if_lost: IfLost) -> WorktreeAsk {
    WorktreeAsk::Remove(RemoveWorktree {
        worktree,
        if_lost,
        end_terminals: false,
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
    let choices = removal_choices(&removing.reasons);
    let snapshotable = choices.contains(&IfLost::Snapshot);
    let mut reasons = div().flex().flex_col().gap(px(2.0));
    for reason in &removing.reasons {
        reasons = reasons.child(
            div()
                .type_(Type::Body)
                .child(format!("\u{2022} {}", words.kept_reason(*reason))),
        );
    }
    let explain = if in_use(&removing.reasons) {
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
    if let Some(error) = &removing.error {
        card = card.child(banner(error.clone(), true, None, tokens));
    }
    card.child(buttons(shell, removing, snapshotable, &choices, tokens, cx))
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
