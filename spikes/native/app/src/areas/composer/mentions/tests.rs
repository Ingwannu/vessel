use super::*;
use crate::{areas::session::Open, routes::Route, shell::language_tests};
use gpui::TestAppContext;
use vessel_proto::{ProjectId, SessionId};

pub(super) fn menu_fixture(
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> (DraftKey, RootAsk) {
    let session = SessionId::random();
    let target = DraftKey::Session { session };
    let machine = Name::parse("fixture").unwrap();
    let root = WorkspaceRoot::Project {
        project: ProjectId::random(),
    };
    shell.route = Route::Session;
    shell.areas.session.open = Some(Open {
        machine: "fixture".into(),
        session,
        title: "Fixture".into(),
        agent: "Fixture".into(),
        structured: true,
        live: true,
    });
    let _tab = shell
        .layout_open_target(
            &vessel_app_view::layout::LayoutKey::Session {
                machine: vessel_app_view::layout::HostAlias::parse("fixture").unwrap(),
                session,
            },
            vessel_app_view::layout::Target::Agent(session),
            vessel_app_view::layout::OpenIntent::New,
            vessel_app_view::layout::Placement::Focused,
            window,
            cx,
        )
        .unwrap();
    let connection =
        crate::areas::composer::prepared::tests::connection(vessel_proto::NodeId::random());
    shell.fleet.machines.push(vessel_app_view::Machine {
        name: "fixture".to_owned(),
        node: Some(connection.node()),
        connection_epoch: Some(connection.epoch()),
        reach: vessel_app_view::Reach::Online,
        sessions: Vec::new(),
        agents: Vec::new(),
        protocol: None,
        version: None,
    });
    let ask = RootAsk {
        connection: connection.clone(),
        ticket: mentions::Ticket::default(),
        machine: machine.clone(),
        session,
    };
    *shell.areas.composer.mention_root.borrow_mut() = Root::Ready(ask.clone(), Ok(Some(root)));
    shell
        .areas
        .session
        .composer
        .replace_range(0..0, "@src", window, cx);
    let scope = Scope {
        connection,
        draft: target,
        machine,
        root,
    };
    let request = shell
        .file_menu(target)
        .borrow_mut()
        .model
        .observe(Some(scope), "@src", 4, Duration::ZERO)
        .unwrap()
        .unwrap();
    assert!(
        shell
            .file_menu(target)
            .borrow_mut()
            .model
            .begin(request.ticket)
            .is_some()
    );
    assert!(shell.file_menu(target).borrow_mut().model.arrived(
        request.ticket,
        Ok(vec![mentions::Suggestion {
            path: "src/main.rs".parse().unwrap(),
            kind: mentions::SuggestionKind::File
        }]),
        Duration::ZERO
    ));
    (target, ask)
}

#[gpui::test]
fn shortcut_admission_reads_exact_token_scope_without_mutating_the_menu(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let (target, ask) = menu_fixture(shell, window, cx);
        let phase = shell.file_menu(target).borrow().model.phase().clone();
        let chosen = shell.file_menu(target).borrow().model.selected();
        assert!(shell.composer_file_menu_owns_key(target, ComposerKey::Tab, cx));
        assert!(shell.composer_file_menu_owns_key(target, ComposerKey::Escape, cx));
        assert_eq!(shell.file_menu(target).borrow().model.phase(), &phase);
        assert_eq!(shell.file_menu(target).borrow().model.selected(), chosen);
        assert_eq!(shell.areas.session.composer.text(cx), "@src");
        *shell.areas.composer.mention_root.borrow_mut() = Root::Ready(
            ask.clone(),
            Ok(Some(WorkspaceRoot::Project {
                project: ProjectId::random(),
            })),
        );
        assert!(!shell.composer_file_menu_owns_key(target, ComposerKey::Tab, cx));
        assert_eq!(shell.file_menu(target).borrow().model.phase(), &phase);
        shell.file_menu(target).borrow_mut().model.dismiss();
        *shell.areas.composer.mention_root.borrow_mut() = Root::Reading(ask);
        assert!(shell.composer_file_menu_owns_key(target, ComposerKey::Escape, cx));
        assert!(!shell.composer_file_menu_owns_key(target, ComposerKey::Tab, cx));
        assert_eq!(
            shell.file_menu(target).borrow().model.phase(),
            &mentions::Phase::Hidden
        );
        let mut admissions = vessel_app_view::connection::ConnectionEpochs::default();
        let machine = vessel_app_view::connection::MachineKey::from_book("fixture");
        let node = shell.fleet.machines[0].node.unwrap();
        let _original = admissions.admit(machine.clone(), node).unwrap();
        let reconnected = admissions.admit(machine, node).unwrap();
        shell.fleet.machines[0].connection_epoch = Some(reconnected.epoch());
        assert!(!shell.composer_file_menu_owns_key(target, ComposerKey::Escape, cx));
        assert!(!shell.composer_file_menu_owns_key(target, ComposerKey::Tab, cx));
    });
}
