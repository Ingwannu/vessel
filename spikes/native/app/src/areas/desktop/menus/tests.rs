//! Real menu keys dispatch into the original input after popup focus is restored.

use super::*;
use gpui::{
    AppContext as _, Entity, InputEvent as _, IntoElement, Modifiers, MouseButton, MouseDownEvent,
    MouseUpEvent, Render, TestAppContext,
};
use std::cell::Cell;
use vessel_app_kit::{Scheme, TextField};
use vessel_app_view::desktop::{CopyState, EditContext, PasteState};

struct Probe {
    shell: Entity<Shell>,
    field: TextField,
    renders: Rc<Cell<usize>>,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.renders.set(self.renders.get().saturating_add(1));
        let tokens = Tokens::of(Scheme::Dark);
        let field = self.field.element(&tokens, false, cx);
        self.shell.update(cx, |shell, cx| {
            div()
                .size_full()
                .child(field)
                .children(render(shell, &tokens, window, cx))
        })
    }
}

#[gpui::test]
fn unsupported_zoom_is_hidden_and_refused_without_changing_native_scale(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(crate::shell::language_tests::shell_without_io);
    shell.update_in(cx, |shell, window, cx| {
        let scale = window.scale_factor();
        for action in [
            DesktopAction::ZoomIn,
            DesktopAction::ZoomOut,
            DesktopAction::ActualSize,
        ] {
            assert!(!Shell::desktop_shortcut_supported(action));
            assert!(!shell.run_desktop_shortcut(action, window, cx));
        }
        assert_eq!(window.scale_factor(), scale);
        assert!(rows(shell, Group::All).iter().all(|row| match row.item {
            Item::Desktop(action) => Shell::desktop_shortcut_supported(action),
            Item::Edit(_)
            | Item::Reload
            | Item::ForceReload
            | Item::Minimize
            | Item::Maximize
            | Item::Quit
            | Item::Help(_) => true,
        }));
        for action in [
            DesktopAction::NewWindow,
            DesktopAction::CloseWindow,
            DesktopAction::Fullscreen,
        ] {
            assert!(Shell::desktop_shortcut_supported(action));
        }
    });
}

#[gpui::test]
fn context_menu_restores_input_focus_and_disabled_copy_is_skipped(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    for (copy, paste) in [
        (CopyState::Available, PasteState::Available),
        (CopyState::Unavailable, PasteState::Available),
        (CopyState::Unavailable, PasteState::Unavailable),
    ] {
        let (view, cx) = cx.add_window_view(|window, cx| Probe {
            shell: cx.new(|cx| crate::shell::language_tests::shell_without_io(window, cx)),
            field: TextField::new("", |_probe, _text, _window, _cx| {}, window, cx),
            renders: Rc::default(),
        });
        cx.run_until_parked();
        view.update_in(cx, |probe, window, cx| {
            window.activate_window();
            probe.field.set_text("selected", window, cx);
            probe.field.focus(window, cx);
            cx.write_to_clipboard(gpui::ClipboardItem::new_string("replacement".to_owned()));
        });
        cx.run_until_parked();
        cx.simulate_keystrokes(if cfg!(target_os = "macos") {
            "cmd-a"
        } else {
            "ctrl-a"
        });
        view.update_in(cx, |probe, window, cx| {
            probe.shell.update(cx, |shell, cx| {
                assert!(crate::areas::desktop::open_edit_context(
                    shell,
                    gpui::point(px(20.0), px(40.0)),
                    EditContext { copy, paste },
                    window,
                    cx
                ));
            });
        });
        cx.run_until_parked();
        // The real menu's keyboard navigation must skip disabled Copy and choose
        // Paste. With a selection, the same first-row press must copy instead.
        cx.simulate_keystrokes("down enter");
        cx.run_until_parked();
        view.update_in(cx, |probe, window, cx| {
            assert!(probe.field.focus_handle(cx).is_focused(window));
            let clipboard = cx.read_from_clipboard().unwrap().text().unwrap();
            match (copy, paste) {
                (CopyState::Available, PasteState::Available | PasteState::Unavailable) => {
                    assert_eq!(clipboard, "selected");
                }
                (CopyState::Unavailable, PasteState::Available) => {
                    assert_eq!(probe.field.text(cx), "replacement");
                }
                (CopyState::Unavailable, PasteState::Unavailable) => {
                    assert_eq!(probe.field.text(cx), "selected");
                }
            }
        });
        if paste == PasteState::Unavailable {
            cx.simulate_input("chosen selection");
            cx.run_until_parked();
            view.read_with(cx, |probe, cx| {
                assert_eq!(probe.field.text(cx), "chosen selection");
            });
        }
    }
}

#[gpui::test]
fn recording_start_retires_old_menu_before_it_can_restore_focus_or_copy(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (view, cx) = cx.add_window_view(|window, cx| Probe {
        shell: cx.new(|cx| crate::shell::language_tests::shell_without_io(window, cx)),
        field: TextField::new("", |_probe, _text, _window, _cx| {}, window, cx),
        renders: Rc::default(),
    });
    view.update_in(cx, |probe, window, cx| {
        window.activate_window();
        probe.field.set_text("selected", window, cx);
        probe.field.focus(window, cx);
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
                gpui::point(px(20.0), px(40.0)),
                EditContext {
                    copy: CopyState::Available,
                    paste: PasteState::Available,
                },
                window,
                cx,
            )
        });
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        let before = view.read(cx).renders.get();
        view.update(cx, |probe, cx| {
            probe.shell.update(cx, |shell, cx| {
                shell.desktop_recording_started(window, cx);
                // The existing recorder blurs immediately after this hook.
                window.blur(cx);
            });
        });
        let at = gpui::point(px(30.0), px(55.0));
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
        assert_eq!(
            view.read(cx).renders.get(),
            before,
            "old frame dispatched before redraw"
        );
        assert!(
            window.focused(cx).is_none(),
            "retired menu cannot steal recorder focus"
        );
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap(),
            "protected"
        );
        assert!(!view.read(cx).shell.read(cx).areas.desktop.menu.is_open());
    });
}
