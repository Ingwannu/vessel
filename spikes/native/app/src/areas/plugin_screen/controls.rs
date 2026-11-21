//! The declared controls of a plugin screen drawn with the app's kit (ADR
//! 0079 contract one, ADR 0115 "Surfaces"), as the page's
//! `features/plugins/surface/`.
//!
//! [Decision Log]
//! - Purpose: one renderer for every declared kind, so a plugin's screen
//!   looks like the app's own pages.
//! - Choice: an exhaustive match over the declared kinds; a kind added to
//!   the wire fails the build here. Layout kinds take the design's spacing,
//!   a card is the settings card with a divider between rows, a status is
//!   the kit's dot with its words.
//! - What a control sends: a text field its text on Enter, a choice its
//!   option id, a toggle its new state (`{value}`), a button its declared
//!   input; a screen's context is global. A toggle or choice shows its new
//!   value only until the call answers; then the declaration wins.
//! - Stated differences: a choice is drawn as the kit's pill choices, and a
//!   popover button unfolds its controls in place under it. The settings
//!   controls are drawn by the settings screens' packet (N2) and draw
//!   nothing here yet.

use vessel_app_kit::settings;
use vessel_app_kit::theme::{radius, spacing};
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, Hsla, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, ControlSizing, Tokens, Type, TypeExt as _, button_with_kind_at, dot,
    markdown,
};
use vessel_app_view::plugins::{
    ControlValue, ScreenAddress, action_needs_admin, control_input, screen_call,
};
use vessel_proto::RawJson;
use vessel_proto::plugins::{
    Action, Button, ButtonBehavior, ContributionId, Control, ControlContent, RpcName, StatusTone,
    Surface,
};

use super::{control_id, inputs};
use crate::shell::Shell;

/// What every control of one drawn screen shares.
#[derive(Debug, Clone, Copy)]
pub(super) struct Scope<'a> {
    /// The screen.
    pub(super) at: &'a ScreenAddress,
    /// Whether this device may call the plugin's methods.
    pub(super) may_invoke: bool,
    /// A phone's width.
    pub(super) compact: bool,
}

impl Scope<'_> {
    const fn sizing(&self) -> ControlSizing {
        if self.compact {
            ControlSizing::Touch
        } else {
            ControlSizing::Pointer
        }
    }
}

/// The text fields of `surface`, nested ones included, with the method
/// each submits to.
pub(super) fn text_fields(surface: &Surface) -> Vec<(ContributionId, RpcName)> {
    let mut found = Vec::new();
    collect_fields(&surface.controls, &mut found);
    found
}

fn collect_fields(controls: &[Control], found: &mut Vec<(ContributionId, RpcName)>) {
    for control in controls {
        match &control.content {
            ControlContent::TextField { submit, .. } => {
                found.push((control.id.clone(), submit.clone()));
            }
            ControlContent::Stack { surface }
            | ControlContent::Row { surface }
            | ControlContent::Section { surface, .. }
            | ControlContent::Card { surface } => collect_fields(&surface.controls, found),
            ControlContent::Button { button } => {
                if let ButtonBehavior::Popover { surface } = &button.behavior {
                    collect_fields(&surface.controls, found);
                }
            }
            ControlContent::Text { .. }
            | ControlContent::Markdown { .. }
            | ControlContent::Choice { .. }
            | ControlContent::Toggle { .. }
            | ControlContent::Status { .. }
            | ControlContent::SettingsSwitch { .. }
            | ControlContent::SettingsSelect { .. }
            | ControlContent::SettingsInput { .. } => {}
        }
    }
}

impl Shell {
    /// Call `method` from the open screen with `input`; `control`'s pending
    /// value clears when the call answers.
    pub(super) fn plugin_screen_call(
        &mut self,
        method: &RpcName,
        input: RawJson,
        control: Option<ContributionId>,
    ) {
        let Some(at) = self.areas.plugin_screen.open.clone() else {
            return;
        };
        let Some(found) = self.areas.plugin_catalogs.screen(&at) else {
            return;
        };
        let call = screen_call(&at, found.installation, method, input);
        let Some((machine, asked)) = self.areas.plugin_catalogs.invoke(call) else {
            return;
        };
        if let Some(control) = control {
            self.areas.plugin_screen.calls.push((asked.ticket, control));
        }
        self.plugin_catalog_calls(vec![(machine, asked)]);
    }

