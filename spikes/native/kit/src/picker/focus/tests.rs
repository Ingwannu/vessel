//! Actual shared combobox input paths; helper equality alone cannot prove that
//! an unmounted focus target stops swallowing the opener's keys.
use super::super::*;
use crate::{Scheme, Tokens};
use gpui::prelude::*;
use gpui::{Entity, KeyDownEvent, Modifiers, Render, TestAppContext, div, point, px};

#[derive(Clone, Copy)]
enum Choice {
    Close,
    Stay,
    NewFocus,
}
struct Probe {
    root: FocusHandle,
    newer: FocusHandle,
    list: ComboboxState,
    search: bool,
    choice: Choice,
    chosen: usize,
    f5: usize,
}
impl Probe {
    fn new(search: bool, choice: Choice, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let root = cx.focus_handle();
        root.focus(window, cx);
        Self {
            root,
            newer: cx.focus_handle(),
            list: ComboboxState::new(window, cx),
            search,
            choice,
            chosen: 0,
            f5: 0,
        }
    }
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let mut root = div()
            .size_full()
            .track_focus(&self.root)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, _| {
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
                        this.list.set_anchor(Some(Bounds {
                            origin: point(px(100.0), px(40.0)),
                            size: gpui::size(px(80.0), px(30.0)),
                        }));
                        this.list.open(this.search.then_some("Search"), window, cx);
                        cx.notify();
                    })),
            );
        root = root.child(
            div()
                .id("newer")
                .track_focus(&self.newer)
                .w(px(40.0))
                .h(px(30.0)),
        );
        let rows = vec![ComboRow::new("Choose")];
        let header = ComboHeader {
            title: None,
            back: false,
            search: self.search,
            empty: "Empty".into(),
            note: None,
            side: Side::Below,
        };
        let handlers = ComboHandlers {
            state: |this: &mut Self| &mut this.list,
            chose: Rc::new(|this, _, window, cx| {
                this.chosen = this.chosen.saturating_add(1);
                match this.choice {
                    Choice::Close => this.list.close(),
                    Choice::Stay => {}
                    Choice::NewFocus => {
                        this.list.close();
                        this.newer.focus(window, cx);
                    }
                }
            }),
            back: None,
        };
        if let Some(popup) = combobox(
            "picker", &self.list, &header, &rows, &handlers, 180.0, &tokens, window, cx,
        ) {
            root = root.child(popup);
        }
        root
    }
}
fn setup(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let initialized = crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark));
        assert!(initialized.is_ok(), "faces and input globals register");
    });
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
fn plain_list_restores_after_pointer_keyboard_escape_and_outside(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|window, cx| Probe::new(false, Choice::Close, window, cx));
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
    cx.simulate_click(point(px(120.0), px(95.0)), Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |this, _| assert_eq!(this.chosen, 2));
    assert_eq!(witness(&view, cx), 4);
    open(cx);
    cx.simulate_click(point(px(350.0), px(350.0)), Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |this, _| assert!(!this.list.is_open()));
    assert_eq!(witness(&view, cx), 5);
}
#[gpui::test]
fn searchable_list_restores_its_opener_and_never_steals_newer_focus(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|window, cx| Probe::new(true, Choice::Close, window, cx));
    cx.run_until_parked();
    open(cx);
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(witness(&view, cx), 1);
    view.update(cx, |this, _| this.choice = Choice::NewFocus);
    open(cx);
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    cx.update(|window, cx| view.read_with(cx, |this, _| assert!(this.newer.is_focused(window))));
}
#[gpui::test]
fn intermediate_choices_keep_list_focus_and_the_original_weak_opener(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|window, cx| Probe::new(false, Choice::Stay, window, cx));
    cx.run_until_parked();
    open(cx);
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    cx.update(|window, cx| {
        view.update(cx, |this, cx| {
            assert!(this.list.is_open());
            assert!(
                this.list.focus.contains_focused(window, cx)
                    || this
                        .list
                        .search
                        .focus_handle(cx)
                        .contains_focused(window, cx)
            );
            this.list.open(Some("Models"), window, cx);
            this.list.close_and_restore(window, cx);
            assert!(this.root.is_focused(window));
        });
    });
    assert_eq!(witness(&view, cx), 1);
}
