//! Generic admission preserves the editor's ownership of save/discard decisions.

use super::*;
use gpui::TestAppContext;

#[gpui::test]
fn all_unsettled_document_states_refuse_without_changing_preferences(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, cx| {
        let before = shell.prefs.onboarding.clone();
        for guard in [
            (|_: &Shell| DocumentReadiness::Dirty) as DocumentGuard,
            (|_: &Shell| DocumentReadiness::Pending) as DocumentGuard,
            (|_: &Shell| DocumentReadiness::Failed) as DocumentGuard,
        ] {
            set_document_guard(guard, cx);
            assert!(!admit_shell(shell, cx));
            assert!(shell.notice.is_some());
            assert_eq!(shell.prefs.onboarding, before);
        }
        set_document_guard(|_shell| DocumentReadiness::Ready, cx);
        assert!(admit_shell(shell, cx));
    });
}
