//! One persistent editor through input, inspection, confirmation and mutation.

use vessel_app_kit::theme::spacing::S3;
use vessel_app_kit::ui::{
    Context, Div, FluentBuilder as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button_with_kind};
use vessel_app_view::{
    labels::{Count, Draft, Host, ManagerState},
    localization::Number,
};

use super::actions::Action;
use crate::shell::Shell;

pub(super) fn editor(
    shell: &Shell,
    host: &Host,
    draft: &Draft,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let state = &shell.areas.labels;
    let words = shell.words.labels();
    div()
        .flex()
        .flex_col()
        .gap(px(S3))
        .type_(Type::Body)
        .child(state.name.element(tokens, false, cx))
        .child(super::colors::choices(shell, draft.color(), tokens, cx))
        .child(
            button_with_kind(
                "labels-back",
                words.back(),
                ButtonKind::Ghost,
                enabled(!state.manager.state().busy()),
                tokens,
            )
            .when(!state.manager.state().busy(), |button| {
                button.on_click(cx.listener(|shell, _, _window, cx| {
                    if let Err(failure) = shell.areas.labels.manager.back() {
                        shell.areas.labels.error = Some(failure);
                    }
                    shell.settle_labels(cx);
                    cx.notify();
                }))
            }),
        )
        .child(actions(shell, host, tokens, cx))
}

fn actions(shell: &Shell, host: &Host, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.labels();
    let row = div().flex().flex_col().gap(px(S3));
    match shell.areas.labels.manager.state() {
        ManagerState::Listing => row,
        ManagerState::Editing(draft) => {
            let current = host.catalog.get(&draft.original().name);
            let online = host.state.connected();
            let can_save =
                online && current.is_some_and(|label| draft.replacement(label).is_some());
            let can_delete = online && current.is_some();
            row.child(
                button_with_kind(
                    "labels-save",
                    words.save(),
                    ButtonKind::Primary,
                    enabled(can_save),
                    tokens,
                )
                .when(can_save, |button| {
                    button.on_click(
                        cx.listener(|shell, _, _window, cx| shell.label_action(Action::Save, cx)),
                    )
                }),
            )
            .child(
                button_with_kind(
                    "labels-delete",
                    words.delete(),
                    ButtonKind::Secondary,
                    enabled(can_delete),
                    tokens,
                )
                .when(can_delete, |button| {
                    button.on_click(
                        cx.listener(|shell, _, _window, cx| {
                            shell.label_action(Action::Inspect, cx)
                        }),
                    )
                }),
            )
        }
        ManagerState::Saving { .. } | ManagerState::Deleting { .. } => row.child(words.pending()),
        ManagerState::Inspecting { .. } => row.child(words.inspecting()),
        ManagerState::Confirming { count, .. } => confirmation(shell, host, *count, tokens, cx),
    }
}

fn confirmation(
    shell: &Shell,
    host: &Host,
    count: Count,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.labels();
    div()
        .flex()
        .flex_col()
        .gap(px(S3))
        .child(words.confirm())
        .child(
            words
                .affected(Number::count(count.get()))
                .unwrap_or_else(|_| words.failed().to_owned()),
        )
        .child(
            button_with_kind(
                "labels-delete-cancel",
                words.cancel(),
                ButtonKind::Secondary,
                ButtonState::Enabled,
                tokens,
            )
            .on_click(cx.listener(|shell, _, _window, cx| {
                let _cancelled = shell.areas.labels.manager.cancel_delete();
                shell.settle_labels(cx);
                cx.notify();
            })),
        )
        .child(
            button_with_kind(
                "labels-delete-confirm",
                words.delete(),
                ButtonKind::Destructive,
                enabled(host.state.connected()),
                tokens,
            )
            .when(host.state.connected(), |button| {
                button.on_click(
                    cx.listener(|shell, _, _window, cx| shell.label_action(Action::Delete, cx)),
                )
            }),
        )
}

const fn enabled(value: bool) -> ButtonState {
    if value {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    }
}
