//! Add project as Paseo's sheet (TRACKER 234; `view::workspace::add_flow`
//! decides, this asks and draws): the sidebar's "+", the composer's "Add
//! project" row and Settings > Projects open it; its pages are the
//! machine, the methods, a directory search, a clone's repository and
//! destination, and a new directory's parent and name.
//!
//! [Decision Log]
//! - Purpose: the add menu and its three inline forms become Paseo's one
//!   flow with its four methods, as on the page (`add-project-flow.tsx`).
//! - Constraint: answers come through the link without a window; a
//!   success moves to the new session or Settings, which needs one.
//! - Choice: one field for every page (its text is the page's), the
//!   machine's folders kept with the machine and the text they answer
//!   (the node review of the page's R2), and the move after a success
//!   kept until the next frame, when the sheet's render defers it with
//!   the window (the timeline's rewind and fork do the same).
//! - Differences (docs/plans/wp-add-project-flow.md): Browse is hidden
//!   until the app knows which machine is this computer (TRACKER 816); a
//!   clone closes the sheet when the node answers `cloning` and its
//!   progress is its row in Settings > Projects (difference 7).

mod calls;
mod render;

pub(crate) use render::render;
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests;

use std::collections::HashMap;

use vessel_app_kit::TextField;
use vessel_app_kit::command::CommandKey;
use vessel_app_kit::focus::ReturnFocus;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{Context, ScrollHandle, Window, point, px};
use vessel_app_link::workspace::{Ask, ProjectAsk};
use vessel_app_view::workspace::add_flow::model::{
    Direction, Flow, Machine, PageId, move_selection,
};
use vessel_app_view::workspace::add_flow::options::Method;
use vessel_app_view::workspace::add_flow::rows::{RowAction, searches_directories};
use vessel_app_view::workspace::host_folders::{FOLDERS, PAUSE, offered};
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::projects::ProjectsView;
use vessel_proto::registry::PROJECT_DIRECTORY_PROTOCOL;
use vessel_proto::workspace::host_folders::{FolderSuggestions, SuggestFolders};
use vessel_proto::workspace::suggestions::{SuggestionLimit, SuggestionQuery};
use vessel_proto::workspace::{AddProject, GitSupport};
use vessel_proto::{ProjectId, ProtocolVersion};

use crate::routes::Route;
use crate::shell::Shell;

/// One `folder.suggest` answer and what it was asked for.
#[derive(Debug, Clone)]
pub(super) struct Folders {
    machine: String,
    query: String,
    /// The folders and their `~` forms; `None` when the search failed.
    found: Option<Vec<(String, String)>>,
}

/// The open sheet.
pub(super) struct Open {
    flow: Flow,
    preferred: Option<String>,
    folders: Option<Folders>,
    /// The latest key's pause: a pause that ends after another key asks
    /// nothing.
    wait: u64,
}

/// What a success leads to, on the next frame (it needs the window).
#[derive(Debug, Clone, PartialEq, Eq)]
enum After {
    /// A new chat in the project.
    Chat { machine: String, project: ProjectId },
    /// Settings > Projects on the machine (a clone's progress).
    Projects(String),
}

/// The sheet's state.
pub(crate) struct AddFlowSheet {
    open: Option<Open>,
    field: TextField,
    scroll: ScrollHandle,
    prior: ReturnFocus,
    /// Where the last clone on each machine went (Paseo's
    /// `lastCloneParentByHost`, for the app's life).
    last_clone_parent: HashMap<String, String>,
    after: Option<After>,
}

impl AddFlowSheet {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            open: None,
            field: TextField::with_change(
                "",
                |_shell: &mut Shell, _text, _window, _cx| {},
                Shell::add_flow_typed,
                window,
                cx,
            ),
            scroll: ScrollHandle::new(),
            prior: ReturnFocus::default(),
            last_clone_parent: HashMap::new(),
            after: None,
        }
    }

    /// Whether the sheet is open.
    pub(crate) const fn is_open(&self) -> bool {
        self.open.is_some()
    }
}

/// A machine that reads `project.create_directory` and `~` (ADR 0113).
fn reads_tilde(protocol: Option<ProtocolVersion>) -> bool {
    matches!((protocol, PROJECT_DIRECTORY_PROTOCOL), (Some(speaks), Some(needs)) if speaks >= needs)
}

/// Whether a machine's Git can clone: its list says Git is there and new
/// enough; a machine not read yet may (the page's rule).
fn can_clone(view: Option<&ProjectsView>) -> bool {
    view.and_then(ProjectsView::list)
        .is_none_or(|list| matches!(list.git, GitSupport::Found { .. }))
}

