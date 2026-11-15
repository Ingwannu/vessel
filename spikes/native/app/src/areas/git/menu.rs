//! The split button's menu (Paseo's `GitActionMenuItem` rows in
//! `git/actions-split-button.tsx`) and the failure shown under the row.
//!
//! The kit's menu does not yet draw Paseo's leading glyph, muted row,
//! pending spinner or success check (G2's menu controls, on the lead's
//! merge list). Until they land the rows keep Paseo's behaviour: an
//! unavailable row is chosen and explains itself in an Info toast; a row
//! running says so in its words; all rows wait while a write is out.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::menu::{MenuHandlers, MenuItem, menu_right};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, IntoElement as _, ParentElement as _, Pixels, Point,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, alert, button_with_kind,
};
use vessel_app_view::workspace::git::{Action, Issue, Unavailable, rows, step};
use vessel_app_view::workspace::git_quick::GitAction;
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::pull::{MenuEntry, PullEntry, merge_label, with_pull};
use vessel_app_view::{GitFailureText, Lang};
use vessel_proto::forge::MergeMethod;
use vessel_proto::workspace::GitStatus;

use crate::shell::Shell;

/// A failure's detail (a hook's output) shows its first lines and no more,
/// so a long log never pushes the menu past a short window.
const DETAIL_MOST: f32 = 160.0;

/// Paseo's `git/action-icons.tsx` glyph for an action.
pub(super) const fn glyph(action: Action) -> Glyph {
    match action {
        Action::Quick(GitAction::Commit) => Glyph::Commit,
        Action::Quick(GitAction::Push) => Glyph::Upload,
        Action::Quick(GitAction::Pull) => Glyph::Download,
        Action::Quick(GitAction::Fetch) => Glyph::Refresh,
        Action::SwitchBranch => Glyph::Branch,
        Action::CreateBranch => Glyph::GitBranchPlus,
        Action::RenameBranch => Glyph::Pencil,
        Action::SaveStash(_) | Action::Stashes => Glyph::Archive,
    }
}

/// The menu's rows, in the view's order, as the kit draws them.
fn items(shell: &Shell) -> Vec<MenuItem> {
    let git = &shell.areas.git;
    let lang = shell.t.lang;
    let Some(status) = git.status.shown() else {
        return Vec::new();
    };
    let running = git.writing.intent();
    entries(shell, status)
        .into_iter()
        .map(|entry| match entry {
            MenuEntry::Git(row) => {
                let label = match running {
                    Some(intent) if intent.action() == Some(row.action) => lang.git_pending(intent),
                    Some(_) | None => lang.git_action_label(row.action, status),
                };
                MenuItem {
                    label: label.into(),
                    tone: vessel_app_kit::menu::Tone::Plain,
                    // Paseo's `isActionDisabled`: only a running write takes
                    // a row out of reach; any other reason is said on choice.
                    disabled: row.unavailable == Some(Unavailable::Writing),
                    starts_group: row.starts_group,
                }
            }
            MenuEntry::Pull(pull) => pull_item(shell, &pull),
            MenuEntry::Merge(method) => merge_item(shell, method),
        })
        .collect()
}

/// The git rows with the pull request rows after the remote ones.
fn entries(shell: &Shell, status: &GitStatus) -> Vec<MenuEntry> {
    let git = &shell.areas.git;
    with_pull(
        rows(&git.status, &git.writing),
        shell.pulls_entry(status),
        &shell.pulls_merges(status),
    )
}

/// Paseo's "Merge PR (squash)" and its kin ("Merging PR..." on the one
/// running); every merge row waits while a merge or a git write is out.
fn merge_item(shell: &Shell, method: MergeMethod) -> MenuItem {
    let merging = shell.areas.pulls.merging.as_deref();
    let label = match merging {
        Some(running) if running.method == method => shell.words.pulls().merging(),
        Some(_) | None => merge_label(method, shell.words),
    };
    MenuItem {
        label: label.into(),
        tone: vessel_app_kit::menu::Tone::Plain,
        disabled: shell.areas.git.writing.busy() || merging.is_some(),
        starts_group: false,
    }
}

/// Paseo's pull request row: "View PR" with one read, else "Create PR"
/// ("Creating PR..." while it is out); it waits while a git write runs
/// (the node pushes the branch before opening one).
fn pull_item(shell: &Shell, entry: &PullEntry) -> MenuItem {
    let words = shell.words.pulls();
    let sending = shell
        .areas
        .pulls
        .form
        .as_ref()
        .is_some_and(|form| form.sending.is_some());
    let label = match entry {
        PullEntry::View { .. } => words.view(),
        PullEntry::Create | PullEntry::Unavailable(_) | PullEntry::Hidden if sending => {
            words.creating()
        }
        PullEntry::Create | PullEntry::Unavailable(_) | PullEntry::Hidden => words.create(),
    };
    MenuItem {
        label: label.into(),
        tone: vessel_app_kit::menu::Tone::Plain,
        disabled: shell.areas.git.writing.busy() || sending,
        starts_group: true,
    }
}

