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

/// Custom data goes through the sole adapter, including absent accent/alpha.
#[gpui::test]
fn validated_contribution_replaces_actual_colors_with_ambient_independence(
    cx: &mut gpui::TestAppContext,
) {
    use vessel_app_view::appearance::contributed::Candidate;
    use vessel_proto::{
        NodeId,
        plugins::{PluginAddress, RuntimeGeneration, ThemeContribution},
    };
    let address = PluginAddress {
        node: NodeId::random(),
        plugin: "fixture".parse().unwrap(),
        generation: RuntimeGeneration::random(),
    };
    let mut contribution = ThemeContribution {
        id: "night".parse().unwrap(),
        name: "Night".parse().unwrap(),
        appearance: PluginAppearance::Dark,
        colors: vessel_proto::plugins::ThemeColors {
            background: "#10203080".parse().unwrap(),
            foreground: "#fefefe".parse().unwrap(),
            raised: "#223344".parse().unwrap(),
            control: "#334455".parse().unwrap(),
            border: "#445566".parse().unwrap(),
            accent: None,
            muted_foreground: "#708090".parse().unwrap(),
            ring: "#aabbcc60".parse().unwrap(),
        },
    };
    cx.update(|cx| {
        vessel_app_kit::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap();
        for (appearance, scheme) in [
            (PluginAppearance::Light, Scheme::Light),
            (PluginAppearance::Dark, Scheme::Dark),
        ] {
            contribution.appearance = appearance;
            for ambient in [WindowAppearance::Light, WindowAppearance::Dark] {
                apply_choice(
                    Resolution::Contributed(Candidate {
                        address: &address,
                        theme: &contribution,
                    }),
                    ambient,
                    cx,
                );
                let actual = vessel_app_kit::tokens(cx);
                assert_eq!(actual.scheme, scheme);
                assert_eq!(actual.neutral.surface0, gpui::rgba(0x1020_3080).into());
                assert_eq!(actual.neutral.foreground, gpui::rgb(0x00fe_fefe).into());
                assert_eq!(actual.neutral.surface1, gpui::rgb(0x0022_3344).into());
                assert_eq!(actual.neutral.surface2, gpui::rgb(0x0033_4455).into());
                assert_eq!(actual.neutral.border, gpui::rgb(0x0044_5566).into());
                assert_eq!(
                    actual.neutral.foreground_muted,
                    gpui::rgb(0x0070_8090).into()
                );
                assert_eq!(
                    actual.neutral.foreground_extra_muted,
                    gpui::rgba(0xaabb_cc60).into()
                );
                assert_eq!(actual.neutral.accent, actual.neutral.foreground);
            }
        }
        contribution.colors.background = "#30303080".parse().unwrap();
        contribution.colors.accent = Some("#ffcc00".parse().unwrap());
        apply_choice(
            Resolution::Contributed(Candidate {
                address: &address,
                theme: &contribution,
            }),
            WindowAppearance::Dark,
            cx,
        );
        assert_eq!(
            vessel_app_kit::tokens(cx).neutral.surface0,
            gpui::rgba(0x3030_3080).into()
        );
        assert_eq!(
            vessel_app_kit::tokens(cx).neutral.accent,
            gpui::rgb(0x00ff_cc00).into()
        );
    });
}
