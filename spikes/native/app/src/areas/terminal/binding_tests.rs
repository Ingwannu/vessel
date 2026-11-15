//! A terminal may never recover missing navigation authority from a saved alias.
use super::*;
use gpui::TestAppContext;
use vessel_app_link::sessions::Binding;
use vessel_app_view::{machine_book::Name, sessions::SessionAddress};
use vessel_proto::NodeId;

fn shown(session: SessionId, binding: Option<Binding>) -> crate::areas::session::Open {
    crate::areas::session::Open {
        machine: "laptop".into(),
        session,
        title: "terminal".into(),
        agent: "fake".into(),
        structured: false,
        live: true,
        archived_at: None,
        binding,
    }
}

#[gpui::test]
fn terminal_without_retained_binding_is_refused_before_any_opening(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        assert!(!shell.terminal_follow(&shown(SessionId::random(), None), cx));
        assert!(shell.areas.terminal.shown.is_none());
        assert_eq!(shell.areas.terminal.phase, Phase::Idle);
        assert!(shell.notice.is_some());
    });
}

#[gpui::test]
fn terminal_binding_must_match_both_requested_host_and_session(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let session = SessionId::random();
        for binding in [
            Binding {
                host: Name::parse("other").unwrap(),
                address: SessionAddress {
                    node: NodeId::random(),
                    session,
                },
            },
            Binding {
                host: Name::parse("laptop").unwrap(),
                address: SessionAddress {
                    node: NodeId::random(),
                    session: SessionId::random(),
                },
            },
        ] {
            assert!(!shell.terminal_follow(&shown(session, Some(binding)), cx));
            assert!(shell.areas.terminal.shown.is_none());
        }
    });
}
