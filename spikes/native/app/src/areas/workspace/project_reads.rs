//! Host-bound project hydration; forms do not own list freshness or peer identity.

use crate::shell::Shell;
use vessel_app_view::machine_book::Name;
use vessel_app_view::workspace::{
    problem::Problem,
    projects::{ProjectsView, navigation::ProjectTarget, reads::ProjectReadKey},
};
use vessel_proto::{NodeId, workspace::ProjectList};

impl Shell {
    pub(super) fn read_projects(&mut self, host: &str) {
        let (Some(context), Ok(name)) = (self.project_connection(host), Name::parse(host)) else {
            return;
        };
        let node = context.node();
        let workspace = &mut self.areas.workspace;
        if workspace.project_reads.node(&name) != Some(node) {
            let _old = workspace
                .projects
                .insert(host.to_owned(), ProjectsView::Loading);
        }
        let Some(key) = workspace
            .project_reads
            .begin_on(ProjectTarget::new(name, node), context.clone())
        else {
            let _old = workspace
                .projects
                .insert(host.to_owned(), ProjectsView::Failed(Problem::Unsent));
            return;
        };
        self.command(vessel_app_link::Command::Projects(
            vessel_app_link::workspace::project_requests::Request {
                context,
                operation: vessel_app_link::workspace::project_requests::Operation::List(key),
            },
        ));
    }

    pub(super) fn projects_read(
        &mut self,
        host: &str,
        key: &ProjectReadKey,
        result: Result<ProjectList, Problem>,
    ) {
        if host != key.target().host().as_str()
            || !self.project_cache_current(key.target().host())
            || self.project_node(host) != Some(key.target().node())
            || !self.areas.workspace.project_reads.finish(key)
        {
            return;
        }
        let workspace = &mut self.areas.workspace;
        workspace
            .projects
            .entry(host.to_owned())
            .or_default()
            .listed_after(result);
    }

