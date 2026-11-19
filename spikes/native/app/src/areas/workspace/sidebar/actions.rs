//! A project row's actions, as Paseo's (`sidebar-workspace-list.tsx`:
//! `ProjectRowTrailingActions`, `NewWorktreeButton`, `ProjectKebabMenu`,
//! `ProjectMenuItems`, `handleRemoveProject`): "New workspace" and the
//! project's menu at the row's end, the menu's Open project settings and
//! Remove project, and the question before removing.
//!
//! [Decision Log] (TRACKER 662)
//! - Purpose: what Paseo's project row offers, where the person sees the
//!   project, without going to Settings first.
//! - Existing: Settings › Projects removes a project at once (its page's
//!   button) and opens a project's detail by a press on its row; the draft
//!   tray places a new chat in a project, local or in a new worktree.
//! - Choice: the plus (a 24 square, the plus 15, muted, the foreground on
//!   hover; Paseo's tooltip "New workspace" under it, at once) starts a new
//!   chat in a new worktree of the project, offered for a ready git
//!   project as Paseo offers it where a host can make a worktree; the
//!   kebab (24, 32 on a phone; `MoreVertical`; muted, the foreground on
//!   hover, `surface2` behind it) opens Paseo's menu (220 wide, a sheet
//!   titled "Project actions" on a phone). Both show while the row is
//!   hovered, always on a phone, and the kebab while its menu is open.
//!   Remove asks Paseo's question first (destructive Remove, Cancel), then
//!   the node removes the project (its files stay); while it does, the
//!   row's mark is a spinner and the menu's row says "Removing...". A
//!   refusal is a toast in the app's words. A removal from Settings shows
//!   the same spinner, its refusal stays on its page.
//! - Differences, named: Paseo's Open in new window and Open in file manager
//!   are its desktop shell's (a local folder in a new window or the system's
//!   file manager); not offered. Paseo's shortcut hint in the plus's tooltip
//!   (its new-worktree key) has no binding here.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use vessel_app_kit::menu::MenuState;
use vessel_app_kit::tooltip::Place;
use vessel_app_kit::ui::{App, Context, Pixels, Point, Window};
use vessel_app_kit::{DialogState, Tooltips};
use vessel_app_link::workspace::{Ask, ProjectAsk};
use vessel_app_view::toasts::ToastTone;
use vessel_app_view::workspace::place::PlaceMode;
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::ProjectId;

use super::super::PlacePick;
use super::super::detail::Detail;
use crate::shell::Shell;

mod draw;

pub(crate) use draw::layers;
pub(super) use draw::trailing;
use draw::menu_key;

/// The project an action is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Target {
    pub(super) machine: String,
    pub(super) project: ProjectId,
    pub(super) name: String,
}

/// Where a removal was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    /// The row's menu: a refusal is a toast.
    Sidebar,
    /// Settings › Projects: a refusal stays on its page.
    Page,
}

/// The project rows' menu, question, removals and tooltips.
pub(crate) struct ProjectRows {
    menu: MenuState,
    /// The project the menu is open for.
    target: Option<Target>,
    /// The project the remove question asks about.
    confirm: Option<Target>,
    dialog: DialogState,
    /// Removals the node has not answered yet.
    removing: HashMap<(String, ProjectId), Origin>,
    tips: Tooltips,
    places: RefCell<HashMap<String, Place>>,
}

impl ProjectRows {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            menu: MenuState::new(cx),
            target: None,
            confirm: None,
            dialog: DialogState::new(cx),
            removing: HashMap::new(),
            tips: Tooltips::new(),
            places: RefCell::new(HashMap::new()),
        }
    }

    /// Whether `project` on `machine` is being removed.
    pub(super) fn is_removing(&self, machine: &str, project: ProjectId) -> bool {
        self.removing.contains_key(&(machine.to_owned(), project))
    }

    fn menu_is_for(&self, target: &Target) -> bool {
        self.menu.is_open() && self.target.as_ref() == Some(target)
    }

    /// `key`'s place for its tooltip, kept across frames.
    fn place(&self, key: &str) -> Place {
        Rc::clone(
            self.places
                .borrow_mut()
                .entry(key.to_owned())
                .or_insert_with(|| Rc::new(std::cell::Cell::new(None))),
        )
    }
}

