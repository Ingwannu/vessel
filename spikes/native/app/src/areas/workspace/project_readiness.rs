//! Synchronous manual-work readiness; onboarding owns its Target and Visit checks.

use crate::shell::Shell;
use vessel_app_view::{
    machine_book::Name,
    workspace::projects::{ProjectsView, actions::ProjectSetupReadiness},
};
use vessel_proto::{NodeId, workspace::ProjectState};

impl Shell {
    /// A stale Ready cache and an empty pending read never choose an onboarding path.
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "onboarding consumes this producer after source review"
        )
    )]
    pub(crate) fn project_inventory_on(
        &self,
        context: &vessel_app_view::connection::ConnectionContext,
    ) -> vessel_app_view::workspace::projects::actions::ProjectInventory {
        use vessel_app_view::workspace::projects::actions::ProjectInventory;
        match self.setup_readiness_on(context) {
            ProjectSetupReadiness::Pending => ProjectInventory::Pending,
            ProjectSetupReadiness::Failed => ProjectInventory::Unavailable,
            ProjectSetupReadiness::Ready => match self
                .areas
                .workspace
                .projects
                .get(context.machine().as_str())
            {
                Some(ProjectsView::Ready(list)) if !list.projects.is_empty() => {
                    ProjectInventory::Existing
                }
                Some(ProjectsView::Ready(list)) if !list.more => ProjectInventory::Empty,
                Some(ProjectsView::Loading) | None => ProjectInventory::Pending,
                Some(ProjectsView::Ready(_) | ProjectsView::Failed(_) | ProjectsView::Older) => {
                    ProjectInventory::Unavailable
                }
            },
        }
    }

    /// Readiness is scoped to the caller's actual connection, including reconnects.
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "onboarding consumes this producer after source review"
        )
    )]
    pub(crate) fn setup_readiness_on(
        &self,
        context: &vessel_app_view::connection::ConnectionContext,
    ) -> ProjectSetupReadiness {
        if self.project_connection(context.machine().as_str()).as_ref() != Some(context) {
            return ProjectSetupReadiness::Failed;
        }
        let Ok(host) = Name::parse(context.machine().as_str()) else {
            return ProjectSetupReadiness::Failed;
        };
        self.setup_readiness(&host, context.node())
    }

    /// Query a fixed authenticated host. This does not acknowledge checked suggestions.
    /// The caller must retain its own Visit and recheck it before advancing onboarding.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "onboarding installs this shared seam in its own package"
        )
    )]
    pub(crate) fn setup_readiness(&self, host: &Name, node: NodeId) -> ProjectSetupReadiness {
        if self.project_node(host.as_str()) != Some(node) {
            return ProjectSetupReadiness::Failed;
        }
        if !self.project_cache_current(host)
            || self.areas.workspace.project_reads.node(host) != Some(node)
            || self.areas.workspace.project_reads.pending(host, node)
        {
            return ProjectSetupReadiness::Pending;
        }
        let page = &self.areas.workspace.page;
        let activity = page.actions.setup_readiness(host, node);
        if activity != ProjectSetupReadiness::Ready {
            return activity;
        }
        if page.form_host.as_ref() == Some(host)
            && page.form_node == Some(node)
            && page.error.is_some()
        {
            return ProjectSetupReadiness::Failed;
        }
        match self.areas.workspace.projects.get(host.as_str()) {
            Some(ProjectsView::Ready(list)) => {
                if list
                    .projects
                    .iter()
                    .any(|project| matches!(project.state, ProjectState::Cloning { .. }))
                {
                    ProjectSetupReadiness::Pending
                } else if list.projects.iter().any(|project| {
                    matches!(
                        project.state,
                        ProjectState::CloneFailed { .. } | ProjectState::Missing
                    )
                }) {
                    ProjectSetupReadiness::Failed
                } else {
                    ProjectSetupReadiness::Ready
                }
            }
            Some(ProjectsView::Loading) | None => ProjectSetupReadiness::Pending,
            Some(ProjectsView::Failed(_) | ProjectsView::Older) => ProjectSetupReadiness::Failed,
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
        workspace::{CloneStage, GitSupport, ProjectInfo, ProjectList},
    };

    #[gpui::test]
    fn readiness_waits_for_clone_and_refuses_a_rebound_host(cx: &mut gpui::TestAppContext) {
        language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update_in(cx, |shell, _, _| {
            let host = Name::parse("laptop").unwrap();
            let node = NodeId::random();
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
            shell.fleet.machines.push(machine);
            assert_eq!(
                shell.setup_readiness(&host, node),
                ProjectSetupReadiness::Pending
            );
            let project = ProjectInfo {
                id: ProjectId::random(),
                name: "cloning".parse().unwrap(),
                root: "/isolated/project".into(),
                git: true,
                default_branch: None,
                state: ProjectState::Cloning {
                    stage: CloneStage::Connecting,
                    percent: None,
                },
                created_at: "1970-01-01T00:00:00Z".parse().unwrap(),
                revision: 0,
            };
            let read = shell
                .areas
                .workspace
                .project_reads
                .begin_on(
                    vessel_app_view::workspace::projects::navigation::ProjectTarget::new(
                        host.clone(),
                        node,
                    ),
                    context.clone(),
                )
                .unwrap();
            shell.projects_read(
                host.as_str(),
                &read,
                Ok(ProjectList {
                    git: GitSupport::Missing,
                    projects: vec![project.clone()],
                    more: false,
                }),
            );
            assert_eq!(
                shell.setup_readiness(&host, node),
                ProjectSetupReadiness::Pending
            );
            readiness_after_clone(shell, &host, node, context, project);
        });
    }
    fn readiness_after_clone(
        shell: &mut Shell,
        host: &Name,
        node: NodeId,
        context: ConnectionContext,
        project: ProjectInfo,
    ) {
        let mut ready = project;
        ready.state = ProjectState::Ready;
        shell
            .areas
            .workspace
            .projects
            .get_mut(host.as_str())
            .unwrap()
            .replace(ready);
        assert_eq!(
            shell.setup_readiness(host, node),
            ProjectSetupReadiness::Ready
        );
        use vessel_app_view::workspace::projects::actions::ProjectInventory;
        assert_eq!(
            shell.project_inventory_on(&context),
            ProjectInventory::Existing
        );
        let target = vessel_app_view::workspace::projects::navigation::ProjectTarget::new(
            host.clone(),
            node,
        );
        let key = shell
            .areas
            .workspace
            .project_reads
            .begin_on(target.clone(), context.clone())
            .unwrap();
        assert_eq!(
            shell.project_inventory_on(&context),
            ProjectInventory::Pending
        );
        shell.projects_read(
            host.as_str(),
            &key,
            Ok(ProjectList {
                git: GitSupport::Missing,
                projects: Vec::new(),
                more: true,
            }),
        );
        assert_eq!(
            shell.project_inventory_on(&context),
            ProjectInventory::Unavailable
        );
        let key = shell
            .areas
            .workspace
            .project_reads
            .begin_on(target, context.clone())
            .unwrap();
        shell.projects_read(
            host.as_str(),
            &key,
            Ok(ProjectList {
                git: GitSupport::Missing,
                projects: Vec::new(),
                more: false,
            }),
        );
        assert_eq!(
            shell.project_inventory_on(&context),
            ProjectInventory::Empty
        );
        shell.fleet.machines[0].node = Some(NodeId::random());
        assert_eq!(
            shell.project_inventory_on(&context),
            ProjectInventory::Unavailable
        );
        assert_eq!(
            shell.setup_readiness(host, node),
            ProjectSetupReadiness::Failed
        );
    }
}
