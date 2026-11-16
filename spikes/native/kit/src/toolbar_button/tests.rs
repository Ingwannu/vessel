//! Pinned20/32 chrome and real disabled dispatch, not ordinary Button geometry.
use super::*;
use crate::icon_chrome::{Phase, toolbar_fill as fill};
use crate::{ControlSizing, Scheme, Tokens, assets::Glyph};
use gpui::{Context, Modifiers, Render, TestAppContext, div, point, px};

#[test]
fn literal_small_chrome_preserves_source_geometry_ink_and_highlight_states() {
    for scheme in [Scheme::Light, Scheme::Dark] {
        let tokens = Tokens::of(scheme);
        for (sizing, frame, adaptive) in [
            (ControlSizing::Pointer, 20.0, 14.0),
            (ControlSizing::Touch, 32.0, 18.0),
        ] {
            let config = ToolbarConfig::new(sizing, ButtonState::Enabled);
            assert_eq!(config.frame(), px(frame));
            let icon = ToolbarIcon::new(Glyph::Plus, ToolbarInk::ExtraMuted);
            assert_eq!(icon.size(sizing), px(14.0));
            assert_eq!(icon.adaptive().size(sizing), px(adaptive));
            let mut drawing = icon.render(sizing, &tokens);
            assert_eq!(
                drawing.style().text.color,
                Some(tokens.neutral.foreground_extra_muted)
            );
            assert_eq!(
                fill(config, Phase::Rest, &tokens),
                gpui::transparent_black()
            );
            assert_eq!(
                fill(config, Phase::Hovered, &tokens),
                tokens.neutral.interaction_highlight
            );
            assert_eq!(
                fill(config, Phase::Pressed, &tokens),
                tokens.neutral.interaction_highlight
            );
            let mut button = ToolbarButton::new("small", "Action", icon, config, &tokens);
            assert_eq!(button.element.style().size.width, Some(px(frame).into()));
            assert_eq!(button.element.style().size.height, Some(px(frame).into()));
            assert_eq!(button.element.style().padding.left, Some(px(0.0).into()));
            assert_eq!(
                button.element.style().corner_radii.top_left,
                Some(px(6.0).into())
            );
        }
    }
}

#[test]
fn selection_and_open_keep_the_same_highlight_and_disabled_stays_at_half_opacity() {
    let tokens = Tokens::of(Scheme::Dark);
    for selection in [Selection::Unselected, Selection::Selected] {
        for disclosure in [Disclosure::Closed, Disclosure::Open] {
            for availability in [ButtonState::Enabled, ButtonState::Disabled] {
                let config = ToolbarConfig::new(ControlSizing::Pointer, availability)
                    .selected(selection)
                    .disclosure(disclosure);
                let expected = if selection == Selection::Selected || disclosure == Disclosure::Open
                {
                    tokens.neutral.interaction_highlight
                } else {
                    gpui::transparent_black()
                };
                assert_eq!(fill(config, Phase::Rest, &tokens), expected);
                assert_eq!(
                    config.opacity().to_bits(),
                    if availability == ButtonState::Disabled {
                        0.5_f32.to_bits()
                    } else {
                        1.0_f32.to_bits()
                    }
                );
                for phase in [Phase::Hovered, Phase::Pressed] {
                    assert_eq!(
                        fill(config, phase, &tokens),
                        tokens.neutral.interaction_highlight
                    );
                }
            }
        }
    }
}

struct Probe {
    sizing: ControlSizing,
    availability: ButtonState,
    ready: usize,
    guarded: usize,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let icon = ToolbarIcon::new(Glyph::Maximize, ToolbarInk::ExtraMuted);
        let mut ready = ToolbarButton::new(
            "ready",
            "Focus pane",
            icon,
            ToolbarConfig::new(self.sizing, ButtonState::Enabled),
            &tokens,
        );
        ready.element = ready.element.debug_selector(|| "toolbar-ready".to_owned());
        let mut guarded = ToolbarButton::new(
            "guarded",
            "Restore pane",
            icon,
            ToolbarConfig::new(self.sizing, self.availability),
            &tokens,
        );
        guarded.element = guarded
            .element
            .debug_selector(|| "toolbar-guarded".to_owned());
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_start()
            .child(
                ready.on_click(
                    cx.listener(|this, _, _, _| this.ready = this.ready.saturating_add(1)),
                ),
            )
            .child(guarded.on_click(
                cx.listener(|this, _, _, _| this.guarded = this.guarded.saturating_add(1)),
            ))
    }
}

#[gpui::test]
fn actual_frame_hitboxes_refuse_disabled_callbacks_and_reenable_on_owner_update(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).unwrap();
    });
    for (sizing, edge, center, next) in [
        (ControlSizing::Pointer, 20.0_f32, 10.0, 30.0),
        (ControlSizing::Touch, 32.0_f32, 16.0, 48.0),
    ] {
        let (view, cx) = cx.add_window_view(move |_, _| Probe {
            sizing,
            availability: ButtonState::Disabled,
            ready: 0,
            guarded: 0,
        });
        cx.run_until_parked();
        let ready = cx.debug_bounds("toolbar-ready").unwrap();
        let guarded = cx.debug_bounds("toolbar-guarded").unwrap();
        assert_eq!(ready.size.width, px(edge));
        assert_eq!(ready.size.height, px(edge));
        assert_eq!(guarded.size, ready.size);
        let x = px(center);
        cx.simulate_click(point(x, px(center)), Modifiers::none());
        cx.simulate_click(point(x, px(next)), Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |probe, _| {
            assert_eq!((probe.ready, probe.guarded), (1, 0));
        });
        view.update(cx, |probe, cx| {
            probe.availability = ButtonState::Enabled;
            cx.notify();
        });
        cx.run_until_parked();
        cx.simulate_click(point(x, px(next)), Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |probe, _| {
            assert_eq!((probe.ready, probe.guarded), (1, 1));
        });
    }
}