    /// A text field's Enter.
    pub(super) fn plugin_screen_submit(
        &mut self,
        _control: &ContributionId,
        method: &RpcName,
        text: String,
        cx: &mut Context<'_, Self>,
    ) {
        self.plugin_screen_call(method, control_input(&ControlValue::Text(text)), None);
        cx.notify();
    }
}

/// `controls`, in order.
pub(super) fn render(
    shell: &Shell,
    scope: &Scope<'_>,
    controls: &[Control],
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    controls
        .iter()
        .filter_map(|control| one(shell, scope, control, tokens, window, cx))
        .collect()
}

/// One control, or nothing (a hidden button, a settings control).
fn one(
    shell: &Shell,
    scope: &Scope<'_>,
    control: &Control,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let element = match &control.content {
        ControlContent::Text { text } => div()
            .type_(Type::Body)
            .text_color(tokens.text_primary)
            .child(text.as_str().to_owned())
            .into_any_element(),
        ControlContent::Markdown { text } => {
            markdown(control_id("md", &control.id), text.as_str().to_owned()).into_any_element()
        }
        ControlContent::Stack { surface } => div()
            .flex()
            .flex_col()
            .gap(px(spacing::S3))
            .children(render(shell, scope, &surface.controls, tokens, window, cx))
            .into_any_element(),
        ControlContent::Row { surface } => div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(spacing::S2))
            .children(render(shell, scope, &surface.controls, tokens, window, cx))
            .into_any_element(),
        ControlContent::Section { title, surface } => div()
            .flex()
            .flex_col()
            .gap(px(spacing::S3))
            .child(
                div()
                    .type_(Type::Label)
                    .text_color(tokens.text_secondary)
                    .child(title.as_str().to_owned()),
            )
            .children(render(shell, scope, &surface.controls, tokens, window, cx))
            .into_any_element(),
        ControlContent::Card { surface } => card(shell, scope, surface, tokens, window, cx),
        ControlContent::Status { label, tone } => div()
            .flex()
            .items_center()
            .gap(px(spacing::S2))
            .child(dot(tone_color(*tone, tokens)))
            .child(
                div()
                    .type_(Type::Meta)
                    .text_color(tokens.text_secondary)
                    .child(label.as_str().to_owned()),
            )
            .into_any_element(),
        ControlContent::Button { button } => {
            return button_element(shell, scope, &control.id, button, tokens, window, cx);
        }
        ControlContent::TextField { label, .. } => {
            inputs::text_field(shell, scope, &control.id, label.as_str(), tokens, cx)
        }
        ControlContent::Choice {
            label,
            options,
            selected,
            submit,
        } => inputs::choices(
            shell,
            scope,
            &control.id,
            label.as_str(),
            options,
            selected.as_ref(),
            submit,
            tokens,
            cx,
        ),
        ControlContent::Toggle {
            label,
            state,
            submit,
        } => inputs::toggle(
            shell,
            scope,
            &control.id,
            label.as_str(),
            *state,
            submit,
            tokens,
            cx,
        ),
        ControlContent::SettingsSwitch { .. }
        | ControlContent::SettingsSelect { .. }
        | ControlContent::SettingsInput { .. } => return None,
    };
    Some(element)
}

/// The settings card with a divider between its direct children (Paseo's
/// `SettingsCard`).
fn card(
    shell: &Shell,
    scope: &Scope<'_>,
    surface: &Surface,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let rows = render(shell, scope, &surface.controls, tokens, window, cx);
    settings::card(tokens)
        .children(rows.into_iter().enumerate().map(|(index, row)| {
            div()
                .px(px(spacing::S4))
                .py(px(spacing::S3))
                .when(index > 0, |row| {
                    row.border_t_1().border_color(tokens.border_subtle)
                })
                .child(row)
        }))
        .into_any_element()
}