/// The menu, when open, under the split button's right edge.
pub(super) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let git = &shell.areas.git;
    if !git.menu.is_open() {
        return None;
    }
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.git.menu,
        chose: std::rc::Rc::new(|shell: &mut Shell, index, window, cx| {
            let Some(status) = shell.areas.git.status.shown().cloned() else {
                return;
            };
            match entries(shell, &status).get(index) {
                Some(MenuEntry::Git(row)) => shell.git_choose(row.action, window, cx),
                Some(MenuEntry::Pull(_)) => {
                    shell.areas.git.menu.close();
                    shell.pulls_chosen(window, cx);
                }
                Some(MenuEntry::Merge(method)) => {
                    let method = *method;
                    shell.areas.git.menu.close();
                    shell.pulls_merge(method, cx);
                }
                None => {}
            }
        }),
    };
    menu_right(
        "git-menu",
        &git.menu,
        &items(shell),
        &handlers,
        tokens,
        touch,
        cx,
    )
}

impl Shell {
    /// Choose `action`, from the menu or the primary half.
    pub(super) fn git_choose(
        &mut self,
        action: Action,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(root) = self.areas.git.target.as_ref().map(|target| target.root) else {
            return;
        };
        let git = &self.areas.git;
        let next = step(action, root, &git.status, &git.writing);
        self.git_step(next, window, cx);
    }

    /// The caret was pressed at `at`.
    pub(super) fn git_open_menu(
        &mut self,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let git = &mut self.areas.git;
        if git.menu.is_open() {
            git.menu.close();
        } else {
            git.switcher.close();
            git.menu.open(at, window, cx);
            // The merge rows need the repository's methods.
            self.pulls_menu_opened();
        }
        cx.notify();
    }

    /// The glyph-only primary half opens the same menu at the same corner.
    pub(super) fn git_open_menu_at_trigger(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(bounds) = self.areas.git.actions_anchor.get() else {
            return;
        };
        self.git_open_menu(bounds.bottom_right(), window, cx);
    }
}

/// A failure's three parts: what happened, what to do, git's own words
/// under them in the fixed-width face.
pub(crate) fn failure_body(text: &GitFailureText, tokens: &Tokens) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S1))
        .child(text.next.clone())
        .children(
            text.detail
                .as_ref()
                .filter(|detail| !detail.trim().is_empty())
                .map(|detail| {
                    div()
                        .mt(px(spacing::S1))
                        .max_h(px(DETAIL_MOST))
                        .overflow_hidden()
                        .type_(Type::Mono)
                        .text_color(tokens.neutral.foreground)
                        .whitespace_normal()
                        .child(detail.clone())
                }),
        )
        .into_any_element()
}

/// The last failure or refusal of a menu's write, under the row.
pub(super) fn issue(
    shell: &Shell,
    tokens: &Tokens,
    _window: &mut Window,
    _cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    let issue = shell.areas.git.issue.as_ref()?;
    Some(issue_alert(shell.t.lang, issue, tokens))
}

/// One issue as the kit's alert.
pub(super) fn issue_alert(lang: Lang, issue: &Issue, tokens: &Tokens) -> Div {
    let text = lang.git_issue(issue);
    alert::alert(
        alert::Content {
            title: Some(text.title.clone().into()),
            description: Some(failure_body(&text, tokens)),
            action: None,
        },
        crate::badge::alert_variant(issue.alert_variant()),
        alert::Size::Small,
        tokens,
    )
}

/// The status could not be read: why, and the way to read it again.
pub(super) fn read_failed(
    shell: &Shell,
    problem: &Problem,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let lang = shell.t.lang;
    let text = lang.git_call_failure(problem);
    let retry = button_with_kind(
        "git-retry",
        lang.git_labels().retry,
        ButtonKind::Secondary,
        ButtonState::Enabled,
        tokens,
    )
    .on_click(cx.listener(|shell, _event, _window, cx| {
        shell.git_read_status();
        cx.notify();
    }));
    alert::alert(
        alert::Content {
            title: Some(text.title.clone().into()),
            description: Some(failure_body(&text, tokens)),
            action: Some(retry.into_any_element()),
        },
        crate::badge::alert_variant(Issue::Call(problem.clone()).alert_variant()),
        alert::Size::Small,
        tokens,
    )
}
