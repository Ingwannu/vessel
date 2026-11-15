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

struct GuardProbe {
    shell: gpui::Entity<Shell>,
}
impl Render for GuardProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        div()
    }
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
    let (view, cx) = cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| crate::shell::language_tests::shell_without_io(window, cx));
        cx.global_mut::<Desktop>().records.push(WindowRecord {
            window: window.window_handle(),
            shell: shell.downgrade(),
            close: WindowClose::Idle,
        });
        GuardProbe { shell }
    });
    let _extra = cx.update(|_window, cx| {
        cx.open_window(gpui::WindowOptions::default(), |_window, cx| {
            cx.new(|_| Empty)
        })
        .unwrap()
    });
    cx.run_until_parked();
    view.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |_shell, cx| {
            request_close(window, cx);
            request_close(window, cx);
        });
    });
    cx.run_until_parked();
    view.read_with(cx, |_probe, cx| {
        assert_eq!(cx.windows().len(), 2);
        assert_eq!(cx.global::<AdmissionAttempts>().0, 1);
        assert_eq!(cx.global::<QuitAttempts>().0, 0);
    });
    cx.update(|_window, cx| super::super::set_window_close_handler(allow_documents, cx));
    view.update_in(cx, |probe, window, cx| {
        probe
            .shell
            .update(cx, |_shell, cx| request_close(window, cx));
    });
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
    first.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |_shell, cx| {
            request_close(window, cx);
            second
                .update(cx, |_view, window, cx| request_close(window, cx))
                .unwrap();
        });
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

fn registered_probe(window: &mut Window, cx: &mut Context<'_, GuardProbe>) -> GuardProbe {
    let shell = cx.new(|cx| crate::shell::language_tests::shell_without_io(window, cx));
    cx.global_mut::<Desktop>().records.push(WindowRecord {
        window: window.window_handle(),
        shell: shell.downgrade(),
        close: WindowClose::Idle,
    });
    GuardProbe { shell }
}
