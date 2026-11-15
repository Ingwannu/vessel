//! A desktop owner consumes the real listener API without adding a key binding.
//! The stand-in effect deliberately performs no window or operating-system I/O.
use super::super::dispatch::{Callbacks, KeyboardHandler};
use gpui::TestAppContext;
use vessel_app_view::settings::Section;
use vessel_app_view::shortcuts::desktop::DesktopAction;
use vessel_app_view::shortcuts::dispatcher::{Handled, Priority};
use vessel_app_view::shortcuts::sequence_capture::{CaptureRole, SequenceCapture};
use vessel_app_view::shortcuts::{Action, Platform, binding_id, default_context};

#[gpui::test]
fn desktop_owner_uses_canonical_binding_and_recorder_keeps_the_key(cx: &mut TestAppContext) {
    let (shell, cx) = super::setup(cx);
    shell.update_in(cx, |shell, window, cx| {
        shell
            .register_shortcut_handler(KeyboardHandler {
                id: "desktop-receiving-fixture".parse().unwrap(),
                actions: vec![Action::NewWindow],
                priority: Priority::new(-100),
                callbacks: Callbacks {
                    enabled: |_, request, _, _| {
                        request.action.desktop_action() == Some(DesktopAction::NewWindow)
                    },
                    perform: |shell, request, _, _| {
                        assert_eq!(
                            request.action.desktop_action(),
                            Some(DesktopAction::NewWindow)
                        );
                        shell.notice = Some("desktop-owned-effect".to_owned());
                        Handled::Yes
                    },
                },
            })
            .unwrap();
        shell.areas.draft.composer.focus(window, cx);
    });
    cx.run_until_parked();
    let key = match Platform::native() {
        Platform::Mac => "cmd-shift-n",
        Platform::Other => "ctrl-shift-n",
    };
    cx.simulate_keystrokes(key);
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        assert_eq!(shell.notice.as_deref(), Some("desktop-owned-effect"));
        shell.notice = None;
        shell.open_settings(Some(Section::Shortcuts), window, cx);
        shell.areas.shortcuts.page.capture = Some(SequenceCapture::new(
            binding_id(Action::Palette, default_context(Platform::native())).unwrap(),
            CaptureRole::Replace,
        ));
        window.blur(cx);
    });
    cx.run_until_parked();
    cx.simulate_keystrokes(key);
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(
            shell.notice.is_none(),
            "recorder precedes the owner callback"
        );
        assert_eq!(
            shell
                .areas
                .shortcuts
                .page
                .capture
                .as_ref()
                .unwrap()
                .chords()
                .count(),
            1
        );
    });
}