impl Shell {
    fn add_flow_machines(&self) -> Vec<Machine> {
        self.fleet
            .online()
            .map(|machine| Machine {
                name: machine.name.clone(),
                label: machine.name.clone(),
                can_add_project: true,
                // TRACKER 816: on once the app knows its own machine.
                can_browse: false,
                can_clone: can_clone(self.areas.workspace.projects.get(&machine.name)),
                can_search_github: false,
                can_create_directory: reads_tilde(machine.protocol),
            })
            .collect()
    }

    fn protocol_of(&self, machine: &str) -> Option<ProtocolVersion> {
        self.fleet
            .machines
            .iter()
            .find(|known| known.name == machine)
            .and_then(|known| known.protocol)
    }

    /// Open the sheet, on `preferred`'s methods when it is connected.
    pub(crate) fn open_add_flow(
        &mut self,
        preferred: Option<String>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let machines = self.add_flow_machines();
        let sheet = &mut self.areas.workspace.add_flow;
        if sheet.open.is_none() {
            sheet.prior.remember(window, cx);
        }
        sheet.open = Some(Open {
            flow: Flow::open(machines, preferred.as_deref()),
            preferred,
            folders: None,
            wait: 0,
        });
        self.add_flow_shown(window, cx);
    }

    /// Close the sheet; the keys go back where they were.
    pub(crate) fn close_add_flow(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let sheet = &mut self.areas.workspace.add_flow;
        if sheet.open.take().is_none() {
            return;
        }
        sheet.field.clear(window, cx);
        let owner = sheet.field.focus_handle(cx);
        sheet.prior.restore_if_owned(&owner, window, cx);
        cx.notify();
    }

    /// A page was opened or shown again: the field takes its text and
    /// placeholder and the keys, its folders are asked.
    fn add_flow_shown(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let words = self.t.lang.add_flow();
        let sheet = &mut self.areas.workspace.add_flow;
        let Some(open) = &sheet.open else {
            return;
        };
        let page = open.flow.page();
        let placeholder = words.placeholder(page.kind.id()).unwrap_or_default();
        sheet.field.set_placeholder(placeholder, window, cx);
        sheet.field.set_text(&page.field, window, cx);
        sheet.field.focus(window, cx);
        sheet
            .scroll
            .set_offset(point(px(spacing::S0), px(spacing::S0)));
        self.add_flow_ask_folders();
        cx.notify();
    }

    /// The language changed: the field's placeholder in the new words, its
    /// text kept.
    pub(crate) fn refresh_add_flow_words(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let sheet = &self.areas.workspace.add_flow;
        if let Some(open) = &sheet.open {
            let words = self.t.lang.add_flow();
            let placeholder = words
                .placeholder(open.flow.page().kind.id())
                .unwrap_or_default();
            sheet.field.set_placeholder(placeholder, window, cx);
        }
    }

