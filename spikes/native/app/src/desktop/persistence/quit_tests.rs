//! The real desktop quit caller must consult documents after its Shell borrow ends.

use crate::desktop::Desktop;
use crate::quit::{self, DocumentReadiness};
use crate::shell::{Shell, language_tests};
use gpui::TestAppContext;
use vessel_app_link::desktop::preferences::Coordinator;

fn assert_document_notice(
    shell: &Shell,
    cx: &gpui::App,
    guard: fn(&Shell, &gpui::App) -> DocumentReadiness,
) {
    let words = shell.words.quit();
    let expected = match guard(shell, cx) {
        DocumentReadiness::Dirty => words.documents_dirty(),
        DocumentReadiness::Pending => words.documents_pending(),
        DocumentReadiness::Failed => words.documents_failed(),
        DocumentReadiness::Ready => panic!("fixture must refuse document admission"),
    };
    assert_eq!(shell.notice.as_deref(), Some(expected));
}

#[gpui::test]
async fn keep_quit_refusal_preserves_the_window_and_releases_the_shared_phase(
    cx: &mut TestAppContext,
) {
    // The production flush includes the real writer and its foreground watch.
    cx.executor().allow_parking();
    language_tests::init(cx);
    let coordinator = Coordinator::start(vessel_app_link::Prefs::default()).unwrap();
    cx.update(|cx| {
        cx.set_global(crate::desktop::testing::desktop(coordinator));
        crate::desktop::set_quit_handler(crate::local_node::request_keep_quit, cx);
    });
    let (shell, window) = cx.add_window_view(Shell::without_io);
    shell.update_in(window, crate::desktop::preferences::start);
    window.update(|window, cx| {
        crate::host_windows::register(&shell, window, cx);
    });
    let guards: [fn(&Shell, &gpui::App) -> DocumentReadiness; 3] = [
        |_shell, _cx| DocumentReadiness::Dirty,
        |_shell, _cx| DocumentReadiness::Pending,
        |_shell, _cx| DocumentReadiness::Failed,
    ];
    for guard in guards {
        shell.update_in(window, |shell, _window, cx| {
            shell.notice = None;
            quit::set_document_guard(guard, cx);
            // Use the actual menu/last-window dispatch while Shell is borrowed.
            // Synchronous document traversal here would reborrow and panic.
            crate::desktop::request_quit(cx);
            crate::desktop::request_quit(cx);
            assert!(quit::is_finalizing(cx));
        });
        window
            .cx
            .condition(&shell, |shell, cx| {
                shell.notice.is_some() && !quit::is_finalizing(cx)
            })
            .await;
        shell.read_with(window, |shell, cx| assert_document_notice(shell, cx, guard));
        window.update(|_window, cx| {
            assert_eq!(cx.windows().len(), 1);
            assert!(!quit::is_finalizing(cx));
        });
    }
}
