//! A plugin screen's inputs (ADR 0115, "What a control sends"): a text
//! field sends its text on Enter, a choice its option id, a toggle its new
//! state, each as `{value}` to its declared method.
//!
//! [Decision Log]
//! - A toggle or choice shows its new value only until the call answers;
//!   then the declaration wins, as on the page.
//! - Without admin each input is drawn disabled: a field as an empty box, a
//!   choice's pills dimmed, a switch off-limits (coordinator ruling on P3b,
//!   Q1).

use vessel_app_kit::settings;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, choice};
use vessel_app_view::plugins::{ControlValue, control_input};
use vessel_proto::plugins::{ContributionId, PluginChoiceOption, RpcName, ToggleState};

use super::controls::Scope;
use super::{Pending, control_id};
use crate::shell::Shell;

/// A label over a control, as a settings field.
fn labelled(label: &str, control: AnyElement, tokens: &Tokens) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(
            div()
                .type_(Type::Label)
                .text_color(tokens.text_secondary)
                .child(label.to_owned()),
        )
        .child(control)
        .into_any_element()
}

pub(super) fn text_field(
    shell: &Shell,
    scope: &Scope<'_>,
    id: &ContributionId,
    label: &str,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let field = shell
        .areas
        .plugin_screen
        .fields
        .get(id)
        .filter(|_| scope.may_invoke);
    let control = match field {
        Some(field) => field.element(tokens, scope.compact, cx).into_any_element(),
        // Without admin, or declared after the screen opened: an empty,
        // disabled box.
        None => div()
            .h(px(32.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(tokens.border_faint)
            .bg(tokens.bg_well)
            .into_any_element(),
    };
    labelled(label, control, tokens)
}

#[expect(
    clippy::too_many_arguments,
    reason = "a choice draws from the scope, its declaration and the shell together"
)]
pub(super) fn choices(
    shell: &Shell,
    scope: &Scope<'_>,
    id: &ContributionId,
    label: &str,
    options: &[PluginChoiceOption],
    selected: Option<&ContributionId>,
    submit: &RpcName,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let pending = shell.areas.plugin_screen.pending.borrow().get(id).cloned();
    let chosen = match pending {
        Some(Pending::Choice(option)) => Some(option),
        Some(Pending::Toggle(_)) | None => selected.cloned(),
    };
    let pills = options.iter().map(|option| {
        let mut pill = choice(
            control_id(&format!("choice-{id}"), &option.id),
            option.title.as_str().to_owned(),
            chosen.as_ref() == Some(&option.id),
            tokens,
        );
        if scope.may_invoke {
            let control = id.clone();
            let picked = option.id.clone();
            let method = submit.clone();
            pill = pill.on_click(cx.listener(move |shell, _event, _window, cx| {
                let _was = shell
                    .areas
                    .plugin_screen
                    .pending
                    .borrow_mut()
                    .insert(control.clone(), Pending::Choice(picked.clone()));
                let input = control_input(&ControlValue::Choice(picked.clone()));
                shell.plugin_screen_call(&method, input, Some(control.clone()));
                cx.notify();
            }));
        } else {
            pill = pill.opacity(0.5);
        }
        pill
    });
    labelled(
        label,
        div()
            .flex()
            .flex_wrap()
            .gap(px(spacing::S2))
            .children(pills)
            .into_any_element(),
        tokens,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "a toggle draws from the scope, its declaration and the shell together"
)]
pub(super) fn toggle(
    shell: &Shell,
    scope: &Scope<'_>,
    id: &ContributionId,
    label: &str,
    state: ToggleState,
    submit: &RpcName,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let pending = shell.areas.plugin_screen.pending.borrow().get(id).cloned();
    let on = match pending {
        Some(Pending::Toggle(on)) => on,
        Some(Pending::Choice(_)) | None => state == ToggleState::On,
    };
    let title = SharedString::from(label.to_owned());
    let mut switch = settings::switch(
        control_id("toggle", id),
        on,
        scope.may_invoke,
        scope.compact,
        0,
        tokens,
    )
    .role(gpui::Role::CheckBox)
    .aria_toggled(if on {
        gpui::Toggled::True
    } else {
        gpui::Toggled::False
    })
    .aria_label(title.clone());
    if scope.may_invoke {
        let control = id.clone();
        let method = submit.clone();
        switch = switch.on_click(cx.listener(move |shell, _event, _window, cx| {
            let _was = shell
                .areas
                .plugin_screen
                .pending
                .borrow_mut()
                .insert(control.clone(), Pending::Toggle(!on));
            let input = control_input(&ControlValue::Toggle(!on));
            shell.plugin_screen_call(&method, input, Some(control.clone()));
            cx.notify();
        }));
    }
    div()
        .flex()
        .items_center()
        .gap(px(spacing::S3))
        .child(switch)
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.text_primary)
                .child(title),
        )
        .into_any_element()
}
