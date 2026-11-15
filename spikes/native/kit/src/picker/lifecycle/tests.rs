//! Real retained GPUI A callbacks run after B opens without an intervening draw.
use super::super::*;
use crate::{
    Scheme, TextField, Tokens,
    dialog::{DialogContent, DialogSize, DialogState},
};
use gpui::prelude::*;
use gpui::{
    Entity, InputEvent as _, KeyDownEvent, Keystroke, Modifiers, MouseDownEvent, MouseUpEvent,
    Render, TestAppContext, VisualTestContext, div, point, px,
};
#[derive(Clone, Copy)]
enum Reopen {
    InPlace,
    AfterClose,
    Replace,
    Drill,
}
#[derive(Clone, Copy)]
enum Selection {
    Close,
    Stay,
    Reopen,
}
struct Probe {
    field: TextField,
    dialog: DialogState,
    visible: bool,
    list: ComboboxState,
    searchable: bool,
    generation: usize,
    choices: Vec<(usize, usize)>,
    selection: Selection,
    parent_closes: usize,
    draws: Rc<Cell<usize>>,
}
impl Probe {
    fn new(
        searchable: bool,
        selection: Selection,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let field = TextField::new("Actual opener", |_, _, _, _| {}, window, cx);
        field.focus(window, cx);
        let mut this = Self {
            field,
            dialog: DialogState::new(cx),
            visible: true,
            list: ComboboxState::new(window, cx),
            searchable,
            generation: 0,
            choices: vec![],
            selection,
            parent_closes: 0,
            draws: Rc::new(Cell::new(0)),
        };
        this.dialog.focus(window, cx);
        this
    }
    fn open(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.list.set_anchor(Some(Bounds {
            origin: point(px(100.0), px(40.0)),
            size: gpui::size(px(80.0), px(30.0)),
        }));
        self.list
            .open(self.searchable.then_some("Search"), window, cx);
        cx.notify();
    }
    fn reopen(&mut self, mode: Reopen, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.generation = self.generation.saturating_add(1);
        match mode {
            Reopen::InPlace => self.open(window, cx),
            Reopen::AfterClose => {
                self.list.close_and_restore(window, cx);
                self.open(window, cx);
            }
            Reopen::Replace => {
                self.list = ComboboxState::new(window, cx);
                self.open(window, cx);
            }
            Reopen::Drill => {
                self.searchable = true;
                self.list.reset_query("Other page", window, cx);
            }
        }
        self.list.highlighted = Some(1);
        self.list.highlighted_label = Some("Last".into());
    }
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.draws.set(self.draws.get().saturating_add(1));
        let tokens = Tokens::of(Scheme::Dark);
        let generation = self.generation;
        let handlers = ComboHandlers {
            state: |this: &mut Self| &mut this.list,
            chose: Rc::new(move |this, index, window, cx| {
                this.choices.push((generation, index));
                match this.selection {
                    Selection::Close => this.list.close(),
                    Selection::Stay => {}
                    Selection::Reopen => this.reopen(Reopen::AfterClose, window, cx),
                }
            }),
            back: None,
        };
        let rows = [ComboRow::new("First"), ComboRow::new("Last")];
        let header = ComboHeader {
            title: None,
            back: false,
            search: self.searchable,
            empty: "Empty".into(),
            note: None,
            side: Side::Below,
        };
        let popup = combobox(
            "lease-picker",
            &self.list,
            &header,
            &rows,
            &handlers,
            180.0,
            &tokens,
            window,
            cx,
        );
        let mut root = div()
            .size_full()
            .child(self.field.element(&tokens, false, cx));
        if self.visible {
            root = root.child(crate::dialog(
                &self.dialog,
                DialogContent {
                    title: "Parent".into(),
                    body: div().min_h(px(250.0)).children(popup).into_any_element(),
                    footer: vec![],
                    size: DialogSize::Normal,
                },
                &tokens,
                window,
                false,
                |this: &mut Self, window, cx| {
                    this.parent_closes = this.parent_closes.saturating_add(1);
                    this.visible = false;
                    this.dialog.dismissed(window, cx);
                    cx.notify();
                },
                cx,
            ));
        }
        root
    }
}
fn initialize(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).unwrap();
        cx.set_reduce_motion(true);
    });
}
fn open(view: &Entity<Probe>, cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| view.update(cx, |this, cx| this.open(window, cx)));
    cx.run_until_parked();
}
fn press(window: &mut Window, cx: &mut App, text: &str) {
    let _dispatch = window.dispatch_event(
        KeyDownEvent {
            keystroke: Keystroke::parse(text).unwrap(),
            is_held: false,
            prefer_character_input: false,
        }
        .to_platform_input(),
        cx,
    );
}
fn click(window: &mut Window, cx: &mut App, at: gpui::Point<Pixels>) {
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
fn assert_b(view: &Entity<Probe>, window: &Window, cx: &App, draws: usize) {
    let this = view.read(cx);
    assert_eq!(
        this.draws.get(),
        draws,
        "the stale A frame was actually retained"
    );
    assert!(this.visible);
    assert_eq!(this.parent_closes, 0);
    assert!(this.list.is_open());
    assert_eq!(this.list.highlighted, Some(1));
    assert!(this.choices.is_empty());
    assert!(
        this.list.focus.contains_focused(window, cx)
            || this
                .list
                .search
                .focus_handle(cx)
                .contains_focused(window, cx)
    );
}
#[gpui::test]
fn retained_combo_pointer_and_outside_cannot_choose_close_or_refocus_b(cx: &mut TestAppContext) {
    initialize(cx);
    for mode in [Reopen::InPlace, Reopen::AfterClose, Reopen::Replace] {
        for at in [point(px(120.0), px(90.0)), point(px(500.0), px(350.0))] {
            let (view, cx) =
                cx.add_window_view(|window, cx| Probe::new(false, Selection::Close, window, cx));
            open(&view, cx);
            cx.update(|window, cx| {
                let draws = view.read(cx).draws.get();
                view.update(cx, |this, cx| this.reopen(mode, window, cx));
                click(window, cx, at);
                assert_b(&view, window, cx, draws);
            });
            cx.run_until_parked();
        }
    }
}
#[gpui::test]
fn stale_combo_keys_are_contained_before_an_enclosing_dialog_and_b_stays_live(
    cx: &mut TestAppContext,
) {
    initialize(cx);
    for searchable in [false, true] {
        for mode in [
            Reopen::InPlace,
            Reopen::AfterClose,
            Reopen::Replace,
            Reopen::Drill,
        ] {
            for key in ["down", "up", "enter", "escape"] {
                let (view, cx) = cx.add_window_view(|window, cx| {
                    Probe::new(searchable, Selection::Stay, window, cx)
                });
                open(&view, cx);
                // GPUI pre-key redraw must finish BEFORE opening B. The actual
                // interceptor sits between that draw and A's captured key path.
                let boundary = Rc::new(Cell::new(None));
                let subscription = cx.update(|_, cx| {
                    let view = view.clone();
                    let boundary = Rc::clone(&boundary);
                    cx.intercept_keystrokes(move |_, window, cx| {
                        if boundary.get().is_none() {
                            boundary.set(Some(view.read(cx).draws.get()));
                            view.update(cx, |this, cx| this.reopen(mode, window, cx));
                        }
                    })
                });
                cx.update(|window, cx| {
                    press(window, cx, key);
                    assert_b(&view, window, cx, boundary.get().unwrap());
                });
                drop(subscription);
                cx.run_until_parked();
                cx.simulate_keystrokes("enter");
                cx.run_until_parked();
                view.read_with(cx, |this, _| {
                    assert_eq!(this.choices, [(1, 1)]);
                    assert!(this.list.is_open());
                });
                cx.simulate_keystrokes("escape");
                cx.run_until_parked();
                view.read_with(cx, |this, _| {
                    assert!(this.visible);
                    assert!(!this.list.is_open());
                });
                cx.simulate_keystrokes("escape");
                cx.run_until_parked();
                cx.update(|window, cx| {
                    let this = view.read(cx);
                    assert!(!this.visible);
                    assert_eq!(this.parent_closes, 1);
                    assert!(this.field.focus_handle(cx).is_focused(window));
                });
            }
        }
    }
}
#[gpui::test]
fn a_choice_opening_a_new_list_does_not_restore_old_focus_over_it(cx: &mut TestAppContext) {
    initialize(cx);
    let (view, cx) =
        cx.add_window_view(|window, cx| Probe::new(false, Selection::Reopen, window, cx));
    open(&view, cx);
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    cx.update(|window, cx| {
        let this = view.read(cx);
        assert_eq!(this.choices, [(0, 0)]);
        assert_eq!(this.generation, 1);
        assert!(this.list.is_open());
        assert!(this.list.focus.is_focused(window));
        assert_eq!(this.parent_closes, 0);
    });
}
