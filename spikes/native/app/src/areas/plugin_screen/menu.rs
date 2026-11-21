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
//! - The button opens its menu under it on a press, and on a click too, so
//!   assistive technology's Click opens it; a click that ends a press
//!   leaves the menu open rather than toggling it shut. It reports whether
//!   its menu is open (app review R1 on P3b-1 N1, as the Settings ›
//!   Plugins row menu's P2 R1).

use std::rc::Rc;

use vessel_app_kit::menu::{Content, MenuHandlers, MenuItem, menu_with};
use vessel_app_kit::ui::{
    AnyElement, ClickEvent, Context, InteractiveElement as _, IntoElement as _, MouseButton,
    MouseDownEvent, ParentElement as _, Pixels, Point, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, anchor_probe, button_with_kind_at};
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
#[expect(
    clippy::too_many_arguments,
    reason = "a menu button draws from the shell, the scope and its declaration together"
)]
pub(super) fn trigger(
    shell: &Shell,
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
    let area = &shell.areas.plugin_screen;
    let open = area.menu.is_open() && area.menu_for.as_ref() == Some(id);
    let anchor = Rc::clone(area.anchors.borrow_mut().entry(id.clone()).or_default());
    let element = element
        .relative()
        .aria_expanded(open)
        .child(anchor_probe(&anchor));
    if !enabled {
        return element.into_any_element();
    }
    let (pressed, clicked) = (id.clone(), id.clone());
    element
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |shell, event: &MouseDownEvent, window, cx| {
                shell.plugin_screen_menu_activate(&pressed, event.position, window, cx);
            }),
        )
        .on_click(cx.listener(move |shell, event: &ClickEvent, window, cx| {
            shell.plugin_screen_menu_activate(&clicked, event.position(), window, cx);
        }))
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
    /// Open menu button `id`'s menu under the button (at `at` before the
    /// button was first drawn), unless it is open for it already.
    pub(super) fn plugin_screen_menu_activate(
        &mut self,
        id: &ContributionId,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.plugin_screen;
        if area.menu.is_open() && area.menu_for.as_ref() == Some(id) {
            return;
        }
        let bounds = area
            .anchors
            .borrow()
            .get(id)
            .and_then(|anchor| anchor.get());
        area.menu_for = Some(id.clone());
        match bounds {
            Some(bounds) => area.menu.open_start(bounds, window, cx),
            None => area.menu.open(at, window, cx),
        }
        cx.notify();
    }

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
