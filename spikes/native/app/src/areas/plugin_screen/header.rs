//! The plugin screen's header: Paseo's (`plugins/surface-screen.tsx`
//! 167-210) in the kit's screen header frame.
//!
//! [Decision Log]
//! - Purpose: the coordinator's RETURN on P3b-1 native N1 (2026-11-21).
//!   The first header was a title with the machine under it and a text
//!   "Close plugin" button; Paseo's is laid out differently.
//! - Paseo on a wide window: the plugin's icon in a muted `HeaderIconBadge`
//!   (the large icon-button frame, 26 here), then `ScreenTitle`. On the
//!   right, the `HostFilter` when the screen is declared on two or more
//!   connected machines. Then `HeaderToggleButton`: an 18 muted X labelled
//!   "Close plugin", with the tooltip "Close" below it.
//! - Paseo on a phone: `CompactHeader` with a back button, the title and
//!   the host filter; no close.
//! - The machine is named only by the host filter, when there is a choice.
//!   A Vessel declaration has no icon, so the badge holds the plugin glyph
//!   that the sidebar rows use (`Blocks`).
//! - Choosing another machine opens the same screen there, and that
//!   machine is the one the sidebar row remembers (Paseo's
//!   `rememberPluginContributionHost`).

use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::screen_header::{Content, Density, frame};
use vessel_app_kit::toolbar_button::{ToolbarButton, ToolbarConfig, ToolbarIcon, ToolbarInk};
use vessel_app_kit::tooltip::Side as TipSide;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, IntoElement as _, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonState, ComboHandlers, ComboHeader, ComboRow, ControlSizing, Leading, Side, Tokens,
    combobox, glyph, host_filter, select,
};
use vessel_app_view::devices::MachineKey;
use vessel_app_view::plugins::ScreenAddress;

use crate::shell::Shell;

/// `HeaderIconBadge`: the large icon-button frame on a pointer
/// (`iconButtonChromeFrameStyle("large")`), holding a 16 icon.
const BADGE: f32 = 26.0;
const BADGE_GLYPH: f32 = 16.0;

/// The connected machines declaring the open screen, in book order.
fn machines(shell: &Shell) -> Vec<MachineKey> {
    let Some(at) = shell.areas.plugin_screen.open.as_ref() else {
        return Vec::new();
    };
    let online: Vec<MachineKey> = shell
        .fleet
        .online()
        .map(|machine| MachineKey::from_book(&machine.name))
        .collect();
    shell
        .areas
        .plugin_catalogs
        .screen_machines(&at.plugin, &at.screen, &online)
}

/// The header for the screen titled `title`.
pub(super) fn render(
    shell: &Shell,
    title: SharedString,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.plugins();
    let area = &shell.areas.plugin_screen;
    let sizing = ControlSizing::from_compact(compact);
    let filter = area
        .open
        .as_ref()
        .filter(|_| machines(shell).len() > 1)
        .map(|at| host_trigger(shell, at, tokens, cx));
    if compact {
        let back = ToolbarButton::new(
            "plugin-screen-back",
            words.screen_close().to_owned(),
            ToolbarIcon::new(Glyph::ArrowLeft, ToolbarInk::Muted).adaptive(),
            ToolbarConfig::new(sizing, ButtonState::Enabled).header_island(),
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| shell.close_plugin_screen(window, cx)));
        return frame(
            Content {
                title,
                leading: Some(back.into_any_element()),
                actions: filter,
                subtitle: None,
            },
            Density::Compact,
            tokens,
        );
    }
    let close = ToolbarButton::new(
        "plugin-screen-close",
        words.screen_close().to_owned(),
        ToolbarIcon::new(Glyph::X, ToolbarInk::Muted).eighteen(),
        ToolbarConfig::new(sizing, ButtonState::Enabled).large(),
        tokens,
    )
    .tooltip_text(
        &area.tips,
        &area.close_place,
        words.screen_close_tip(),
        TipSide::Bottom,
    )
    .on_click(cx.listener(|shell, _event, window, cx| shell.close_plugin_screen(window, cx)));
    let badge = div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(BADGE))
        .child(glyph(
            Glyph::Blocks,
            BADGE_GLYPH,
            tokens.neutral.foreground_muted,
        ));
    let actions = div()
        .flex()
        .items_center()
        .children(filter)
        .child(close)
        .into_any_element();
    frame(
        Content {
            title,
            leading: Some(badge.into_any_element()),
            actions: Some(actions),
            subtitle: None,
        },
        Density::Desktop,
        tokens,
    )
}

/// The host filter naming the screen's machine.
fn host_trigger(
    shell: &Shell,
    at: &ScreenAddress,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let area = &shell.areas.plugin_screen;
    let online = shell
        .fleet
        .online()
        .any(|held| MachineKey::from_book(&held.name) == at.machine);
    host_filter::trigger(
        "plugin-screen-host-filter",
        at.machine.as_str().to_owned(),
        online,
        true,
        &area.host_anchor,
        tokens,
    )
    .on_click(cx.listener(|shell, _event, window, cx| {
        let area = &mut shell.areas.plugin_screen;
        area.host_list.set_anchor(area.host_anchor.get());
        area.host_list.open(None, window, cx);
        cx.notify();
    }))
    .into_any_element()
}

/// The host list under the filter, while it is open.
pub(super) fn list(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.plugin_screen;
    let at = area.open.as_ref()?;
    let choices = machines(shell);
    let rows: Vec<ComboRow> = choices
        .iter()
        .map(|machine| {
            let mut row = ComboRow::new(machine.as_str().to_owned());
            row.leading = Some(Leading::Dot(host_filter::status(true, tokens)));
            row.checked = *machine == at.machine;
            row
        })
        .collect();
    let header = ComboHeader {
        side: Side::Below,
        ..select::header()
    };
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.plugin_screen.host_list,
        chose: Rc::new(move |shell: &mut Shell, index, window, cx| {
            let Some(machine) = choices.get(index) else {
                return;
            };
            let area = &mut shell.areas.plugin_screen;
            area.host_list.close_and_restore(window, cx);
            // The list was drawn for the screen open then.
            let Some(open) = area.open.clone() else {
                return;
            };
            let at = ScreenAddress {
                machine: machine.clone(),
                ..open
            };
            shell.open_plugin_screen(at, window, cx);
        }),
        back: None,
    };
    combobox(
        "plugin-screen-host-list",
        &area.host_list,
        &header,
        &rows,
        &handlers,
        select::WIDTH,
        tokens,
        window,
        cx,
    )
}
