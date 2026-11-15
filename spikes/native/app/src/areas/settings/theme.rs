//! The sole preference-to-kit palette adapter. System resolves from the
//! window; named palettes never follow it. No preference I/O or extra list.
//!
//! [Decision Log]
//! - Constraint: kit and view cannot depend on each other (ADR 0070).
//! - Choice: one exhaustive mapping, shared by application and tile preview.
//! - Why: Scheme-only equality would silently ignore Dark-to-Zinc changes.
//! - Effect: controls and the component layer refresh from the same tokens.

use gpui::{App, WindowAppearance};
use vessel_app_kit::{Palette, Scheme, Tokens, theme::palette::ContributedColors};
use vessel_app_view::appearance::contributed::Resolution;
use vessel_app_view::prefs::Theme;
use vessel_proto::plugins::Appearance as PluginAppearance;

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

/// One semantic projection for built-ins and validated plugin palettes.
/// Packed colours were parsed by the plugin boundary; preserve their alpha.
fn tokens_of(choice: Resolution<'_>, ambient: WindowAppearance) -> Tokens {
    match choice {
        Resolution::Builtin(theme) => Tokens::palette(resolved(theme, ambient)),
        Resolution::Contributed(candidate) => {
            let colors = &candidate.theme.colors;
            let scheme = match candidate.theme.appearance {
                PluginAppearance::Light => Scheme::Light,
                PluginAppearance::Dark => Scheme::Dark,
            };
            Tokens::contributed(
                scheme,
                ContributedColors {
                    background: gpui::rgba(colors.background.rgba()).into(),
                    foreground: gpui::rgba(colors.foreground.rgba()).into(),
                    raised: gpui::rgba(colors.raised.rgba()).into(),
                    control: gpui::rgba(colors.control.rgba()).into(),
                    border: gpui::rgba(colors.border.rgba()).into(),
                    accent: gpui::rgba(colors.effective_accent().rgba()).into(),
                    muted_foreground: gpui::rgba(colors.muted_foreground.rgba()).into(),
                    ring: gpui::rgba(colors.ring.rgba()).into(),
                },
            )
        }
    }
}
/// Avoid redundant refresh without collapsing same-scheme plugin colours.
pub(super) fn apply_choice(choice: Resolution<'_>, ambient: WindowAppearance, cx: &mut App) {
    let desired = tokens_of(choice, ambient);
    if vessel_app_kit::tokens(cx) != desired {
        vessel_app_kit::set_tokens(cx, &desired);
    }
}
/// Existing built-in entry point retains its meaning and override guard.
pub(super) fn apply(theme: Theme, ambient: WindowAppearance, cx: &mut App) {
    apply_choice(Resolution::Builtin(theme), ambient, cx);
}

#[cfg(test)]
mod tests;
