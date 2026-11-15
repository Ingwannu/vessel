//! Palette entry presentation: the shared Settings glyphs, platform key
//! badges and list-child positions. Query ranking stays in the view crate.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::SharedString;
use vessel_app_view::palette::PaletteTarget;
use vessel_app_view::prefs::Theme;
use vessel_app_view::settings::Section;
use vessel_app_view::shortcuts::{Action, Platform, default_context, modified, primary_keys};

use super::Target;
use crate::areas::settings;
use crate::shell::Shell;

/// An entry's glyph (the page's `iconOf`).
pub(super) const fn glyph_of(target: &Target) -> Glyph {
    match target {
        PaletteTarget::NewSession(_) => Glyph::Plus,
        PaletteTarget::AddMachine | PaletteTarget::Machine(_) => Glyph::Machine,
        PaletteTarget::Board => Glyph::Board,
        PaletteTarget::Messages => Glyph::Message,
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
pub(super) const fn action_of(target: &Target) -> Option<Action> {
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
        PaletteTarget::Messages
        | PaletteTarget::AddMachine
        | PaletteTarget::Machine(_)
        | PaletteTarget::Session { .. }
        | PaletteTarget::Theme(_) => None,
    }
}

/// Every chip reads the same primary effective binding as help and matching.
pub(super) fn action_keys(action: Action, shell: &Shell) -> Vec<SharedString> {
    let platform = Platform::native();
    let joiner = if platform == Platform::Mac { "" } else { "+" };
    primary_keys(
        action,
        &shell.prefs.settings.shortcuts,
        default_context(platform),
    )
    .ok()
    .flatten()
    .map(|keys| {
        vessel_app_view::shortcuts::help::display_caps(
            action,
            &keys,
            modified(action, &shell.prefs.settings.shortcuts),
            platform,
        )
        .into_iter()
        .map(|caps| caps.join(joiner).into())
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
