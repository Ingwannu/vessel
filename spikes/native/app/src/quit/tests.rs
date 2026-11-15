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

#[gpui::test]
fn queued_local_policy_changes_cannot_enter_after_finalization_begins(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, cx| {
        let before = shell.prefs.local_node;
        assert!(begin(cx));
        shell.manage_local_node(false, cx);
        assert_eq!(shell.prefs.local_node, before);
        finish(cx);
    });
}

#[gpui::test]
fn a_node_filter_reuses_global_phase_and_is_conservative_until_registered(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, cx| {
        let node = vessel_proto::NodeId::random();
        set_document_guard(|_shell| DocumentReadiness::Dirty, cx);
        assert_eq!(readiness_on(shell, node, cx), DocumentReadiness::Dirty);
        assert!(begin(cx));
        set_node_document_guard(|_shell, _node| DocumentReadiness::Ready, cx);
        assert_eq!(readiness_on(shell, node, cx), DocumentReadiness::Ready);
        assert_eq!(readiness(shell, cx), DocumentReadiness::Dirty);
        assert!(is_finalizing(cx));
        assert!(!admit_shell(shell, cx));
        finish(cx);
    });
}
