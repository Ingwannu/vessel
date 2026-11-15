//! Opening a project coordinates its independent read surfaces.
use super::detail::Detail;
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::{Ask, ProjectAsk, WorktreeAsk};
use vessel_proto::workspace::{GetProjectCommands, ListWorktrees, ProjectInfo};

impl Shell {
    /// Read the project's commands, worktrees and snapshots.
    pub(super) fn load_detail(&self, machine: &str, project: &ProjectInfo) {
        self.ask(
            machine,
            Ask::Project(ProjectAsk::Commands(GetProjectCommands {
                project: project.id,
                base: None,
            })),
        );
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

    pub(super) fn open_project(&mut self, project: &ProjectInfo, cx: &mut Context<'_, Self>) {
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
                self.load_detail(&machine, project);
                self.open_scripts(&machine, project.id, cx);
            }
        }
        cx.notify();
    }
}
