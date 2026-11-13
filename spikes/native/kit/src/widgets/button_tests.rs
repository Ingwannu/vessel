//! Exact values from Paseo button.tsx and its light/default-dark theme.
use super::*;
use crate::Scheme;

#[test]
fn filled_button_palette_and_each_interaction_match_both_reference_themes() {
    for (scheme, destructive, secondary, foreground, muted) in [
        (
            Scheme::Light,
            0x00b0_4138,
            0x00e4_e4e7,
            0x001a_1a1e,
            0x0071_717a,
        ),
        (
            Scheme::Dark,
            0x00c6_4f43,
            0x0043_4645,
            0x00fa_fafa,
            0x00a1_a5a4,
        ),
    ] {
        let tokens = Tokens::of(scheme);
        assert_eq!(tokens.neutral.destructive, gpui::rgb(destructive).into());
        assert_eq!(
            tokens.neutral.destructive_foreground,
            gpui::rgb(0x00ff_ffff).into()
        );
        for (kind, fill, text, hover) in [
            (
                ButtonKind::Primary,
                gpui::rgb(crate::assets::BRAND).into(),
                gpui::rgb(0x00ff_ffff).into(),
                gpui::rgb(0x00ff_ffff).into(),
            ),
            (
                ButtonKind::Secondary,
                gpui::rgb(secondary).into(),
                gpui::rgb(foreground).into(),
                gpui::rgb(foreground).into(),
            ),
            (
                ButtonKind::Destructive,
                gpui::rgb(destructive).into(),
                gpui::rgb(0x00ff_ffff).into(),
                gpui::rgb(0x00ff_ffff).into(),
            ),
            (
                ButtonKind::Ghost,
                gpui::transparent_black(),
                gpui::rgb(muted).into(),
                gpui::rgb(foreground).into(),
            ),
        ] {
            for (name, phase, opacity, text) in [
                ("idle", ButtonPhase::Idle, 1.0, text),
                ("hovered", ButtonPhase::Hovered, 1.0, hover),
                ("pressed", ButtonPhase::Pressed, 0.85, text),
                ("disabled", ButtonPhase::Disabled, 0.5, text),
            ] {
                assert_eq!(
                    button_paint(kind, phase, &tokens),
                    ButtonPaint {
                        fill,
                        border: fill,
                        text,
                        opacity
                    },
                    "{name}"
                );
            }
        }
    }
}

#[test]
fn destructive_text_does_not_inherit_a_future_accents_foreground() {
    let mut tokens = Tokens::of(Scheme::Dark);
    tokens.neutral.accent_foreground = gpui::rgb(0x0000_0000).into();
    assert_eq!(
        button_paint(ButtonKind::Destructive, ButtonPhase::Idle, &tokens).text,
        gpui::rgb(0x00ff_ffff).into()
    );
    assert_eq!(BUTTON_HEIGHT, px(32.0));
    assert_eq!(BUTTON_HORIZONTAL, px(16.0));
    assert_eq!(BUTTON_GAP, px(8.0));
}

#[test]
fn medium_button_geometry_and_legacy_sizing_alias_are_exact() {
    assert_eq!(
        button_geometry(ControlSizing::Pointer),
        ButtonGeometry {
            height: px(32.0),
            horizontal: px(16.0)
        }
    );
    assert_eq!(
        button_geometry(ControlSizing::Touch),
        ButtonGeometry {
            height: px(44.0),
            horizontal: px(24.0)
        }
    );
    let alias: ControlSizing = crate::menu::TriggerSizing::Touch;
    assert_eq!(alias, ControlSizing::Touch);
    for scheme in [Scheme::Light, Scheme::Dark] {
        for kind in [
            ButtonKind::Primary,
            ButtonKind::Secondary,
            ButtonKind::Destructive,
            ButtonKind::Ghost,
        ] {
            for state in [ButtonState::Enabled, ButtonState::Disabled] {
                let tokens = Tokens::of(scheme);
                let mut legacy = button_with_kind("legacy", "Action", kind, state, &tokens);
                assert_eq!(legacy.style().min_size.height, Some(px(32.0).into()));
                assert_eq!(legacy.style().padding.left, Some(px(16.0).into()));
                for sizing in [ControlSizing::Pointer, ControlSizing::Touch] {
                    let mut actual =
                        button_with_kind_at("sized", "Try again", kind, state, sizing, &tokens);
                    let style = actual.style();
                    assert_eq!(
                        style.min_size.height,
                        Some(button_geometry(sizing).height.into())
                    );
                    assert_eq!(
                        style.padding.left,
                        Some(button_geometry(sizing).horizontal.into())
                    );
                    assert_eq!(
                        style.padding.right,
                        Some(button_geometry(sizing).horizontal.into())
                    );
                }
            }
        }
    }
}
