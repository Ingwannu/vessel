//! Existing buttons, guarded against navigation and duplicate reads.
use super::*;
use models::Refresh;
use vessel_app_kit::ui::{FluentBuilder as _, StatefulInteractiveElement as _};
use vessel_app_kit::{ButtonKind, ButtonState, button_with_kind};

#[derive(Clone, Copy)]
enum Action {
    Refresh,
    Retry,
}
pub(super) fn refresh(
    state: Refresh,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    control(Action::Refresh, state, shell, tokens, compact, cx)
}
pub(super) fn retry(
    state: Refresh,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    control(Action::Retry, state, shell, tokens, compact, cx)
}
fn control(
    action: Action,
    state: Refresh,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.models();
    let (id, label, kind) = match action {
        Action::Refresh => (
            "models-refresh",
            words.refresh_label(state),
            ButtonKind::Ghost,
        ),
        Action::Retry => (
            "models-retry",
            words.retry_label(state),
            ButtonKind::Primary,
        ),
    };
    let scope = shell.areas.settings.models.page.machine().cloned();
    let enabled = matches!(state, Refresh::Ready);
    button_with_kind(
        id,
        label,
        kind,
        if enabled {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        tokens,
    )
    .when(compact, |button| {
        button.min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
    })
    .when(enabled, |button| {
        button.on_click(cx.listener(move |shell, _, _, cx| {
            if shell.route == Route::Settings
                && shell.areas.settings.section == Section::Models
                && shell.areas.settings.models.page.machine() == scope.as_ref()
            {
                shell.models_refresh();
                cx.notify();
            }
        }))
    })
    .into_any_element()
}
