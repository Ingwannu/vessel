use super::*;
use crate::shell::language_tests;
use gpui::TestAppContext;
use vessel_app_view::creation::Ticket;
use vessel_proto::NodeId;
#[gpui::test]
fn pending_intake_and_recovered_unknown_input_block_creation_before_any_snapshot(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, cx| {
        shell.local_intake_started(DraftKey::New, cx);
        assert!(shell.draft_has_payload(cx));
        assert_eq!(
            shell.prepare_draft_input(
                Ticket::new(1),
                vessel_proto::timeline::ClientMessageId::random(),
                NodeId::random(),
                "fixture",
                "text"
            ),
            Err(vessel_app_view::composer::attach::PrepareProblem::Busy)
        );
        assert!(shell.areas.composer.starting.borrow().is_none());
        shell.local_intake_finished(DraftKey::New, cx);
        assert!(!shell.composer_attachments_blocked(DraftKey::New));
        let _new = shell.areas.composer.local.recovered.insert(DraftKey::New);
        assert!(shell.draft_has_payload(cx));
        assert_eq!(
            shell.prepare_draft_input(
                Ticket::new(2),
                vessel_proto::timeline::ClientMessageId::random(),
                NodeId::random(),
                "fixture",
                "text"
            ),
            Err(vessel_app_view::composer::attach::PrepareProblem::Busy)
        );
    });
}
