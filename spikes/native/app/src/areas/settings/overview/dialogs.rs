//! The Overview page's dialogs: renaming the machine and removing it
//! (split from `overview.rs`, at the file budget, docs/QUALITY.md M3).

use vessel_app_kit::dialog::{DialogContent, DialogSize, dialog};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button_with_kind};
use vessel_app_view::machine_book::Failure;
use vessel_app_view::overview::Editing;

use super::State;
use crate::shell::Shell;

/// The rename and remove dialogs, when open.
pub(crate) fn dialogs(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> impl Iterator<Item = AnyElement> {
    let rename = rename_dialog(shell, tokens, window, cx);
    let remove = remove_dialog(shell, tokens, window, cx);
    rename.into_iter().chain(remove)
}

fn rename_dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.overview;
    let open = state.rename.as_ref()?;
    let words = shell.t.lang.overview();
    let compact = crate::areas::draft::compact(window);
    let typed = state.field.text(cx);
    let pending = open.pending.is_some();
    // Paseo's submit is off while the name is unchanged or being saved.
    let can_submit = !pending && typed.trim() != open.machine.as_str();
    let error = state
        .problem
        .clone()
        .or_else(|| refusal(open.refused.as_ref(), shell));
    let body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(state.field.element(tokens, compact, cx))
        .children(error.map(|error| {
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.destructive)
                .child(error)
        }));
    let submit = button_with_kind(
        "settings-overview-rename-submit",
        words.rename_submit,
        ButtonKind::Primary,
        enabled(can_submit),
        tokens,
    )
    .when(can_submit, |button| {
        button.on_click(cx.listener(|shell, _event, _window, cx| {
            let typed = shell.areas.settings.overview.field.text(cx);
            shell.overview_rename(&typed, cx);
        }))
    });
    Some(dialog(
        &state.rename_dialog,
        DialogContent {
            title: words.rename_title.into(),
            body: body.into_any_element(),
            footer: vec![
                cancel(
                    "settings-overview-rename-cancel",
                    pending,
                    shell,
                    tokens,
                    cx,
                    |state| &mut state.rename,
                ),
                submit.into_any_element(),
            ],
            size: DialogSize::Normal,
        },
        tokens,
        window,
        compact,
        |shell: &mut Shell, _window, _cx| close(&mut shell.areas.settings.overview.rename),
        cx,
    ))
}

fn remove_dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.overview;
    let open = state.remove.as_ref()?;
    let words = shell.t.lang.overview();
    let pending = open.pending.is_some();
    let body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.remove_confirm(open.machine.as_str())),
        )
        .children(refusal(open.refused.as_ref(), shell).map(|error| {
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.destructive)
                .child(error)
        }));
    let confirm = button_with_kind(
        "settings-overview-remove-confirm",
        words.remove,
        ButtonKind::Destructive,
        enabled(!pending),
        tokens,
    )
    .when(!pending, |button| {
        button.on_click(cx.listener(|shell, _event, _window, cx| shell.overview_remove(cx)))
    });
    Some(dialog(
        &state.remove_dialog,
        DialogContent {
            title: words.remove_title.into(),
            body: body.into_any_element(),
            footer: vec![
                cancel(
                    "settings-overview-remove-cancel",
                    pending,
                    shell,
                    tokens,
                    cx,
                    |state| &mut state.remove,
                ),
                confirm.into_any_element(),
            ],
            size: DialogSize::Normal,
        },
        tokens,
        window,
        crate::areas::draft::compact(window),
        |shell: &mut Shell, _window, _cx| close(&mut shell.areas.settings.overview.remove),
        cx,
    ))
}

/// A dialog's Cancel: closes it, unless its edit is out (Paseo waits).
fn cancel(
    id: &'static str,
    pending: bool,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
    dialog: fn(&mut State) -> &mut Option<Editing>,
) -> AnyElement {
    button_with_kind(
        id,
        shell.t.lang.overview().cancel,
        ButtonKind::Secondary,
        enabled(!pending),
        tokens,
    )
    .when(!pending, |button| {
        button.on_click(cx.listener(move |shell, _event, _window, cx| {
            close(dialog(&mut shell.areas.settings.overview));
            cx.notify();
        }))
    })
    .into_any_element()
}

/// Close a dialog unless its edit is out.
fn close(dialog: &mut Option<Editing>) {
    if dialog.as_ref().is_some_and(|open| open.pending.is_none()) {
        *dialog = None;
    }
}

/// The book's refusal, in words.
fn refusal(failure: Option<&Failure>, shell: &Shell) -> Option<String> {
    failure.map(|failure| shell.t.lang.machine_book_failure(failure))
}

const fn enabled(on: bool) -> ButtonState {
    if on {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    }
}
