//! A deferred close belongs to the original Shell, including its checking flag.

use super::*;
use gpui::Entity;

fn replace(window: &mut Window, cx: &mut App) -> Entity<Shell> {
    let replacement = window.replace_root(cx, Shell::without_io);
    crate::host_windows::register(&replacement, window, cx);
    replacement
}

fn setup(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    cx.update(|cx| {
        cx.set_global(QuitAttempts(0));
        cx.set_global(AdmissionAttempts(0));
        cx.set_global(test_desktop());
        super::super::super::set_window_close_handler(refuse_documents, cx);
    });
}

#[gpui::test]
fn old_close_cannot_admit_or_clear_a_new_roots_coalesced_close(cx: &mut TestAppContext) {
    setup(cx);
    let (old, cx) = cx.add_window_view(registered_probe);
    let _extra = cx.update(|_, cx| {
        cx.open_window(gpui::WindowOptions::default(), |_, cx| cx.new(|_| Empty))
            .unwrap()
    });
    old.update_in(cx, |_old, window, cx| {
        request_close(window, cx); // A's retained deferred callback is first.
        let handle = window.window_handle();
        cx.defer(move |cx| {
            handle
                .update(cx, |_, window, cx| {
                    assert!(
                        cx.global::<Registry>().records[0].close.is_checking(),
                        "A cannot clear B's checking state"
                    );
                    request_close(window, cx); // Must coalesce with B already queued.
                })
                .unwrap();
        });
        let replacement = replace(window, cx);
        replacement.update(cx, |_shell, cx| request_close(window, cx));
    });
    cx.run_until_parked();
    old.read_with(cx, |_old, cx| {
        assert_eq!(cx.windows().len(), 2);
        assert_eq!(
            cx.global::<AdmissionAttempts>().0,
            1,
            "only B reaches its document guard"
        );
        assert_eq!(cx.global::<QuitAttempts>().0, 0);
    });
}

#[gpui::test]
fn stale_close_cannot_start_quit_after_replacement_becomes_the_last_window(
    cx: &mut TestAppContext,
) {
    setup(cx);
    let (old, cx) = cx.add_window_view(registered_probe);
    let extra = cx.update(|_, cx| {
        cx.open_window(gpui::WindowOptions::default(), |_, cx| cx.new(|_| Empty))
            .unwrap()
    });
    old.update_in(cx, |_old, window, cx| {
        request_close(window, cx);
        let _replacement = replace(window, cx);
        extra
            .update(cx, |_, window, _| window.remove_window())
            .unwrap();
    });
    cx.run_until_parked();
    old.read_with(cx, |_old, cx| {
        assert_eq!(cx.windows().len(), 1);
        assert_eq!(cx.global::<AdmissionAttempts>().0, 0);
        assert_eq!(
            cx.global::<QuitAttempts>().0,
            0,
            "A's old intent cannot quit B"
        );
    });
}

#[gpui::test]
fn a_new_registration_of_the_same_root_does_not_inherit_its_old_close(cx: &mut TestAppContext) {
    setup(cx);
    let (old, cx) = cx.add_window_view(registered_probe);
    let _extra = cx.update(|_, cx| {
        cx.open_window(gpui::WindowOptions::default(), |_, cx| cx.new(|_| Empty))
            .unwrap()
    });
    old.update_in(cx, |_old, window, cx| {
        request_close(window, cx);
        let original = cx.entity();
        let intermediate = cx.new(|cx| Shell::without_io(window, cx));
        // An actual registry A -> B -> A cycle while the same root remains
        // visible must not revive A's retained close operation.
        crate::host_windows::register(&intermediate, window, cx);
        crate::host_windows::register(&original, window, cx);
    });
    cx.run_until_parked();
    old.read_with(cx, |_old, cx| {
        assert_eq!(cx.global::<AdmissionAttempts>().0, 0);
        assert_eq!(cx.global::<QuitAttempts>().0, 0);
        assert_eq!(cx.windows().len(), 2);
    });
}
