//! Settings' list of sections, as Paseo's settings sidebar
//! (`screens/settings-screen.tsx` `SettingsSidebar`, `sidebarStyles`):
//! on a wide window in the sidebar's place, a way back above the page's
//! groups; on a phone the root page, its groups as islands of tall rows.
//!
//! [Decision Log]
//! - Purpose: the page lists Settings' sections in the sidebar's place
//!   (`features/shell/sidebar-settings.tsx`); Paseo's look for it.
//! - Reference (Paseo, desktop): a group 8 in from every side, its label
//!   14 in `foregroundExtraMuted` 8 across and 4 down, its rows 2 apart;
//!   a row at least 28 tall, 4 down and 8 across, radius 8, a 16 glyph 8
//!   before a 14 label, both muted, both the foreground when it is the
//!   section shown; hovered or shown on `surfaceSidebarHover` (`surface3`
//!   light, `surface1` dark), pressed on `surface2`. Compact: each group
//!   an island (radius 16) of rows 52 tall on `surface1`, 16 across, a 16
//!   label 12 after its glyph, 2 apart.
//! - Choice: the page's groups (the same sections in the same order on
//!   every surface, ADR 0036); Paseo's host picker (its quiet row: a dot
//!   and the machine's name in the foreground) heads the first group about
//!   a machine, so it governs every section listed after it, as Paseo's
//!   heads its Host group. Without a machine its place is "Add machine",
//!   as Paseo's "Add host".
//! - Differences: the list stands on the shell, as the workspace sidebar
//!   does, at its width (264), not on `surfaceSidebar` with an edge at
//!   Paseo's 320: the frame beside it does not move when Settings opens.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    Context, Div, ElementId, FluentBuilder as _, Hsla, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Stateful, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{
    ComboHeader, Scheme, Side, Tokens, Type, TypeExt as _, anchor_probe, dot, glyph,
};
use vessel_app_view::settings::{Group, Section};

use super::choices::{self, List};
use super::page;
use crate::routes::Route;
use crate::shell::Shell;

/// The machine picker's list (Paseo's `desktopMinWidth`).
pub(crate) const PICKER_WIDTH: f32 = 240.0;

/// The workspace sidebar's width (`areas/sidebar.rs`).
const WIDTH: f32 = 264.0;

/// A section's glyph (Paseo's for the sections it has).
pub(crate) const fn section_glyph(section: Section) -> Glyph {
    match section {
        Section::General => Glyph::Gear,
        Section::Appearance => Glyph::Palette,
        Section::Shortcuts => Glyph::Keyboard,
        Section::Notifications => Glyph::Bell,
        Section::Agents => Glyph::Bot,
        Section::Models => Glyph::Sparkles,
        Section::Machines => Glyph::Server,
        Section::Projects => Glyph::Folder,
        Section::Devices => Glyph::Smartphone,
        Section::Sync => Glyph::Refresh,
        Section::About => Glyph::Info,
    }
}

/// Paseo's `surfaceSidebarHover`.
fn hover_fill(tokens: &Tokens) -> Hsla {
    match tokens.scheme {
        Scheme::Light => tokens.neutral.surface3,
        Scheme::Dark => tokens.neutral.surface1,
    }
}

/// A row of the list: `leading`, then `label`; `shown` when it is the
/// section on screen; `compact` on a phone.
fn item(
    id: impl Into<ElementId>,
    leading: impl gpui::IntoElement,
    label: impl Into<gpui::SharedString>,
    shown: bool,
    compact: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let fill = hover_fill(tokens);
    div()
        .id(id)
        .relative()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(if compact { 12.0 } else { 8.0 }))
        .min_h(px(if compact { 52.0 } else { 28.0 }))
        .py(px(4.0))
        .px(px(if compact { 16.0 } else { 8.0 }))
        .when(!compact, |row| row.rounded(px(8.0)))
        .when(compact, |row| row.bg(neutral.surface1))
        .when(shown, |row| row.bg(fill))
        .hover(move |style| style.bg(fill))
        .active(|style| style.bg(neutral.surface2))
        .cursor_pointer()
        .child(leading)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .type_(Type::Body)
                .when(compact, |label| {
                    label.text_size(px(vessel_app_kit::fonts::ui(16.0)))
                })
                .text_color(if shown {
                    neutral.foreground
                } else {
                    neutral.foreground_muted
                })
                .child(label.into()),
        )
}

