//! Behavioural coverage of source expansion and light/dark resolution.
use super::*;

#[test]
fn only_dracula_ignores_the_light_dark_axis() {
    let light = Scheme::Light;
    let dark = Scheme::Dark;
    assert_eq!(
        SyntaxPalette::Dracula.colours(light),
        SyntaxPalette::Dracula.colours(dark)
    );
    for palette in [
        SyntaxPalette::Github,
        SyntaxPalette::Catppuccin,
        SyntaxPalette::TokyoNight,
        SyntaxPalette::One,
        SyntaxPalette::Nord,
        SyntaxPalette::Gruvbox,
        SyntaxPalette::Solarized,
    ] {
        assert_ne!(palette.colours(light), palette.colours(dark));
    }
}

#[test]
fn role_expansion_and_the_hand_tuned_github_exceptions_remain_distinct() {
    let one = SyntaxPalette::default().colours(Scheme::Dark);
    assert_eq!(
        one.rgb(SyntaxStyle::Definition),
        one.rgb(SyntaxStyle::Function)
    );
    assert_eq!(one.rgb(SyntaxStyle::Class), one.rgb(SyntaxStyle::Type));
    assert_eq!(
        one.rgb(SyntaxStyle::Variable),
        one.rgb(SyntaxStyle::Punctuation)
    );
    let github = SyntaxPalette::Github.colours(Scheme::Dark);
    assert_ne!(
        github.rgb(SyntaxStyle::Class),
        github.rgb(SyntaxStyle::Type)
    );
    assert_ne!(
        github.rgb(SyntaxStyle::Heading),
        github.rgb(SyntaxStyle::Function)
    );
    for style in SyntaxStyle::ALL {
        assert!(github.rgb(style) <= 0x00ff_ffff);
    }
}
