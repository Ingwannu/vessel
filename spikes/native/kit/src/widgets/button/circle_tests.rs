//! Literal overlay geometry/tones and real input guard through the common body.
use super::*;
use crate::assets::Glyph;
use gpui::{Context, Modifiers, Render, TestAppContext, Window, point};

#[test]
fn circles_have_fixed_size_tones_and_no_hover_or_pressed_override() {
    for (scheme, surface, border, foreground) in [
        (crate::Scheme::Light, 0x00ff_ffff, 0x00e4_e4e7, 0x001a_1a1e),
        (crate::Scheme::Dark, 0x0018_1b1a, 0x0025_2b2a, 0x00fa_fafa),
    ] {
        let tokens = Tokens::of(scheme);
        for (tone, fill, border, text) in [
            (CircleTone::Surface, surface, border, foreground),
            (CircleTone::Danger, 0x00dc_2626, 0x0099_1b1b, 0x00ff_ffff),
        ] {
            for state in [ButtonState::Enabled, ButtonState::Disabled] {
                for loading in [false, true] {
                    let config = CircleConfig::new(tone, state);
                    let config = if loading { config.loading() } else { config };
                    let design = ButtonDesign::from(config);
                    assert_eq!(
                        design.geometry(),
                        ButtonGeometry {
                            height: px(44.0),
                            horizontal: px(0.0)
                        }
                    );
                    assert_eq!(design.hover_ink(&tokens), None);
                    assert_eq!(design.pressed_opacity(&tokens), None);
                    for phase in [
                        ButtonPhase::Idle,
                        ButtonPhase::Hovered,
                        ButtonPhase::Pressed,
                        ButtonPhase::Disabled,
                    ] {
                        assert_eq!(
                            design.paint(phase, &tokens),
                            ButtonPaint {
                                fill: gpui::rgb(fill).into(),
                                border: gpui::rgb(border).into(),
                                text: gpui::rgb(text).into(),
                                opacity: if matches!(phase, ButtonPhase::Disabled) {
                                    0.5
                                } else {
                                    1.0
                                },
                            }
                        );
                    }
                    let mut control = button_with_design(
                        "circle",
                        ButtonContent::Icon(ButtonIcon::new(Glyph::Stop)),
                        config.into(),
                        &tokens,
                    );
                    assert_eq!(control.style().size.width, Some(px(44.0).into()));
                    assert_eq!(control.style().size.height, Some(px(44.0).into()));
                    assert_eq!(control.style().padding.left, Some(px(0.0).into()));
                    assert_eq!(control.style().max_size.width, None);
                    assert_eq!(control.style().background, Some(gpui::rgb(fill).into()));
                    assert_eq!(control.style().border_color, Some(gpui::rgb(border).into()));
                    assert_eq!(
                        control.style().opacity,
                        Some(if config.availability() == ButtonState::Enabled {
                            1.0
                        } else {
                            0.5
                        })
                    );
                    let mut icon = design.icon(ButtonIcon::new(Glyph::Stop), &tokens);
                    assert_eq!(icon.style().size.width, Some(px(20.0).into()));
                    assert_eq!(icon.style().size.height, Some(px(20.0).into()));
                    assert_eq!(icon.style().text.color, Some(gpui::rgb(text).into()));
                }
            }
        }
    }
}

struct Probe {
    clicks: [usize; 3],
    pending: bool,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(crate::Scheme::Dark);
        let mut root = div().flex().flex_col().items_start().size_full();
        for (index, state) in [
            ButtonState::Enabled,
            ButtonState::Disabled,
            ButtonState::Enabled,
        ]
        .into_iter()
        .enumerate()
        {
            let config = CircleConfig::new(
                if index == 2 {
                    CircleTone::Danger
                } else {
                    CircleTone::Surface
                },
                state,
            );
            let config = if index == 2 && self.pending {
                config.loading()
            } else {
                config
            };
            root = root.child(
                button_with_design(
                    ("circle", index),
                    ButtonContent::Icon(ButtonIcon::new(Glyph::Stop)),
                    config.into(),
                    &tokens,
                )
                .aria_label("Action / 동작")
                .on_click(cx.listener(move |this, _event, _window, _cx| {
                    this.clicks[index] = this.clicks[index].saturating_add(1);
                })),
            );
        }
        root
    }
}
#[gpui::test]
fn circle_disabled_and_pending_callbacks_are_refused_then_reenabled(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (view, cx) = cx.add_window_view(|_, _| Probe {
        clicks: [0; 3],
        pending: true,
    });
    cx.run_until_parked();
    for y in [22.0, 66.0, 110.0] {
        cx.simulate_click(point(px(22.0), px(y)), Modifiers::none());
        cx.run_until_parked();
    }
    view.read_with(cx, |this, _| assert_eq!(this.clicks, [1, 0, 0]));
    view.update(cx, |this, cx| {
        this.pending = false;
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_click(point(px(22.0), px(110.0)), Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |this, _| assert_eq!(this.clicks, [1, 0, 1]));
}
