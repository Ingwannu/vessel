//! Retained real GPUI frame A must not authorize input against opening B.
use super::*;
use crate::{Scheme, TextField, Tokens};
use gpui::prelude::*;
use gpui::{
    Entity, InputEvent as _, KeyDownEvent, Keystroke, Modifiers, MouseButton, MouseDownEvent,
    MouseUpEvent, Render, TestAppContext, VisualTestContext, div, point, px,
};
use std::cell::Cell;

struct Probe {
    field: TextField,
    menu: MenuState,
    dismissal: Dismissal,
    generation: usize,
    chosen: Vec<(usize, usize)>,
    renders: Rc<Cell<usize>>,
}
impl Probe {
    fn new(dismissal: Dismissal, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let field = TextField::new("Actual opener", |_, _, _, _| {}, window, cx);
        field.focus(window, cx);
        Self {
            field,
            menu: MenuState::new(cx),
            dismissal,
            generation: 0,
            chosen: Vec::new(),
            renders: Rc::new(Cell::new(0)),
        }
    }
    fn open(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.menu.open(point(px(100.0), px(40.0)), window, cx);
        cx.notify();
    }
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.renders.set(self.renders.get().saturating_add(1));
        let tokens = Tokens::of(Scheme::Dark);
        let generation = self.generation;
        let handlers = MenuHandlers {
            state: |this: &mut Self| &mut this.menu,
            chose: Rc::new(move |this, index, _, _| this.chosen.push((generation, index))),
        };
        let items = [
            MenuItem::new("First").with_dismissal(self.dismissal),
            MenuItem::new("Last").with_dismissal(self.dismissal),
        ];
        div()
            .size_full()
            .child(self.field.element(&tokens, false, cx))
            .children(menu_with(
                Content::new("lease-menu", &items).with_width(px(220.0)),
                &self.menu,
                &handlers,
                &tokens,
                false,
                cx,
            ))
    }
}
fn initialize(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).expect("fonts");
        cx.set_reduce_motion(true);
    });
}
fn open_a(view: &Entity<Probe>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| view.update(cx, |this, cx| this.open(window, cx)));
    cx.run_until_parked();
}
fn key(window: &mut Window, cx: &mut App, key: &'static str) {
    let _result = window.dispatch_event(
        KeyDownEvent {
            keystroke: Keystroke::parse(key).expect("test key"),
            is_held: false,
            prefer_character_input: false,
        }
        .to_platform_input(),
        cx,
    );
}
fn click(window: &mut Window, cx: &mut App, at: Point<Pixels>) {
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
#[derive(Clone, Copy)]
enum Opening {
    InPlace,
    AfterClose,
    MeasuredEnd,
}
// One outer window update intentionally retains A's rendered event tree while
// opening B. Dispatch outside the entity update avoids re-entrant borrowing;
// the render count proves no redraw silently replaced A's handlers beforehand.
fn stale_event(
    view: &Entity<Probe>,
    cx: &mut VisualTestContext,
    opening: Opening,
    event: impl FnOnce(&mut Window, &mut App),
) {
    cx.update(|window, cx| {
        let before = view.read(cx).renders.get();
        view.update(cx, |this, cx| {
            this.generation = 1;
            match opening {
                Opening::InPlace => this.open(window, cx),
                Opening::AfterClose => {
                    this.menu.close_and_restore(window, cx);
                    this.open(window, cx);
                }
                Opening::MeasuredEnd => {
                    this.menu.open_end(
                        Bounds {
                            origin: point(px(100.0), px(10.0)),
                            size: gpui::size(px(220.0), px(30.0)),
                        },
                        window,
                        cx,
                    );
                    cx.notify();
                }
            }
            this.menu.highlighted = Some(1);
        });
        event(window, cx);
        let this = view.read(cx);
        assert_eq!(this.renders.get(), before, "must dispatch A before redraw");
        assert!(this.menu.is_open(), "B must survive A's dismissal");
        assert!(this.menu.focus.is_focused(window), "B retains keys");
        assert!(!this.field.focus_handle(cx).is_focused(window));
        assert_eq!(this.menu.highlighted, Some(1), "A cannot move B's row");
        assert!(this.chosen.is_empty(), "A cannot dispatch either action");
    });
    cx.run_until_parked();
}
#[gpui::test]
fn old_render_pointer_and_outside_cannot_close_new_open_or_run_keep_open_actions(
    cx: &mut TestAppContext,
) {
    initialize(cx);
    for dismissal in [Dismissal::Close, Dismissal::KeepOpen] {
        for (at, opening) in [
            (point(px(120.0), px(62.0)), Opening::InPlace),
            (point(px(500.0), px(350.0)), Opening::InPlace),
            (point(px(120.0), px(62.0)), Opening::AfterClose),
            (point(px(500.0), px(350.0)), Opening::AfterClose),
            (point(px(120.0), px(62.0)), Opening::MeasuredEnd),
            (point(px(500.0), px(350.0)), Opening::MeasuredEnd),
        ] {
            let (view, cx) = cx.add_window_view(|window, cx| Probe::new(dismissal, window, cx));
            cx.run_until_parked();
            open_a(&view, cx);
            stale_event(&view, cx, opening, |window, cx| click(window, cx, at));
            cx.simulate_click(point(px(120.0), px(62.0)), Modifiers::none());
            cx.run_until_parked();
            cx.update(|window, cx| {
                let this = view.read(cx);
                assert_eq!(this.chosen, [(1, 0)], "fresh B row works");
                assert_eq!(this.menu.is_open(), dismissal == Dismissal::KeepOpen);
                if dismissal == Dismissal::Close {
                    assert!(this.field.focus_handle(cx).is_focused(window));
                }
            });
        }
    }
}
#[gpui::test]
fn old_render_keys_cannot_highlight_close_restore_focus_or_choose_in_new_open(
    cx: &mut TestAppContext,
) {
    initialize(cx);
    for dismissal in [Dismissal::Close, Dismissal::KeepOpen] {
        for old_key in [
            "down", "up", "tab", "home", "end", "enter", "space", "escape",
        ] {
            let (view, cx) = cx.add_window_view(|window, cx| Probe::new(dismissal, window, cx));
            cx.run_until_parked();
            open_a(&view, cx);
            stale_event(&view, cx, Opening::AfterClose, |window, cx| {
                key(window, cx, old_key)
            });
            cx.simulate_keystrokes("enter");
            cx.run_until_parked();
            view.read_with(cx, |this, _| {
                assert_eq!(this.chosen, [(1, 1)], "fresh B key works");
                assert_eq!(this.menu.is_open(), dismissal == Dismissal::KeepOpen);
            });
            if dismissal == Dismissal::KeepOpen {
                cx.simulate_keystrokes("escape");
                cx.run_until_parked();
            }
            cx.update(|window, cx| {
                assert!(view.read(cx).field.focus_handle(cx).is_focused(window));
            });
        }
    }
}
#[gpui::test]
fn closed_and_replaced_state_never_accept_a_retained_open_lease(cx: &mut TestAppContext) {
    initialize(cx);
    let (view, cx) = cx.add_window_view(|window, cx| Probe::new(Dismissal::Close, window, cx));
    cx.run_until_parked();
    open_a(&view, cx);
    cx.update(|window, cx| {
        let before = view.read(cx).renders.get();
        view.update(cx, |this, cx| {
            let lease = this.menu.render_lease();
            this.menu.close();
            assert!(!this.menu.accepts(&lease));
            this.menu = MenuState::new(cx);
            this.field.focus(window, cx);
            this.open(window, cx);
            assert!(!this.menu.same_cycle(&lease));
        });
        click(window, cx, point(px(120.0), px(62.0)));
        click(window, cx, point(px(500.0), px(350.0)));
        let this = view.read(cx);
        assert_eq!(this.renders.get(), before);
        assert!(this.menu.is_open());
        assert!(this.menu.focus.is_focused(window));
        assert!(this.chosen.is_empty());
    });
}

#[gpui::test]
fn in_place_reopen_also_rejects_old_keys(cx: &mut TestAppContext) {
    initialize(cx);
    let (view, cx) = cx.add_window_view(|window, cx| Probe::new(Dismissal::KeepOpen, window, cx));
    cx.run_until_parked();
    open_a(&view, cx);
    stale_event(&view, cx, Opening::InPlace, |window, cx| {
        key(window, cx, "down");
        key(window, cx, "enter");
        key(window, cx, "escape");
    });
    cx.simulate_keystrokes("enter escape");
    cx.run_until_parked();
    cx.update(|window, cx| {
        let this = view.read(cx);
        assert_eq!(this.chosen, [(1, 1)]);
        assert!(!this.menu.is_open());
        assert!(this.field.focus_handle(cx).is_focused(window));
    });
}

#[gpui::test]
fn state_only_close_invalidates_real_handlers_without_restoring_focus(cx: &mut TestAppContext) {
    initialize(cx);
    let (view, cx) = cx.add_window_view(|window, cx| Probe::new(Dismissal::KeepOpen, window, cx));
    cx.run_until_parked();
    open_a(&view, cx);
    cx.update(|window, cx| {
        let before = view.read(cx).renders.get();
        view.update(cx, |this, _| this.menu.close());
        click(window, cx, point(px(120.0), px(62.0)));
        key(window, cx, "home");
        key(window, cx, "enter");
        key(window, cx, "escape");
        click(window, cx, point(px(500.0), px(350.0)));
        let this = view.read(cx);
        assert_eq!(this.renders.get(), before);
        assert!(!this.menu.is_open());
        assert_eq!(this.menu.highlighted, None);
        assert!(
            this.menu.focus.is_focused(window),
            "state-only close stays state-only"
        );
        assert!(!this.field.focus_handle(cx).is_focused(window));
        assert!(this.chosen.is_empty());
    });
}
