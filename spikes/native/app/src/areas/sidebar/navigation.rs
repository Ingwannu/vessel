//! One current inventory for the header/footer and their settings rows.
//! Plugin groups plug into this inventory when their checked producer lands;
//! absent contributions do not erase saved choices from the pure model.
//!
//! [Decision Log]
//! Settings and navigation must resolve identical keys/order/visibility from
//! the existing preferences. Keep one projection instead of hardcoded lists
//! per caller. Current builtin actions retain their existing owners; no fake
//! History/Workspace/Usage callback substitutes for a missing producer. Hints
//! read the existing effective registry until W22 supplies checked `primary_keys`;
//! no duplicate defaults or shortcut dispatcher are created here.
use crate::shell::Shell;
use vessel_app_kit::ui::{AnyElement, Context, Div, IntoElement as _, SharedString, Window};
use vessel_app_kit::{Tokens, assets::Glyph};
use vessel_app_view::sidebar::navigation::{Builtin, Item, Section};
#[derive(Debug, Clone)]
pub(super) struct NavItem {
    pub section: Section,
    pub item: Item,
    pub label: SharedString,
    pub glyph: Glyph,
}
pub(super) fn items(shell: &Shell, section: Section) -> Vec<Item> {
    shell
        .prefs
        .settings
        .sidebar_navigation
        .entries(section)
        .resolve(section, &[])
}
pub(super) fn inventory(shell: &Shell) -> Vec<NavItem> {
    let words = shell.words.sidebar();
    [Section::Header, Section::Footer]
        .into_iter()
        .flat_map(|section| {
            items(shell, section).into_iter().filter_map(move |item| {
                let builtin = item.key.builtin()?;
                Some(NavItem {
                    section,
                    label: words.builtin(builtin).into(),
                    glyph: glyph(builtin),
                    item,
                })
            })
        })
        .collect()
}
fn glyph(item: Builtin) -> Glyph {
    match item {
        Builtin::NewWorkspace => Glyph::Plus,
        Builtin::History => Glyph::History,
        Builtin::Search => Glyph::Search,
        Builtin::Schedules => Glyph::CalendarClock,
        Builtin::Usage => Glyph::Gauge,
    }
}
fn action(item: Builtin) -> Option<vessel_app_view::shortcuts::Action> {
    use vessel_app_view::shortcuts::Action;
    match item {
        Builtin::NewWorkspace => Some(Action::NewSession),
        Builtin::Search => Some(Action::Palette),
        Builtin::History | Builtin::Schedules | Builtin::Usage => None,
    }
}
pub(super) fn hint(
    item: Option<Builtin>,
    shell: &Shell,
    compact: bool,
    tokens: &Tokens,
) -> Option<AnyElement> {
    use vessel_app_view::shortcuts::{Platform, bindings_of, chord_caps};
    if compact {
        return None;
    }
    let action = action(item?)?;
    let platform = Platform::native();
    let joiner = match platform {
        Platform::Mac => "",
        Platform::Other => "+",
    };
    // Transitional read of the existing effective registry, not copied keys.
    // Replace only this projection with W22b primary_keys when its checked API
    // lands; canonical bindings/defaults/dispatch stay with that owner.
    let binding = bindings_of(action, &shell.prefs.settings.shortcuts)
        .into_iter()
        .next()?;
    let caps = binding
        .chords()
        .map(|chord| SharedString::from(chord_caps(chord, platform).join(joiner)))
        .collect::<Vec<_>>();
    Some(vessel_app_kit::command::key_badges(&caps, tokens).into_any_element())
}

/// Search uses the same current preference inventory as Settings, one callback
/// and one shared row. Density follows the sidebar's own600 form, not PHONE.
pub(crate) fn search_navigation(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    use vessel_app_kit::sidebar_header_row::{Density, Selection, SidebarHeaderRow, Variant};
    use vessel_app_view::sidebar::navigation::Visibility;
    let entry = inventory(shell).into_iter().find(|entry| {
        entry.item.key.builtin() == Some(Builtin::Search) && entry.item.visible == Visibility::Shown
    })?;
    let compact = vessel_app_view::panels::sidebar_form(f32::from(window.viewport_size().width))
        == vessel_app_view::panels::SidebarForm::Sheet;
    let mut row = SidebarHeaderRow::new("nav-search", entry.label, Variant::Compact)
        .leading(entry.glyph)
        .selected(if shell.areas.palette.is_open() {
            Selection::Active
        } else {
            Selection::Inactive
        });
    if let Some(hint) = hint(Some(Builtin::Search), shell, compact, tokens) {
        row = row.shortcut(hint);
    }
    Some(row.element(
        if compact {
            Density::Compact
        } else {
            Density::Desktop
        },
        tokens,
        cx.listener(|shell: &mut Shell, _, window, cx| shell.open_palette(window, cx)),
    ))
}
