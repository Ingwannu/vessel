//! Opening a project coordinates its independent read surfaces.
use super::detail::Detail;
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::workspace::{Ask, WorktreeAsk};
use vessel_proto::workspace::{ListWorktrees, ProjectInfo};

impl Shell {
    /// Read the project's commands, worktrees and snapshots.
    pub(super) fn load_detail(
        &mut self,
        machine: &str,
        project: &ProjectInfo,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.read_project_commands(machine, project.id, true);
        self.open_scripts(machine, project.id, window, cx);
        self.ask(
            machine,
            Ask::Worktree(WorktreeAsk::List(ListWorktrees {
                project: Some(project.id),
            })),
        );
        if project.git {
            self.ask(machine, Ask::Worktree(WorktreeAsk::Snapshots(project.id)));
        }
    }

    pub(super) fn open_project(
        &mut self,
        project: &ProjectInfo,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let page = &mut self.areas.workspace.page;
        if page
            .detail
            .as_ref()
            .is_some_and(|open| open.project == project.id)
        {
            page.detail = None;
        } else {
            page.detail = Some(Detail::new(project.id));
            if let Some(machine) = self.projects_machine() {
                self.load_detail(&machine, project, window, cx);
            }
        }
        cx.notify();
    }
}
