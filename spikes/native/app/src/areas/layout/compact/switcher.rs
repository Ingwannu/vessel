//! The tab switcher's sheet, "Workspace tabs" (Paseo's
//! `MobileWorkspaceTabSwitcher`): a row per tab with the current one
//! checked and a trailing "…", and a footer with New agent and New
//! terminal.
//!
//! [Decision Log]
//! - Values (Paseo's `WorkspaceTabOptionRow`): a row at least 36 tall, 4
//!   inside, 4 from the sides and under it, 8 between its parts; its main
//!   press 8 inside, `surface1` under the pointer, a 16 slot for the tab's
//!   icon, the label 14 in the foreground on one line; the check 16 muted
//!   in a 16 slot; the "…" 28 square at radius 6 with a 14 muted glyph,
//!   `surface2` while its menu is open or it is pressed. The footer: two
//!   secondary buttons side by side, `SquarePen` and `SquareTerminal`, 12
//!   between them, 24 across and 12 down.
//! - Choice: the kit dialog as a fitting sheet (Paseo's combobox sheet is a
//!   bottom sheet with a title, a scrolling body and a footer); the rows are
//!   drawn here because the kit combobox has neither a row's trailing
//!   control nor a footer.
//! - New terminal stays disabled until a terminal tab can be made in a
//!   pane, as 776's new-tab menu disables Terminal.

use super::{Compact, current, tabs};
use crate::areas::layout::draw::tab_label;
use crate::routes::Route;
use crate::shell::Shell;
use vessel_app_kit::agent_mark::{Size, Space};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::dialog::{CompactLayout, DesktopHeight, DialogLayout};
use vessel_app_kit::sheet::snap::Percentage;
use vessel_app_kit::theme::spacing::{S0, S1, S2, S3};
use vessel_app_kit::theme::{icon, radius};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, Stateful, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonContent, ButtonIcon, ButtonKind, ButtonState, ControlSizing, DialogContent, DialogSize,
    Tokens, Type, TypeExt as _, anchor_probe, glyph,
};
use vessel_app_view::layout::{Layout, Tab, TabId, Target};

/// Paseo's `optionRow` `minHeight`.
const ROW_MIN: f32 = 36.0;
/// Paseo's `mobileTabMenuTrigger`: 28 square.
const TRIGGER: f32 = 28.0;
/// Paseo's ellipsis in that trigger.
const TRIGGER_GLYPH: f32 = 14.0;
/// The kit dialog's desktop width (the sheet is a phone's only).
const WIDTH: u16 = 520;
/// The kit combobox sheet's cap: 90% of the window.
const SHEET_CAP: u8 = 90;

/// A fitting sheet as tall as its rows, at most 90% of the window; the rows
/// carry their own insets.
fn size() -> DialogSize {
    Percentage::new(SHEET_CAP)
        .and_then(|cap| {
            DialogLayout::new(
                WIDTH,
                DesktopHeight::AtMost(cap),
                CompactLayout::Fixed(cap),
            )
        })
        .map_or(DialogSize::Normal, |layout| {
            DialogSize::Configured(
                layout.with_body_spacing(vessel_app_kit::dialog::BodySpacing::Caller),
            )
        })
}

/// The sheet, when open.
pub(super) fn sheet(
    shell: &Shell,
    layout: &Layout,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.layout.compact;
    if !area.switcher_open {
        return None;
    }
    let words = shell.words.layout();
    let shown = current(layout);
    let rows = tabs(layout)
        .into_iter()
        .filter_map(|id| layout.tab(id))
        .map(|tab| row(shell, area, tab, shown == Some(tab.id()), tokens, cx));
    let body = div().flex().flex_col().pt(px(S1)).children(rows);
    Some(vessel_app_kit::dialog(
        &area.switcher,
        DialogContent {
            title: words.workspace_tabs().into(),
            body: body.into_any_element(),
            footer: vec![footer(shell, tokens, cx).into_any_element()],
            size: size(),
        },
        tokens,
        window,
        vessel_app_kit::breakpoint::phone(window),
        |shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>| {
            shell.compact_close_switcher(window, cx);
        },
        cx,
    ))
}

/// One tab: its press shows it; its "…" opens the tab's menu.
fn row(
    shell: &Shell,
    area: &Compact,
    tab: &Tab,
    selected: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let id = tab.id();
    let neutral = tokens.neutral;
    let label = tab_label(shell, tab.target());
    let hover = neutral.surface1;
    let main = div()
        .id(SharedString::from(format!("compact-tab-{}", id.get())))
        .flex()
        .flex_row()
        .items_center()
        .flex_1()
        .min_w(px(S0))
        .gap(px(S2))
        .p(px(S2))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .active(move |style| style.bg(hover))
        .child(
            div()
                .flex()
                .flex_none()
                .justify_center()
                .w(px(icon::MD))
                .child(tab_icon(shell, tab.target(), selected, tokens)),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(S0))
                .truncate()
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .child(label.clone()),
        )
        .on_click(cx.listener(move |shell, _event, window, cx| {
            shell.compact_select(id, window, cx);
        }));
    let check = selected.then(|| {
        div()
            .flex()
            .flex_none()
            .justify_center()
            .w(px(icon::MD))
            .child(glyph(Glyph::Check, icon::MD, neutral.foreground_muted))
    });
    let row = div();
    #[cfg(test)]
    let row = row.debug_selector(move || format!("compact-tab-row-{}", id.get()));
    row.flex()
        .flex_row()
        .items_center()
        .min_h(px(ROW_MIN))
        .gap(px(S2))
        .p(px(S1))
        .mx(px(S1))
        .mb(px(S1))
        .child(main)
        .children(check)
        .child(trigger(shell, area, id, &label, tokens, cx))
}

