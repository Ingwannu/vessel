//! The count sits in the toolbar's own frame, as Paseo's trigger.
use super::super::{ToolbarButton, ToolbarConfig};
use crate::{ButtonState, ControlSizing, Scheme};
use gpui::prelude::*;
use gpui::{Context, Modifiers, Render, TestAppContext, Window, div, px};

struct CountProbe {
    count: usize,
    calls: usize,
}

impl Render for CountProbe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = crate::tokens(cx);
        let button = ToolbarButton::count(
            "tab-count",
            "Switch tabs",
            self.count,
            ToolbarConfig::new(ControlSizing::Touch, ButtonState::Enabled),
            &tokens,
        );
        div().size_full().flex().items_start().child(
            div()
                .flex_none()
                .debug_selector(|| "tab-count-frame".to_owned())
                .child(button.on_click(cx.listener(|this, _, _, _| {
                    this.calls = this.calls.saturating_add(1);
                }))),
        )
    }
}

#[gpui::test]
fn the_count_is_a_sixteen_badge_in_the_touch_frame_and_presses_as_one(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (view, cx) = cx.add_window_view(|_, _| CountProbe { count: 3, calls: 0 });
    cx.run_until_parked();
    let frame = cx.debug_bounds("tab-count-frame").unwrap();
    let badge = cx.debug_bounds("toolbar-count-badge").unwrap();
    assert_eq!(frame.size, gpui::size(px(32.0), px(32.0)));
    assert_eq!(badge.size.height, px(16.0));
    assert_eq!(
        badge.size.width,
        px(16.0),
        "one digit keeps the least width"
    );
    assert_eq!(badge.center(), frame.center(), "centred as a glyph is");
    cx.simulate_click(badge.center(), Modifiers::none());
    view.read_with(cx, |probe, _| assert_eq!(probe.calls, 1));
    view.update(cx, |probe, cx| {
        probe.count = 120;
        cx.notify();
    });
    cx.run_until_parked();
    let wide = cx.debug_bounds("toolbar-count-badge").unwrap();
    assert!(wide.size.width > px(16.0), "more digits widen it");
    assert_eq!(wide.size.height, px(16.0));
}
