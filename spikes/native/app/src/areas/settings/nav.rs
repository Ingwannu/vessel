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
//! - Choice: Paseo's two groups (`SettingsSidebar`): App, then the
//!   machine's (Paseo's Host) headed by Paseo's host picker (its quiet
//!   row: a dot and the machine's name in the foreground), which governs
//!   every section under it. Without a machine that group is one row
//!   under no heading, "Add a machine", as Paseo's "Add host". Paseo's
//!   settings list has no search, so neither has this.
//! - Differences: the list stands on the shell, as the workspace sidebar
//!   does, at its width (dragged at the same edge, `areas::sidebar::frame`),
//!   not on `surfaceSidebar` with an edge at Paseo's 320: the frame beside
//!   it does not move when Settings opens.

use gpui::AnimationExt as _;
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

/// A section's glyph: Paseo's (`SIDEBAR_SECTION_ITEMS`,
/// `HOST_SECTION_ITEMS`, Lucide's names in the comments) where the kit
/// has it. Where it does not yet, the nearest kit glyph stands in, named
/// as one; each of those is a section not drawn yet, so not listed.
pub(crate) const fn section_glyph(section: Section) -> Glyph {
    match section {
        Section::General => Glyph::Gear,
        Section::Appearance => Glyph::Palette,
        Section::Sidebar => Glyph::PanelLeft,
        Section::Chat => Glyph::Message,
        Section::Terminal | Section::Terminals => Glyph::Terminal,
        Section::Shortcuts => Glyph::Keyboard,
        Section::Notifications => Glyph::Bell,
        Section::Permissions => Glyph::Shield,
        Section::Diagnostics => Glyph::Stethoscope,
        Section::About => Glyph::Info,
        Section::Overview => Glyph::Server,
        Section::Projects | Section::Workspaces => Glyph::FolderGit,
        Section::Devices => Glyph::Smartphone,
        Section::Agents => Glyph::Bot,
        Section::Metadata | Section::Models => Glyph::Sparkles,
        Section::Providers => Glyph::Boxes,
        // Stand-ins: Paseo's Files, Globe (Browser), Code2 (Editor), Puzzle
        // (Integrations), Network (Connections), Mic (Voice), Gauge
        // (Usage), Blocks (Plugins).
        Section::Files => Glyph::File,
        Section::Browser
        | Section::Editor
        | Section::Integrations
        | Section::Connections
        | Section::Voice
        | Section::Usage
        | Section::Plugins => Glyph::Sliders,
        // Vessel's own.
        Section::Sync => Glyph::Refresh,
    }
}

/// Paseo's `densityIconSize(rt, "md")`: 16 from its `md` breakpoint up,
/// 20 on a compact layout (here: the phone's list).
const fn icon_size(compact: bool) -> f32 {
    if compact { 20.0 } else { 16.0 }
}

/// Paseo's `surfaceSidebarHover`.
fn hover_fill(tokens: &Tokens) -> Hsla {
    match tokens.scheme {
        Scheme::Light => tokens.neutral.surface3,
        Scheme::Dark => tokens.neutral.surface1,
    }
}

/// A row of the list: `leading`, then `label`; `shown` when it is the
/// section on screen (or the list it opens is open), with the count of
/// arrivals that made it so: above 0 its fill fades in (a selection
/// moving, `motion::QUICK`), 0 draws it in place; `compact` on a phone.
fn item(
    id: impl Into<ElementId>,
    leading: impl gpui::IntoElement,
    label: impl Into<gpui::SharedString>,
    shown: Option<u64>,
    compact: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let fill = hover_fill(tokens);
    let selected = shown.map(|turn| {
        let layer = div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .when(!compact, |layer| layer.rounded(px(8.0)))
            .bg(fill);
        if turn == 0 {
            layer.into_any_element()
        } else {
            layer
                .with_animation(
                    ElementId::NamedInteger("settings-nav-selected".into(), turn),
                    vessel_app_kit::motion::transition(vessel_app_kit::motion::QUICK),
                    gpui::Styled::opacity,
                )
                .into_any_element()
        }
    });
    let shown = shown.is_some();
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
        .hover(move |style| style.bg(fill))
        .active(|style| style.bg(neutral.surface2))
        .cursor_pointer()
        // Under the glyph and the label, over the row's own fill.
        .children(selected)
        // The label takes the row's colour, so a caller can set another
        // (the machine picker's name is in the foreground).
        .text_color(if shown {
            neutral.foreground
        } else {
            neutral.foreground_muted
        })
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
    let state = &shell.areas.settings;
    // On a phone no section is "shown" in the list: it is the page.
    let shown = !compact && state.section == section;
    let color = if shown {
        tokens.neutral.foreground
    } else {
        tokens.neutral.foreground_muted
    };
    item(
        ElementId::Name(format!("settings-{}", section.slug()).into()),
        glyph(section_glyph(section), icon_size(compact), color),
        words.section(section, shell.words),
        shown.then_some(state.turn),
        compact,
        tokens,
    )
    .debug_selector(move || format!("settings-section-{}", section.slug()))
    .on_click(cx.listener(move |shell, _event, _window, cx| {
        shell.settings_section(section, cx);
    }))
}

