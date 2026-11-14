//! The repository's saved entries (the page's `stash-dialog.tsx`; Paseo
//! saves and restores entries only around a branch switch and has no
//! list): each entry's message, its `stash@{n}` and when it was saved,
//! with Apply, Pop and Drop. Drop is confirmed in Paseo's confirm dialog
//! (title, what it deletes, Cancel and a destructive button), never one
//! click. An entry is sent with its index and its commit, so an entry
//! that moved since it was listed is refused rather than the wrong one
//! changed (`view::workspace::git::stash_request`).

use chrono::Utc;
use vessel_app_kit::dialog::{DialogContent, DialogSize};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, breakpoint, button_with_kind,
};
use vessel_app_link::workspace::GitWrite;
use vessel_app_view::workspace::git::{
    DropStash, Reading, StashChoice, WriteIntent, stash_request, stash_subject,
};
use vessel_proto::workspace::StashEntry;

use super::{Open, menu::issue_alert, name};

/// Before the first measure: a card's text column beside its three buttons.
const STASH_ROOM_ESTIMATE: f32 = 220.0;

/// The drop confirmation's body: a card's width less its insets.
const DROP_ROOM: f32 = 460.0;

/// A sheet's insets around the drop confirmation's body.
const DROP_INSETS: f32 = 64.0;

/// A stash row's text column never narrows below this, so its name and age
/// stay readable beside the row's buttons.
const TEXT_LEAST: f32 = 160.0;
use crate::shell::Shell;

impl Shell {
    /// Open the list and read it.
    pub(super) fn git_open_stashes(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let git = &mut self.areas.git;
        git.menu.close();
        git.issue = None;
        git.open = Some(Open::Stashes);
        git.dialog.focus(window, cx);
        self.git_read_stashes();
    }

    /// Apply or pop the listed `entry`.
    fn git_stash(&mut self, entry: StashEntry, choice: StashChoice, cx: &mut Context<'_, Self>) {
        let Some(root) = self.areas.git.target.as_ref().map(|target| target.root) else {
            return;
        };
        let write = GitWrite::Stash(stash_request(root, &entry, choice));
        self.git_write(write, WriteIntent::Stash { choice, entry }, cx);
    }

    /// Ask to drop the listed `entry`.
    fn git_ask_drop(&mut self, entry: StashEntry, cx: &mut Context<'_, Self>) {
        let Some(root) = self.areas.git.target.as_ref().map(|target| target.root) else {
            return;
        };
        self.areas.git.issue = None;
        self.areas.git.dropping = Some(DropStash::of(root, entry));
        cx.notify();
    }

    /// The drop was confirmed: exactly the entry reviewed.
    fn git_confirm_drop(&mut self, cx: &mut Context<'_, Self>) {
        let Some(decision) = self.areas.git.dropping.clone() else {
            return;
        };
        let entry = decision.entry().clone();
        let intent = WriteIntent::Stash {
            choice: StashChoice::Drop,
            entry,
        };
        self.git_write(GitWrite::Stash(decision.confirm()), intent, cx);
    }

    /// The drop's Cancel: back to the list.
    fn git_keep(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.git.dropping = None;
        self.areas.git.issue = None;
        cx.notify();
    }
}

/// The list, or the drop's confirmation over it.
pub(super) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let git = &shell.areas.git;
    if git.open != Some(Open::Stashes) {
        return None;
    }
    if git.dropping.is_some() {
        return confirm_drop(shell, tokens, window, cx);
    }
    let lang = shell.t.lang;
    let labels = lang.git_labels();
    let narrow = breakpoint::phone(window);
    let neutral = tokens.neutral;
    let note = |words: String| {
        div()
            .type_(Type::Body)
            .text_color(neutral.foreground_muted)
            .child(words)
            .into_any_element()
    };
    let body = match &git.stashes {
        Reading::Failed(problem) => issue_alert(
            lang,
            &vessel_app_view::workspace::git::Issue::Call(problem.clone()),
            tokens,
        )
        .into_any_element(),
        read @ (Reading::Empty | Reading::Loading { .. } | Reading::Ready(_)) => {
            match read.shown() {
                None => note(labels.reading_stashes.to_owned()),
                Some(list) if list.entries.is_empty() => note(labels.empty_stashes.to_owned()),
                Some(list) => {
                    let now = Utc::now();
                    let offset = *chrono::Local::now().offset();
                    // The dialog's body scrolls (the kit's dialog), so
                    // a long list needs no scroller of its own.
                    div()
                        .flex()
                        .flex_col()
                        .children(list.entries.iter().enumerate().map(|(at, entry)| {
                            row(shell, (at, entry), (now, offset), tokens, window, cx)
                        }))
                        .children(list.more.then(|| note(labels.stashes_more.to_owned())))
                        .into_any_element()
                }
            }
        }
    };
    let body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .child(body)
        .children(
            git.issue
                .as_ref()
                .map(|issue| issue_alert(lang, issue, tokens)),
        )
        .into_any_element();
    let footer = vec![
        button_with_kind(
            "git-stashes-close",
            labels.close,
            ButtonKind::Secondary,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| shell.git_close(window, cx)))
        .into_any_element(),
    ];
    Some(vessel_app_kit::dialog::dialog(
        &git.dialog,
        DialogContent {
            title: labels.stashes_title.into(),
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

/// One entry: its message, then `stash@{n}` and when; its three actions.
fn row(
    shell: &Shell,
    (at, entry): (usize, &StashEntry),
    clock: (chrono::DateTime<Utc>, chrono::FixedOffset),
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let git = &shell.areas.git;
    let lang = shell.t.lang;
    let neutral = tokens.neutral;
    let meta = lang.git_stash_meta(entry, clock.0, clock.1);
    let subject = stash_subject(&entry.message);
    let running = git.writing.intent().filter(|intent| {
        [StashChoice::Apply, StashChoice::Pop]
            .iter()
            .any(|choice| intent.matches_stash(*choice, entry))
    });
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(spacing::S2))
        .py(px(spacing::S2))
        .when(at > 0, |row| row.border_t_1().border_color(neutral.border))
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w(px(TEXT_LEAST))
                .gap(px(spacing::S0_5))
                // Every row's text column is as wide as the first's.
                .when(at == 0, |column| {
                    column.child(vessel_app_kit::picker::anchor_probe(&shell.areas.git.stash_room))
                })
                .child(
                    div()
                        .type_(Type::Body)
                        .text_color(neutral.foreground)
                        .whitespace_normal()
                        .child(subject.message.to_owned()),
                )
                .children(subject.branch.map(|branch| {
                    // The branch it was saved on: one line, cut in its
                    // middle (`name.rs`), never broken inside the message.
                    let room = name::room(&shell.areas.git.stash_room, 1.0, STASH_ROOM_ESTIMATE, window);
                    div()
                        .type_(Type::Mono)
                        .text_color(neutral.foreground_muted)
                        .whitespace_nowrap()
                        .child(name::one_line(window, branch, Type::Mono, room).text)
                }))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(spacing::S2))
                        .type_(Type::Meta)
                        .text_color(neutral.foreground_muted)
                        .child(div().type_(Type::Mono).child(meta.reference))
                        .child(meta.saved),
                ),
        )
        .children(running.map(|intent| {
            div()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(lang.git_pending(intent))
        }))
        .child(entry_actions(shell, at, entry, tokens, cx))
        .into_any_element()
}

