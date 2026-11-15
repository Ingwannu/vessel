//! Exact values from Paseo button.tsx and its light/default-dark theme.
//! Kept as one419-line literal matrix so palette, legacy compatibility, content
//! and size cases remain readable together; actual input has its own module.
use super::*;
use crate::Scheme;
use crate::widgets::glyph_body;

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
    assert_eq!(
        crate::control_sizing::TOUCH_TARGET.to_bits(),
        crate::breakpoint::TOUCH_TARGET.to_bits()
    );
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

#[test]
fn icon_only_content_has_exact_square_geometry_and_no_label_padding() {
    use crate::assets::Glyph;
    for scheme in [Scheme::Light, Scheme::Dark] {
        let tokens = Tokens::of(scheme);
        for (sizing, extent) in [(ControlSizing::Pointer, 32.0), (ControlSizing::Touch, 44.0)] {
            for state in [ButtonState::Enabled, ButtonState::Disabled] {
                let mut icon = button_with_content(
                    "icon",
                    ButtonContent::Icon(ButtonIcon::muted(Glyph::ArrowUp)),
                    ButtonKind::Ghost,
                    state,
                    sizing,
                    &tokens,
                );
                assert_eq!(icon.style().size.width, Some(px(extent).into()));
                assert_eq!(icon.style().min_size.height, Some(px(extent).into()));
                assert_eq!(icon.style().padding.left, Some(px(0.0).into()));
                assert_eq!(icon.style().padding.right, Some(px(0.0).into()));
                let opacity = match state {
                    ButtonState::Enabled => 1.0,
                    ButtonState::Disabled => 0.5,
                };
                assert_eq!(icon.style().opacity, Some(opacity));
                let mut vector = icon_element(
                    ButtonIcon::muted(Glyph::ArrowUp),
                    ButtonKind::Ghost,
                    state,
                    ButtonSize::Medium,
                    &tokens,
                );
                assert_eq!(vector.style().size.width, Some(px(14.0).into()));
                assert_eq!(vector.style().size.height, Some(px(14.0).into()));
                assert_eq!(
                    vector.style().text.color,
                    Some(tokens.neutral.foreground_muted)
                );
            }
        }
    }
}
#[test]
fn typed_content_preserves_legacy_label_geometry_and_explicit_muted_ink() {
    use crate::assets::Glyph;
    for scheme in [Scheme::Light, Scheme::Dark] {
        let tokens = Tokens::of(scheme);
        for sizing in [ControlSizing::Pointer, ControlSizing::Touch] {
            for kind in [
                ButtonKind::Primary,
                ButtonKind::Secondary,
                ButtonKind::Destructive,
                ButtonKind::Ghost,
            ] {
                for state in [ButtonState::Enabled, ButtonState::Disabled] {
                    let mut legacy =
                        button_with_kind_at("legacy", "Action", kind, state, sizing, &tokens);
                    let mut typed = button_with_content(
                        "typed",
                        ButtonContent::Label("Action".into()),
                        kind,
                        state,
                        sizing,
                        &tokens,
                    );
                    assert_eq!(legacy.style(), typed.style());
                    let mut mixed = button_with_content(
                        "mixed",
                        ButtonContent::IconAndLabel {
                            icon: ButtonIcon::new(Glyph::ArrowUp),
                            label: "Action".into(),
                        },
                        kind,
                        state,
                        sizing,
                        &tokens,
                    );
                    assert_eq!(mixed.style(), legacy.style());
                }
                for phase in [
                    ButtonPhase::Idle,
                    ButtonPhase::Hovered,
                    ButtonPhase::Pressed,
                    ButtonPhase::Disabled,
                ] {
                    assert_eq!(
                        icon_ink(ButtonIcon::muted(Glyph::ArrowUp), kind, phase, &tokens),
                        tokens.neutral.foreground_muted
                    );
                    assert_eq!(
                        icon_ink(ButtonIcon::new(Glyph::ArrowUp), kind, phase, &tokens),
                        button_paint(kind, phase, &tokens).text
                    );
                }
            }
        }
    }
}

#[test]
fn catalog_small_outline_and_loading_have_literal_source_geometry_and_paint() {
    for scheme in [Scheme::Light, Scheme::Dark] {
        let tokens = Tokens::of(scheme);
        for sizing in [ControlSizing::Pointer, ControlSizing::Touch] {
            for pending in [false, true] {
                let config = ButtonConfig::new(ButtonKind::Outline, ButtonState::Enabled, sizing)
                    .with_size(ButtonSize::Small);
                let config = if pending { config.loading() } else { config };
                let mut actual = button_control(
                    "catalog",
                    ButtonContent::Label("Add".into()),
                    config,
                    &tokens,
                );
                assert_eq!(actual.style().min_size.height, Some(px(28.0).into()));
                assert_eq!(actual.style().padding.left, Some(px(12.0).into()));
                assert_eq!(actual.style().padding.right, Some(px(12.0).into()));
                assert_eq!(actual.style().text.font_size, Some(px(12.0).into()));
                assert_eq!(
                    actual.style().text.font_weight,
                    Some(gpui::FontWeight::MEDIUM)
                );
                assert_eq!(
                    actual.style().border_color,
                    Some(tokens.neutral.border_accent)
                );
                assert_eq!(actual.style().text.color, Some(tokens.neutral.foreground));
                assert_eq!(
                    actual.style().opacity,
                    Some(if pending { 0.5 } else { 1.0 })
                );
                assert_eq!(
                    config.availability(),
                    if pending {
                        ButtonState::Disabled
                    } else {
                        ButtonState::Enabled
                    }
                );
            }
        }
    }
}

