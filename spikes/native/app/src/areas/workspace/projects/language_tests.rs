use super::*;
use crate::shell::language_tests;

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