impl Shell {
    fn project_menu_open(
        &mut self,
        target: Target,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let rows = &mut self.areas.workspace.sidebar;
        // Under the kebab, its right edge on the kebab's (Paseo's
        // `align="end"`), once the kebab has been drawn.
        let kebab = rows.place(&menu_key(&target)).get();
        rows.target = Some(target);
        match kebab {
            Some(bounds) => rows.menu.open_end(bounds, window, cx),
            None => rows.menu.open(at, window, cx),
        }
        cx.notify();
    }

    fn project_menu_chose(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(target) = self.areas.workspace.sidebar.target.take() else {
            return;
        };
        // The menu's rows: Open project settings, then Remove project.
        if index == 0 {
            self.show_project_settings(&target, window, cx);
        } else {
            self.areas.workspace.sidebar.confirm = Some(target);
            self.areas.workspace.sidebar.dialog.focus(window, cx);
        }
        cx.notify();
    }

    /// Settings › Projects on the project's machine, its detail open.
    fn show_project_settings(
        &mut self,
        target: &Target,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(info) = self
            .areas
            .workspace
            .projects
            .get(&target.machine)
            .and_then(|view| view.project(&target.project))
            .cloned()
        else {
            return;
        };
        self.projects_page(Some(target.machine.clone()), false, window, cx);
        self.areas.workspace.page.detail = Some(Detail::new(info.id));
        self.load_detail(&target.machine, &info);
    }

    /// A new chat in a new worktree of the project (Paseo's new workspace).
    fn new_workspace_in(
        &mut self,
        target: &Target,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.chat_in_project(target.machine.clone(), target.project, window, cx);
        self.choose_place(PlacePick::Isolation(PlaceMode::Worktree), window, cx);
    }

    /// The question closed unanswered (Cancel, Escape, the scrim, the
    /// sheet let go): the keys go back to whoever had them before the menu,
    /// and the dialog's cycle and sheet start fresh next time.
    fn project_remove_dismiss(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let rows = &mut self.areas.workspace.sidebar;
        rows.confirm = None;
        rows.dialog.dismissed(window, cx);
        cx.notify();
    }

    fn project_remove_confirmed(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let rows = &mut self.areas.workspace.sidebar;
        let Some(target) = rows.confirm.take() else {
            return;
        };
        rows.dialog.dismissed(window, cx);
        self.remove_project_from(&target.machine, target.project, Origin::Sidebar);
        cx.notify();
    }

    /// The kebab activated by a click (a pointer's press already opened
    /// the menu on its way down; assistive technology clicks only): open
    /// the menu unless it is already open for this project.
    fn project_menu_activate(
        &mut self,
        target: Target,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.areas.workspace.sidebar.menu_is_for(&target) {
            self.project_menu_open(target, at, window, cx);
        }
    }

    /// Ask `machine` to forget `project`, its row spinning until it
    /// answers.
    pub(crate) fn remove_project_from(
        &mut self,
        machine: &str,
        project: ProjectId,
        origin: Origin,
    ) {
        let _was = self
            .areas
            .workspace
            .sidebar
            .removing
            .insert((machine.to_owned(), project), origin);
        self.ask(machine, Ask::Project(ProjectAsk::Remove(project)));
    }

    /// The node answered a removal: the row stops spinning; the sidebar's
    /// says nothing when done and a toast when refused, the page's says
    /// both on its page.
    pub(crate) fn project_removal_answered(
        &mut self,
        machine: &str,
        project: ProjectId,
        result: Result<(), Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        let origin = self
            .areas
            .workspace
            .sidebar
            .removing
            .remove(&(machine.to_owned(), project));
        match (origin, result) {
            (Some(Origin::Sidebar), Err(problem)) => {
                let words = self.words();
                let name = self
                    .areas
                    .workspace
                    .projects
                    .get(machine)
                    .and_then(|view| view.project(&project))
                    .map_or_else(|| project.to_string(), |info| info.name.to_string());
                let said = words.not_removed(&name, &words.problem(&problem));
                self.toast(said, ToastTone::Error, cx);
            }
            (Some(Origin::Sidebar), Ok(())) => {
                // Paseo's row just goes; the page's "forgot" line at the
                // frame's foot is the page's.
                let notice = self.notice.clone();
                self.forgot_project(machine, project, Ok(()));
                self.notice = notice;
            }
            (Some(Origin::Page) | None, result) => self.forgot_project(machine, project, result),
        }
    }
}

#[cfg(test)]
#[path = "actions_tests.rs"]
mod tests;
