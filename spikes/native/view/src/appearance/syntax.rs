//! The independently chosen syntax catalogue, not the app's colour scheme.
//! Reference: packages/highlight/src/themes.ts; colours stay in the kit.
use serde::{Deserialize, Serialize};

/// A syntax palette id, validated by the closed catalogue at the file edge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SyntaxTheme {
    /// GitHub's paired light and dark palettes.
    Github,
    /// Catppuccin Latte/Mocha.
    Catppuccin,
    /// Dracula's single dark palette.
    Dracula,
    /// Tokyo Night Day/Night.
    TokyoNight,
    /// The source default: One Light/Dark.
    #[default]
    One,
    /// Nord's single dark palette.
    Nord,
    /// Gruvbox light/dark.
    Gruvbox,
    /// Solarized light/dark.
    Solarized,
}

impl SyntaxTheme {
    /// Omit the default from legacy preference documents.
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// The selector order from the reference catalogue.
    pub const ALL: [Self; 8] = [
        Self::Github,
        Self::Catppuccin,
        Self::Dracula,
        Self::TokyoNight,
        Self::One,
        Self::Nord,
        Self::Gruvbox,
        Self::Solarized,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_ids_round_trip_and_unknown_ids_are_not_preferences() {
        for (theme, id) in SyntaxTheme::ALL.into_iter().zip([
            "github",
            "catppuccin",
            "dracula",
            "tokyo-night",
            "one",
            "nord",
            "gruvbox",
            "solarized",
        ]) {
            assert_eq!(serde_json::to_string(&theme).unwrap(), format!("\"{id}\""));
            assert_eq!(
                serde_json::from_str::<SyntaxTheme>(&format!("\"{id}\"")).unwrap(),
                theme
            );
        }
        assert_eq!(SyntaxTheme::default(), SyntaxTheme::One);
        assert!(serde_json::from_str::<SyntaxTheme>("\"unknown\"").is_err());
    }
}