/// A section's row.
fn section_row(
    section: Section,
    shell: &Shell,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let words = shell.t.lang.settings();
    // On a phone no section is "shown" in the list: it is the page.
    let shown = !compact && shell.areas.settings.section == section;
    let color = if shown {
        tokens.neutral.foreground
    } else {
        tokens.neutral.foreground_muted
    };
    item(
        ElementId::Name(format!("settings-{}", section.slug()).into()),
        glyph(section_glyph(section), 16.0, color),
        words.section(section),
        shown,
        compact,
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, _window, cx| {
        shell.settings_section(section, cx);
    }))
}

/// The machine picker's trigger, or "Add machine" without a machine.
fn picker(
    shell: &Shell,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let Some(machine) = choices::chosen_machine(shell) else {
        return item(
            "settings-add-machine",
            glyph(Glyph::Plus, 16.0, tokens.neutral.foreground_muted),
            shell.t.add_machine,
            false,
            compact,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| {
            shell.go(Route::Pair, window, cx);
        }));
    };
    let open = shell.areas.settings.showing() == Some(List::Machine);
    item(
        "settings-machine",
        // Paseo keeps the dot in the glyphs' 16 so the names line up.
        div()
            .size(px(16.0))
            .flex()
            .items_center()
            .justify_center()
            .child(dot(choices::reach_color(machine, tokens))),
        machine.name.clone(),
        open,
        compact,
        tokens,
    )
    // The name in the foreground, as Paseo's trigger shows it.
    .text_color(tokens.neutral.foreground)
    .child(anchor_probe(shell.areas.settings.anchors.of(List::Machine)))
    .on_click(cx.listener(|shell, _event, window, cx| {
        shell.open_list(List::Machine, window, cx);
    }))
}

/// The groups, each a label over its rows.
fn groups(shell: &Shell, compact: bool, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.settings();
    let neutral = tokens.neutral;
    let mut groups = div().flex().flex_col();
    let mut picked = false;
    for group in Group::ALL {
        // Only the sections drawn in this build (`super::drawn`); a group
        // left with none is not shown.
        let sections: Vec<Section> = group
            .sections()
            .into_iter()
            .filter(|section| super::drawn(*section))
            .collect();
        if sections.is_empty() {
            continue;
        }
        let mut rows = Vec::new();
        if !picked && sections.iter().any(|section| section.per_machine()) {
            picked = true;
            rows.push(picker(shell, compact, tokens, cx));
        }
        for section in sections {
            rows.push(section_row(section, shell, compact, tokens, cx));
        }
        // An island's corners are its first and last rows' (a clip is a
        // rectangle here, so the island cannot round them).
        let last = rows.len().saturating_sub(1);
        let island =
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .children(rows.into_iter().enumerate().map(|(index, row)| {
                    row.when(compact && index == 0, |row| row.rounded_t(px(16.0)))
                        .when(compact && index == last, |row| row.rounded_b(px(16.0)))
                }));
        groups = groups.child(
            div()
                .flex()
                .flex_col()
                .p(px(8.0))
                .gap(px(if compact { 4.0 } else { 2.0 }))
                .child(
                    div()
                        .px(px(if compact { 16.0 } else { 8.0 }))
                        .py(px(4.0))
                        .type_(Type::Body)
                        .text_color(neutral.foreground_extra_muted)
                        .child(words.group(group)),
                )
                .child(island),
        );
    }
    groups
}

/// On a wide window, in the sidebar's place: the way back, then the
/// groups.
pub(crate) fn sidebar(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.settings();
    div()
        .w(px(WIDTH))
        .flex_shrink_0()
        .h_full()
        .flex()
        .flex_col()
        .bg(tokens.bg_shell)
        .child(
            div().p(px(8.0)).child(
                item(
                    "settings-back",
                    glyph(Glyph::ArrowLeft, 16.0, tokens.neutral.foreground_muted),
                    words.back,
                    false,
                    false,
                    tokens,
                )
                .on_click(cx.listener(|shell, _event, window, cx| {
                    shell.settings_back(false, window, cx);
                })),
            ),
        )
        .child(
            div()
                .id("settings-sidebar")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(groups(shell, false, tokens, cx)),
        )
}

/// On a phone, the root page: a back header titled Settings, then the
/// groups as islands.
pub(crate) fn compact(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.settings();
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(page::compact_header(words.settings, false, tokens, cx))
        .child(
            div()
                .id("settings-list")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(
                    div()
                        .p(px(8.0))
                        .child(groups(shell, true, tokens, cx))
                        .into_any_element(),
                ),
        )
}

/// The machine picker's list: no search; titled in a phone's sheet.
pub(crate) fn picker_header(shell: &Shell, compact: bool) -> ComboHeader {
    let words = shell.t.lang.settings();
    ComboHeader {
        title: compact.then(|| words.switch_machine.into()),
        back: false,
        search: false,
        empty: words.no_machines.into(),
        side: Side::Below,
    }
}
