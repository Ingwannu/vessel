//! Real key and owner dispatch share book-read UI admission, not node readiness.
use super::*;
use crate::routes::Route;
use vessel_app_view::shortcuts::{Action, default_context, engine::KeyRepeat};

#[gpui::test]
fn global_keys_require_a_read_nonempty_book_except_in_settings(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    for (loaded, configured, route, enabled) in [
        (false, false, Route::Draft, false),
        (false, true, Route::Draft, false),
        (true, false, Route::Draft, false),
        (true, true, Route::Draft, true),
        (false, false, Route::Settings, true),
        (true, false, Route::Settings, true),
        (true, false, Route::Pair, false),
    ] {
        let before = view.update_in(cx, |probe, _window, cx| {
            probe.shell.update(cx, |shell, _cx| {
                configured_book(shell);
                if !configured {
                    shell.fleet.machines.clear();
                }
                shell.loaded = loaded;
                shell.route = route;
                assert_eq!(shell.global_shortcuts_enabled(), enabled);
                shell.prefs.settings.appearance.theme
            })
        });
        cx.simulate_keystrokes(chord());
        cx.run_until_parked();
        view.read_with(cx, |probe, cx| {
            assert_eq!(
                probe.shell.read(cx).prefs.settings.appearance.theme,
                if enabled { before.next() } else { before },
                "loaded={loaded}, configured={configured}, route={route:?}"
            );
        });
    }
}

#[gpui::test]
fn direct_owner_dispatch_cannot_bypass_global_book_admission(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    view.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell.fleet.machines.clear();
            let request = crate::areas::shortcuts::dispatch::ShortcutRequest {
                action: Action::CycleTheme,
                scope: Action::CycleTheme.scope(),
                index: None,
                context: default_context(Platform::native()),
                repeat: KeyRepeat::Initial,
            };
            let before = shell.prefs.settings.appearance.theme;
            assert!(!shell.shortcut_handler_available(&request, window, cx));
            assert!(!shell.dispatch_shortcut_to_owners(request, window, cx));
            assert_eq!(shell.prefs.settings.appearance.theme, before);
            configured_book(shell); // Offline, no NodeId, protocol or lease.
            assert!(shell.shortcut_handler_available(&request, window, cx));
            assert!(shell.dispatch_shortcut_to_owners(request, window, cx));
            assert_eq!(shell.prefs.settings.appearance.theme, before.next());
        });
    });
}

#[gpui::test]
fn losing_the_book_retires_a_chord_before_global_input_returns(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    view.update_in(cx, |probe, _window, cx| {
        probe.shell.update(cx, |shell, _cx| {
            drop(shell.prefs.settings.shortcuts.insert(
                "go-board".parse().unwrap(),
                vessel_app_view::prefs::Bindings::try_from(vec!["G S".parse().unwrap()]).unwrap(),
            ));
        });
    });
    cx.simulate_keystrokes("g");
    cx.run_until_parked();
    view.update_in(cx, |probe, _window, cx| {
        probe.shell.update(cx, |shell, _cx| {
            assert!(shell.areas.shortcuts.engine.is_pending());
            shell.fleet.machines.clear();
            crate::areas::shortcuts::book_changed(shell);
            assert!(!shell.areas.shortcuts.engine.is_pending());
            configured_book(shell);
        });
    });
    cx.simulate_keystrokes("s");
    cx.run_until_parked();
    view.read_with(cx, |probe, cx| {
        assert_eq!(probe.shell.read(cx).route, Route::Draft);
    });
}
