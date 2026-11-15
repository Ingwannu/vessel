//! The actual parent dialog must survive events from an obsolete child menu.
use super::*;
use crate::dialog::{DialogContent, DialogSize, DialogState, dialog};

struct Nested {
    dialog: DialogState,
    open: bool,
    field: TextField,
    menu: MenuState,
    anchor: TriggerAnchor,
    scheme: Scheme,
    dismissal: Dismissal,
    renders: usize,
    parent_keys: usize,
    chosen: usize,
}
impl Nested {
    fn new(
        scheme: Scheme,
        dismissal: Dismissal,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let dialog = DialogState::new(cx);
        dialog.focus(window, cx);
        let field = TextField::new("Nested opener", |_, _, _, _| {}, window, cx);
        field.focus(window, cx);
        Self {
            dialog,
            open: true,
            field,
            menu: MenuState::new(cx),
            anchor: TriggerAnchor::default(),
            scheme,
            dismissal,
            renders: 0,
            parent_keys: 0,
            chosen: 0,
        }
    }
    fn open_menu(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let bounds = self.anchor.get().expect("actual measured trigger");
        self.menu.open_end(bounds, window, cx);
        cx.notify();
    }
}
impl Render for Nested {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.renders = self.renders.saturating_add(1);
        let tokens = Tokens::of(self.scheme);
        let items = [MenuItem::new("Nested action").with_dismissal(self.dismissal)];
        let handlers = MenuHandlers {
            state: |this: &mut Self| &mut this.menu,
            chose: Rc::new(|this, _, _, _| this.chosen = this.chosen.saturating_add(1)),
        };
        let body = div()
            .h(px(240.0))
            .on_key_down(cx.listener(|this, _: &KeyDownEvent, _, _| {
                this.parent_keys = this.parent_keys.saturating_add(1);
            }))
            .child(self.field.element(&tokens, false, cx))
            .child(
                div()
                    .id("nested-trigger")
                    .size(px(32.0))
                    .child(crate::picker::anchor_probe(&self.anchor)),
            )
            .children(menu_with(
                Content::new("nested-lease-menu", &items)
                    .with_width(px(220.0))
                    .with_layer(Layer::AboveDialog),
                &self.menu,
                &handlers,
                &tokens,
                false,
                cx,
            ));
        div().size_full().children(self.open.then(|| {
            dialog(
                &self.dialog,
                DialogContent {
                    title: "Actual parent dialog".into(),
                    body: body.into_any_element(),
                    footer: Vec::new(),
                    size: DialogSize::Normal,
                },
                &tokens,
                window,
                false,
                |this: &mut Self, window, cx| {
                    this.open = false;
                    this.dialog.dismissed(window, cx);
                },
                cx,
            )
        }))
    }
}
fn prepare(view: &Entity<Nested>, cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| view.update(cx, |this, cx| this.open_menu(window, cx)));
    cx.run_until_parked();
}
fn survives(view: &Entity<Nested>, window: &Window, cx: &App) {
    let this = view.read(cx);
    assert!(this.open, "old menu cannot dismiss its parent");
    assert!(this.menu.is_open(), "B remains open");
    assert!(this.menu.focus.is_focused(window), "B retains focus");
    assert_eq!(
        this.parent_keys, 0,
        "menu keys cannot escape along A's path"
    );
    assert_eq!(this.chosen, 0);
}
#[gpui::test]
fn stale_nested_menu_key_keeps_parent_and_b_then_fresh_escape_closes_only_one_layer(
    cx: &mut TestAppContext,
) {
    initialize(cx);
    for scheme in [Scheme::Light, Scheme::Dark] {
        for dismissal in [Dismissal::Close, Dismissal::KeepOpen] {
            for old_key in [
                "escape", "down", "up", "tab", "home", "end", "enter", "space",
            ] {
                let (view, cx) =
                    cx.add_window_view(|window, cx| Nested::new(scheme, dismissal, window, cx));
                prepare(&view, cx);
                let boundary = Rc::new(Cell::new(None));
                let subscription = cx.update(|_, cx| {
                    let view = view.clone();
                    let boundary = Rc::clone(&boundary);
                    cx.intercept_keystrokes(move |_, window, cx| {
                        if boundary.get().is_none() {
                            boundary.set(Some(view.read(cx).renders));
                            view.update(cx, |this, cx| this.open_menu(window, cx));
                        }
                    })
                });
                cx.update(|window, cx| {
                    key(window, cx, old_key);
                    assert_eq!(
                        Some(view.read(cx).renders),
                        boundary.get(),
                        "A handler before B redraw"
                    );
                    survives(&view, window, cx);
                });
                drop(subscription);
                cx.run_until_parked();
                cx.simulate_keystrokes("escape");
                cx.run_until_parked();
                cx.update(|window, cx| {
                    let this = view.read(cx);
                    assert!(this.open);
                    assert!(!this.menu.is_open());
                    assert!(this.field.focus_handle(cx).is_focused(window));
                    assert_eq!(this.parent_keys, 0);
                });
                cx.simulate_keystrokes("escape");
                cx.run_until_parked();
                view.read_with(cx, |this, _| {
                    assert!(!this.open, "next Escape reaches actual parent")
                });
            }
        }
    }
}
#[gpui::test]
fn stale_nested_pointer_and_outside_do_not_dismiss_parent_or_b(cx: &mut TestAppContext) {
    initialize(cx);
    for scheme in [Scheme::Light, Scheme::Dark] {
        for dismissal in [Dismissal::Close, Dismissal::KeepOpen] {
            let (view, cx) =
                cx.add_window_view(|window, cx| Nested::new(scheme, dismissal, window, cx));
            prepare(&view, cx);
            cx.update(|window, cx| {
                let before = view.read(cx).renders;
                let at = view.update(cx, |this, cx| {
                    let bounds = this.anchor.get().expect("trigger");
                    this.open_menu(window, cx);
                    point(bounds.right() - px(30.0), bounds.bottom() + px(18.0))
                });
                click(window, cx, at);
                click(window, cx, point(px(5.0), px(5.0)));
                assert_eq!(view.read(cx).renders, before);
                survives(&view, window, cx);
            });
            cx.run_until_parked();
            cx.simulate_keystrokes("escape");
            cx.run_until_parked();
            view.read_with(cx, |this, _| {
                assert!(this.open);
                assert!(!this.menu.is_open());
            });
        }
    }
}
