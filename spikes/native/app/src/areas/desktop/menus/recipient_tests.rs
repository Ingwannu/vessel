//! A popup's edit target cannot become a newer focused input during dispatch.

use super::*;
use gpui::{
    AppContext as _, Entity, InputEvent as _, IntoElement, KeyDownEvent, Keystroke, Render,
    TestAppContext,
};
use std::cell::Cell;
use vessel_app_kit::{Scheme, TextField};
use vessel_app_view::desktop::{CopyState, EditContext, PasteState};

struct Probe {
    shell: Entity<Shell>,
    original: TextField,
    newer: TextField,
    renders: Rc<Cell<usize>>,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.renders.set(self.renders.get().saturating_add(1));
        let tokens = Tokens::of(Scheme::Dark);
        let original = self.original.element(&tokens, false, cx);
        let newer = self.newer.element(&tokens, false, cx);
        self.shell.update(cx, |shell, cx| {
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(original)
                .child(newer)
                .children(render(shell, &tokens, window, cx))
        })
    }
}

#[gpui::test]
fn retained_edit_choice_cannot_copy_or_paste_into_a_newer_focus(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    for copy in [CopyState::Available, CopyState::Unavailable] {
        let (view, cx) = cx.add_window_view(|window, cx| Probe {
            shell: cx.new(|cx| crate::shell::language_tests::shell_without_io(window, cx)),
            original: TextField::new("", |_probe, _text, _window, _cx| {}, window, cx),
            newer: TextField::new("", |_probe, _text, _window, _cx| {}, window, cx),
            renders: Rc::default(),
        });
        view.update_in(cx, |probe, window, cx| {
            window.activate_window();
            probe.original.set_text("original", window, cx);
            probe.newer.set_text("newer", window, cx);
            probe.original.focus(window, cx);
            cx.write_to_clipboard(gpui::ClipboardItem::new_string("protected".to_owned()));
        });
        cx.run_until_parked();
        cx.simulate_keystrokes(if cfg!(target_os = "macos") {
            "cmd-a"
        } else {
            "ctrl-a"
        });
        view.update_in(cx, |probe, window, cx| {
            probe.shell.update(cx, |shell, cx| {
                open_context(
                    shell,
                    gpui::point(px(20.0), px(80.0)),
                    EditContext {
                        copy,
                        paste: PasteState::Available,
                    },
                    window,
                    cx,
                );
            });
        });
        cx.run_until_parked();
        cx.simulate_keystrokes("down");
        let boundary = Rc::new(Cell::new(None));
        let subscription = cx.update(|_, cx| {
            let view = view.clone();
            let boundary = Rc::clone(&boundary);
            // The real interceptor runs after key-path capture and redraw, but
            // before the retained menu's actual Enter listener and close ACK.
            cx.intercept_keystrokes(move |_, window, cx| {
                if boundary.get().is_none() {
                    boundary.set(Some(view.read(cx).renders.get()));
                    view.update(cx, |probe, cx| probe.newer.focus(window, cx));
                }
            })
        });
        cx.update(|window, cx| {
            let _event = window.dispatch_event(
                KeyDownEvent {
                    keystroke: Keystroke::parse("enter").unwrap(),
                    is_held: false,
                    prefer_character_input: false,
                }
                .to_platform_input(),
                cx,
            );
            let probe = view.read(cx);
            assert_eq!(
                Some(probe.renders.get()),
                boundary.get(),
                "retained frame delivered"
            );
            assert!(!probe.shell.read(cx).areas.desktop.menu.is_open());
            assert!(probe.newer.focus_handle(cx).is_focused(window));
        });
        drop(subscription);
        cx.run_until_parked();
        view.read_with(cx, |probe, cx| {
            assert_eq!(probe.original.text(cx), "original");
            assert_eq!(probe.newer.text(cx), "newer");
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().unwrap(),
                "protected"
            );
        });
    }
}
