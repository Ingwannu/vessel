//! Real window removal is deferred while the host's quit finalizer retains it.

use super::*;
use gpui::{Global, Render, TestAppContext};
use vessel_app_kit::ui::{IntoElement, div};

struct Empty;
impl Render for Empty {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        div()
    }
}

struct QuitAttempts(usize);
impl Global for QuitAttempts {}

fn retain_while_finalizing(cx: &mut App) {
    let attempts = &mut cx.global_mut::<QuitAttempts>().0;
    *attempts = attempts.saturating_add(1);
}

fn test_desktop() -> Desktop {
    let mut desktop =
        super::super::testing::desktop(
            vessel_app_link::desktop::preferences::Coordinator::start(
                vessel_app_link::Prefs::default(),
            )
            .unwrap(),
        );
    desktop.quit = Some(retain_while_finalizing);
    desktop
}

#[cfg(not(target_os = "macos"))]
#[gpui::test]
fn last_window_stays_open_until_quit_finishes_but_additional_windows_close(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    cx.update(|cx| {
        cx.set_global(QuitAttempts(0));
        cx.set_global(test_desktop());
        vessel_app_kit::chrome::set_close_handler(request_close, cx);
    });
    let (first, cx) = cx.add_window_view(|_window, _cx| Empty);
    cx.run_until_parked();
    cx.update(vessel_app_kit::chrome::request_close);
    cx.run_until_parked();
    first.read_with(cx, |_view, cx| {
        assert_eq!(
            cx.windows().len(),
            1,
            "a pending/refused finalizer keeps its last window"
        );
        assert_eq!(cx.global::<QuitAttempts>().0, 1);
    });
    let extra = cx.update(|_window, cx| {
        cx.open_window(gpui::WindowOptions::default(), |_window, cx| {
            cx.new(|_cx| Empty)
        })
        .unwrap()
    });
    cx.run_until_parked();
    cx.update(|_window, cx| {
        extra
            .update(cx, |_view, window, cx| {
                vessel_app_kit::chrome::request_close(window, cx);
            })
            .unwrap();
    });
    cx.run_until_parked();
    first.read_with(cx, |_view, cx| {
        assert_eq!(cx.windows().len(), 1);
        assert_eq!(
            cx.global::<QuitAttempts>().0,
            1,
            "closing an additional window does not quit the app"
        );
    });
}

struct AdmissionAttempts(usize);
impl Global for AdmissionAttempts {}
fn refuse_documents(_shell: &mut Shell, cx: &mut Context<'_, Shell>) -> bool {
    let attempts = &mut cx.global_mut::<AdmissionAttempts>().0;
    *attempts = attempts.saturating_add(1);
    false
}
fn allow_documents(_shell: &mut Shell, cx: &mut Context<'_, Shell>) -> bool {
    let attempts = &mut cx.global_mut::<AdmissionAttempts>().0;
    *attempts = attempts.saturating_add(1);
    true
}

#[gpui::test]
fn window_document_admission_defers_entity_reborrow_coalesces_and_keeps_refused_window(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    cx.update(|cx| {
        cx.set_global(QuitAttempts(0));
        cx.set_global(AdmissionAttempts(0));
        cx.set_global(test_desktop());
        super::super::set_window_close_handler(refuse_documents, cx);
    });
    let (view, cx) = cx.add_window_view(registered_probe);
    let _extra = cx.update(|_window, cx| {
        cx.open_window(gpui::WindowOptions::default(), |_window, cx| {
            cx.new(|_| Empty)
        })
        .unwrap()
    });
    cx.run_until_parked();
    view.update_in(cx, |_shell, window, cx| {
        request_close(window, cx);
        request_close(window, cx);
    });
    cx.run_until_parked();
    view.read_with(cx, |_probe, cx| {
        assert_eq!(cx.windows().len(), 2);
        assert_eq!(cx.global::<AdmissionAttempts>().0, 1);
        assert_eq!(cx.global::<QuitAttempts>().0, 0);
    });
    cx.update(|_window, cx| super::super::set_window_close_handler(allow_documents, cx));
    view.update_in(cx, |_shell, window, cx| request_close(window, cx));
    cx.run_until_parked();
    view.read_with(cx, |_probe, cx| {
        assert_eq!(cx.windows().len(), 1);
        assert_eq!(cx.global::<AdmissionAttempts>().0, 2);
        assert_eq!(cx.global::<QuitAttempts>().0, 0);
    });
}

#[gpui::test]
fn two_window_close_requests_recheck_the_last_window_before_removal(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    cx.update(|cx| {
        cx.set_global(QuitAttempts(0));
        cx.set_global(AdmissionAttempts(0));
        cx.set_global(test_desktop());
        super::super::set_window_close_handler(allow_documents, cx);
    });
    let (first, cx) = cx.add_window_view(registered_probe);
    let second = cx.update(|_window, cx| {
        cx.open_window(gpui::WindowOptions::default(), |window, cx| {
            cx.new(|cx| registered_probe(window, cx))
        })
        .unwrap()
    });
    cx.run_until_parked();
    first.update_in(cx, |_shell, window, cx| {
        request_close(window, cx);
        second
            .update(cx, |_view, window, cx| request_close(window, cx))
            .unwrap();
    });
    cx.run_until_parked();
    first.read_with(cx, |_probe, cx| {
        assert_eq!(
            cx.windows().len(),
            1,
            "the finalizer retains the last window"
        );
        assert_eq!(cx.global::<AdmissionAttempts>().0, 1);
        assert_eq!(cx.global::<QuitAttempts>().0, 1);
    });
}

