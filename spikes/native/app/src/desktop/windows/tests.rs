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

#[cfg(not(target_os = "macos"))]
#[gpui::test]
fn last_window_stays_open_until_quit_finishes_but_additional_windows_close(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    cx.update(|cx| {
        cx.set_global(QuitAttempts(0));
        cx.set_global(Desktop {
            records: Vec::new(),
            saved: None,
            latest: WindowState::default(),
            writer: None,
            environment: vessel_app_link::desktop::environment::ShellEnvironment::inherited(),
            quit: Some(retain_while_finalizing),
            session: None,
            project: None,
            preferences: vessel_app_link::desktop::preferences::Coordinator::start(
                vessel_app_link::Prefs::default(),
            )
            .unwrap(),
        });
        vessel_app_kit::chrome::set_close_handler(request_close, cx);
    });
    let (first, cx) = cx.add_window_view(|_window, _cx| Empty);
    cx.run_until_parked();
    cx.update(|window, cx| vessel_app_kit::chrome::request_close(window, cx));
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
                vessel_app_kit::chrome::request_close(window, cx)
            })
            .unwrap()
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
