//! Palette entry presentation: the shared Settings glyphs, platform key
//! badges and list-child positions. Query ranking stays in the view crate.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::SharedString;
use vessel_app_view::palette::{PaletteGroup, PaletteTarget};
use vessel_app_view::prefs::Theme;
use vessel_app_view::settings::Section;
use vessel_app_view::shortcuts::{Action, Platform, bindings_of, chord_caps};

use super::{Entry, Target};
use crate::areas::settings;
use crate::shell::Shell;

/// The listed entries' ids, top to bottom.
pub(super) fn ids<'a>(groups: &[(PaletteGroup, Vec<&'a Entry>)]) -> Vec<&'a str> {
    groups
        .iter()
        .flat_map(|(_, entries)| entries.iter().map(|entry| entry.id.as_str()))
        .collect()
}

/// Where the entry `id`'s row is among the list's children (each group's
/// heading is one before its rows).
pub(super) fn child_at(groups: &[(PaletteGroup, Vec<&Entry>)], id: &str) -> Option<usize> {
    let mut at = 0_usize;
    for (_, entries) in groups {
        at = at.saturating_add(1);
        for entry in entries {
            if entry.id == id {
                return Some(at);
            }
            at = at.saturating_add(1);
        }
    }
    None
}

/// An entry's glyph (the page's `iconOf`).
pub(super) const fn glyph_of(target: &Target) -> Glyph {
    match target {
        PaletteTarget::NewSession(_) => Glyph::Plus,
        PaletteTarget::AddMachine | PaletteTarget::Machine(_) => Glyph::Machine,
        PaletteTarget::Board => Glyph::Board,
        PaletteTarget::Settings(section) => settings::section_glyph(*section),
        PaletteTarget::Session { structured, .. } => {
            if *structured {
                Glyph::Message
            } else {
                Glyph::Terminal
            }
        }
        PaletteTarget::Theme(Theme::System) => Glyph::Monitor,
        PaletteTarget::Theme(Theme::Light) => Glyph::Sun,
        PaletteTarget::Theme(Theme::Dark) => Glyph::Moon,
    }
}

/// The shortcut that does what `target` does, if one does.
const fn action_of(target: &Target) -> Option<Action> {
    match target {
        PaletteTarget::NewSession(_) => Some(Action::NewSession),
        PaletteTarget::Board => Some(Action::GoBoard),
        PaletteTarget::Settings(section) => match section {
            Section::General => Some(Action::Settings),
            Section::Machines => Some(Action::GoMachines),
            Section::Sync => Some(Action::GoSync),
            Section::Devices => Some(Action::GoDevices),
            Section::Appearance
            | Section::Shortcuts
            | Section::Notifications
            | Section::Agents
            | Section::Models
            | Section::Projects
            | Section::About => None,
        },
        PaletteTarget::AddMachine
        | PaletteTarget::Machine(_)
        | PaletteTarget::Session { .. }
        | PaletteTarget::Theme(_) => None,
    }
}

/// The badges of the first key bound to what `target` does, one per
/// chord (Paseo's `Shortcut`), as the person has the keys.
pub(super) fn keys_of(target: &Target, shell: &Shell) -> Vec<SharedString> {
    let Some(action) = action_of(target) else {
        return Vec::new();
    };
    let platform = Platform::native();
    let joiner = match platform {
        Platform::Mac => "",
        Platform::Other => "+",
    };
    bindings_of(action, &shell.prefs.settings.shortcuts)
        .first()
        .map(|binding| {
            binding
                .chords()
                .map(|chord| chord_caps(chord, platform).join(joiner).into())
                .collect()
        })
        .unwrap_or_default()
}
