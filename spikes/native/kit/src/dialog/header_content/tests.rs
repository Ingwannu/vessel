//! Literal title-row rails and real caller-owned actions on both presentations.

use super::*;
use crate::{DialogSize, DialogState, Scheme, SchemeChoice, TextField, Tokens, dialog};
use gpui::prelude::*;
use gpui::{
    Bounds, Context, Entity, InputEvent as _, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent,
    Pixels, Render, TestAppContext, VisualTestContext, Window, div, px, size,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy)]
enum Mode {
    Legacy,
    Actions,
}

struct Probe {
    state: DialogState,
    outside: TextField,
    open: bool,
    narrow: bool,
    mode: Mode,
    epoch: u8,
    pressed: Vec<usize>,
    controls: Rc<RefCell<Vec<Bounds<Pixels>>>>,
    renders: Cell<usize>,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.renders.set(self.renders.get().saturating_add(1));
        let tokens = Tokens::of(Scheme::Dark);
        let content = DialogContent {
            title: match self.mode {
                Mode::Legacy => "Usage",
                Mode::Actions => "Usage with a long title that must yield to the caller controls",
            }
            .into(),
            body: div().h(px(40.0)).into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Wide,
        };
        let expected = self.epoch;
        let measured = Rc::clone(&self.controls);
        let controls = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.0))
            .on_children_prepainted(move |bounds, _, _| *measured.borrow_mut() = bounds)
            .children(
                [80.0, 32.0, 32.0]
                    .into_iter()
                    .enumerate()
                    .map(|(at, width)| {
                        div()
                            .id(("title-action", at))
                            .w(px(width))
                            .h(px(32.0))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |probe: &mut Self, _, _, cx| {
                                    // The shared slot deliberately does not authorize host/ticket effects.
                                    if probe.epoch == expected {
                                        probe.pressed.push(at);
                                    }
                                    cx.stop_propagation();
                                }),
                            )
                    }),
            );
        let input: DialogInput = match self.mode {
            Mode::Legacy => content.into(),
            Mode::Actions => content.with_header_actions(controls),
        };
        let modal = self.open.then(|| {
            dialog(
                &self.state,
                input,
                &tokens,
                window,
                self.narrow,
                move |probe, window, cx| {
                    if probe.epoch == expected && probe.open {
                        probe.open = false;
                        probe.state.dismissed(window, cx);
                    }
                },
                cx,
            )
        });
        div()
            .size_full()
            .child(self.outside.element(&tokens, false, cx))
            .children(modal)
    }
}

fn mount(
    cx: &mut TestAppContext,
    mode: Mode,
    narrow: bool,
) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap();
        cx.set_reduce_motion(true);
    });
    let (probe, cx) = cx.add_window_view(move |window, cx| {
        window.activate_window();
        let outside = TextField::new("", |_, _, _, _| {}, window, cx);
        outside.focus(window, cx);
        let state = DialogState::new(cx);
        state.focus(window, cx);
        Probe {
            state,
            outside,
            open: true,
            narrow,
            mode,
            epoch: 1,
            pressed: Vec::new(),
            controls: Rc::default(),
            renders: Cell::new(0),
        }
    });
    cx.simulate_resize(size(px(if narrow { 390.0 } else { 1280.0 }), px(900.0)));
    cx.run_until_parked();
    (probe, cx)
}

fn distance(from: Pixels, to: Pixels) -> Pixels {
    px(f32::from(from).mul_add(-1.0, f32::from(to)))
}

#[gpui::test]
fn optional_title_controls_use_paseos_24_16_8_rails_and_keep_legacy_literals(
    cx: &mut TestAppContext,
) {
    for narrow in [false, true] {
        for mode in [Mode::Legacy, Mode::Actions] {
            let (_probe, cx) = mount(cx, mode, narrow);
            let header = cx.debug_bounds("dialog-header").unwrap();
            let title = cx.debug_bounds("dialog-title").unwrap();
            let close = cx.debug_bounds("dialog-close").unwrap();
            assert_eq!(distance(header.left(), title.left()), px(24.0));
            assert_eq!(distance(close.right(), header.right()), px(24.0));
            assert_eq!(close.size.height, px(if narrow { 44.0 } else { 28.0 }));
            match mode {
                Mode::Legacy => {
                    assert!(cx.debug_bounds("dialog-header-actions").is_none());
                    assert_eq!(header.size.height, px(if narrow { 77.0 } else { 61.0 }));
                }
                Mode::Actions => {
                    let actions = cx.debug_bounds("dialog-header-actions").unwrap();
                    assert_eq!(distance(title.right(), actions.left()), px(8.0));
                    assert_eq!(distance(actions.right(), close.left()), px(8.0));
                    assert_eq!(
                        distance(header.top(), actions.top().min(close.top())),
                        px(16.0)
                    );
                    assert_eq!(
                        distance(actions.bottom().max(close.bottom()), header.bottom()),
                        px(17.0)
                    );
                    assert_eq!(actions.size.height, px(32.0));
                    assert!(title.size.width > px(0.0));
                    assert_eq!(header.size.height, px(if narrow { 77.0 } else { 65.0 }));
                }
            }
        }
    }
}

#[gpui::test]
fn header_actions_keep_the_dialog_open_and_close_reuses_actual_opener_focus(
    cx: &mut TestAppContext,
) {
    for narrow in [false, true] {
        let (probe, cx) = mount(cx, Mode::Actions, narrow);
        let at = probe.read_with(cx, |probe, _| {
            probe.controls.borrow().get(1).unwrap().center()
        });
        cx.simulate_click(at, Modifiers::none());
        probe.read_with(cx, |probe, _| {
            assert_eq!(probe.pressed, [1]);
            assert!(probe.open);
        });
        let at = cx.debug_bounds("dialog-close").unwrap().center();
        cx.simulate_click(at, Modifiers::none());
        probe.update_in(cx, |probe, window, cx| {
            assert!(!probe.open);
            assert!(probe.outside.focus_handle(cx).is_focused(window));
        });
    }
}

fn press(window: &mut Window, cx: &mut gpui::App, at: gpui::Point<Pixels>) {
    let _down = window.dispatch_event(
        MouseDownEvent {
            position: at,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 1,
            first_mouse: false,
        }
        .to_platform_input(),
        cx,
    );
    let _up = window.dispatch_event(
        MouseUpEvent {
            position: at,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 1,
        }
        .to_platform_input(),
        cx,
    );
}

#[gpui::test]
fn retained_header_action_authority_stays_with_its_captured_caller_context(
    cx: &mut TestAppContext,
) {
    for narrow in [false, true] {
        let (probe, cx) = mount(cx, Mode::Actions, narrow);
        let at = probe.read_with(cx, |probe, _| {
            probe.controls.borrow().get(1).unwrap().center()
        });
        cx.update(|window, cx| {
            let before = probe.read(cx).renders.get();
            probe.update(cx, |probe, cx| {
                probe.epoch = 2;
                cx.notify();
            });
            press(window, cx, at);
            assert_eq!(
                probe.read(cx).renders.get(),
                before,
                "retained frame A dispatched"
            );
            assert!(
                probe.read(cx).pressed.is_empty(),
                "caller refuses old context"
            );
            assert!(probe.read(cx).open);
        });
        cx.run_until_parked();
        cx.simulate_click(at, Modifiers::none());
        probe.read_with(cx, |probe, _| assert_eq!(probe.pressed, [1]));
    }
}
