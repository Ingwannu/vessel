//! One persistent editor through input, inspection, confirmation and mutation,
//! laid out as Paseo's `WorkspaceLabelEditView` and its footer
//! (manager-modal.tsx 142-167, 289-344): a "Name" field, a "Color" field
//! holding the swatch row, then Delete (outline, never red) and Save side by
//! side in the dialog's footer. The way back is the header's (draw.rs).

use vessel_app_kit::theme::spacing::{S2, S4};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{
    ButtonConfig, ButtonContent, ButtonKind, ButtonState, ControlSizing, Tokens, Type, TypeExt as _,
    button_control, button_with_kind,
};
use vessel_app_view::{
    labels::{Count, Draft, Host, ManagerState},
    localization::Number,
};

use super::actions::{Action, Target};
use crate::shell::Shell;

/// `saveButton.minWidth`.
const SAVE_MIN: f32 = 112.0;

pub(super) fn editor(
    shell: &Shell,
    draft: &Draft,
    target: &Target,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let state = &shell.areas.labels;
    let words = shell.words.labels();
    div()
        .flex()
        .flex_col()
        .gap(px(S4))
        .child(field(
            words.field_name(),
            state.name.element(tokens, false, cx).into_any_element(),
            tokens,
        ))
        .child(field(
            words.field_color(),
            super::colors::choices(shell, draft.color(), target, tokens, cx).into_any_element(),
            tokens,
        ))
}

/// Paseo's `Field`: its label in 14 `foregroundMuted`, 8 above the control.
fn field(label: &'static str, control: AnyElement, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(S2))
        .child(
            div()
                .min_w_0()
                .truncate()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .child(label),
        )
        .child(control)
}

/// The dialog's footer for the draft: Delete then Save while editing (Save
/// only once the draft differs, both off while anything is under way); the
/// confirmation's Cancel and Delete while it asks.
pub(super) fn footer(
    shell: &Shell,
    host: &Host,
    target: &Target,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let manager = &shell.areas.labels.manager;
    let online = host.state.connected()
        && target.matches(
            shell.areas.labels.opening,
            manager,
            host.connection.as_ref(),
        );
    match manager.state() {
        ManagerState::Listing | ManagerState::Deleting { .. } => Vec::new(),
        ManagerState::Editing(draft) => {
            let current = host.catalog.get(&draft.original().name);
            let can_save =
                online && current.is_some_and(|label| draft.replacement(label).is_some());
            let can_delete = online && current.is_some();
            vec![edit_actions(
                shell, target, can_save, can_delete, false, tokens, cx,
            )]
        }
        // Paseo's `loading={state.pending}`: any operation under way spins in
        // Save, a delete check too.
        ManagerState::Saving { .. } | ManagerState::Inspecting { .. } => {
            vec![edit_actions(shell, target, false, false, true, tokens, cx)]
        }
        ManagerState::Confirming { ticket, .. } => {
            confirm_actions(shell, target, *ticket, online, tokens, cx)
        }
    }
}

/// Paseo's edit footer: one row, Delete (outline) at the start and Save at
/// the end (`justifyContent: "space-between"`). `pending`: Save shows the
/// kit's loading state (the spinner before its word, off).
fn edit_actions(
    shell: &Shell,
    target: &Target,
    can_save: bool,
    can_delete: bool,
    pending: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.labels();
    let delete = button_with_kind(
        "labels-delete",
        words.delete(),
        ButtonKind::Outline,
        enabled(can_delete),
        tokens,
    )
    .when(can_delete, |button| {
        let action = Action::Inspect(target.clone());
        button.on_click(cx.listener(move |shell, _, _window, cx| {
            shell.label_action(&action, cx);
        }))
    });
    let config = ButtonConfig::new(
        ButtonKind::Primary,
        enabled(can_save),
        ControlSizing::Pointer,
    );
    // The kit's `Button` attaches the click only while enabled and not pending.
    let action = Action::Save(target.clone());
    let save = button_control(
        "labels-save",
        ButtonContent::Label(words.save().into()),
        if pending { config.loading() } else { config },
        tokens,
    )
    .min_w(px(SAVE_MIN))
    .on_click(cx.listener(move |shell, _, _window, cx| {
        shell.label_action(&action, cx);
    }));
    div()
        .debug_selector(|| "labels-footer".to_owned())
        .flex_1()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(S2))
        .child(delete)
        .child(save)
        .into_any_element()
}

/// The confirmation's Cancel and Delete (destructive), as Paseo's
/// `confirmDialog` buttons.
fn confirm_actions(
    shell: &Shell,
    target: &Target,
    inspection: vessel_app_view::labels::Ticket,
    online: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let words = shell.words.labels();
    let cancel = super::surface::Dismiss::Confirmation {
        opening: target.opening(),
        inspection,
    };
    let action = Action::Delete {
        target: target.clone(),
        inspection,
    };
    vec![
        button_with_kind(
            "labels-delete-cancel",
            words.cancel(),
            ButtonKind::Secondary,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(move |shell, _, window, cx| {
            shell.labels_close(cancel, window, cx);
        }))
        .into_any_element(),
        button_with_kind(
            "labels-delete-confirm",
            words.delete(),
            ButtonKind::Destructive,
            enabled(online),
            tokens,
        )
        .when(online, |button| {
            button.on_click(cx.listener(move |shell, _, _window, cx| {
                shell.label_action(&action, cx);
            }))
        })
        .into_any_element(),
    ]
}

/// Paseo's `confirmDialog` content: the question and the affected count.
pub(super) fn confirmation(shell: &Shell, count: Count) -> Div {
    let words = shell.words.labels();
    div()
        .flex()
        .flex_col()
        .gap(px(S2))
        .child(words.confirm())
        .child(
            words
                .affected(Number::count(count.get()))
                .unwrap_or_else(|_| words.failed().to_owned()),
        )
}

const fn enabled(value: bool) -> ButtonState {
    if value {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    }
}
