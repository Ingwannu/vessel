//! Actual Retry dispatch in the retained list, with a real enclosing dialog.
use super::*;
use crate::{
    Scheme, TextField,
    dialog::{DialogContent, DialogSize, DialogState},
};
use gpui::{
    App, Bounds, InputEvent as _, MouseButton, MouseDownEvent, MouseUpEvent, Pixels, Point, Render,
    TestAppContext, Window, point, size,
};
use std::{cell::Cell, rc::Rc};
struct Probe {
    field: TextField,
    dialog: DialogState,
    list: ComboboxState,
    visible: bool,
    pending: bool,
    generation: usize,
    calls: Vec<usize>,
    parent_closes: usize,
    draws: Rc<Cell<usize>>,
    scheme: Scheme,
}
impl Probe {
    fn new(scheme: Scheme, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let field = TextField::new("Actual opener", |_, _, _, _| {}, window, cx);
        field.focus(window, cx);
        let this = Self {
            field,
            dialog: DialogState::new(cx),
            list: ComboboxState::new(window, cx),
            visible: true,
            pending: false,
            generation: 0,
            calls: Vec::new(),
            parent_closes: 0,
            draws: Rc::new(Cell::new(0)),
            scheme,
        };
        this.dialog.focus(window, cx);
        this
    }
    fn open(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.list.set_anchor(Some(Bounds {
            origin: point(px(100.0), px(100.0)),
            size: size(px(240.0), px(32.0)),
        }));
        self.list.open(None, window, cx);
        cx.notify();
    }
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.draws.set(self.draws.get().saturating_add(1));
        let tokens = Tokens::of(self.scheme);
        let generation = self.generation;
        let retry = if self.pending {
            Retry::Pending("Retrying...".into())
        } else {
            Retry::Ready {
                label: "Retry".into(),
                action: Rc::new(move |owner: &mut Self, _, _| owner.calls.push(generation)),
            }
        };
        let empty = Empty::Failed {
            message: "Could not load models".into(),
            retry: Some(retry),
        };
        let header = super::super::ComboHeader {
            title: None,
            search: false,
            back: false,
            note: None,
            empty: "Legacy empty".into(),
            side: super::super::Side::Below,
        };
        let handlers = ComboHandlers {
            state: |owner: &mut Self| &mut owner.list,
            chose: Rc::new(|_, _, _, _| {}),
            back: None,
        };
        let popup = super::super::combobox_with_empty(
            super::super::ComboboxContent::new(
                "retry-list",
                &header,
                &[],
                super::super::ComboboxWidth::Fixed(px(240.0)),
            ),
            &self.list,
            &handlers,
            &empty,
            &tokens,
            window,
            cx,
        );
        let mut body = div()
            .size_full()
            .child(self.field.element(&tokens, false, cx));
        if self.visible {
            body = body.child(crate::dialog(
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
                |owner: &mut Self, window, cx| {
                    owner.visible = false;
                    owner.parent_closes = owner.parent_closes.saturating_add(1);
                    owner.dialog.dismissed(window, cx);
                    cx.notify();
                },
                cx,
            ));
        }
        body
    }
}
fn click(window: &mut Window, cx: &mut App, at: Point<Pixels>) {
    for event in [
        MouseDownEvent {
            position: at,
            button: MouseButton::Left,
            click_count: 1,
            ..Default::default()
        }
        .to_platform_input(),
        MouseUpEvent {
            position: at,
            button: MouseButton::Left,
            click_count: 1,
            ..Default::default()
        }
        .to_platform_input(),
    ] {
        let _dispatch = window.dispatch_event(event, cx);
    }
}
#[gpui::test]
fn old_retry_cannot_fire_or_refocus_b_and_current_retry_stays_open(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let initialized = crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark));
        assert!(initialized.is_ok());
        cx.set_reduce_motion(true);
    });
    for scheme in [Scheme::Light, Scheme::Dark] {
        for width in [390.0, 1280.0] {
            let (view, cx) = cx.add_window_view(|window, cx| Probe::new(scheme, window, cx));
            cx.simulate_resize(size(px(width), px(800.0)));
            cx.run_until_parked();
            view.update_in(cx, |owner, window, cx| owner.open(window, cx));
            cx.run_until_parked();
            let at = cx.debug_bounds("model-browser-retry");
            assert!(at.is_some());
            let Some(at) = at.map(|bounds| bounds.center()) else {
                return;
            };
            cx.update(|window, cx| {
                let draws = view.read(cx).draws.get();
                view.update(cx, |owner, cx| {
                    owner.generation = owner.generation.saturating_add(1);
                    owner.open(window, cx);
                });
                click(window, cx, at);
                let owner = view.read(cx);
                assert_eq!(
                    owner.draws.get(),
                    draws,
                    "actual A was retained while B opened"
                );
                assert!(owner.calls.is_empty());
                assert!(owner.visible);
                assert!(owner.list.is_open());
                assert_eq!(owner.parent_closes, 0);
                assert!(owner.list.focus.contains_focused(window, cx));
            });
            cx.run_until_parked();
            let at = cx.debug_bounds("model-browser-retry");
            assert!(at.is_some());
            let Some(at) = at.map(|bounds| bounds.center()) else {
                return;
            };
            cx.update(|window, cx| click(window, cx, at));
            view.update_in(cx, |owner, _, cx| {
                assert_eq!(owner.calls, [1]);
                assert!(owner.list.is_open());
                owner.pending = true;
                cx.notify();
            });
            cx.run_until_parked();
            cx.update(|window, cx| click(window, cx, at));
            view.read_with(cx, |owner, _| {
                assert_eq!(owner.calls, [1]);
                assert!(owner.visible);
            });
            cx.simulate_keystrokes("escape");
            cx.run_until_parked();
            view.read_with(cx, |owner, _| {
                assert!(!owner.list.is_open());
                assert!(owner.visible);
                assert_eq!(owner.parent_closes, 0);
            });
            cx.simulate_keystrokes("escape");
            cx.run_until_parked();
            view.read_with(cx, |owner, _| assert_eq!(owner.parent_closes, 1));
        }
    }
}
