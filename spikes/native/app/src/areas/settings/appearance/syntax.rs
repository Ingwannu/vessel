//! One closed preference-to-kit syntax adapter and its translated names.
use vessel_app_kit::syntax::SyntaxPalette;
use vessel_app_view::{appearance::SyntaxTheme, localization::Translator};

pub(in crate::areas::settings) const fn palette(theme: SyntaxTheme) -> SyntaxPalette {
    match theme {
        SyntaxTheme::Github => SyntaxPalette::Github,
        SyntaxTheme::Catppuccin => SyntaxPalette::Catppuccin,
        SyntaxTheme::Dracula => SyntaxPalette::Dracula,
        SyntaxTheme::TokyoNight => SyntaxPalette::TokyoNight,
        SyntaxTheme::One => SyntaxPalette::One,
        SyntaxTheme::Nord => SyntaxPalette::Nord,
        SyntaxTheme::Gruvbox => SyntaxPalette::Gruvbox,
        SyntaxTheme::Solarized => SyntaxPalette::Solarized,
    }
}
pub(in crate::areas::settings) fn label(theme: SyntaxTheme, words: &Translator) -> &str {
    let words = words.appearance();
    match theme {
        SyntaxTheme::Github => words.github(),
        SyntaxTheme::Catppuccin => words.catppuccin(),
        SyntaxTheme::Dracula => words.dracula(),
        SyntaxTheme::TokyoNight => words.tokyo_night(),
        SyntaxTheme::One => words.one(),
        SyntaxTheme::Nord => words.nord(),
        SyntaxTheme::Gruvbox => words.gruvbox(),
        SyntaxTheme::Solarized => words.solarized(),
    }
}
