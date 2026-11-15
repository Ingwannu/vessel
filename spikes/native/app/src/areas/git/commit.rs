//! GOAT's commit form (`components/GitActionsControl.tsx` 1898–2072,
//! "Commit changes"): a summary card with the branch (and a warning on
//! the default branch) and the files the commit takes, then the message,
//! then Cancel and Commit. Paseo commits without a form (its agent writes
//! the message); the node needs a message, so the form is GOAT's.
//!
//! Differences, each for a reason: the node commits by scope (staged,
//! tracked, everything), so the scope is a select where GOAT has per-file
//! checkboxes; the row names the change in one word where GOAT shows
//! line counts, which the status does not carry; there is no "Commit on
//! new branch" (the node has no such single act; New branch… then Commit
//! does it); the message is required (the node does not write one).

use vessel_app_kit::dialog::{DialogContent, DialogSize};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, breakpoint, button_with_kind, fonts,
    form_summary,
};
use vessel_app_link::workspace::GitWrite;
use vessel_app_view::workspace::git::{
    CommitDestination, CommitForm, Issue, WriteIntent, can_commit, commit_files, commit_request,
    committed_state, project_default,
};
use vessel_app_view::workspace::git_quick::GitAction;
use vessel_proto::workspace::{GitStatus, RefName};

use super::{Open, menu::issue_alert, name};

/// Before the first measure: a card's width less its insets, the label
/// "Branch" and the default-branch warning.
const COMMIT_NAME_ESTIMATE: f32 = 280.0;

/// A file row: the code face's 20-point line and 4 above and below.
const FILE_ROW: f32 = 28.0;

/// GOAT's `max-h-44`: the list grows with its files up to 176 points (the
/// kit's frame is that tall whatever it holds; one file left a tall
/// empty box that pushed the message out of an 800 by 600 window).
const FILES_MOST: f32 = 176.0;

/// A file row's corner: six points, between the kit's `R1` and `R2`; the
/// kit has no radius of this size yet.
const FILE_ROW_RADIUS: f32 = 6.0;

/// Below this window height the list stops at three rows, so the message
/// and the actions stay in view without scrolling (the owner's word 40:
/// nothing is cut at small window sizes).
const SHORT_WINDOW: f32 = 720.0;

/// The list's height for `count` files in a window `tall` points high:
/// its rows and its frame's insets (`spacing::S1` above and below).
fn files_height(count: usize, tall: f32) -> f32 {
    let most_rows: usize = if tall < SHORT_WINDOW { 3 } else { 6 };
    let rows = u16::try_from(count.min(most_rows)).map_or(0.0, f32::from);
    FILE_ROW.mul_add(rows, spacing::S2).min(FILES_MOST)
}
use crate::shell::Shell;

impl Shell {
    /// Open the form on the facts shown now, its scope as the page sets it.
    pub(super) fn git_open_commit(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let placeholder = self.t.lang.git_labels().message_hint;
        let git = &mut self.areas.git;
        // Shown, not only ready: the form may open while a refresh is out
        // (`step`); its Commit waits for the newer read.
        let Some(status) = git.status.shown() else {
            return;
        };
        git.stage = CommitForm::of(status).stage;
        git.files.reset(commit_files(status, git.stage).count());
        git.menu.close();
        git.issue = None;
        git.open = Some(Open::Commit);
        // GOAT empties the message whenever its form closes; a new form
        // starts empty.
        git.message.clear(window, cx);
        git.message.set_placeholder(placeholder, window, cx);
        git.dialog.focus(window, cx);
        git.message.focus(window, cx);
        // The branch list says which branch is the project's default (the
        // form's warning, as GOAT's).
        self.git_read_branches();
    }

    /// Mod+Enter in the message, or the Commit button.
    pub(super) fn git_commit_submitted(
        &mut self,
        message: &str,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let git = &mut self.areas.git;
        let (Some(Open::Commit), Some(target)) = (git.open.clone(), git.target.clone()) else {
            return;
        };
        if !can_commit(&git.status, &git.writing, message, git.stage) {
            if let Err(error) = commit_request(target.root, message, git.stage) {
                git.issue = Some(Issue::Message(error));
            }
            cx.notify();
            return;
        }
        match commit_request(target.root, message, git.stage) {
            Ok(commit) => {
                self.git_write(
                    GitWrite::Commit(commit),
                    WriteIntent::Quick(GitAction::Commit),
                    cx,
                );
            }
            Err(error) => self.areas.git.issue = Some(Issue::Message(error)),
        }
        cx.notify();
    }
}

/// The project's default branch, when the branch list has been read
/// (local, or remote-tracking when the remote's HEAD names it).
fn default_branch(shell: &Shell) -> Option<RefName> {
    project_default(shell.areas.git.branches.shown()?).cloned()
}