/// The row's "…" (Paseo's `MobileTabTrailingAccessory`).
fn trigger(
    shell: &Shell,
    area: &Compact,
    id: TabId,
    label: &str,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let layout = &shell.areas.layout;
    let open = layout.menu.is_open()
        && layout.menu_surface == crate::areas::layout::menu::Surface::Switcher
        && layout.menu_tab.as_ref().is_some_and(|(_, tab)| *tab == id);
    let name = shell
        .words
        .layout()
        .tab_menu_for(label)
        .unwrap_or_else(|_| shell.words.layout().menu().to_owned());
    let pressed = neutral.surface2;
    let place = area.row_place(id);
    let trigger = div();
    #[cfg(test)]
    let trigger = trigger.debug_selector(move || format!("compact-tab-menu-{}", id.get()));
    trigger
        .id(SharedString::from(format!("compact-tab-menu-{}", id.get())))
        .role(gpui::Role::Button)
        .aria_label(name)
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(TRIGGER))
        .rounded(px(radius::MD))
        .cursor_pointer()
        .when(open, move |trigger| trigger.bg(pressed))
        .active(move |style| style.bg(pressed))
        .child(glyph(Glyph::More, TRIGGER_GLYPH, neutral.foreground_muted))
        .child(anchor_probe(&place))
        .on_click(cx.listener(move |shell, _event, window, cx| {
            shell.compact_row_menu(id, window, cx);
        }))
}

/// The tab's icon (Paseo's `WorkspaceTabIcon`), in the foreground for the
/// current tab and muted for the others.
fn tab_icon(shell: &Shell, target: &Target, selected: bool, tokens: &Tokens) -> AnyElement {
    let ink = if selected {
        tokens.neutral.foreground
    } else {
        tokens.neutral.foreground_muted
    };
    let mark = match target {
        Target::Agent(id) => {
            let harness = shell
                .areas
                .conversation
                .info
                .as_deref()
                .filter(|info| info.id == *id)
                .map(|info| &info.spec.harness);
            let art = if selected {
                crate::agent_mark::mark(harness, Size::Sixteen, Space::Compact, tokens)
            } else {
                crate::agent_mark::mark_muted(harness, Size::Sixteen, Space::Compact, tokens)
            };
            return art.into_any_element();
        }
        Target::Terminal(_) => Glyph::SquareTerminal,
        Target::Draft(_) => Glyph::SquarePen,
        Target::NewTab => Glyph::Plus,
        Target::Browser(_) => Glyph::Globe,
        Target::Changes | Target::WorkingDiff | Target::CommitDiff(_) => Glyph::FileDiff,
        Target::Files => Glyph::Files,
        Target::PullRequest => Glyph::PullRequest,
        Target::File { .. }
        | Target::ProviderSubagent { .. }
        | Target::Scripts
        | Target::Plugin { .. }
        | Target::Setup(_) => Glyph::File,
    };
    glyph(mark, icon::MD, ink).into_any_element()
}

/// Paseo's footer: New agent and New terminal, side by side.
fn footer(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.layout();
    let agent = vessel_app_kit::button_with_content(
        "compact-tab-switcher-new-agent",
        ButtonContent::IconAndLabel {
            icon: ButtonIcon::new(Glyph::SquarePen),
            label: words.new_agent().into(),
        },
        ButtonKind::Secondary,
        ButtonState::Enabled,
        ControlSizing::Touch,
        tokens,
    )
    .flex_1()
    .on_click(cx.listener(|shell, _event, window, cx| {
        shell.compact_close_switcher(window, cx);
        shell.go(Route::Draft, window, cx);
    }));
    let terminal = vessel_app_kit::button_with_content(
        "compact-tab-switcher-new-terminal",
        ButtonContent::IconAndLabel {
            icon: ButtonIcon::new(Glyph::SquareTerminal),
            label: words.new_terminal().into(),
        },
        ButtonKind::Secondary,
        ButtonState::Disabled,
        ControlSizing::Touch,
        tokens,
    )
    .flex_1();
    let footer = div();
    #[cfg(test)]
    let footer = footer.debug_selector(|| "compact-tab-switcher-footer".to_owned());
    footer
        .flex()
        .flex_row()
        .w_full()
        .gap(px(S3))
        .child(agent)
        .child(terminal)
}