fn registered_probe(window: &mut Window, cx: &mut Context<'_, Shell>) -> Shell {
    let shell = Shell::without_io(window, cx);
    crate::host_windows::register(&cx.entity(), window, cx);
    shell
}

#[gpui::test]
fn production_zoom_runs_after_shell_borrow_and_preserves_dpi_bounds_and_saved_state(
    cx: &mut TestAppContext,
) {
    use vessel_app_view::{
        desktop::zoom::{ZoomChange, ZoomLevel},
        shortcuts::desktop::DesktopAction,
    };
    crate::shell::language_tests::init(cx);
    cx.update(|cx| cx.set_global(test_desktop()));
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    let original = cx.update(|window, _cx| {
        (
            window.window_bounds(),
            window.viewport_size(),
            window.scale_factor(),
        )
    });
    let mut zoom = ZoomLevel::default();
    zoom.adjust(ZoomChange::In);
    shell.update_in(cx, |shell, window, cx| {
        watch_window(shell, window, cx);
        assert!(shell.run_desktop_shortcut(DesktopAction::ZoomIn, window, cx));
        assert_eq!(
            window.zoom().to_bits(),
            1.0_f32.to_bits(),
            "effect waits for this Shell borrow"
        );
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        assert_eq!(window.zoom().to_bits(), zoom.factor().to_bits());
        assert_eq!(window.window_bounds(), original.0);
        assert!(window.viewport_size().width < original.1.width);
        assert_eq!(
            window.scale_factor().to_bits(),
            original.2.mul_add(zoom.factor(), 0.0).to_bits()
        );
        assert_eq!(cx.global::<Desktop>().saved.unwrap().zoom(), zoom);
    });
    shell.update_in(cx, |shell, window, cx| {
        assert!(shell.run_desktop_shortcut(DesktopAction::ActualSize, window, cx));
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        assert_eq!(window.zoom().to_bits(), 1.0_f32.to_bits());
        assert_eq!(window.scale_factor().to_bits(), original.2.to_bits());
        assert_eq!(window.window_bounds(), original.0);
        assert_eq!(window.viewport_size(), original.1);
        assert_eq!(
            cx.global::<Desktop>().saved.unwrap().zoom(),
            ZoomLevel::default()
        );
    });
}

#[gpui::test]
fn queued_half_steps_reach_both_native_zoom_limits_without_changing_platform_bounds(
    cx: &mut TestAppContext,
) {
    use vessel_app_view::shortcuts::desktop::DesktopAction;
    crate::shell::language_tests::init(cx);
    cx.update(|cx| cx.set_global(test_desktop()));
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, watch_window);
    let bounds = cx.update(|window, _cx| window.window_bounds());
    for (action, limit) in [
        (DesktopAction::ZoomOut, Window::MIN_ZOOM),
        (DesktopAction::ZoomIn, Window::MAX_ZOOM),
    ] {
        shell.update_in(cx, |shell, window, cx| {
            for _ in 0..100 {
                assert!(shell.run_desktop_shortcut(action, window, cx));
            }
        });
        cx.run_until_parked();
        cx.update(|window, _cx| {
            assert_eq!(window.zoom().to_bits(), limit.to_bits());
            assert_eq!(window.window_bounds(), bounds);
        });
    }
}

#[gpui::test]
fn extra_window_zoom_is_independent_and_replaced_root_refuses_a_queued_zoom(
    cx: &mut TestAppContext,
) {
    use vessel_app_view::shortcuts::desktop::DesktopAction;
    crate::shell::language_tests::init(cx);
    cx.update(|cx| cx.set_global(test_desktop()));
    let (primary, cx) = cx.add_window_view(Shell::without_io);
    primary.update_in(cx, |shell, window, cx| {
        watch_window(shell, window, cx);
        assert!(shell.run_desktop_shortcut(DesktopAction::ZoomIn, window, cx));
    });
    cx.run_until_parked();
    let saved = cx.update(|_, cx| cx.global::<Desktop>().saved.unwrap());
    let extra = cx.update(|_, cx| {
        cx.open_window(gpui::WindowOptions::default(), |window, cx| {
            cx.new(|cx| Shell::without_io(window, cx))
        })
        .unwrap()
    });
    let retained = cx.update(|_, cx| extra.entity(cx).unwrap());
    cx.update(|_, cx| {
        extra
            .update(cx, |shell, window, cx| {
                assert!(shell.run_desktop_shortcut(DesktopAction::ZoomOut, window, cx));
            })
            .unwrap();
    });
    cx.run_until_parked();
    let before = cx.update(|_, cx| {
        extra
            .update(cx, |shell, window, _cx| {
                (window.zoom().to_bits(), shell.areas.desktop.zoom)
            })
            .unwrap()
    });
    cx.update(|_, cx| {
        extra
            .update(cx, |shell, window, cx| {
                assert!(shell.run_desktop_shortcut(DesktopAction::ZoomIn, window, cx));
                let _replacement = window.replace_root(cx, |_, _| Empty);
            })
            .unwrap();
    });
    cx.run_until_parked();
    let handle: gpui::AnyWindowHandle = extra.into();
    cx.update(|_, cx| {
        handle
            .update(cx, |_, window, _| {
                assert_eq!(window.zoom().to_bits(), before.0);
            })
            .unwrap();
        assert_eq!(retained.read(cx).areas.desktop.zoom, before.1);
        assert_eq!(cx.global::<Desktop>().saved, Some(saved));
    });
}

mod root_aba;
