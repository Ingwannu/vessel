//! Unknown attachment cannot gain archive/hide authority from creator provenance.
use crate::{routes::Route, shell::Shell};
use gpui::TestAppContext;
use vessel_proto::{Creator, HarnessKind, NodeId, SessionId, SessionInfo, SessionMode, SessionSpec};

#[gpui::test]
fn immutable_creator_cannot_authorize_close_without_current_parentage(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    for creator in [Creator::Owner, Creator::Agent { node: NodeId::random(), session: SessionId::random() }] {
        shell.update_in(cx, |shell, window, cx| {
            let mut info = SessionInfo::new(SessionId::random(), SessionSpec::new(HarnessKind::ClaudeCode,
                SessionMode::Structured { model: None, resume: None, options: Vec::new() }), chrono::DateTime::UNIX_EPOCH);
            info.created_by = creator;
            let card = vessel_app_view::Card::from_session("fixture", &info);
            shell.route = Route::Session;
            shell.areas.session.open = Some(crate::areas::session::Open::of(&card));
            shell.areas.conversation.info = Some(Box::new(info));
            shell.request_session_close(window, cx);
            assert_eq!(shell.route, Route::Session);
            assert!(!shell.sessions_lifecycle_busy());
            assert_eq!(shell.areas.session.open.as_ref().unwrap().session, card.session);
            assert!(shell.notice.is_some());
        });
    }
}
