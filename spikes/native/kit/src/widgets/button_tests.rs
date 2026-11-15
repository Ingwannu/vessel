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

/// Paseo's Remove (host-page.tsx `RemoveHostSection`): an outline button
/// with `leftIcon` Trash2 at `theme.iconSize.sm` and the words after it,
/// the button's `gap: theme.spacing[2]` between them, both centred.
#[gpui::test]
fn a_leading_glyph_sits_before_the_words_at_paseos_size_and_gap(cx: &mut gpui::TestAppContext) {
    use gpui::{Bounds, Context, Pixels, Render, Window, point, size};
    use std::{cell::RefCell, rc::Rc};
    type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
    struct Probe(Seen);
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
            let tokens = Tokens::of(Scheme::Dark);
            let seen = Rc::clone(&self.0);
            // At the top, so the row does not stretch the pill.
            div().flex().items_start().child(
                glyph_body(
                    crate::assets::Glyph::Trash2,
                    "Remove",
                    tokens.neutral.destructive,
                    true,
                    &tokens,
                )
                .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds),
            )
        }
    }
    let seen = Seen::default();
    let (_, cx) = cx.add_window_view({
        let seen = Rc::clone(&seen);
        move |_, _| Probe(seen)
    });
    cx.run_until_parked();
    let seen = seen.borrow();
    let [glyph, words] = seen.as_slice() else {
        panic!("the glyph and the words, in that order: {seen:?}");
    };
    // The hairline and the pill's 16 of padding before the glyph, 14
    // square, centred in the 32 pill; the words 8 after it (17 + 14 + 8),
    // their 20 line centred on the same middle.
    assert_eq!(
        *glyph,
        Bounds::new(point(px(17.0), px(9.0)), size(px(14.0), px(14.0)))
    );
    assert_eq!(words.origin.x, px(39.0));
    assert_eq!(words.origin.y, px(6.0));
    assert_eq!(words.size.height, px(20.0));
    assert_eq!(glyph.center().y, words.center().y);
}
