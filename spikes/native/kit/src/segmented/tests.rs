//! Literal reference geometry plus actual scene paints, without OS/pixel claims.
use super::*;
use crate::{Scheme, SchemeChoice};
use gpui::{
    Context, Hsla, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext, Window, point,
};
#[derive(Clone, Copy)]
struct Expected {
    size: SegmentSize,
    sizing: ControlSizing,
    outer: f32,
    inner: f32,
    horizontal: f32,
    text: f32,
}
const CASES: [Expected; 4] = [
    Expected {
        size: SegmentSize::Medium,
        sizing: ControlSizing::Pointer,
        outer: 32.0,
        inner: 28.0,
        horizontal: 12.0,
        text: 14.0,
    },
    Expected {
        size: SegmentSize::Medium,
        sizing: ControlSizing::Touch,
        outer: 44.0,
        inner: 40.0,
        horizontal: 12.0,
        text: 14.0,
    },
    Expected {
        size: SegmentSize::Small,
        sizing: ControlSizing::Pointer,
        outer: 28.0,
        inner: 24.0,
        horizontal: 8.0,
        text: 12.0,
    },
    Expected {
        size: SegmentSize::Small,
        sizing: ControlSizing::Touch,
        outer: 28.0,
        inner: 24.0,
        horizontal: 8.0,
        text: 12.0,
    },
];
fn assert_source_style(expected: Expected, tokens: &Tokens) {
    let Expected {
        size,
        sizing,
        outer,
        inner,
        horizontal,
        text,
    } = expected;
    let mut row = segmented_at(size, sizing);
    assert_eq!(row.style().min_size.height, Some(px(outer).into()));
    assert_eq!(row.style().gap.width, Some(px(4.0).into()));
    let mut item = segment_at("source", "Available", false, true, size, sizing, tokens);
    let style = item.style();
    assert_eq!(style.min_size.height, Some(px(inner).into()));
    assert_eq!(style.padding.left, Some(px(horizontal).into()));
    assert_eq!(
        style.text.font_size,
        Some(px(crate::fonts::ui(text)).into())
    );
    assert_eq!(style.text.font_weight, Some(gpui::FontWeight::NORMAL));
    assert_eq!(style.text.color, Some(tokens.neutral.foreground_muted));
    assert_eq!(style.background, None);
    let mut selected = segment_at("selected", "Selected", true, false, size, sizing, tokens);
    assert_eq!(
        selected.style().background,
        Some(tokens.neutral.surface3.into())
    );
    assert_eq!(selected.style().text.color, Some(tokens.neutral.foreground));
    assert_eq!(selected.style().opacity, Some(0.5));
}
#[test]
fn both_themes_keep_legacy_defaults_and_source_layout_values_literal() {
    for scheme in [Scheme::Light, Scheme::Dark] {
        let tokens = Tokens::of(scheme);
        for expected in CASES {
            assert_source_style(expected, &tokens);
        }
        let mut row = segmented(SegmentSize::Medium);
        assert_eq!(row.style().min_size.height, Some(px(32.0).into()));
        let mut item = segment(
            "legacy",
            "Legacy",
            false,
            true,
            SegmentSize::Medium,
            &tokens,
        );
        assert_eq!(item.style().min_size.height, Some(px(28.0).into()));
    }
}
struct Probe {
    scheme: Scheme,
    sizing: ControlSizing,
    source: bool,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let item = if self.source {
            segment_at(
                "item",
                "Available",
                false,
                true,
                SegmentSize::Medium,
                self.sizing,
                &tokens,
            )
        } else {
            segment(
                "item",
                "Available",
                false,
                true,
                SegmentSize::Medium,
                &tokens,
            )
        };
        div()
            .size_full()
            .p(px(16.0))
            .bg(tokens.neutral.surface0)
            .child(
                segmented_at(SegmentSize::Medium, self.sizing)
                    .child(item.debug_selector(|| "segment-target".to_owned())),
            )
    }
}
fn painted(cx: &mut VisualTestContext, colour: Hsla) -> bool {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        window
            .painted_quads()
            .iter()
            .any(|quad| quad.background == gpui::solid_background(colour))
    })
}
#[gpui::test]
fn actual_source_hover_and_press_change_scene_fill_while_legacy_press_stays_unchanged(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        assert!(crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).is_ok());
    });
    for scheme in [Scheme::Light, Scheme::Dark] {
        for sizing in [ControlSizing::Pointer, ControlSizing::Touch] {
            for source in [false, true] {
                let (_view, cx) = cx.add_window_view(|_, _| Probe {
                    scheme,
                    sizing,
                    source,
                });
                cx.run_until_parked();
                let bounds = cx.debug_bounds("segment-target").unwrap();
                let expected = if source && sizing == ControlSizing::Touch {
                    40.0
                } else {
                    28.0
                };
                assert_eq!(bounds.size.height, px(expected));
                let tokens = Tokens::of(scheme);
                cx.simulate_mouse_move(point(px(600.0), px(500.0)), None, Modifiers::none());
                assert!(!painted(cx, tokens.neutral.surface2));
                assert!(!painted(cx, tokens.neutral.surface3));
                cx.simulate_mouse_move(bounds.center(), None, Modifiers::none());
                assert!(painted(cx, tokens.neutral.surface2));
                cx.simulate_mouse_down(bounds.center(), MouseButton::Left, Modifiers::none());
                assert_eq!(painted(cx, tokens.neutral.surface3), source);
                assert_eq!(painted(cx, tokens.neutral.surface2), !source);
                cx.simulate_mouse_up(bounds.center(), MouseButton::Left, Modifiers::none());
                assert!(painted(cx, tokens.neutral.surface2));
            }
        }
    }
}