const fn tone_color(tone: StatusTone, tokens: &Tokens) -> Hsla {
    match tone {
        StatusTone::Neutral => tokens.text_tertiary,
        StatusTone::Success => tokens.st_ready,
        StatusTone::Warning => tokens.st_needs_you,
        StatusTone::Danger => tokens.danger,
    }
}

/// Whether this device takes `action` now: without admin only opening a
/// screen. Settings screens come with their own packet (N2); until then a
/// button opening one is drawn disabled rather than doing nothing.
pub(super) const fn takes(action: &Action, may_invoke: bool) -> bool {
    match action {
        Action::Invoke { .. } | Action::OpenScreen { .. } => {
            may_invoke || !action_needs_admin(action)
        }
        Action::OpenSettings { .. } => false,
    }
}

fn state_of(enabled: bool) -> ButtonState {
    if enabled {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    }
}

/// A declared button: an action, a menu (`menu.rs`) or a popover unfolded
/// in place.
fn button_element(
    shell: &Shell,
    scope: &Scope<'_>,
    id: &ContributionId,
    button: &Button,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    use vessel_proto::plugins::Availability;

    let shown = match button.availability {
        Availability::Hidden => return None,
        Availability::Enabled => true,
        Availability::Disabled => false,
    };
    let title = SharedString::from(button.title.as_str().to_owned());
    match &button.behavior {
        ButtonBehavior::Action { action } => {
            let enabled = shown && takes(action, scope.may_invoke);
            let mut element = button_with_kind_at(
                control_id("button", id),
                title,
                ButtonKind::Secondary,
                state_of(enabled),
                scope.sizing(),
                tokens,
            );
            if enabled {
                let action = action.clone();
                element = element.on_click(cx.listener(move |shell, _event, window, cx| {
                    shell.plugin_screen_action(&action, window, cx);
                }));
            }
            Some(element.into_any_element())
        }
        ButtonBehavior::Menu { items } => Some(super::menu::trigger(
            scope, id, title, items, shown, tokens, cx,
        )),
        ButtonBehavior::Popover { surface } => {
            let open = shell.areas.plugin_screen.unfolded.borrow().contains(id);
            let mut element = button_with_kind_at(
                control_id("popover", id),
                title,
                ButtonKind::Secondary,
                state_of(shown),
                scope.sizing(),
                tokens,
            );
            if shown {
                let toggled = id.clone();
                element = element.on_click(cx.listener(move |shell, _event, _window, cx| {
                    let mut unfolded = shell.areas.plugin_screen.unfolded.borrow_mut();
                    if let Some(at) = unfolded.iter().position(|held| *held == toggled) {
                        let _closed = unfolded.remove(at);
                    } else {
                        unfolded.push(toggled.clone());
                    }
                    drop(unfolded);
                    cx.notify();
                }));
            }
            let inside = open.then(|| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::S3))
                    .p(px(spacing::S3))
                    .rounded(px(radius::R2))
                    .border_1()
                    .border_color(tokens.border_subtle)
                    .children(render(shell, scope, &surface.controls, tokens, window, cx))
            });
            Some(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::S2))
                    .child(element)
                    .children(inside)
                    .into_any_element(),
            )
        }
    }
}

impl Shell {
    /// Take a declared action from the open screen.
    pub(super) fn plugin_screen_action(
        &mut self,
        action: &Action,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        match action {
            Action::Invoke { method, input } => {
                self.plugin_screen_call(method, input.clone(), None);
            }
            Action::OpenScreen { screen } => {
                if let Some(open) = self.areas.plugin_screen.open.clone() {
                    let at = ScreenAddress {
                        screen: screen.clone(),
                        ..open
                    };
                    self.open_plugin_screen(at, window, cx);
                }
            }
            // Never enabled until settings screens come (N2, `takes`).
            Action::OpenSettings { .. } => {}
        }
        cx.notify();
    }
}
