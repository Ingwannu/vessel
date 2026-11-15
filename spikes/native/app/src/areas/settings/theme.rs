//! The sole preference-to-kit palette adapter. System resolves from the
//! window; named palettes never follow it. No preference I/O or extra list.
//!
//! [Decision Log]
//! - Constraint: kit and view cannot depend on each other (ADR 0070).
//! - Choice: one exhaustive mapping, shared by application and tile preview.
//! - Why: Scheme-only equality would silently ignore Dark-to-Zinc changes.
//! - Effect: controls and the component layer refresh from the same tokens.

use gpui::{App, WindowAppearance};
use vessel_app_kit::{Palette, Tokens};
use vessel_app_view::prefs::Theme;

/// System has no fixed palette; named choices are mapped exactly once.
pub(super) const fn fixed(theme: Theme) -> Option<Palette> {
    match theme {
        Theme::System => None,
        Theme::Light => Some(Palette::Light),
        Theme::Dark => Some(Palette::Dark),
        Theme::Zinc => Some(Palette::Zinc),
        Theme::Midnight => Some(Palette::Midnight),
        Theme::Claude => Some(Palette::Claude),
        Theme::Ghostty => Some(Palette::Ghostty),
        Theme::PureBlack => Some(Palette::PureBlack),
    }
}

fn resolved(theme: Theme, appearance: WindowAppearance) -> Palette {
    fixed(theme).unwrap_or_else(|| vessel_app_kit::scheme_of(appearance).into())
}

/// Avoid a redundant refresh, without collapsing every dark palette to Dark.
pub(super) fn apply(theme: Theme, appearance: WindowAppearance, cx: &mut App) {
    let palette = resolved(theme, appearance);
    if vessel_app_kit::tokens(cx) != Tokens::palette(palette) {
        vessel_app_kit::set_palette(cx, palette);
    }
}

#[cfg(test)]
mod tests;
