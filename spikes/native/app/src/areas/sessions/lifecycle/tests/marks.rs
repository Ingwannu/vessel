//! The confirmation keeps one immutable directory intent across alias replacement.
use super::*;
use vessel_app_view::session_marks::{CapturedSessionTarget, Requests};
use vessel_proto::Mark;
#[gpui::test]
fn archive_confirmation_keeps_original_admission_after_replacement(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let (info, _, _, pending) = pending_exit(shell);
        let _finished = shell.areas.sessions.lifecycle.running.finish(pending);
        shell.areas.sessions.lifecycle.after = None;
        let mut original = shell.fleet.machines[0].clone();
        let context = vessel_app_view::connection::ConnectionEpochs::default()
            .admit(
                vessel_app_view::connection::MachineKey::from_book("fixture"),
                original.node.unwrap(),
            )
            .unwrap();
        original.connection_epoch = Some(context.epoch());
        assert_eq!(original.sessions[0].session, info.id);
        let target = CapturedSessionTarget::capture(&original, &original.sessions[0]).unwrap();
        shell.fleet.machines = vec![original];
        shell.route = Route::Sessions;
        shell.history_lifecycle(target.clone(), HistoryAction::Archive, window, cx);
        assert_eq!(
            shell
                .areas
                .sessions
                .lifecycle
                .asking
                .as_ref()
                .unwrap()
                .directory
                .as_ref(),
            Some(&target)
        );
        shell.fleet.machines[0].node = Some(NodeId::random());
        // Confirmation retains the original value. The real actor must refuse it;
        // an App node check is not substituted for selected-lease authority.
        let ticket = shell
            .areas
            .sessions
            .lifecycle
            .confirming
            .pending()
            .unwrap()
            .ticket;
        shell.confirm_session_lifecycle(ticket, window, cx);
        let mut comparison = Requests::default();
        let original = comparison.begin(target, Mark::Archive).unwrap();
        assert!(shell.areas.session_marks.finish(&original));
        assert!(shell.areas.sessions.lifecycle.running.pending().is_none());
        assert_eq!(shell.route, Route::Sessions);
    });
}
