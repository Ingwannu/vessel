//! Exact values from Paseo button.tsx and its light/default-dark theme.
use super::*;
use crate::Scheme;

#[test]
fn filled_button_palette_and_each_interaction_match_both_reference_themes() {
    for (scheme, destructive, secondary, foreground) in [
        (Scheme::Light, 0x00b0_4138, 0x00e4_e4e7, 0x001a_1a1e),
        (Scheme::Dark, 0x00c6_4f43, 0x0043_4645, 0x00fa_fafa),
    ] {
        let tokens = Tokens::of(scheme);
        assert_eq!(tokens.neutral.destructive, gpui::rgb(destructive).into());
        assert_eq!(
            tokens.neutral.destructive_foreground,
            gpui::rgb(0x00ff_ffff).into()
        );
        for (kind, fill, text) in [
            (
                ButtonKind::Primary,
                gpui::rgb(crate::assets::BRAND).into(),
                gpui::rgb(0x00ff_ffff).into(),
            ),
            (
                ButtonKind::Secondary,
                gpui::rgb(secondary).into(),
                gpui::rgb(foreground).into(),
            ),
            (
                ButtonKind::Destructive,
                gpui::rgb(destructive).into(),
                gpui::rgb(0x00ff_ffff).into(),
            ),
        ] {
            for (phase, opacity) in [
                (ButtonPhase::Idle, 1.0),
                (ButtonPhase::Hovered, 1.0),
                (ButtonPhase::Pressed, 0.85),
                (ButtonPhase::Disabled, 0.5),
            ] {
                assert_eq!(
                    button_paint(kind, phase, &tokens),
                    ButtonPaint {
                        fill,
                        border: fill,
                        text,
                        opacity
                    }
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
