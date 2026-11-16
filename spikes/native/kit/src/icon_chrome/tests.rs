//! Footer square hitbox and the existing Toolbar's shared source frame.
use super::*;
use crate::Scheme;
use gpui::{Context, Modifiers, Render, TestAppContext, Window, point};

#[test]
fn footer_profile_keeps_square_hit_area_separate_from_round_pressed_layer() {
    for scheme in [Scheme::Dark, Scheme::Light] {
        let tokens = Tokens::of(scheme);
        for (sizing, edge, icon) in [
            (ControlSizing::Pointer, 28.0, 16.0),
            (ControlSizing::Touch, 44.0, 20.0),
        ] {
            assert_eq!(footer_geometry(sizing), (px(edge), px(icon)));
            let mut outer = render(
                "footer",
                "Usage",
                Profile::SidebarFooter {
                    glyph: Glyph::Plus,
                    sizing,
                },
                &tokens,
            );
            assert_eq!(outer.style().size.width, Some(px(edge).into()));
            assert_eq!(outer.style().size.height, Some(px(edge).into()));
            assert_eq!(outer.style().padding.left, Some(px(4.0).into()));
            assert!(outer.style().corner_radii.top_left.is_none());
            assert!(outer.style().background.is_none());
            let mut layer = footer_highlight("layer".into(), &tokens);
            assert!(layer.style().corner_radii.top_left.is_some());
            let mut vector = footer_icon(Glyph::Plus, sizing, &tokens);
            assert_eq!(vector.style().size.width, Some(px(icon).into()));
            assert_eq!(
                vector.style().text.color,
                Some(tokens.neutral.foreground_muted)
            );
            assert_eq!(
                footer_ink(Phase::Hovered, &tokens),
                tokens.neutral.foreground
            );
            assert_eq!(
                footer_ink(Phase::Pressed, &tokens),
                tokens.neutral.foreground_muted
            );
        }
    }
}

struct Probe {
    sizing: ControlSizing,
    clicks: usize,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        div().size_full().child(
            render(
                "footer",
                "Usage",
                Profile::SidebarFooter {
                    glyph: Glyph::Plus,
                    sizing: self.sizing,
                },
                &tokens,
            )
            .debug_selector(|| "footer-chrome".to_owned())
            .on_click(cx.listener(|this, _, _, _| this.clicks = this.clicks.saturating_add(1))),
        )
    }
}
#[gpui::test]
fn actual_footer_corner_click_reaches_same_square_owner_hitbox(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).unwrap();
    });
    for (sizing, edge, far_corner) in [
        (ControlSizing::Pointer, 28.0, 27.0),
        (ControlSizing::Touch, 44.0, 43.0),
    ] {
        let (view, cx) = cx.add_window_view(move |_, _| Probe { sizing, clicks: 0 });
        cx.run_until_parked();
        let bounds = cx.debug_bounds("footer-chrome").unwrap();
        assert_eq!(bounds.size.width, px(edge));
        assert_eq!(bounds.size.height, px(edge));
        cx.simulate_click(point(px(1.0), px(1.0)), Modifiers::none());
        cx.simulate_click(point(px(far_corner), px(far_corner)), Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |probe, _| {
            assert_eq!(probe.clicks, 2);
        });
    }
}

#[test]
fn viewport_profile_is_the_small_frame_on_surface2_with_muted_ink_and_hidden_presence() {
    for scheme in [Scheme::Dark, Scheme::Light] {
        let tokens = Tokens::of(scheme);
        for (sizing, edge) in [(ControlSizing::Pointer, 20.0), (ControlSizing::Touch, 32.0)] {
            for availability in [ButtonState::Enabled, ButtonState::Disabled] {
                let config = ToolbarConfig::new(sizing, availability);
                let hidden = match availability {
                    ButtonState::Enabled => 0.0,
                    ButtonState::Disabled => 0.5,
                };
                for (presence, opacity) in
                    [(Presence::Shown, config.opacity()), (Presence::Hidden, hidden)]
                {
                    let mut outer = render(
                        "zoom-in",
                        "Zoom in",
                        Profile::Viewport {
                            config,
                            glyph: Glyph::ZoomIn,
                            presence,
                        },
                        &tokens,
                    );
                    let style = outer.style();
                    assert_eq!(style.size.width, Some(px(edge).into()));
                    assert_eq!(style.size.height, Some(px(edge).into()));
                    assert_eq!(style.padding.left, Some(px(0.0).into()));
                    assert_eq!(style.corner_radii.top_left, Some(px(6.0).into()));
                    assert_eq!(style.background, Some(tokens.neutral.surface2.into()));
                    assert_eq!(style.opacity, Some(opacity));
                }
            }
            let config = ToolbarConfig::new(sizing, ButtonState::Enabled);
            assert_eq!(
                viewport_fill(config, Phase::Rest, &tokens),
                tokens.neutral.surface2
            );
            for phase in [Phase::Hovered, Phase::Pressed] {
                assert_eq!(
                    viewport_fill(config, phase, &tokens),
                    tokens.neutral.interaction_highlight
                );
            }
        }
    }
}

#[test]
fn only_an_enabled_shown_viewport_action_answers_the_pointer() {
    for sizing in [ControlSizing::Pointer, ControlSizing::Touch] {
        for availability in [ButtonState::Enabled, ButtonState::Disabled] {
            for presence in [Presence::Shown, Presence::Hidden] {
                let config = ToolbarConfig::new(sizing, availability);
                assert_eq!(
                    viewport_responds(config, presence),
                    availability == ButtonState::Enabled && presence == Presence::Shown
                );
            }
        }
    }
}

struct ViewportHover {
    availability: ButtonState,
}
impl Render for ViewportHover {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        div().size_full().p(px(16.0)).child(
            render(
                "reset",
                "Reset zoom",
                Profile::Viewport {
                    config: ToolbarConfig::new(ControlSizing::Pointer, self.availability),
                    glyph: Glyph::Scan,
                    presence: Presence::Shown,
                },
                &tokens,
            )
            .debug_selector(|| "viewport-hover".to_owned()),
        )
    }
}
fn backgrounds(cx: &mut gpui::VisualTestContext) -> Vec<gpui::Background> {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        window
            .painted_quads()
            .iter()
            .map(|quad| quad.background)
            .collect()
    })
}
#[gpui::test]
fn a_disabled_viewport_action_paints_the_same_under_hover_and_press(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).unwrap();
    });
    let highlight = gpui::solid_background(Tokens::of(Scheme::Dark).neutral.interaction_highlight);
    for availability in [ButtonState::Enabled, ButtonState::Disabled] {
        let (_view, cx) = cx.add_window_view(move |_, _| ViewportHover { availability });
        cx.run_until_parked();
        let bounds = cx.debug_bounds("viewport-hover").unwrap();
        cx.simulate_mouse_move(point(px(600.0), px(500.0)), None, Modifiers::none());
        let rest = backgrounds(cx);
        cx.simulate_mouse_move(bounds.center(), None, Modifiers::none());
        let hovered = backgrounds(cx);
        cx.simulate_mouse_down(bounds.center(), gpui::MouseButton::Left, Modifiers::none());
        let pressed = backgrounds(cx);
        cx.simulate_mouse_up(bounds.center(), gpui::MouseButton::Left, Modifiers::none());
        match availability {
            ButtonState::Enabled => {
                assert!(!rest.contains(&highlight));
                assert!(hovered.contains(&highlight));
                assert!(pressed.contains(&highlight));
            }
            ButtonState::Disabled => {
                assert_eq!(hovered, rest);
                assert_eq!(pressed, rest);
            }
        }
    }
}
