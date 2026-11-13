//! Projects, worktrees, git and files (track F): Settings > Projects with
//! a project's detail, the commands approval, a worktree's removal, the
//! sidebar's projects; the place picker the composer's tray embeds, the
//! worktree start's card, and the code panel beside a session.
//!
//! What these screens mean is `view::workspace`'s; what they ask of a
//! machine is `link::workspace`'s (one `Command::Workspace`, answered by
//! one `Event::Workspace`, dispatched here by [`Shell::workspace_answer`]);
//! their words are `Lang::workspace()`'s.

mod approval;
mod detail;
mod projects;
mod remove;
mod widgets;

use std::collections::HashMap;

use vessel_app_kit::TextField;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_link::workspace::{Answer, Ask, ProjectAsk};
use vessel_app_view::workspace::projects::ProjectsView;
use vessel_app_view::workspace::words::WorkspaceWords;

pub(crate) use projects::render as projects_content;

use crate::shell::Shell;

/// Every workspace screen's state.
pub(crate) struct Workspace {
    /// Each machine's projects as last read, by machine.
    pub(crate) projects: HashMap<String, ProjectsView>,
    /// Settings > Projects.
    pub(crate) page: projects::Page,
}

impl Workspace {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            projects: HashMap::new(),
            page: projects::Page::new(window, cx),
        }
    }
}

/// A one-line field whose Enter does nothing of its own (a form's fields;
/// the form's button submits).
pub(super) fn field(window: &mut Window, cx: &mut Context<'_, Shell>) -> TextField {
    TextField::new("", |_shell: &mut Shell, _text, _window, _cx| {}, window, cx)
}

impl Shell {
    /// The workspace's words, in the person's language.
    pub(crate) fn words(&self) -> &'static dyn WorkspaceWords {
        self.t.lang.workspace()
    }

    /// Ask `machine` something about its workspace.
    pub(crate) fn ask(&self, machine: &str, ask: Ask) {
        self.command(Command::Workspace {
            machine: machine.to_owned(),
            ask,
        });
    }

    /// Read every online machine's projects that has not been read yet
    /// (the fleet was read again): the sidebar's group and the composer's
    /// tray show them without a visit to Settings first.
    pub(crate) fn workspace_fleet(&mut self) {
        let unread: Vec<String> = self
            .fleet
            .online()
            .filter(|machine| !self.areas.workspace.projects.contains_key(&machine.name))
            .map(|machine| machine.name.clone())
            .collect();
        for machine in unread {
            let _was = self
                .areas
                .workspace
                .projects
                .insert(machine.clone(), ProjectsView::Loading);
            self.ask(&machine, Ask::Project(ProjectAsk::List));
        }
    }

    /// A machine answered a workspace ask.
    pub(crate) fn workspace_answer(
        &mut self,
        machine: &str,
        answer: Answer,
        cx: &mut Context<'_, Self>,
    ) {
        match answer {
            Answer::Projects(result) => {
                let _was = self
                    .areas
                    .workspace
                    .projects
                    .insert(machine.to_owned(), ProjectsView::listed(result));
            }
            Answer::Project(project) => {
                if let Some(view) = self.areas.workspace.projects.get_mut(machine) {
                    view.replace(project);
                }
            }
            Answer::Took { form, result } => self.took_project(machine, form, result),
            Answer::Forgot { project, result } => self.forgot_project(machine, project, result),
            Answer::Commands {
                project,
                approved,
                result,
            } => self.project_commands(machine, project, approved, result),
            Answer::Worktrees { project, result } => {
                self.project_worktrees(machine, project, result);
            }
            Answer::Snapshots { project, result } => {
                self.project_snapshots(machine, project, result);
            }
            Answer::Dropped { ask, result } => self.snapshot_dropped(machine, &ask, result),
            Answer::SetupStarted { worktree, result } => {
                self.setup_started(machine, worktree, result);
            }
            Answer::Removed { ask, result } => self.worktree_removed(machine, &ask, result),
            // The code panel's and the start's answers: their screens come
            // in the next slices; until then nothing asks them.
            Answer::Status { .. }
            | Answer::Changes { .. }
            | Answer::Diff { .. }
            | Answer::Branches { .. }
            | Answer::Log { .. }
            | Answer::Stashes { .. }
            | Answer::Git { .. }
            | Answer::Listed { .. }
            | Answer::Read { .. }
            | Answer::Saved { .. }
            | Answer::Found { .. }
            | Answer::Grepped { .. }
            | Answer::PathChanged { .. }
            | Answer::Start { .. }
            | Answer::StartEnded { .. } => {}
        }
        cx.notify();
    }
}
