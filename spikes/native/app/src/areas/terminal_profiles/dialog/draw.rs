//! The edit and remove dialogs, drawn (Paseo's `TerminalProfileEditModal`
//! and its remove confirmation); their state and actions are the parent's.

use vessel_app_kit::dialog::{CompactLayout, DesktopHeight, DialogLayout};
use vessel_app_kit::sheet::snap::Percentage;
use vessel_app_kit::theme::spacing::{S2, S4};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, ControlSizing, DialogContent, DialogSize, Tokens, Type, TypeExt as _,
    button_with_kind_at, dialog,
};
use vessel_app_view::terminal_profiles::Target;
use vessel_proto::terminal_profiles::ProfileError;

use super::refusal;
use crate::shell::Shell;

/// Paseo's `desktopMaxWidth`.
const WIDTH: u16 = 480;
/// The kit's dialog caps (`DialogSize::Normal`'s).
const DESKTOP_CAP: u8 = 85;
const COMPACT_CAP: u8 = 90;

/// A bound a draft breaks, in the app's words.
fn invalid(error: ProfileError, shell: &Shell) -> &'static str {
    let words = shell.words.terminal_profiles();
    match error {
        ProfileError::TooLong { .. }
        | ProfileError::TooManyArguments
        | ProfileError::CommandTooLarge => words.too_long(),
        ProfileError::Character(_) => words.character(),
        ProfileError::TooManyProfiles => words.too_many(),
        ProfileError::Empty(_) | ProfileError::ReservedId | ProfileError::DuplicateId => {
            words.unable_to_save()
        }
    }
}

/// The open dialogs.
pub(in crate::areas::terminal_profiles) fn dialogs(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> impl Iterator<Item = AnyElement> {
    let edit = edit_dialog(shell, tokens, window, cx);
    let remove = remove_dialog(shell, tokens, window, cx);
    edit.into_iter().chain(remove)
}

fn size() -> DialogSize {
    Percentage::new(DESKTOP_CAP)
        .zip(Percentage::new(COMPACT_CAP))
        .and_then(|(desktop, compact)| {
            DialogLayout::new(
                WIDTH,
                DesktopHeight::AtMost(desktop),
                CompactLayout::Fixed(compact),
            )
        })
        .map_or(DialogSize::Normal, DialogSize::Configured)
}

/// A labelled field (Paseo's `Field`: the label 14 muted, 8 to the field,
/// then its error or hint at 12).
fn field(
    label: &'static str,
    input: AnyElement,
    under: Option<(&'static str, bool)>,
    tokens: &Tokens,
) -> Div {
    let neutral = tokens.neutral;
    div()
        .flex()
        .flex_col()
        .gap(px(S2))
        .child(
            div()
                .type_(Type::Body)
                .text_color(neutral.foreground_muted)
                .child(label),
        )
        .child(input)
        .children(under.map(|(text, error)| {
            div()
                .type_(Type::Meta)
                .text_color(if error {
                    neutral.destructive
                } else {
                    neutral.foreground_muted
                })
                .child(text)
        }))
}

const fn enabled(on: bool) -> ButtonState {
    if on {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    }
}

fn edit_dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let edit = shell.areas.terminal_profiles.edit.as_ref()?;
    let words = shell.words.terminal_profiles();
    let narrow = vessel_app_kit::breakpoint::phone(window);
    let sizing = ControlSizing::from_compact(narrow);
    let title = match edit.target {
        Target::Add(_) => words.add_title(),
        Target::Edit(_) => words.edit_title(),
    };
    let submit = edit
        .refused
        .map(|problem| refusal(problem, shell))
        .or_else(|| edit.errors.invalid.map(|error| invalid(error, shell)));
    let body = div()
        .flex()
        .flex_col()
        .gap(px(S4))
        .pb(px(S2))
        .child(field(
            words.name_label(),
            edit.name.element(tokens, narrow, cx),
            edit.errors.name.then(|| (words.name_required(), true)),
            tokens,
        ))
        .child(field(
            words.command_label(),
            edit.command.element(tokens, narrow, cx),
            edit.errors
                .command
                .then(|| (words.command_required(), true)),
            tokens,
        ))
        .child(field(
            words.args_label(),
            edit.args.element(tokens, narrow, cx),
            Some((words.args_hint(), false)),
            tokens,
        ))
        .children(submit.map(|text| {
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.destructive)
                .child(text)
        }))
        .child(actions(edit, shell, sizing, tokens, cx));
    Some(dialog(
        &shell.areas.terminal_profiles.edit_dialog,
        DialogContent {
            title: title.into(),
            body: body.into_any_element(),
            footer: Vec::new(),
            size: size(),
        },
        tokens,
        window,
        narrow,
        Shell::terminal_profile_cancel,
        cx,
    ))
}