/// The commit form, when open.
pub(super) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let git = &shell.areas.git;
    if git.open != Some(Open::Commit) {
        return None;
    }
    let status = git.status.shown()?;
    let lang = shell.t.lang;
    let labels = lang.git_labels();
    let narrow = breakpoint::phone(window);
    let message = git.message.text(cx);
    let ready = can_commit(&git.status, &git.writing, &message, git.stage);
    let body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S4))
        .child(summary(shell, status, tokens, (window, narrow), cx))
        .child(super::commit_scope::scope(shell, tokens, narrow, cx))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(spacing::S1))
                .child(
                    div()
                        .type_(Type::Body)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(tokens.neutral.foreground)
                        .child(labels.message),
                )
                .child(git.message.element(tokens, narrow, cx)),
        )
        .children(
            git.issue
                .as_ref()
                .map(|issue| issue_alert(lang, issue, tokens)),
        )
        .into_any_element();
    let confirm = match git.writing.intent() {
        Some(intent) => lang.git_pending(intent),
        None => lang.workspace().action(GitAction::Commit),
    };
    let state = if ready {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    };
    let commit = button_with_kind(
        "git-commit-confirm",
        confirm,
        ButtonKind::Primary,
        state,
        tokens,
    );
    let commit = if ready {
        commit
            .on_click(cx.listener(|shell, _event, window, cx| {
                let message = shell.areas.git.message.text(cx);
                shell.git_commit_submitted(&message, window, cx);
            }))
            .into_any_element()
    } else {
        commit.into_any_element()
    };
    let footer = vec![
        button_with_kind(
            "git-commit-cancel",
            labels.cancel,
            ButtonKind::Secondary,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| shell.git_close(window, cx)))
        .into_any_element(),
        commit,
    ];
    Some(vessel_app_kit::dialog::dialog(
        &git.dialog,
        DialogContent {
            title: labels.commit_title.into(),
            body,
            footer,
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        Shell::git_close,
        cx,
    ))
}

/// GOAT's summary card: the branch, then the files.
fn summary(
    shell: &Shell,
    status: &GitStatus,
    tokens: &Tokens,
    (window, touch): (&Window, bool),
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let git = &shell.areas.git;
    let lang = shell.t.lang;
    let labels = lang.git_labels();
    let neutral = tokens.neutral;
    let default = default_branch(shell);
    let destination = CommitDestination::of(status, default.as_ref());
    let header = lang.git_header(status, vessel_app_view::workspace::git::HeadActivity::Idle);
    let count = commit_files(status, git.stage).count();
    let branch = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .type_(Type::Body)
        .child(
            div()
                .flex_none()
                .text_color(neutral.foreground_muted)
                .child(labels.commit_branch),
        )
        .child({
            // One line in the code face, cut in its middle; the whole name
            // in a tooltip when cut (`name.rs`). GOAT's medium weight.
            let room = name::room(&git.commit_branch_room, 1.0, COMMIT_NAME_ESTIMATE, window);
            let shown = name::one_line(window, &header.name, Type::MonoBody, room);
            let element = div()
                .id("git-commit-branch")
                .flex_1()
                .min_w_0()
                .type_(Type::MonoBody)
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(neutral.foreground)
                .whitespace_nowrap()
                .child(vessel_app_kit::picker::anchor_probe(
                    &git.commit_branch_room,
                ))
                .child(shown.text);
            if shown.cut {
                git.tips
                    .on_code(element, &git.commit_branch_tip, header.name.clone(), touch)
                    .into_any_element()
            } else {
                element.into_any_element()
            }
        })
        .children((destination == CommitDestination::DefaultBranch).then(|| {
            div()
                .flex_none()
                .text_color(tokens.status().warning.foreground)
                .child(labels.commit_default_branch)
        }));
    // GOAT's "Files" alone; the list under it is the count.
    let files_head = div()
        .type_(Type::Body)
        .text_color(neutral.foreground_muted)
        .child(labels.commit_files);
    // Read before the list lays its rows out: asking the list for its
    // width from inside a row borrows its state twice (a panic).
    let width = f32::from(git.files.width());
    let files = if count == 0 {
        div()
            .type_(Type::Body)
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(neutral.foreground)
            .child(labels.commit_files_none)
            .into_any_element()
    } else {
        form_summary::files(tokens)
            .h(px(files_height(
                count,
                f32::from(window.viewport_size().height),
            )))
            .p(px(spacing::S1))
            .child(git.files.element(cx.processor(
                move |shell: &mut Shell, index: usize, window, cx| {
                    let tokens = vessel_app_kit::tokens(cx);
                    file_row(shell, index, width, &tokens, window)
                },
            )))
            .into_any_element()
    };
    form_summary::summary(tokens)
        .child(branch)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(spacing::S1))
                .child(files_head)
                .child(files),
        )
        .children(status.more.then(|| {
            div()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(labels.changes_more)
        }))
        .into_any_element()
}

/// One file the commit takes: GOAT's monospaced path at the left, what
/// changed at the right. A path wider than its room is cut in its middle
/// (the kit's rule for a path: the root and the file's name stay), where
/// GOAT cuts its start.
fn file_row(
    shell: &Shell,
    index: usize,
    width: f32,
    tokens: &Tokens,
    window: &Window,
) -> AnyElement {
    let git = &shell.areas.git;
    let lang = shell.t.lang;
    let Some(file) = git
        .status
        .shown()
        .and_then(|status| commit_files(status, git.stage).nth(index))
    else {
        return div().into_any_element();
    };
    let state = lang.git_file_word(committed_state(file, git.stage));
    // The row's width less its two insets, the gap and the state word
    // (`mul_add`: the workspace denies bare float arithmetic).
    let taken = spacing::S2.mul_add(2.0, spacing::S3);
    let room = fonts::measure(window, state, Type::Meta).mul_add(-1.0, taken.mul_add(-1.0, width));
    let path = fonts::elide_middle(window, file.path.as_str(), Type::MonoBody, room.max(0.0));
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(spacing::S3))
        .px(px(spacing::S2))
        .py(px(spacing::S1))
        .rounded(px(FILE_ROW_RADIUS))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .type_(Type::MonoBody)
                .text_color(tokens.neutral.foreground)
                .whitespace_nowrap()
                .child(path),
        )
        .child(
            div()
                .flex_none()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(state),
        )
        .into_any_element()
}
