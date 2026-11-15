use super::*;
use crate::shell::language_tests;

#[gpui::test]
fn a_fixed_project_target_ignores_settings_fallback_and_refuses_alias_rebinding(
    cx: &mut gpui::TestAppContext,
) {
    use vessel_app_view::workspace::projects::navigation::{NavigationRefusal, ProjectTarget};
    use vessel_app_view::{Machine, Reach, machine_book::Name};
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, window, cx| {
            let mut remote = Machine::unreachable("remote-first".to_owned(), "fixture");
            remote.reach = Reach::Online;
            remote.node = Some(vessel_proto::NodeId::random());
            let node = vessel_proto::NodeId::random();
            let mut chosen = Machine::unreachable("fixed-target".to_owned(), "fixture");
            chosen.reach = Reach::Online;
            chosen.node = Some(node);
            let mut epochs = vessel_app_view::connection::ConnectionEpochs::default();
            let context = epochs
                .admit(
                    vessel_app_view::connection::MachineKey::from_book("fixed-target"),
                    node,
                )
                .unwrap();
            chosen.connection_epoch = Some(context.epoch());
            shell.fleet.machines = vec![remote, chosen];
            assert_eq!(shell.projects_machine().as_deref(), Some("remote-first"));
            let target = ProjectTarget::new(Name::parse("fixed-target").unwrap(), node);
            shell
                .open_add_for_connection(AddForm::Folder, &context, window, cx)
                .unwrap();
            assert_eq!(
                shell.areas.workspace.page.form_host.as_ref(),
                Some(target.host())
            );
            assert_eq!(shell.areas.workspace.page.form_node, Some(node));
            shell.fleet.machines[1].node = Some(vessel_proto::NodeId::random());
            assert_eq!(
                shell.open_add_for_target(AddForm::Clone, &target, window, cx),
                Err(NavigationRefusal::HostUnavailable)
            );
            assert_eq!(shell.areas.workspace.page.adding, Some(AddForm::Folder));
            assert_eq!(shell.areas.workspace.page.form_node, Some(node));
        },
        |shell| &shell.areas.workspace.page.path,
        |shell| {
            assert_eq!(shell.areas.workspace.page.adding, Some(AddForm::Folder));
        },
    );
}

#[gpui::test]
fn a_language_change_keeps_both_form_values_carets_selections_and_focus(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    let targets: [fn(&Shell) -> &TextField; 2] = [
        |shell| &shell.areas.workspace.page.path,
        |shell| &shell.areas.workspace.page.url,
    ];
    for target in targets {
        language_tests::preserves(
            cx,
            |shell, window, cx| {
                shell.open_add(AddForm::Clone, window, cx);
            },
            target,
            |shell| {
                assert_eq!(shell.areas.workspace.page.adding, Some(AddForm::Clone));
                assert!(shell.areas.workspace.page.actions.pending().is_none());
            },
        );
    }
}

#[gpui::test]
fn a_cancelled_reply_cannot_hydrate_a_rebound_host_or_settle_the_new_form(
    cx: &mut gpui::TestAppContext,
) {
    use vessel_app_view::machine_book::Name;
    use vessel_app_view::workspace::projects::actions::ProjectChanged;
    use vessel_proto::workspace::{GitSupport, ProjectList, ProjectState};
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, window, cx| {
            shell.open_add(AddForm::Clone, window, cx);
            let page = &mut shell.areas.workspace.page;
            let old = page
                .actions
                .begin(
                    Name::parse("laptop").unwrap(),
                    vessel_proto::NodeId::random(),
                )
                .unwrap();
            shell.project_host_detached("laptop");
            shell.open_add(AddForm::Clone, window, cx);
            let page = &mut shell.areas.workspace.page;
            let new = page
                .actions
                .begin(
                    Name::parse("laptop").unwrap(),
                    vessel_proto::NodeId::random(),
                )
                .unwrap();
            let _previous = shell.areas.workspace.projects.insert(
                "laptop".to_owned(),
                ProjectsView::Ready(ProjectList {
                    git: GitSupport::Missing,
                    projects: Vec::new(),
                    more: false,
                }),
            );
            let project = ProjectInfo {
                id: ProjectId::random(),
                name: "Previous node".parse().unwrap(),
                root: "/old-node/project".into(),
                git: false,
                default_branch: None,
                state: ProjectState::Ready,
                created_at: "1970-01-01T00:00:00Z".parse().unwrap(),
                revision: 0,
            };
            shell.project_changed("laptop", &old, Ok(ProjectChanged::Registered(project)));
            assert_eq!(shell.areas.workspace.page.actions.pending(), Some(&new));
            assert!(
                shell.areas.workspace.projects["laptop"]
                    .list()
                    .unwrap()
                    .projects
                    .is_empty()
            );
        },
        |shell| &shell.areas.workspace.page.path,
        |shell| {
            assert_eq!(shell.areas.workspace.page.adding, Some(AddForm::Clone));
            assert!(shell.areas.workspace.page.actions.pending().is_some());
        },
    );
}
