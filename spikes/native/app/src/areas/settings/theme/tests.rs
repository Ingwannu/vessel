//! Preference resolution and the real global/component apply path.
use super::*;
use vessel_app_kit::{Scheme, SchemeChoice};

#[test]
fn system_follows_both_window_schemes_but_every_named_choice_stays_fixed() {
    for appearance in [
        WindowAppearance::Light,
        WindowAppearance::VibrantLight,
        WindowAppearance::Dark,
        WindowAppearance::VibrantDark,
    ] {
        let system: Palette = vessel_app_kit::scheme_of(appearance).into();
        assert_eq!(resolved(Theme::System, appearance), system);
        for (theme, expected) in [
            (Theme::Light, Palette::Light),
            (Theme::Dark, Palette::Dark),
            (Theme::Zinc, Palette::Zinc),
            (Theme::Midnight, Palette::Midnight),
            (Theme::Claude, Palette::Claude),
            (Theme::Ghostty, Palette::Ghostty),
            (Theme::PureBlack, Palette::PureBlack),
        ] {
            assert_eq!(resolved(theme, appearance), expected);
        }
    }
}

#[gpui::test]
fn applying_same_scheme_variants_replaces_the_actual_tokens(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        vessel_app_kit::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap();
        for theme in [
            Theme::Dark,
            Theme::Zinc,
            Theme::Midnight,
            Theme::Claude,
            Theme::Ghostty,
            Theme::PureBlack,
        ] {
            apply(theme, WindowAppearance::Light, cx);
            let expected = Tokens::palette(resolved(theme, WindowAppearance::Light));
            assert_eq!(vessel_app_kit::tokens(cx), expected, "{theme:?}");
            assert_eq!(expected.scheme, Scheme::Dark);
        }
        for appearance in [WindowAppearance::Light, WindowAppearance::Dark] {
            apply(Theme::System, appearance, cx);
            assert_eq!(
                vessel_app_kit::tokens(cx),
                Tokens::of(vessel_app_kit::scheme_of(appearance))
            );
        }
    });
}