/// "Add a machine", where the machine's group is while the book is
/// empty (Paseo's "Add host").
fn add_machine(
    compact: bool,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    item(
        "settings-add-machine",
        glyph(
            Glyph::Plus,
            icon_size(compact),
            tokens.neutral.foreground_muted,
        ),
        shell.t.add_machine,
        None,
        compact,
        tokens,
    )
    .on_click(cx.listener(|shell, _event, window, cx| {
        shell.go(Route::Pair, window, cx);
    }))
}

/// The machine picker's trigger, heading the machine's group.
fn picker(
    shell: &Shell,
    machine: &vessel_app_view::Machine,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    item(
        "settings-machine",
        // Paseo keeps the dot in the glyphs' box so the names line up.
        div()
            .size(px(icon_size(compact)))
            .flex()
            .items_center()
            .justify_center()
            .child(dot(choices::reach_color(machine, tokens))),
        machine.name.clone(),
        // Paseo's trigger has no open fill, only hover and press.
        None,
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

/// The groups, each a label over its rows (Paseo's `SidebarGroup`).
fn groups(shell: &Shell, compact: bool, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.settings();
    let section_rows = |group: Group, cx: &mut Context<'_, Shell>| -> Vec<Stateful<Div>> {
        group
            .sections(super::sections::SURFACE)
            .into_iter()
            .filter(|section| super::sections::drawn(*section))
            .map(|section| section_row(section, shell, compact, tokens, cx))
            .collect()
    };
    let app = section_rows(Group::App, cx);
    let machine = match choices::chosen_machine(shell) {
        Some(chosen) => {
            let mut rows = vec![picker(shell, chosen, compact, tokens, cx)];
            rows.extend(section_rows(Group::Machine, cx));
            group(Some(words.group(Group::Machine)), rows, compact, tokens)
        }
        None => group(
            None,
            vec![add_machine(compact, shell, tokens, cx)],
            compact,
            tokens,
        ),
    };
    div()
        .flex()
        .flex_col()
        .children(
            (!app.is_empty()).then(|| group(Some(words.group(Group::App)), app, compact, tokens)),
        )
        .child(machine)
}

/// One group: its label, if it has one, over its rows; on a phone the
/// rows are one island.
fn group(label: Option<&str>, rows: Vec<Stateful<Div>>, compact: bool, tokens: &Tokens) -> Div {
    // An island's corners are its first and last rows' (a clip is a
    // rectangle here, so the island cannot round them).
    let last = rows.len().saturating_sub(1);
    let island = div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .children(rows.into_iter().enumerate().map(|(index, row)| {
            row.when(compact && index == 0, |row| row.rounded_t(px(16.0)))
                .when(compact && index == last, |row| row.rounded_b(px(16.0)))
        }));
    div()
        .flex()
        .flex_col()
        .p(px(8.0))
        .gap(px(if compact { 4.0 } else { 2.0 }))
        .children(label.map(|label| {
            div()
                .px(px(if compact { 16.0 } else { 8.0 }))
                .py(px(4.0))
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_extra_muted)
                .child(label.to_owned())
        }))
        .child(island)
}

/// On a wide window, in the sidebar's place: the way back, then the
/// groups.
pub(crate) fn sidebar(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.settings();
    div()
        .debug_selector(|| "settings-nav-docked".to_owned())
        .size_full()
        .flex()
        .flex_col()
        .bg(tokens.bg_shell)
        .child(
            div().p(px(8.0)).child(
                item(
                    "settings-back",
                    glyph(
                        Glyph::ArrowLeft,
                        icon_size(false),
                        tokens.neutral.foreground_muted,
                    ),
                    words.back,
                    None,
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
        .debug_selector(|| "settings-nav-list".to_owned())
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
        note: None,
        side: Side::Below,
    }
}
