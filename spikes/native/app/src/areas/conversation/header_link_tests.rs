//! A pause's reopen uses the same follow command as the first open.

use crate::areas::session::Open;
use crate::shell::language_tests;
use gpui::TestAppContext;
use vessel_app_link::Command;
use vessel_app_link::sessions::{Access, Binding};
use vessel_app_view::machine_book::Name;
use vessel_app_view::sessions::SessionAddress;
use vessel_app_view::{Machine, Reach};
use vessel_proto::{NodeId, SessionId};

fn machine(name: &str, online: bool) -> Machine {
    Machine {
        name: name.to_owned(),
        reach: if online {
            Reach::Online
        } else {
            Reach::Unreachable("down".to_owned())
        },
        sessions: Vec::new(),
        agents: Vec::new(),
        protocol: None,
        version: None,
        node: None,
        connection_epoch: None,
    }
}

fn open(session: SessionId, binding: Option<Binding>) -> Open {
    Open {
        machine: "fixture".to_owned(),
        session,
        title: String::new(),
        agent: String::new(),
        structured: true,
        live: true,
        archived_at: None,
        binding,
    }
}

#[gpui::test]
fn a_bound_reopen_follows_the_binding_and_an_unbound_one_opens(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(crate::shell::Shell::without_io);
    shell.update_in(cx, |_shell, window, cx| {
        let (mut shell, mut commands) = language_tests::with_commands(window, cx);
        let session = SessionId::random();
        let host = Name::parse("fixture").unwrap();
        let address = SessionAddress {
            node: NodeId::random(),
            session,
        };
        let binding = Binding {
            host: host.clone(),
            address,
        };
        shell.areas.conversation.open("fixture", session);
        shell.areas.conversation.lost = Some("offline".to_owned());
        shell.areas.session.open = Some(open(session, Some(binding.clone())));
        shell.fleet.machines = vec![machine("fixture", false)];
        shell.session_header_follows_link(cx);
        assert!(commands.try_recv().is_err());
        shell.fleet.machines = vec![machine("fixture", true)];
        shell.session_header_follows_link(cx);
        assert!(matches!(
            commands.try_recv().unwrap(),
            Command::SessionAccess(Access::Follow(got))
                if got.host.as_str() == host.as_str() && got.address == address
        ));
        assert!(commands.try_recv().is_err());
        assert!(shell.areas.conversation.lost.is_none());
        shell.areas.conversation.lost = Some("offline".to_owned());
        shell.areas.session.open = Some(open(session, None));
        shell.fleet.machines = vec![machine("fixture", false)];
        shell.session_header_follows_link(cx);
        assert!(commands.try_recv().is_err());
        shell.fleet.machines = vec![machine("fixture", true)];
        shell.session_header_follows_link(cx);
        assert!(matches!(
            commands.try_recv().unwrap(),
            Command::Open {
                machine,
                session: got,
            } if machine == "fixture" && got == session
        ));
    });
}
