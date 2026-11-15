//! Actual shell controls and source callbacks, with no link or external I/O.
use crate::shell::{Shell, language_tests};
use gpui::{Entity, TestAppContext, VisualTestContext};
use vessel_app_view::settings::Section;
use vessel_app_view::shortcuts::Platform;

fn setup(cx: &mut TestAppContext) -> (Entity<Shell>, &mut VisualTestContext) {
    language_tests::init(cx);
    cx.update(|cx| cx.set_reduce_motion(true));
    cx.add_window_view(|window, cx| {
        let mut shell = Shell::without_io(window, cx);
        super::listen(&mut shell, window, cx);
        shell
    })
}

#[gpui::test]
fn help_search_and_escape_restore_the_existing_focus(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    let prior = shell.update_in(cx, |shell, window, cx| {
        shell.go(crate::routes::Route::Draft, window, cx);
        shell.areas.draft.composer.focus(window, cx);
        let prior = window.focused(cx).unwrap();
        shell.open_shortcuts_help(window, cx);
        prior
    });
    cx.run_until_parked();
    cx.simulate_input("command+k");
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(shell.areas.shortcuts.help.is_open());
        assert_eq!(shell.areas.shortcuts.help.query, "command+k");
    });
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, _| {
        assert!(!shell.areas.shortcuts.help.is_open());
        assert!(prior.is_focused(window));
    });
}

#[gpui::test]
fn settings_records_multiple_steps_and_navigation_cancels_capture(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update_in(cx, |shell, window, cx| {
        shell.open_settings(Some(Section::Shortcuts), window, cx)
    });
    cx.run_until_parked();
    let selector = match Platform::native() {
        Platform::Mac => "shortcut-menu-command-center-toggle-cmd-k-mac",
        Platform::Other => "shortcut-menu-command-center-toggle-ctrl-k-non-mac",
    };
    let menu = cx
        .debug_bounds(selector)
        .expect("actual canonical row menu");
    cx.simulate_click(menu.center(), gpui::Modifiers::default());
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(shell.areas.shortcuts.page.recording())
    });
    cx.simulate_keystrokes("ctrl-j s");
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        let capture = shell.areas.shortcuts.page.capture.as_ref().unwrap();
        assert_eq!(capture.chords().count(), 2);
        assert!(!shell.areas.palette.is_open());
    });
    cx.simulate_keystrokes("backspace");
    shell.read_with(cx, |shell, _| {
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
        )
    });
    shell.update_in(cx, |shell, window, cx| {
        shell.open_settings(Some(Section::General), window, cx)
    });
    shell.read_with(cx, |shell, _| {
        assert!(!shell.areas.shortcuts.page.recording())
    });
}