#[test]
fn every_variant_size_layout_and_content_has_literal_source_dimensions() {
    use crate::assets::Glyph;
    for scheme in [Scheme::Light, Scheme::Dark] {
        let tokens = Tokens::of(scheme);
        for (size, sizing, height, horizontal, glyph, text) in [
            (
                ButtonSize::Small,
                ControlSizing::Pointer,
                28.0,
                12.0,
                12.0,
                12.0,
            ),
            (
                ButtonSize::Small,
                ControlSizing::Touch,
                28.0,
                12.0,
                12.0,
                12.0,
            ),
            (
                ButtonSize::Medium,
                ControlSizing::Pointer,
                32.0,
                16.0,
                14.0,
                14.0,
            ),
            (
                ButtonSize::Medium,
                ControlSizing::Touch,
                44.0,
                24.0,
                14.0,
                14.0,
            ),
        ] {
            for kind in [
                ButtonKind::Primary,
                ButtonKind::Secondary,
                ButtonKind::Destructive,
                ButtonKind::Ghost,
                ButtonKind::Outline,
            ] {
                for state in [ButtonState::Enabled, ButtonState::Disabled] {
                    for content in [
                        ButtonContent::Label("Action".into()),
                        ButtonContent::Icon(ButtonIcon::new(Glyph::ArrowUp)),
                        ButtonContent::IconAndLabel {
                            icon: ButtonIcon::new(Glyph::ArrowUp),
                            label: "Action".into(),
                        },
                    ] {
                        let only_icon = matches!(content, ButtonContent::Icon(_));
                        let mut element = button_control(
                            "matrix",
                            content,
                            ButtonConfig::new(kind, state, sizing).with_size(size),
                            &tokens,
                        );
                        assert_eq!(element.style().min_size.height, Some(px(height).into()));
                        assert_eq!(
                            element.style().padding.left,
                            Some(px(if only_icon { 0.0 } else { horizontal }).into())
                        );
                        assert_eq!(
                            element.style().padding.right,
                            Some(px(if only_icon { 0.0 } else { horizontal }).into())
                        );
                        assert_eq!(
                            element.style().size.width,
                            only_icon.then(|| px(height).into())
                        );
                        assert_eq!(element.style().text.font_size, Some(px(text).into()));
                        assert_eq!(
                            element.style().text.font_weight,
                            Some(gpui::FontWeight::MEDIUM)
                        );
                        let mut actual_icon = icon_element(
                            ButtonIcon::new(Glyph::ArrowUp),
                            kind,
                            state,
                            size,
                            &tokens,
                        );
                        assert_eq!(actual_icon.style().size.width, Some(px(glyph).into()));
                        assert_eq!(actual_icon.style().size.height, Some(px(glyph).into()));
                    }
                }
            }
        }
    }
}
#[test]
fn outline_has_the_literal_reference_border_and_same_interaction_opacity() {
    for (scheme, border, foreground) in [
        (Scheme::Light, 0x00ec_ecf1, 0x001a_1a1e),
        (Scheme::Dark, 0x002f_3534, 0x00fa_fafa),
    ] {
        let tokens = Tokens::of(scheme);
        for (phase, opacity) in [
            (ButtonPhase::Idle, 1.0),
            (ButtonPhase::Hovered, 1.0),
            (ButtonPhase::Pressed, 0.85),
            (ButtonPhase::Disabled, 0.5),
        ] {
            assert_eq!(
                button_paint(ButtonKind::Outline, phase, &tokens),
                ButtonPaint {
                    fill: gpui::transparent_black(),
                    border: gpui::rgb(border).into(),
                    text: gpui::rgb(foreground).into(),
                    opacity
                }
            );
        }
        for enabled in [true, false] {
            let mut legacy =
                super::super::outline_button("legacy-outline", "Action", enabled, &tokens);
            assert_eq!(legacy.style().min_size.height, Some(px(32.0).into()));
            assert_eq!(legacy.style().padding.left, Some(px(16.0).into()));
            assert_eq!(legacy.style().padding.right, Some(px(16.0).into()));
            assert_eq!(legacy.style().border_color, Some(gpui::rgb(border).into()));
        }
    }
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
