//! Actual window focus and key routing, distinct from selection helper tests.
use super::*;
use crate::{Scheme, Tokens};
use gpui::prelude::*;
use gpui::{Entity, Modifiers, Render, TestAppContext, div, point, px};

struct Probe {
    root: FocusHandle,
    menu: MenuState,
    items: Vec<MenuItem>,
    chosen: usize,
    f5: usize,
}
impl Probe {
    fn new(dismissal: Dismissal, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let root = cx.focus_handle();
        root.focus(window, cx);
        Self {
            root,
            menu: MenuState::new(cx),
            items: vec![MenuItem::new("Run").with_dismissal(dismissal)],
            chosen: 0,
            f5: 0,
        }
    }
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let mut root = div()
            .size_full()
            .track_focus(&self.root)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, _| {
                if event.keystroke.key == "f5" {
                    this.f5 = this.f5.saturating_add(1);
                }
            }))
            .child(
                div()
                    .id("open")
                    .w(px(60.0))
                    .h(px(30.0))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.menu.open(point(px(100.0), px(40.0)), window, cx);
                        cx.notify();
                    })),
            );
        let handlers = MenuHandlers {
            state: |this: &mut Self| &mut this.menu,
            chose: Rc::new(|this, _, _, _| this.chosen = this.chosen.saturating_add(1)),
        };
        if let Some(menu) = menu(
            "test-menu",
            &self.menu,
            &self.items,
            &handlers,
            &tokens,
            false,
            cx,
        ) {
            root = root.child(menu);
        }
        root
    }
}

fn open(cx: &mut gpui::VisualTestContext) {
    cx.simulate_click(point(px(10.0), px(10.0)), Modifiers::none());
    cx.run_until_parked();
}
fn witness(view: &Entity<Probe>, cx: &mut gpui::VisualTestContext) -> usize {
    cx.simulate_keystrokes("f5");
    cx.run_until_parked();
    view.read_with(cx, |this, _| this.f5)
}

#[gpui::test]
fn closing_by_real_pointer_enter_escape_and_outside_restores_root_keys(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (view, cx) = cx.add_window_view(|window, cx| Probe::new(Dismissal::Close, window, cx));
    cx.run_until_parked();
    assert_eq!(witness(&view, cx), 1);
    open(cx);
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    view.read_with(cx, |this, _| assert_eq!(this.chosen, 1));
    assert_eq!(witness(&view, cx), 2);
    open(cx);
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(witness(&view, cx), 3);
    open(cx);
    // Actual anchored row: x100..280, y44..80 (top padding4, row28).
    cx.simulate_click(point(px(120.0), px(62.0)), Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |this, _| assert_eq!(this.chosen, 2));
    assert_eq!(witness(&view, cx), 4);
    open(cx);
    cx.simulate_click(point(px(350.0), px(350.0)), Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |this, _| assert!(!this.menu.is_open()));
    assert_eq!(witness(&view, cx), 5);
}

#[gpui::test]
fn keep_open_retains_menu_focus_and_reopening_never_replaces_the_return_target(
    cx: &mut TestAppContext,
) {
    let (view, cx) = cx.add_window_view(|window, cx| Probe::new(Dismissal::KeepOpen, window, cx));
    cx.run_until_parked();
    open(cx);
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    cx.update(|window, cx| {
        view.update(cx, |this, cx| {
            assert!(this.menu.is_open());
            assert!(this.menu.focus.is_focused(window));
            this.menu.open_end(
                Bounds {
                    origin: point(px(20.0), px(30.0)),
                    size: gpui::size(px(40.0), px(30.0)),
                },
                window,
                cx,
            );
            this.menu.close_and_restore(window, cx);
            assert!(this.root.is_focused(window));
        });
    });
    assert_eq!(witness(&view, cx), 1);
}

#[gpui::test]
fn closing_cannot_steal_new_focus_or_retain_a_removed_return_handle(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| Probe::new(Dismissal::Close, window, cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        view.update(cx, |this, cx| {
            this.menu.open(point(px(10.0), px(20.0)), window, cx);
            let newer = cx.focus_handle();
            newer.focus(window, cx);
            this.menu.close_and_restore(window, cx);
            assert!(newer.is_focused(window));
            let previous = cx.focus_handle();
            previous.focus(window, cx);
            this.menu.open(point(px(10.0), px(20.0)), window, cx);
            drop(previous);
            this.menu.close_and_restore(window, cx);
            assert!(window.focused(cx).is_none());
        });
    });
}

#[gpui::test]
fn state_only_close_then_new_open_captures_the_current_caller(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| Probe::new(Dismissal::Close, window, cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        view.update(cx, |this, cx| {
            this.menu.open(point(px(10.0), px(20.0)), window, cx);
            this.menu.close();
            let current = cx.focus_handle();
            current.focus(window, cx);
            this.menu.open(point(px(30.0), px(40.0)), window, cx);
            this.menu.close_and_restore(window, cx);
            assert!(current.is_focused(window));
        });
    });
}
