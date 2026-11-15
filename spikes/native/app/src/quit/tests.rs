//! Generic admission preserves the editor's ownership of save/discard decisions.

use super::*;
use gpui::TestAppContext;

#[gpui::test]
fn all_unsettled_document_states_refuse_without_changing_preferences(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, cx| {
        let before = shell.prefs.onboarding.clone();
        let guards: [DocumentGuard; 3] = [
            |_shell| DocumentReadiness::Dirty,
            |_shell| DocumentReadiness::Pending,
            |_shell| DocumentReadiness::Failed,
        ];
        for guard in guards {
            set_document_guard(guard, cx);
            assert!(!admit_shell(shell, cx));
            assert!(shell.notice.is_some());
            assert_eq!(shell.prefs.onboarding, before);
        }
        set_document_guard(|_shell| DocumentReadiness::Ready, cx);
        assert!(admit_shell(shell, cx));
    });
}

#[gpui::test]
fn replacing_an_editor_callback_cannot_release_an_inflight_finalization(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |_shell, _window, cx| {
        assert!(begin(cx));
        assert!(is_finalizing(cx));
        assert!(
            !begin(cx),
            "a second quit must not create another finalizer"
        );
        set_document_guard(|_shell| DocumentReadiness::Dirty, cx);
        assert!(is_finalizing(cx), "new windows cannot reset an active quit");
        finish(cx);
        assert!(!is_finalizing(cx));
        assert!(begin(cx));
        finish(cx);
    });
}