/// Cancel and Save, side by side and as wide (Paseo's `actions`, 8 above);
/// both held while the save waits, Save then reading "Saving...".
fn actions(
    edit: &super::Edit,
    shell: &Shell,
    sizing: ControlSizing,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.terminal_profiles();
    let cancel = button_with_kind_at(
        "terminal-profile-cancel",
        words.cancel(),
        ButtonKind::Secondary,
        enabled(!edit.saving),
        sizing,
        tokens,
    )
    .debug_selector(|| "terminal-profile-cancel".to_owned())
    .flex_1()
    .when(!edit.saving, |button| {
        button.on_click(cx.listener(|shell, _event, window, cx| {
            shell.terminal_profile_cancel(window, cx);
        }))
    });
    let save = button_with_kind_at(
        "terminal-profile-save",
        if edit.saving {
            words.saving()
        } else {
            words.save()
        },
        ButtonKind::Primary,
        enabled(!edit.saving),
        sizing,
        tokens,
    )
    .debug_selector(|| "terminal-profile-save".to_owned())
    .flex_1()
    .when(!edit.saving, |button| {
        button.on_click(cx.listener(|shell, _event, window, cx| {
            shell.terminal_profile_save(window, cx);
        }))
    });
    div()
        .mt(px(S2))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(S2))
        .child(cancel)
        .child(save)
}

fn remove_dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.terminal_profiles;
    let id = state.removing.as_ref()?;
    let profile = shell.terminal_profiles_page_list()?.get(id)?;
    let words = shell.words.terminal_profiles();
    let message = words
        .remove_message(profile.name().as_str())
        .unwrap_or_else(|_unformatted| profile.name().as_str().to_owned());
    let narrow = vessel_app_kit::breakpoint::phone(window);
    let keep = button_with_kind_at(
        "terminal-profile-remove-cancel",
        words.cancel(),
        ButtonKind::Secondary,
        ButtonState::Enabled,
        ControlSizing::from_compact(narrow),
        tokens,
    )
    .debug_selector(|| "terminal-profile-remove-cancel".to_owned())
    .on_click(cx.listener(|shell, _event, window, cx| {
        shell.terminal_profile_remove_keep(window, cx);
    }));
    let remove = button_with_kind_at(
        "terminal-profile-remove-confirm",
        words.remove(),
        ButtonKind::Destructive,
        ButtonState::Enabled,
        ControlSizing::from_compact(narrow),
        tokens,
    )
    .debug_selector(|| "terminal-profile-remove-confirm".to_owned())
    .on_click(cx.listener(|shell, _event, window, cx| {
        shell.terminal_profile_remove(window, cx);
    }));
    let body = div()
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(message);
    Some(dialog(
        &state.remove_dialog,
        DialogContent {
            title: words.remove_title().into(),
            body: body.into_any_element(),
            footer: vec![keep.into_any_element(), remove.into_any_element()],
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        Shell::terminal_profile_remove_keep,
        cx,
    ))
}
