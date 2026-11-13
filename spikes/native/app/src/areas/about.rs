//! About: the running build and its updates (Zed's About and its
//! `auto_update` setting, as the first part of a Settings screen): what
//! the updater last did, "Check for updates", and "Update automatically".
//! Also the note shown once at the top of the frame after an update
//! ("Updated to 0.0.0.0.3", with the feed's notes), dismissible.

use vessel_app_kit::ui::{
    Context, Div, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, button, choice, kicker};
use vessel_app_updater::UpdateState;

use super::common::title_bar;
use crate::shell::Shell;
use crate::update::{Updated, percent};

/// The About screen.
pub(crate) fn render(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.update();
    let mut column = div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .p(px(24.0))
        .max_w(px(640.0))
        .child(vessel_app_kit::mark(40.0, tokens))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(div().type_(Type::Title).child(shell.t.app))
                .child(
                    div()
                        .type_(Type::Meta)
                        .text_color(tokens.text_secondary)
                        .child(words.version(crate::LABEL)),
                ),
        )
        .child(kicker(words.updates, tokens));
    if shell.updates_on() {
        column = column
            .child(line(status(shell), tokens.text_secondary))
            .child(actions(shell, tokens, cx))
            .child(automatic(shell, tokens, cx));
    } else {
        column = column.child(line(words.no_updates.to_owned(), tokens.text_tertiary));
    }
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(title_bar(words.about.to_owned(), None, tokens))
        .child(column)
}

/// What the updater last did, in a sentence.
fn status(shell: &Shell) -> String {
    let words = shell.t.lang.update();
    match &shell.update_state {
        UpdateState::Idle => words.not_checked.to_owned(),
        UpdateState::Checking => words.checking.to_owned(),
        UpdateState::UpToDate { latest } => words.up_to_date(&latest.to_string()),
        UpdateState::Downloading {
            received, total, ..
        } => words.downloading(percent(received.get(), total.get())),
        UpdateState::Ready { label, .. } => words.ready(&label.to_string()),
        UpdateState::Failed { error } => words.failed_because(&error.to_string()),
    }
}

/// "Check for updates", or "Restart to update" once a build is in place.
fn actions(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.update();
    let row = div().flex().flex_row().gap(px(8.0));
    if matches!(shell.update_state, UpdateState::Ready { .. }) {
        return row.child(
            button("about-restart", words.restart, true, tokens)
                .on_click(cx.listener(|this, _event, _window, cx| this.restart_to_update(cx))),
        );
    }
    row.child(
        button("about-check", words.check_now, false, tokens)
            .on_click(cx.listener(|this, _event, _window, cx| this.check_for_updates(cx))),
    )
}

/// "Update automatically", on or off, and what off means.
fn automatic(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.update();
    let on = shell.updates_automatic();
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .child(
                    div()
                        .type_(Type::Label)
                        .text_color(tokens.text_primary)
                        .child(words.automatic),
                )
                .child(
                    choice(
                        "update-automatically",
                        if on { words.on } else { words.off },
                        on,
                        tokens,
                    )
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.set_update_automatically(!on, cx);
                    })),
                ),
        )
        .child(line(words.automatic_note.to_owned(), tokens.text_tertiary))
}

/// The note shown once after an update, at the top of the frame.
pub(crate) fn updated(
    shell: &Shell,
    updated: &Updated,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.t.lang.update();
    div()
        .flex()
        .flex_row()
        .items_start()
        .justify_between()
        .gap(px(12.0))
        .px(px(16.0))
        .py(px(10.0))
        .bg(tokens.bg_well)
        .border_b_1()
        .border_color(tokens.border_faint)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .type_(Type::Label)
                        .text_color(tokens.text_primary)
                        .child(words.updated_to(&updated.label)),
                )
                .children(
                    updated
                        .notes
                        .clone()
                        .map(|notes| line(notes, tokens.text_secondary)),
                ),
        )
        .child(
            button("dismiss-updated", words.dismiss, false, tokens).on_click(cx.listener(
                |this, _event, _window, cx| {
                    this.updated = None;
                    cx.notify();
                },
            )),
        )
}

fn line(text: String, color: vessel_app_kit::ui::Hsla) -> Div {
    div().type_(Type::Meta).text_color(color).child(text)
}
