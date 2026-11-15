//! Project settings navigation validates captured placement before changing any screen.
//! Workspace/pane activation is owned by the shared layout adapter, not this module.

use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::settings::Section;
use vessel_app_view::workspace::projects::{ProjectsView, navigation::ProjectTarget};
use vessel_proto::ProjectId;

/// Navigation does not turn a missing or unverified list into another host's project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectSettingsRefusal {
    /// Captured alias is offline, rebound, or its cache belongs to another node.
    Unavailable,
    /// A current complete project list is still being read.
    NotHydrated,
    /// The authenticated project's current list no longer contains this id.
    Missing,
}

impl Shell {
    /// Open a row's project settings without substituting a same-node reconnect.
    ///
    /// # Errors
    /// A retired row, an unhydrated list or a missing project preserves the route.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "sidebar and palette consume this producer after source review"
        )
    )]
    pub(crate) fn open_project_settings_for_connection(
        &mut self,
        context: &vessel_app_view::connection::ConnectionContext,
        project: ProjectId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), ProjectSettingsRefusal> {
        if self.project_connection(context.machine().as_str()).as_ref() != Some(context) {
            return Err(ProjectSettingsRefusal::Unavailable);
        }
        let host = vessel_app_view::machine_book::Name::parse(context.machine().as_str())
            .map_err(|_invalid| ProjectSettingsRefusal::Unavailable)?;
        self.open_project_settings(
            &ProjectTarget::new(host, context.node()),
            project,
            window,
            cx,
        )
    }

    /// Open one exact host/project from a sidebar, palette or project row.
    /// All refusals preserve the previous route, host selection and project detail.
    ///
    /// # Errors
    /// Refuses unavailable peers, unhydrated project data and an absent project id.
    pub(crate) fn open_project_settings(
        &mut self,
        target: &ProjectTarget,
        project: ProjectId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), ProjectSettingsRefusal> {
        let workspace = &self.areas.workspace;
        if !self.project_cache_current(target.host())
            || self.project_node(target.host().as_str()) != Some(target.node())
            || workspace.project_reads.node(target.host()) != Some(target.node())
        {
            return Err(ProjectSettingsRefusal::Unavailable);
        }
        if workspace
            .project_reads
            .pending(target.host(), target.node())
        {
            return Err(ProjectSettingsRefusal::NotHydrated);
        }
        let info = match workspace.projects.get(target.host().as_str()) {
            Some(ProjectsView::Ready(list)) => list
                .projects
                .iter()
                .find(|info| info.id == project)
                .ok_or(ProjectSettingsRefusal::Missing)?
                .clone(),
            Some(ProjectsView::Loading) | None => return Err(ProjectSettingsRefusal::NotHydrated),
            Some(ProjectsView::Failed(_) | ProjectsView::Older) => {
                return Err(ProjectSettingsRefusal::Unavailable);
            }
        };
        self.areas.settings.machine = Some(target.host().as_str().to_owned());
        self.open_settings(Some(Section::Projects), window, cx);
        self.areas.workspace.page.detail = Some(super::detail::Detail::new(project));
        self.load_detail(target.host().as_str(), &info);
        cx.notify();
        Ok(())
    }
}
