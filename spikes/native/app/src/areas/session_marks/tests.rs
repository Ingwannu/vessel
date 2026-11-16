//! A directory refusal cannot clear a resident draft or complete panel lifecycle.
use crate::{routes::Route, shell::Shell};
use gpui::TestAppContext;
use vessel_app_link::session_marks::{Answer, Failure};
use vessel_app_view::{
    Card, Machine, Reach, connection::ConnectionEpochs, machine_book::Name,
    session_marks::CapturedSessionTarget,
};
use vessel_proto::{HarnessKind, Mark, NodeId, SessionId, SessionInfo, SessionMode, SessionSpec};
fn machine() -> Machine {
    let info = SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![],
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    );
    let context = ConnectionEpochs::default()
        .admit(Name::parse("fixture").unwrap(), NodeId::random())
        .unwrap();
    Machine {
        name: "fixture".to_owned(),
        node: Some(context.node()),
        connection_epoch: Some(context.epoch()),
        reach: Reach::Online,
        sessions: vec![Card::from_session("fixture", &info)],
        agents: vec![],
        protocol: None,
        version: None,
    }
}
#[gpui::test]
fn directory_refusal_preserves_actual_input_route_and_successor_pending(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let machine = machine();
        let card = &machine.sessions[0];
        let target = CapturedSessionTarget::capture(&machine, card).unwrap();
        shell.route = Route::Session;
        shell.areas.session.open = Some(crate::areas::session::Open::of(card));
        shell
            .areas
            .session
            .composer
            .set_text("keep composing 한글", window, cx);
        shell.fleet.machines = vec![machine];
        shell.mark_session_target(target.clone(), Mark::Archive, cx);
        let first = shell.areas.session_marks.pending().unwrap().clone();
        shell.session_mark_answer(Answer::refused(first.clone(), Failure::Unsent), cx);
        assert!(!shell.areas.session_marks.is_pending());
        let second = shell
            .areas
            .session_marks
            .begin(target, Mark::Unpin)
            .unwrap();
        shell.session_mark_answer(Answer::refused(first, Failure::Uncertain), cx);
        assert!(shell.areas.session_marks.is_pending());
        shell.session_mark_answer(Answer::refused(second, Failure::Changed), cx);
        assert!(!shell.areas.session_marks.is_pending());
        assert_eq!(shell.route, Route::Session);
        assert_eq!(shell.areas.session.composer.text(cx), "keep composing 한글");
        assert!(
            shell.fleet.machines[0].sessions[0]
                .detail
                .marks
                .archived_at
                .is_none()
        );
    });
}
