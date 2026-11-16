//! Actual native hit/focus/key input through the wrapper and shared chrome.
use super::*;
use crate::Scheme;
use gpui::{Bounds, Context, Modifiers, MouseButton, Pixels, Render, TestAppContext, point};
use std::{cell::Cell, rc::Rc, time::Duration};

struct Probe {
    scheme: Scheme,
    sizing: ControlSizing,
    focus: FocusHandle,
    tips: Tooltips,
    place: Place,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    allowed: Rc<Cell<bool>>,
    calls: Rc<Cell<usize>>,
    keyboard: Rc<Cell<usize>>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let allowed = Rc::clone(&self.allowed);
        let calls = Rc::clone(&self.calls);
        let keyboard = Rc::clone(&self.keyboard);
        let tokens = Tokens::of(self.scheme);
        let button =
            FooterIconButton::new("footer-test", "Usage", Glyph::Gauge, self.sizing, &tokens)
                .track_focus(&self.focus)
                .on_click(move |event, _, _| {
                    if allowed.get() {
                        calls.set(calls.get().saturating_add(1));
                        if matches!(event, ClickEvent::Keyboard(_)) {
                            keyboard.set(keyboard.get().saturating_add(1));
                        }
                    }
                })
                .tooltip(&self.tips, &self.place);
        let bounds = Rc::clone(&self.bounds);
        div()
            .size_full()
            .child(
                div()
                    .mt(gpui::px(160.0))
                    .on_children_prepainted(move |children, _, _| {
                        bounds.set(children.first().copied());
                    })
                    .child(button),
            )
            .children(self.tips.render(&tokens))
    }
}
fn setup(
    cx: &mut TestAppContext,
    scheme: Scheme,
    sizing: ControlSizing,
) -> (gpui::Entity<Probe>, &mut gpui::VisualTestContext) {
    let (view, cx) = cx.add_window_view(move |_, cx| Probe {
        scheme,
        sizing,
        focus: cx.focus_handle().tab_stop(true),
        tips: Tooltips::new(),
        place: Rc::default(),
        bounds: Rc::default(),
        allowed: Rc::new(Cell::new(true)),
        calls: Rc::default(),
        keyboard: Rc::default(),
    });
    cx.run_until_parked();
    (view, cx)
}
#[gpui::test]
fn footer_wrapper_preserves_literal_square_corner_hits_and_owner_refusal(cx: &mut TestAppContext) {
    for scheme in [Scheme::Light, Scheme::Dark] {
        for sizing in [ControlSizing::Pointer, ControlSizing::Touch] {
            let (view, cx) = setup(cx, scheme, sizing);
            let bounds = view.read_with(cx, |view, _| view.bounds.get().unwrap());
            let edge = match sizing {
                ControlSizing::Pointer => 28.0,
                ControlSizing::Touch => 44.0,
            };
            assert_eq!(bounds.size.width, gpui::px(edge));
            assert_eq!(bounds.size.height, gpui::px(edge));
            let corner = point(
                gpui::px(bounds.left().as_f32().mul_add(1.0, 1.0)),
                gpui::px(bounds.top().as_f32().mul_add(1.0, 1.0)),
            );
            cx.simulate_click(corner, Modifiers::none());
            view.read_with(cx, |view, _| {
                assert_eq!(view.calls.get(), 1);
                view.allowed.set(false);
            });
            cx.simulate_click(bounds.center(), Modifiers::none());
            view.read_with(cx, |view, _| {
                assert_eq!(view.calls.get(), 1);
            });
        }
    }
}
#[gpui::test]
fn footer_owner_focus_uses_native_enter_and_space_clicks_without_another_listener(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx, Scheme::Dark, ControlSizing::Pointer);
    let focus = view.read_with(cx, |view, _| view.focus.clone());
    cx.update(|window, cx| {
        focus.focus(window, cx);
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("enter space");
    view.read_with(cx, |view, _| {
        assert_eq!(view.calls.get(), 2);
        assert_eq!(view.keyboard.get(), 2);
        view.allowed.set(false);
    });
    cx.simulate_keystrokes("enter");
    view.read_with(cx, |view, _| {
        assert_eq!(view.calls.get(), 2);
    });
}
#[gpui::test]
fn footer_tooltip_uses_existing_top_delay_and_compact_suppression(cx: &mut TestAppContext) {
    for sizing in [ControlSizing::Pointer, ControlSizing::Touch] {
        let (view, cx) = setup(cx, Scheme::Light, sizing);
        let at = view.read_with(cx, |view, _| view.bounds.get().unwrap().center());
        cx.simulate_mouse_move(at, None, Modifiers::none());
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(299));
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert!(view.tips.render(&Tokens::of(view.scheme)).is_none());
        });
        cx.executor().advance_clock(Duration::from_millis(1));
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert_eq!(
                view.tips.render(&Tokens::of(view.scheme)).is_some(),
                sizing == ControlSizing::Pointer
            );
        });
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
        view.read_with(cx, |view, _| {
            assert!(view.tips.render(&Tokens::of(view.scheme)).is_none());
        });
        cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    }
}