    pub(super) fn project_followed(
        &mut self,
        host: &str,
        node: NodeId,
        project: vessel_proto::workspace::ProjectInfo,
    ) {
        let Ok(name) = Name::parse(host) else {
            return;
        };
        if self.project_cache_current(&name)
            && self.project_node(host) == Some(node)
            && self.areas.workspace.project_reads.node(&name) == Some(node)
            && let Some(view) = self.areas.workspace.projects.get_mut(host)
        {
            view.replace(project);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::language_tests;
    use vessel_app_view::{Machine, Reach};
    use vessel_proto::{
        ProjectId,
        workspace::{GitSupport, ProjectInfo, ProjectState},
    };

    #[gpui::test]
    fn delayed_list_and_clone_progress_cannot_hydrate_a_rebound_alias(
        cx: &mut gpui::TestAppContext,
    ) {
        language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update_in(cx, |shell, window, cx| {
            let host = Name::parse("laptop").unwrap();
            let old = NodeId::random();
            let node = NodeId::random();
            let key = shell
                .areas
                .workspace
                .project_reads
                .begin(ProjectTarget::new(host.clone(), old))
                .unwrap();
            let mut machine = Machine::unreachable(host.as_str().to_owned(), "fixture");
            machine.reach = Reach::Online;
            machine.node = Some(node);
            let mut epochs = vessel_app_view::connection::ConnectionEpochs::default();
            let context = epochs
                .admit(
                    vessel_app_view::connection::MachineKey::from_book(host.as_str()),
                    node,
                )
                .unwrap();
            machine.connection_epoch = Some(context.epoch());
            shell.fleet.machines = vec![machine];
            let project = ProjectInfo {
                id: ProjectId::random(),
                name: "current".parse().unwrap(),
                root: "/fixture/current".into(),
                git: false,
                default_branch: None,
                state: ProjectState::Ready,
                created_at: "1970-01-01T00:00:00Z".parse().unwrap(),
                revision: 5,
            };
            let list = ProjectList {
                git: GitSupport::Missing,
                projects: vec![project.clone()],
                more: false,
            };
            shell.projects_read(host.as_str(), &key, Ok(list.clone()));
            assert!(!shell.areas.workspace.projects.contains_key(host.as_str()));
            let current = shell
                .areas
                .workspace
                .project_reads
                .begin_on(ProjectTarget::new(host.clone(), node), context.clone())
                .unwrap();
            shell.projects_read(host.as_str(), &current, Ok(list.clone()));
            let mut stale = project.clone();
            stale.name = "wrong-peer".parse().unwrap();
            stale.revision = 100;
            shell.project_followed(host.as_str(), old, stale);
            assert_eq!(
                shell.areas.workspace.projects[host.as_str()].project(&project.id),
                Some(&project)
            );
            // A session-list refusal is not a retired authenticated admission.
            shell.fleet.machines[0].reach = Reach::Unreachable("sessions refused".to_owned());
            assert_eq!(shell.project_connection(host.as_str()), Some(context.clone()));
            let selected = ProjectTarget::new(host.clone(), node);
            assert_eq!(
                shell.open_project_settings_for_connection(&context, project.id, window, cx),
                Ok(())
            );
            assert_eq!(shell.areas.settings.machine.as_deref(), Some(host.as_str()));
            assert_eq!(
                shell
                    .areas
                    .workspace
                    .page
                    .detail
                    .as_ref()
                    .map(|detail| detail.project),
                Some(project.id)
            );
            assert_eq!(
                shell.open_project_settings(&selected, ProjectId::random(), window, cx),
                Err(super::super::project_navigation::ProjectSettingsRefusal::Missing)
            );
            assert_eq!(
                shell.open_project_settings(
                    &ProjectTarget::new(host.clone(), old),
                    project.id,
                    window,
                    cx
                ),
                Err(super::super::project_navigation::ProjectSettingsRefusal::Unavailable)
            );
            assert_eq!(
                shell
                    .areas
                    .workspace
                    .page
                    .detail
                    .as_ref()
                    .map(|detail| detail.project),
                Some(project.id)
            );
            shell.fleet.machines[0].node = Some(old);
            shell.confirm_project_removal(project.id, cx);
            assert!(
                shell.areas.workspace.page.removing.is_none(),
                "a rebound peer cannot inherit removal authority from the cache"
            );
            shell.fleet.machines[0].node = Some(node);
            shell.confirm_project_removal(project.id, cx);
            assert!(shell.areas.workspace.page.removing.is_some());
            shell.areas.workspace.page.removing = None;
            let newest = shell
                .areas
                .workspace
                .project_reads
                .begin_on(ProjectTarget::new(host.clone(), node), context.clone())
                .unwrap();
            assert_eq!(
                shell.open_project_settings(&selected, project.id, window, cx),
                Err(super::super::project_navigation::ProjectSettingsRefusal::NotHydrated)
            );
            assert_eq!(
                shell.setup_readiness(&host, node),
                vessel_app_view::workspace::projects::actions::ProjectSetupReadiness::Pending
            );
            shell.projects_read(host.as_str(), &current, Err(Problem::Unsent));
            assert!(
                shell.areas.workspace.projects[host.as_str()]
                    .list()
                    .is_some()
            );
            shell.projects_read(host.as_str(), &newest, Ok(list));
            assert_eq!(
                shell.setup_readiness(&host, node),
                vessel_app_view::workspace::projects::actions::ProjectSetupReadiness::Ready
            );
            shell.open_add_for_connection(vessel_app_view::workspace::words::AddForm::Folder, &context, window, cx).unwrap();
            let tokens = vessel_app_kit::Tokens::of(vessel_app_kit::Scheme::Dark);
            assert!(super::super::project_form::for_connection(shell, &context, &tokens, false, cx).is_some());
            let next = epochs
                .admit(
                    vessel_app_view::connection::MachineKey::from_book(host.as_str()),
                    node,
                )
                .unwrap();
            shell.fleet.machines[0].connection_epoch = Some(next.epoch());
            assert_eq!(shell.open_project_settings_for_connection(&context, project.id, window, cx), Err(super::super::project_navigation::ProjectSettingsRefusal::Unavailable));
            assert!(super::super::project_form::for_connection(shell, &context, &tokens, false, cx).is_none());
            assert!(super::super::project_form::for_connection(shell, &next, &tokens, false, cx).is_none());
            assert_eq!(shell.setup_readiness_on(&context), vessel_app_view::workspace::projects::actions::ProjectSetupReadiness::Failed);
            assert_eq!(shell.open_add_for_connection(vessel_app_view::workspace::words::AddForm::Folder, &context, window, cx), Err(vessel_app_view::workspace::projects::navigation::NavigationRefusal::HostUnavailable));
            assert_eq!(
                shell.setup_readiness(&host, node),
                vessel_app_view::workspace::projects::actions::ProjectSetupReadiness::Pending
            );
            assert_eq!(
                shell.open_project_settings(&selected, project.id, window, cx),
                Err(super::super::project_navigation::ProjectSettingsRefusal::Unavailable)
            );
            let mut delayed = project.clone();
            delayed.name = "retired-physical-connection".parse().unwrap();
            delayed.revision = 1000;
            shell.selected_project_answer(
                vessel_app_link::workspace::project_requests::Response {
                    context,
                    answer: vessel_app_link::workspace::Answer::Project {
                        node,
                        project: delayed,
                    },
                },
                cx,
            );
            assert_eq!(
                shell.areas.workspace.projects[host.as_str()].project(&project.id),
                Some(&project)
            );
        });
    }
}
