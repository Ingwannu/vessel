//! Real GPUI hit testing and App flag, independent of OS accessibility wiring.
use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    Context, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render, ScrollHandle,
    TestAppContext, Window, div, point, px,
};

use super::{
    CommandAnimation, CommandFrame, CommandHandlers, CommandKey, CommandMotion, CommandState,
    command_frame_animated,
};
use crate::theme::{Scheme, Tokens};

struct Probe {
    state: CommandState,
    scroll: ScrollHandle,
    last: Option<CommandMotion>,
    clicks: Rc<Cell<usize>>,
    closes: Rc<Cell<usize>>,
}

impl Probe {
    fn key(_: &mut Self, _: CommandKey, _: &mut Window, _: &mut Context<'_, Self>) {}
    fn close(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) {
        self.state.close();
        self.closes.set(self.closes.get().saturating_add(1));
        cx.notify();
    }
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let motion = self
            .state
            .sample(true, window.viewport_size().height, cx.reduce_motion());
        self.last = Some(motion);
        if motion.hidden {
            return div().into_any_element();
        }
        let clicks = Rc::clone(&self.clicks);
        let list = div()
            .id("probe-list")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .child(div().h(px(2000.0)).child("Rows"))
            .on_click(move |_, _, _| clicks.set(clicks.get().saturating_add(1)))
            .into_any_element();
        command_frame_animated(
            CommandFrame {
                compact: true,
                search: div().child("Search").into_any_element(),
                list,
            },
            CommandAnimation {
                state: &self.state,
                scroll: &self.scroll,
                motion,
            },
            CommandHandlers {
                on_key: Self::key,
                close: Self::close,
            },
            &Tokens::of(Scheme::Dark),
            window,
            cx,
        )
    }
}

#[gpui::test]
fn app_reduced_flag_keeps_real_drag_and_prevents_row_activation(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).expect("probe fonts");
        cx.set_reduce_motion(true);
    });
    let clicks = Rc::new(Cell::new(0));
    let closes = Rc::new(Cell::new(0));
    let (probe, cx) = cx.add_window_view({
        let clicks = Rc::clone(&clicks);
        let closes = Rc::clone(&closes);
        move |_, _| {
            let mut state = CommandState::default();
            state.present();
            Probe {
                state,
                scroll: ScrollHandle::new(),
                last: None,
                clicks,
                closes,
            }
        }
    });
    cx.run_until_parked();
    let first = probe.read_with(cx, |probe, _| probe.last.unwrap());
    assert!(!first.animating);
    let top = first.sheet.unwrap().top;
    let height = f32::from(top).mul_add(0.4_f32.recip(), 0.0);
    let handle = point(px(100.0), px(f32::from(top).mul_add(1.0, 12.0)));
    down(cx, handle);
    move_to(cx, point(handle.x, px(height.mul_add(0.1, 12.0))));
    cx.run_until_parked();
    let dragged = probe.read_with(cx, |probe, _| probe.last.unwrap());
    assert_eq!(
        dragged.sheet.unwrap().movement,
        crate::sheet::snap::Movement::Dragging
    );
    up(cx, point(handle.x, px(height.mul_add(0.1, 12.0))));
    cx.run_until_parked();
    let expanded = probe.read_with(cx, |probe, _| probe.last.unwrap());
    assert!(!expanded.animating);
    assert!(
        height
            .mul_add(-0.1, f32::from(expanded.sheet.unwrap().top))
            .abs()
            < 0.01
    );
    assert_eq!(clicks.get(), 0);
    assert_eq!(closes.get(), 0);
    // Begin on the clickable list, rather than the handle. A drag must
    // scroll and consume release, while a subsequent stationary click works.
    let content = point(px(100.0), px(height.mul_add(0.5, 0.0)));
    let moved = point(content.x, px(f32::from(content.y).mul_add(1.0, -80.0)));
    down(cx, content);
    move_to(cx, moved);
    cx.run_until_parked();
    up(cx, moved);
    cx.run_until_parked();
    assert_eq!(clicks.get(), 0, "a content pan activated its row");
    assert!(probe.read_with(cx, |probe, _| probe.scroll.offset().y < px(-70.0)));
    down(cx, content);
    up(cx, content);
    cx.run_until_parked();
    assert_eq!(clicks.get(), 1, "stationary row clicks must still work");
    let handle = point(px(100.0), px(height.mul_add(0.1, 12.0)));
    let below = point(handle.x, px(height.mul_add(0.9, 12.0)));
    down(cx, handle);
    move_to(cx, below);
    up(cx, below);
    cx.run_until_parked();
    assert!(probe.read_with(cx, |probe, _| probe.last.unwrap().hidden));
    assert_eq!(closes.get(), 1);
    assert_eq!(clicks.get(), 1);
}

fn down(cx: &mut gpui::VisualTestContext, position: gpui::Point<gpui::Pixels>) {
    cx.simulate_event(MouseDownEvent {
        position,
        button: MouseButton::Left,
        ..Default::default()
    });
}
fn move_to(cx: &mut gpui::VisualTestContext, position: gpui::Point<gpui::Pixels>) {
    cx.simulate_event(MouseMoveEvent {
        position,
        pressed_button: Some(MouseButton::Left),
        ..Default::default()
    });
}
fn up(cx: &mut gpui::VisualTestContext, position: gpui::Point<gpui::Pixels>) {
    cx.simulate_event(MouseUpEvent {
        position,
        button: MouseButton::Left,
        ..Default::default()
    });
}