    /// The field changed (typed, or set for a page: the same text then).
    fn add_flow_typed(&mut self, text: String, cx: &mut Context<'_, Self>) {
        let Some(open) = &mut self.areas.workspace.add_flow.open else {
            return;
        };
        if open.flow.page().field == text {
            return;
        }
        open.flow.set_field(&text);
        open.wait = open.wait.wrapping_add(1);
        let wait = open.wait;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(PAUSE).await;
            let _gone = this.update(cx, |shell, cx| {
                let current = shell
                    .areas
                    .workspace
                    .add_flow
                    .open
                    .as_ref()
                    .map(|open| open.wait);
                if current == Some(wait) {
                    shell.add_flow_ask_folders();
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    /// Ask the page's machine for folders matching the field, on a page
    /// that searches them and a machine that answers.
    fn add_flow_ask_folders(&mut self) {
        let Some(open) = &self.areas.workspace.add_flow.open else {
            return;
        };
        let page = open.flow.page();
        let Some(machine) = page.kind.machine().map(str::to_owned) else {
            return;
        };
        if !searches_directories(page.kind.id()) || !offered(self.protocol_of(&machine)) {
            return;
        }
        let Ok(query) = SuggestionQuery::try_from(page.field.clone()) else {
            return;
        };
        self.ask(
            &machine,
            Ask::Project(ProjectAsk::SuggestFolders(SuggestFolders {
                query,
                limit: SuggestionLimit::try_from(FOLDERS).ok(),
            })),
        );
    }

    /// Folders arrived for `query` on `machine`.
    pub(super) fn add_flow_folders(
        &mut self,
        machine: &str,
        query: &str,
        result: Result<FolderSuggestions, Problem>,
    ) {
        let Some(open) = &mut self.areas.workspace.add_flow.open else {
            return;
        };
        open.folders = Some(Folders {
            machine: machine.to_owned(),
            query: query.to_owned(),
            found: result.ok().map(|found| {
                found
                    .folders
                    .into_iter()
                    .map(|folder| (folder.path.display().to_string(), folder.shown))
                    .collect()
            }),
        });
    }

    /// Machines connected or left: while the machine page is the only
    /// page, it skips to the preferred or only one (the method page's field
    /// is hidden, so the field needs no new text).
    pub(super) fn add_flow_fleet(&mut self) {
        let machines = self.add_flow_machines();
        if let Some(open) = &mut self.areas.workspace.add_flow.open {
            open.flow.set_machines(machines, open.preferred.as_deref());
        }
    }

    fn add_flow_key(&mut self, key: CommandKey, window: &mut Window, cx: &mut Context<'_, Self>) {
        match key {
            CommandKey::Up | CommandKey::Down => {
                let rows = render::rows_of(self);
                let Some(open) = &mut self.areas.workspace.add_flow.open else {
                    return;
                };
                let selectable: Vec<bool> = rows.iter().map(|row| !row.disabled).collect();
                let active = open.flow.page().active.min(rows.len().saturating_sub(1));
                let direction = if key == CommandKey::Down {
                    Direction::Next
                } else {
                    Direction::Previous
                };
                let next = move_selection(active, &selectable, direction);
                open.flow.set_active(next);
                // The page's error, when shown, is the list's first child.
                let leading = usize::from(open.flow.page().error.is_some());
                self.areas
                    .workspace
                    .add_flow
                    .scroll
                    .scroll_to_item(next.saturating_add(leading));
                cx.notify();
            }
            CommandKey::Enter => {
                let page = self
                    .areas
                    .workspace
                    .add_flow
                    .open
                    .as_ref()
                    .map(|open| open.flow.page().clone());
                let Some(page) = page else {
                    return;
                };
                if page.kind.id() == PageId::NewDirectoryName {
                    self.add_flow_create(cx);
                    return;
                }
                let rows = render::rows_of(self);
                let active = page.active.min(rows.len().saturating_sub(1));
                if let Some(row) = rows.into_iter().nth(active).filter(|row| !row.disabled) {
                    self.add_flow_choose(row.action, window, cx);
                }
            }
            CommandKey::Escape => self.add_flow_back(window, cx),
        }
    }

    /// One page back; the first page closes the sheet.
    fn add_flow_back(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let went = self
            .areas
            .workspace
            .add_flow
            .open
            .as_mut()
            .is_some_and(|open| open.flow.back());
        if went {
            self.add_flow_shown(window, cx);
        } else {
            self.close_add_flow(window, cx);
        }
    }

    /// A row chosen, by Enter or a click.
    fn add_flow_choose(
        &mut self,
        action: RowAction,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(open) = &mut self.areas.workspace.add_flow.open else {
            return;
        };
        if open.flow.page().submitting {
            return;
        }
        let machine = open.flow.page().kind.machine().map(str::to_owned);
        match (action, machine) {
            (RowAction::Machine(name), _) => open.flow.choose_machine(&name),
            (RowAction::AddMachine, _) => {
                self.close_add_flow(window, cx);
                self.go(Route::Pair, window, cx);
                return;
            }
            (RowAction::Method(method), Some(machine)) => match method {
                Method::DirectorySearch => open.flow.open_directory_search(&machine),
                Method::Github => open.flow.open_github_search(&machine),
                Method::NewDirectory => open.flow.open_new_directory_parent(&machine),
                // Hidden until TRACKER 816.
                Method::Browse => return,
            },
            (RowAction::Repository(repository), Some(machine)) => {
                open.flow.open_github_location(&machine, repository);
            }
            (RowAction::Parent(parent), Some(machine)) => {
                open.flow.open_new_directory_name(&machine, &parent);
            }
            (RowAction::AddPath(path), Some(machine)) => {
                open.flow.begin(PageId::DirectorySearch);
                self.ask(
                    &machine,
                    Ask::Project(ProjectAsk::Add(AddProject {
                        path: path.into(),
                        name: None,
                    })),
                );
                cx.notify();
                return;
            }
            (RowAction::Clone(parent), Some(machine)) => {
                self.add_flow_clone(&machine, parent, cx);
                return;
            }
            (
                RowAction::Method(_)
                | RowAction::Repository(_)
                | RowAction::Parent(_)
                | RowAction::AddPath(_)
                | RowAction::Clone(_),
                None,
            ) => return,
        }
        self.add_flow_shown(window, cx);
    }
}
