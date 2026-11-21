//! Projects, worktrees, git and files (track F): Settings > Projects with
//! a project's detail, Add project's sheet, the commands approval, a
//! worktree's removal, the sidebar's projects; the place picker the composer's tray embeds, the
//! worktree start's card, and the code panel beside a session.
//!
//! What these screens mean is `view::workspace`'s; what they ask of a
//! machine is `link::workspace`'s (one `Command::Workspace`, answered by
//! one `Event::Workspace`, dispatched here by [`Shell::workspace_answer`]);
//! their words are `Lang::workspace()`'s.

mod add_flow;
mod approval;
mod background;
mod book;
mod code;
mod detail;
mod open_targets;
mod place;
mod place_rows;
mod project_icons;
mod projects;
mod remove;
mod sidebar;
mod start_card;
mod start_header;
mod widgets;

use std::collections::HashMap;

use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_link::workspace::{Answer, Ask, ProjectAsk};
use vessel_app_view::workspace::projects::ProjectsView;
use vessel_app_view::workspace::words::WorkspaceWords;

pub(crate) use add_flow::render as add_flow_sheet;
#[expect(unused_imports, reason = "track P's seam: its first user is P's `@`")]
pub(crate) use code::FindRefused;
#[cfg(test)]
pub(crate) use code::{ShownChangesContext, ShownDiffContext};
pub(crate) use code::{code_beside, code_toggle, frame_row};
pub(crate) use open_targets::Origin as OpenOrigin;
pub(crate) use open_targets::open_button as open_in_editor;
pub(crate) use place::{PlacePick, Rows as PlaceRows};
pub(crate) use projects::render as projects_content;
pub(crate) use sidebar::layers as sidebar_project_layers;
pub(crate) use sidebar::projects as sidebar_projects;
pub(crate) use sidebar::tree as sidebar_tree;
pub(crate) use start_card::render as start_card;

use crate::shell::Shell;

/// Every workspace screen's state.
pub(crate) struct Workspace {
    /// Each machine's projects as last read, by machine.
    pub(crate) projects: HashMap<String, ProjectsView>,
    /// Settings > Projects.
    pub(crate) page: projects::Page,
    /// Add project's sheet.
    pub(crate) add_flow: add_flow::AddFlowSheet,
    /// The composer tray's place.
    pub(crate) place: place::DraftPlace,
    /// A start in a project, in flight.
    pub(crate) start: Option<start_card::Starting>,
    /// Starts so far (each start's key).
    /// The code panel beside a session.
    pub(crate) code: code::CodePanel,
    /// The sidebar's projects unfolded past their first rows ("All
    /// projects (N)" pressed).
    pub(crate) projects_unfolded: bool,
    /// Projects' own icons (ADR 0112).
    pub(crate) icons: project_icons::ProjectIcons,
    /// Each machine's background file work (`background.rs`).
    pub(crate) background: background::Background,
    /// The sidebar's project rows: their menu, question and removals.
    pub(crate) sidebar: sidebar::ProjectRows,
    /// Open in editor and reveal (TRACKER 467).
    pub(crate) open: open_targets::OpenTargets,
}

impl Workspace {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            projects: HashMap::new(),
            page: projects::Page::new(),
            add_flow: add_flow::AddFlowSheet::new(window, cx),
            place: place::DraftPlace::new(window, cx),
            start: None,
            code: code::CodePanel::new(window, cx),
            projects_unfolded: false,
            icons: project_icons::ProjectIcons::default(),
            background: background::Background::default(),
            sidebar: sidebar::ProjectRows::new(cx),
            open: open_targets::OpenTargets::new(window, cx),
        }
    }
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
        self.add_flow_fleet();
        self.pulls_fleet();
    }

    /// A machine answered a workspace ask. Returns whether what is drawn
    /// may have changed; `false` only for an answer that changed nothing
    /// (track G1's refresh of an unchanged status), so the shell asks for
    /// no frame.
    pub(crate) fn workspace_answer(
        &mut self,
        machine: &str,
        answer: Answer,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        match answer {
            Answer::Layout(answer) => self.layout_renamed(answer, cx),
            Answer::Projects(result) => {
                let _was = self
                    .areas
                    .workspace
                    .projects
                    .insert(machine.to_owned(), ProjectsView::listed(result));
                self.projects_listed(machine);
                self.ask_project_icons(machine);
            }
            Answer::Project(project) => {
                if let Some(view) = self.areas.workspace.projects.get_mut(machine) {
                    view.replace(project);
                }
            }
            Answer::Took { form, result } => self.took_project(machine, form, result),
            Answer::Folders { query, result } => self.add_flow_folders(machine, &query, result),
            Answer::Repositories { query, result } => {
                self.add_flow_repositories(machine, &query, result);
            }
            Answer::Icon { project, result } => {
                self.background_answered(machine);
                self.project_icon_read(machine, project, result);
            }
            Answer::Forgot { project, result } => {
                self.project_removal_answered(machine, project, result, cx);
                // A removed project's fold goes with it.
                self.projects_listed(machine);
            }
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
            Answer::Branches { root, result } => {
                self.git_branches_read(machine, &root, &result);
                self.branches_read(machine, &root, result);
            }
            Answer::Start { key, step } => self.start_step(key, step),
            Answer::StartEnded { key, ended } => self.start_ended(key, ended, cx),
            Answer::Session { session, result } => self.code_session_read(session, result),
            Answer::Changes { ask, result } => self.changes_read(&ask, result),
            Answer::Diff { ask, result } => self.diff_read(&ask, result),
            Answer::Listed { ask, result } => self.folder_read(&ask, result),
            Answer::Read { ask, result } => {
                let version = result.as_ref().ok().map(|read| read.version.clone());
                let seen = version.as_ref().and_then(|version| version.hash.clone());
                self.file_read(&ask, result, cx);
                self.code_file_seen(&ask, seen);
                self.code_image_seen(&ask, version);
            }
            Answer::Found { ask, result } => self.files_found(&ask, result),
            Answer::Grepped { ask, result } => self.files_grepped(&ask, result),
            Answer::PathChanged { change, result } => self.path_changed(&change, result),
            Answer::Observed { ask, result } => self.file_observed(&ask, result, cx),
            Answer::Duplicated { ask, result } => self.entry_duplicated(&ask, result),
            Answer::Inspected { ask, result } => {
                self.background_answered(machine);
                self.folder_inspected(&ask, result);
            }
            Answer::Downloaded { result, .. } => self.file_downloaded(result, cx),
            Answer::ImageRead { ask, result } => self.code_image_read(&ask, result, cx),
            // Track G1's: the git toolbar's reads and writes.
            Answer::Status { root, result } => {
                // An unchanged status asks for no frame (track G1's
                // five-second refresh of an idle panel draws nothing).
                if !self.git_status_read(machine, &root, result) {
                    return false;
                }
            }
            Answer::Stashes { root, result } => self.git_stashes_read(machine, &root, result),
            Answer::Git { write, result } => self.git_written(machine, &write, result, cx),
            // Track G1b's: the pull request form's defaults and its calls.
            Answer::Log { ask, result } => self.pulls_log_read(machine, &ask, result),
            Answer::Pull(answer) => self.pulls_answered(machine, answer, cx),
            Answer::Saved { root, path, saved } => self.editor_saved(&root, &path, saved, cx),
            Answer::Workspaces(result) => {
                if !self.diff_stat_answered(machine, result, cx) {
                    return false;
                }
            }
        }
        cx.notify();
        true
    }
}
