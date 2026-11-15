//! Nested menus must win paint/hit order without dismissing their host dialog.
use super::*;
use crate::{Scheme, Tokens, dialog};
use gpui::prelude::*;
use gpui::{Entity, Modifiers, Render, TestAppContext, div, point, px};

struct Probe {
    dialog: dialog::DialogState,
    dialog_open: bool,
    field: FocusHandle,
    menu: MenuState,
    anchor: TriggerAnchor,
    scheme: Scheme,
    selected: usize,
    underlying: usize,
    keys: usize,
}
impl Probe {
    fn new(window: &mut Window, cx: &mut Context<'_, Self>, scheme: Scheme) -> Self {
        let dialog = dialog::DialogState::new(cx);
        dialog.focus(window, cx);
        let field = cx.focus_handle();
        field.focus(window, cx);
        Self {
            dialog,
            dialog_open: true,
            field,
            menu: MenuState::new(cx),
            anchor: TriggerAnchor::default(),
            scheme,
            selected: 0,
            underlying: 0,
            keys: 0,
        }
    }
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let items = [MenuItem::new("Nested action / 안쪽 동작")];
        let handlers = MenuHandlers {
            state: |this: &mut Self| &mut this.menu,
            chose: Rc::new(|this, _, _, _| this.selected = this.selected.saturating_add(1)),
        };
        let mut body = div()
            .track_focus(&self.field)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, _| {
                if event.keystroke.key == "f5" {
                    this.keys = this.keys.saturating_add(1);
                }
            }))
            .w_full()
            .h(px(260.0))
            .child(
                div()
                    .id("nested-opener")
                    .size(px(32.0))
                    .child(crate::picker::anchor_probe(&self.anchor)),
            );
        if let Some(menu) = menu_with(
            Content::new("nested-menu", &items)
                .with_width(px(220.0))
                .with_layer(Layer::AboveDialog),
            &self.menu,
            &handlers,
            &tokens,
            false,
            cx,
        ) {
            body = body.child(menu);
        }
        let mut root =
            div()
                .size_full()
                .child(div().id("underlying").size_full().on_click(cx.listener(
                    |this, _, _, _| {
                        this.underlying = this.underlying.saturating_add(1);
                    },
                )));
        if self.dialog_open {
            root = root.child(dialog(
                &self.dialog,
                dialog::DialogContent {
                    title: "Dialog / 대화상자".into(),
                    body: body.into_any_element(),
                    footer: Vec::new(),
                    size: dialog::DialogSize::Normal,
                },
                &tokens,
                window,
                false,
                |this: &mut Self, window, cx| {
                    this.dialog_open = false;
                    this.dialog.dismissed(window, cx);
                },
                cx,
            ));
        }
        root
    }
}
fn open(view: &Entity<Probe>, cx: &mut gpui::VisualTestContext) -> Bounds<Pixels> {
    cx.update(|window, cx| {
        view.update(cx, |this, cx| {
            let bounds = this.anchor.get().expect("rendered dialog trigger");
            this.field.focus(window, cx);
            this.menu.open_end(bounds, window, cx);
            cx.notify();
            bounds
        })
    })
}
fn check_host(view: &Entity<Probe>, cx: &mut gpui::VisualTestContext, selected: usize) {
    view.read_with(cx, |this, _| {
        assert!(this.dialog_open);
        assert!(!this.menu.is_open());
        assert_eq!(this.selected, selected);
        assert_eq!(this.underlying, 0);
    });
    cx.simulate_keystrokes("f5");
    cx.run_until_parked();
    cx.update(|window, cx| {
        view.read_with(cx, |this, _| assert!(this.field.is_focused(window)));
    });
}
#[test]
fn existing_entries_default_to_screen_and_modal_order_is_explicit() {
    let items = [MenuItem::new("Action")];
    let content = Content::new("default", &items);
    assert_eq!(content.layer, Layer::Screen);
    assert_eq!(content.layer.priority(), 1);
    assert_eq!(content.with_layer(Layer::AboveDialog).layer.priority(), 3);
}
#[gpui::test]
fn nested_menu_pointer_escape_and_outside_keep_dialog_and_focus(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    for scheme in [Scheme::Light, Scheme::Dark] {
        let (view, cx) = cx.add_window_view(move |window, cx| Probe::new(window, cx, scheme));
        cx.run_until_parked();
        let bounds = open(&view, cx);
        cx.run_until_parked();
        // Measured end alignment; fixed220, offset4, border1, padding4, row28.
        cx.simulate_click(
            point(
                px(f32::from(bounds.right()).mul_add(1.0, -110.0)),
                px(f32::from(bounds.bottom()).mul_add(1.0, 23.0)),
            ),
            Modifiers::none(),
        );
        cx.run_until_parked();
        check_host(&view, cx, 1);
        let _bounds = open(&view, cx);
        cx.run_until_parked();
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        check_host(&view, cx, 1);
        let _bounds = open(&view, cx);
        cx.run_until_parked();
        cx.simulate_click(point(px(5.0), px(5.0)), Modifiers::none());
        cx.run_until_parked();
        check_host(&view, cx, 1);
        view.read_with(cx, |this, _| assert_eq!(this.keys, 3));
        // A second outside press belongs to the dialog, proving the first
        // was consumed by the menu rather than permanently disabling dismissal.
        cx.simulate_click(point(px(5.0), px(5.0)), Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |this, _| {
            assert!(!this.dialog_open);
            assert_eq!(this.underlying, 0);
        });
    }
}
