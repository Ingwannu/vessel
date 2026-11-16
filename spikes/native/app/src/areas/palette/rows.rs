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

/// An entry's glyph (the page's `iconOf`). The rows Paseo registers as
/// command-centre root actions use its explicit heavier strokes
/// (`root-registration.tsx` 65-107: New at 2.4, Settings at 2.2); every
/// other row is a shared line glyph that the kit's command row draws at the
/// same 2.2 (`Glyph::command_weight_path`), so Vessel's own rows (machines,
/// the board, themes, sessions, settings sections) match Paseo's.
pub(super) const fn glyph_of(target: &Target) -> Glyph {
    match target {
        PaletteTarget::NewSession(_) => Glyph::CommandPlus,
        PaletteTarget::AddMachine | PaletteTarget::Machine(_) => Glyph::Machine,
        PaletteTarget::Board => Glyph::Board,
        PaletteTarget::Settings(Section::General) => Glyph::CommandSettings,
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
        PaletteTarget::Theme(
            Theme::Dark
            | Theme::Zinc
            | Theme::Midnight
            | Theme::Claude
            | Theme::Ghostty
            | Theme::PureBlack,
        ) => Glyph::Moon,
    }
}

/// The shortcut that does what `target` does, if one does.
const fn action_of(target: &Target) -> Option<Action> {
    match target {
        PaletteTarget::NewSession(_) => Some(Action::NewSession),
        PaletteTarget::Board => Some(Action::GoBoard),
        PaletteTarget::Settings(section) => match section {
            Section::General => Some(Action::Settings),
            Section::Overview => Some(Action::GoMachines),
            Section::Sync => Some(Action::GoSync),
            Section::Devices => Some(Action::GoDevices),
            // The rest have no shortcut of their own.
            Section::Appearance
            | Section::Sidebar
            | Section::Chat
            | Section::Terminal
            | Section::Browser
            | Section::Editor
            | Section::Files
            | Section::Shortcuts
            | Section::Integrations
            | Section::Notifications
            | Section::Permissions
            | Section::Diagnostics
            | Section::About
            | Section::Projects
            | Section::Connections
            | Section::Agents
            | Section::Metadata
            | Section::Voice
            | Section::Workspaces
            | Section::Providers
            | Section::Usage
            | Section::Terminals
            | Section::Plugins
            | Section::Models => None,
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

/// Map semantic view text into the shared control; no typography lives here.
pub(super) fn subtitle(
    detail: &vessel_app_view::palette::Subtitle,
    query: &str,
) -> vessel_app_kit::command::CommandSubtitle {
    use vessel_app_kit::command::{CommandSubtitle, CommandText};
    use vessel_app_view::palette::{MarkedSubtitle, TextPart};
    let text = |part: TextPart| CommandText::new(part.text, part.marks);
    match detail.marked(&vessel_app_view::matcher::Query::new(query)) {
        MarkedSubtitle::Plain(part) => CommandSubtitle::Text(text(part)),
        MarkedSubtitle::StatusPath { prefix, path } => CommandSubtitle::CodePath {
            prefix: Some(text(prefix)),
            path: text(path),
        },
    }
}

#[cfg(test)]
mod stroke_tests {
    use super::*;

    /// Paseo's command-centre root actions draw New at 2.4 and Settings at
    /// 2.2; other rows keep the shared glyphs.
    #[test]
    fn root_action_rows_use_the_command_stroke_roles() {
        assert_eq!(
            glyph_of(&PaletteTarget::NewSession(None)),
            Glyph::CommandPlus
        );
        assert_eq!(
            glyph_of(&PaletteTarget::Settings(Section::General)),
            Glyph::CommandSettings
        );
        assert_eq!(
            glyph_of(&PaletteTarget::Settings(Section::Appearance)),
            settings::section_glyph(Section::Appearance)
        );
        assert_eq!(glyph_of(&PaletteTarget::Board), Glyph::Board);
    }

    /// Every other row draws at Paseo's 2.2 through the kit's command-weight
    /// asset: none is left at the provider's 1.5.
    #[test]
    fn every_other_row_draws_at_the_command_weight() {
        let mut targets = vec![PaletteTarget::AddMachine, PaletteTarget::Board];
        targets.extend(Section::ALL.map(PaletteTarget::Settings));
        targets.extend(Theme::ALL.map(PaletteTarget::Theme));
        let mut glyphs: Vec<Glyph> = targets.iter().map(glyph_of).collect();
        // Session rows (structured and terminal).
        glyphs.extend([Glyph::Message, Glyph::Terminal]);
        for glyph in glyphs {
            let path = glyph.command_weight_path();
            assert!(
                matches!(glyph, Glyph::CommandPlus | Glyph::CommandSettings)
                    || path.starts_with("kit/command-weight/"),
                "{glyph:?} {path}"
            );
        }
    }
}