/// Apply, Pop and Drop… for one entry; none while a write is out.
fn entry_actions(
    shell: &Shell,
    at: usize,
    entry: &StashEntry,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let labels = shell.t.lang.git_labels();
    let busy = shell.areas.git.writing.busy();
    let state = if busy {
        ButtonState::Disabled
    } else {
        ButtonState::Enabled
    };
    let act = |id: &'static str, label: &'static str, choice: StashChoice| {
        let entry = entry.clone();
        let button = button_with_kind((id, at), label, ButtonKind::Secondary, state, tokens);
        if busy {
            button.into_any_element()
        } else {
            button
                .on_click(cx.listener(move |shell, _event, _window, cx| {
                    shell.git_stash(entry.clone(), choice, cx);
                }))
                .into_any_element()
        }
    };
    let apply = act("git-stash-apply", labels.apply, StashChoice::Apply);
    let pop = act("git-stash-pop", labels.pop, StashChoice::Pop);
    let drop = {
        let entry = entry.clone();
        let button = button_with_kind(
            ("git-stash-drop", at),
            labels.drop,
            ButtonKind::Destructive,
            state,
            tokens,
        );
        if busy {
            button.into_any_element()
        } else {
            button
                .on_click(cx.listener(move |shell, _event, _window, cx| {
                    shell.git_ask_drop(entry.clone(), cx);
                }))
                .into_any_element()
        }
    };
    div()
        .flex()
        .flex_row()
        .flex_none()
        .gap(px(spacing::S1))
        .child(apply)
        .child(pop)
        .child(drop)
        .into_any_element()
}

/// Paseo's confirm dialog for the reviewed entry.
fn confirm_drop(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let git = &shell.areas.git;
    let decision = git.dropping.as_ref()?;
    let entry = decision.entry();
    let lang = shell.t.lang;
    let labels = lang.git_labels();
    let narrow = breakpoint::phone(window);
    let neutral = tokens.neutral;
    let busy = git.writing.busy();
    let body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .type_(Type::Body)
        .text_color(neutral.foreground_muted)
        .child(lang.git_drop_copy())
        .child(
            div()
                .text_color(neutral.foreground)
                .whitespace_normal()
                .child(stash_subject(&entry.message).message.to_owned()),
        )
        .children(stash_subject(&entry.message).branch.map(|branch| {
            div()
                .type_(Type::Mono)
                .text_color(neutral.foreground_muted)
                .whitespace_nowrap()
                .child({
                    // The card is 520 wide; a sheet is the window's width.
                    let room = DROP_INSETS
                        .mul_add(-1.0, f32::from(window.viewport_size().width))
                        .min(DROP_ROOM);
                    name::one_line(window, branch, Type::Mono, room).text
                })
        }))
        .child(
            div()
                .type_(Type::Mono)
                .text_color(neutral.foreground)
                .child(format!("stash@{{{}}}", entry.index)),
        )
        .children(
            git.issue
                .as_ref()
                .map(|issue| issue_alert(lang, issue, tokens)),
        )
        .into_any_element();
    let confirm_label = match git.writing.intent() {
        Some(intent) if busy => lang.git_pending(intent),
        Some(_) | None => labels.drop_confirm,
    };
    let confirm = button_with_kind(
        "git-drop-confirm",
        confirm_label,
        ButtonKind::Destructive,
        if busy {
            ButtonState::Disabled
        } else {
            ButtonState::Enabled
        },
        tokens,
    );
    let confirm = if busy {
        confirm.into_any_element()
    } else {
        confirm
            .on_click(cx.listener(|shell, _event, _window, cx| shell.git_confirm_drop(cx)))
            .into_any_element()
    };
    let footer = vec![
        button_with_kind(
            "git-drop-cancel",
            labels.cancel,
            ButtonKind::Secondary,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| shell.git_keep(window, cx)))
        .into_any_element(),
        confirm,
    ];
    Some(vessel_app_kit::dialog::dialog(
        &git.dialog,
        DialogContent {
            title: labels.drop_title.into(),
            body,
            footer,
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        Shell::git_keep,
        cx,
    ))
}
