//! Where a new chat runs, in the tray under the composer's card: the
//! desktop lead's `draft/pickers.rs` draws the pills and opens the kit's
//! lists; the rows, what a chosen row does and the start in a project are
//! here. Paseo's new-workspace form (`screens/new-workspace/form/`):
//! Project (its rows lead with the project's mark, its folder under the
//! name, "Add project" last), Isolation ("Local" or "New worktree", only for
//! a repository) and, for a new worktree, Branch ("Start from").
//!
//! [Decision Log]
//! - Purpose: start a chat in a machine's project, as it is or in a new
//!   worktree, without typing a path (page inventory section 5).
//! - Constraint: the tray's folder pill is the lead's and stays for machines
//!   without projects and for folders no project holds; Paseo's form has
//!   no folder field (a Paseo workspace is always a project's).
//! - Alternatives: a project field beside the folder pill (two pills naming
//!   one place); a dropdown of this track's own (the owner's rule: the
//!   lead's popover-and-list for every picker).
//! - Choice: the folder pill's list leads with the machine's projects
//!   (Paseo's project rows), then the folders as the lead has them; a
//!   project chosen turns the pill into the project's (mark and name) and
//!   brings Paseo's Isolation and Branch pills after it.
//! - Cost: one list holds both kinds; the headings ("Projects", "Folders")
//!   keep them apart. Isolation is remembered while the app runs, not
//!   across launches as Paseo's preference is (the lead's preferences file
//!   is not this track's).

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use vessel_app_kit::ui::{Context, Window};
use vessel_app_kit::{ComboRow, ComboboxState};
use vessel_app_link::workspace::{Ask, GitAsk, StartAsk, StartKey};
use vessel_app_view::workspace::place::{Place, PlaceMode, ProjectPlace, place_problem};
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::setup_log::SetupLog;
use vessel_app_view::workspace::start::Step;
use vessel_proto::workspace::{BranchList, ProjectInfo, RefName, WorkspaceRoot};
use vessel_proto::{HarnessRef, OptionChoice, ProjectId, SessionMode, SessionSpec};

use super::start_card::Starting;
use crate::shell::Shell;

/// The tray's place, beside the lead's draft.
pub(crate) struct DraftPlace {
    /// The project chosen and the machine it is on (a choice made on
    /// another machine does not apply).
    chosen: Option<(String, ProjectPlace)>,
    /// The isolation chosen last (Paseo remembers it for the next project).
    mode: PlaceMode,
    /// Isolation's list.
    pub(crate) isolation: ComboboxState,
    /// Start from's list.
    pub(crate) base: ComboboxState,
    /// Each project's branches as last read.
    pub(super) branches: HashMap<(String, ProjectId), Branches>,
    /// Where the place, isolation and branch pills are drawn, for their
    /// tooltips (the composer's, one shown at most).
    pub(crate) tips: [vessel_app_kit::tooltip::Place; 3],
}

/// A project's branches, for Start from.
pub(super) enum Branches {
    Loading,
    Ready(BranchList),
    Failed(Problem),
}

impl DraftPlace {
    /// The machine was renamed in the book.
    pub(super) fn machine_renamed(&mut self, from: &str, to: &str) {
        if let Some((machine, _)) = self.chosen.as_mut().filter(|(machine, _)| machine == from) {
            to.clone_into(machine);
        }
        super::book::rekey_pairs(&mut self.branches, from, to);
    }

    /// The machine was forgotten.
    pub(super) fn machine_forgot(&mut self, name: &str) {
        if self
            .chosen
            .as_ref()
            .is_some_and(|(machine, _)| machine == name)
        {
            self.chosen = None;
        }
        super::book::forget_pairs(&mut self.branches, name);
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            chosen: None,
            mode: PlaceMode::Folder,
            isolation: ComboboxState::new(window, cx),
            base: ComboboxState::new(window, cx),
            branches: HashMap::new(),
            tips: [
                Rc::new(Cell::new(None)),
                Rc::new(Cell::new(None)),
                Rc::new(Cell::new(None)),
            ],
        }
    }
}

/// What a row of the place lists means.
#[derive(Debug, Clone)]
pub(crate) enum PlacePick {
    /// Run in this project.
    Project(ProjectId),
    /// Add a project (Settings > Projects, its add menu open).
    AddProject,
    /// Local or a new worktree.
    Isolation(PlaceMode),
    /// Where the new worktree starts (`None`: the project's default).
    Base(Option<RefName>),
}

/// A list's rows, each with what it means.
pub(crate) type Rows = Vec<(ComboRow, PlacePick)>;

impl Shell {
    /// The place chosen, when it is on the draft's machine.
    pub(crate) fn chosen_place(&self) -> Option<&ProjectPlace> {
        let machine = self.areas.draft.machine.as_deref()?;
        let (on, place) = self.areas.workspace.place.chosen.as_ref()?;
        (on == machine).then_some(place)
    }

