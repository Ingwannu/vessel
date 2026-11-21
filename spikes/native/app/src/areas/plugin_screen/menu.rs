//! A declared menu button on a plugin screen (ADR 0079 contract one): the
//! kit's menu under the button, as the Settings › Plugins row menu opens
//! its own (`settings/plugins/rows.rs`), one menu open at a time.
//!
//! [Decision Log]
//! - The menu lists the button's entries in order. An entry's action is
//!   taken as a button's would be (`controls::takes`): without admin, an
//!   entry that calls a method is disabled; opening a screen stays enabled.
//! - Stated difference: a nested menu or a popover inside a menu is not
//!   offered (a popover has no anchor once the menu closes), as on the
//!   page.

use std::rc::Rc;

use vessel_app_kit::menu::{Content, MenuHandlers, MenuItem, menu_with};
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, MouseButton, MouseDownEvent,
    SharedString, Window, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, button_with_kind_at};
use vessel_proto::plugins::{
    Action, Availability, ButtonBehavior, ContributionId, Control, ControlContent, MenuEntry,
};

use super::control_id;
use super::controls::{Scope, takes};
use crate::shell::Shell;

/// Paseo's menu width.
const MENU_WIDTH: f32 = 220.0;

/// A menu's entries as drawn: words, the action, whether it can be taken.
fn entries(items: &[MenuEntry], may_invoke: bool) -> Vec<(SharedString, Action, bool)> {
    items
        .iter()
        .filter_map(|entry| match entry {
            MenuEntry::Separator { .. } => None,
            MenuEntry::Item {
                title,
                availability,
                behavior,
                ..
            } => {
                let ButtonBehavior::Action { action } = behavior else {
                    return None;
                };
                let enabled = match availability {
                    Availability::Hidden => return None,
                    Availability::Enabled => takes(action, may_invoke),
                    Availability::Disabled => false,
                };
                Some((
                    SharedString::from(title.as_str().to_owned()),
                    action.clone(),
                    enabled,
                ))
            }
        })
        .collect()
}

/// The menu button `id`, opening its menu where it is pressed.
pub(super) fn trigger(
    scope: &Scope<'_>,
    id: &ContributionId,
    title: SharedString,
    items: &[MenuEntry],
    shown: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let enabled = shown && !entries(items, scope.may_invoke).is_empty();
    let state = if enabled {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    };
    let element = button_with_kind_at(
        control_id("menu", id),
        title,
        ButtonKind::Secondary,
        state,
        if scope.compact {
            vessel_app_kit::ControlSizing::Touch
        } else {
            vessel_app_kit::ControlSizing::Pointer
        },
        tokens,
    );
    if !enabled {
        return element.into_any_element();
    }
    let opened = id.clone();
    element
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |shell, event: &MouseDownEvent, window, cx| {
                let area = &mut shell.areas.plugin_screen;
                area.menu_for = Some(opened.clone());
                area.menu.open(event.position, window, cx);
                cx.notify();
            }),
        )
        .into_any_element()
}

/// The open menu, while one is open.
pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    phone: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.plugin_screen;
    let id = area.menu_for.as_ref()?;
    let found = shell.areas.plugin_catalogs.screen(area.open.as_ref()?)?;
    let items = find_menu(&found.surface.controls, id)?;
    let drawn: Vec<MenuItem> = entries(items, found.may_invoke)
        .into_iter()
        .map(|(title, _action, enabled)| {
            let mut item = MenuItem::new(title);
            item.disabled = !enabled;
            item
        })
        .collect();
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.plugin_screen.menu,
        chose: Rc::new(|shell: &mut Shell, index, window, cx| {
            shell.plugin_screen_menu_chose(index, window, cx);
        }),
    };
    let content = Content::new("plugin-screen-menu", &drawn).with_width(px(MENU_WIDTH));
    menu_with(content, &area.menu, &handlers, tokens, phone, cx)
}

/// The entries of the menu button `id` anywhere in `controls`.
fn find_menu<'a>(controls: &'a [Control], id: &ContributionId) -> Option<&'a [MenuEntry]> {
    controls.iter().find_map(|control| match &control.content {
        ControlContent::Button { button } => match &button.behavior {
            ButtonBehavior::Menu { items } if control.id == *id => Some(items.as_slice()),
            ButtonBehavior::Popover { surface } => find_menu(&surface.controls, id),
            ButtonBehavior::Menu { .. } | ButtonBehavior::Action { .. } => None,
        },
        ControlContent::Stack { surface }
        | ControlContent::Row { surface }
        | ControlContent::Section { surface, .. }
        | ControlContent::Card { surface } => find_menu(&surface.controls, id),
        ControlContent::Text { .. }
        | ControlContent::Markdown { .. }
        | ControlContent::TextField { .. }
        | ControlContent::Choice { .. }
        | ControlContent::Toggle { .. }
        | ControlContent::Status { .. }
        | ControlContent::SettingsSwitch { .. }
        | ControlContent::SettingsSelect { .. }
        | ControlContent::SettingsInput { .. } => None,
    })
}

impl Shell {
    /// Entry `index` of the open menu was chosen.
    fn plugin_screen_menu_chose(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.plugin_screen;
        let Some(id) = area.menu_for.take() else {
            return;
        };
        let Some(open) = area.open.as_ref() else {
            return;
        };
        let Some(found) = self.areas.plugin_catalogs.screen(open) else {
            return;
        };
        let chosen = find_menu(&found.surface.controls, &id)
            .and_then(|items| entries(items, found.may_invoke).into_iter().nth(index))
            .filter(|(_, _, enabled)| *enabled)
            .map(|(_, action, _)| action);
        if let Some(action) = chosen {
            self.plugin_screen_action(&action, window, cx);
        }
    }
}