    /// The chosen project as last listed.
    pub(super) fn chosen_project(&self) -> Option<&ProjectInfo> {
        let machine = self.areas.draft.machine.as_deref()?;
        let place = self.chosen_place()?;
        self.areas
            .workspace
            .projects
            .get(machine)?
            .project(&place.project)
    }

    /// A folder was chosen instead of a project.
    pub(crate) fn forget_place(&mut self) {
        self.areas.workspace.place.chosen = None;
    }

    /// A row of a place list was chosen.
    pub(crate) fn choose_place(
        &mut self,
        pick: PlacePick,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(machine) = self.areas.draft.machine.clone() else {
            return;
        };
        match pick {
            PlacePick::Project(project) => {
                let git = self
                    .areas
                    .workspace
                    .projects
                    .get(&machine)
                    .and_then(|view| view.project(&project))
                    .is_some_and(|info| info.git);
                let mut place = ProjectPlace::new(project);
                if git {
                    place.mode = self.areas.workspace.place.mode;
                }
                self.areas.workspace.place.chosen = Some((machine.clone(), place));
                if git {
                    self.read_branches(&machine, project);
                }
            }
            PlacePick::AddProject => {
                self.areas.settings.machine = Some(machine);
                self.open_settings(
                    Some(vessel_app_view::settings::Section::Projects),
                    window,
                    cx,
                );
                self.open_add_menu();
            }
            PlacePick::Isolation(mode) => {
                self.areas.workspace.place.mode = mode;
                let project = self
                    .areas
                    .workspace
                    .place
                    .chosen
                    .as_mut()
                    .map(|(_, place)| {
                        place.mode = mode;
                        place.project
                    });
                if let Some(project) = project {
                    self.read_branches(&machine, project);
                }
            }
            PlacePick::Base(base) => {
                if let Some((_, place)) = self.areas.workspace.place.chosen.as_mut() {
                    place.base = base;
                }
            }
        }
        cx.notify();
    }

    /// Read a project's branches for Start from, unless read already.
    fn read_branches(&mut self, machine: &str, project: ProjectId) {
        let key = (machine.to_owned(), project);
        if matches!(
            self.areas.workspace.place.branches.get(&key),
            Some(Branches::Ready(_) | Branches::Loading)
        ) {
            return;
        }
        let _was = self
            .areas
            .workspace
            .place
            .branches
            .insert(key, Branches::Loading);
        self.ask(
            machine,
            Ask::Git(GitAsk::Branches(WorkspaceRoot::Project { project })),
        );
    }

    /// A project's branches arrived.
    pub(super) fn branches_read(
        &mut self,
        machine: &str,
        root: &WorkspaceRoot,
        result: Result<BranchList, Problem>,
    ) {
        let WorkspaceRoot::Project { project, .. } = root else {
            return;
        };
        let read = match result {
            Ok(list) => Branches::Ready(list),
            Err(problem) => Branches::Failed(problem),
        };
        let _was = self
            .areas
            .workspace
            .place
            .branches
            .insert((machine.to_owned(), *project), read);
    }

    /// Why Send cannot start where the tray says (`None`: it can, or no
    /// project is chosen).
    pub(crate) fn place_blocked(&self) -> Option<String> {
        let place = self.chosen_place()?;
        place_problem(&Place::Project(place.clone()), self.chosen_project())
            .map(|problem| self.words().place(problem))
    }

    /// Start the draft in the chosen project: in its folder, or in a new
    /// worktree made first, each step shown on the session's screen
    /// (`start_card`). `false` when no project is chosen: the lead's
    /// `Command::Start` runs in the folder.
    pub(crate) fn start_in_place(
        &mut self,
        machine: &str,
        harness: &HarnessRef,
        options: &[OptionChoice],
        prompt: &str,
    ) -> bool {
        let Some(place) = self.chosen_place().cloned() else {
            return false;
        };
        let project = self
            .chosen_project()
            .map_or_else(String::new, |project| project.name.to_string());
        let mode = SessionMode::Structured {
            model: None,
            resume: None,
            options: options.to_vec(),
        };
        let spec = SessionSpec::new(harness.clone(), mode);
        let (answers, asked) = tokio::sync::mpsc::channel(4);
        let workspace = &mut self.areas.workspace;
        workspace.starts = workspace.starts.wrapping_add(1);
        let key = StartKey(workspace.starts);
        workspace.start = Some(Starting {
            key,
            machine: machine.to_owned(),
            project,
            worktree: place.mode != PlaceMode::Folder,
            step: Step::Idle,
            log: SetupLog::default(),
            answers: Some(answers),
            folded: false,
        });
        self.ask(
            machine,
            Ask::Start(Box::new(StartAsk {
                key,
                place,
                spec,
                prompt: prompt.to_owned(),
                answers: asked,
            })),
        );
        true
    }
}
