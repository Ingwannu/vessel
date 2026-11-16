//! The code panel beside a session (Paseo's Explorer: Changes and Files,
//! `docs/explorer-sidebar.md`, `git/diff-pane.tsx`, `panels/files-panel.tsx`;
//! the page's code panel for what it does): the session's changes with
//! each file's diff in place, and the session's files as a tree with a
//! file shown when chosen.
//!
//! [Decision Log]
//! - Purpose: see what an agent changed and read the code it works on
//!   beside the conversation, without leaving the session.
//! - Constraint: the session screen is the lead's and its bar track C's:
//!   the panel is drawn through two slots (`code_beside`, the session
//!   screen with the panel beside its conversation column, and
//!   `code_toggle` in the bar's actions). The
//!   session's root is not on its card: the panel reads the session
//!   (`session.get`) when it opens on one.
//! - Alternatives: diffs opened in a pane of their own (Paseo's desktop
//!   opens a diff tab in the main pane; Vessel has no panes); every
//!   file's diff read at once (a hundred calls for a monorepo).
//! - Choice: Paseo's Explorer at its default place and width; Changes as
//!   Paseo's diff pane draws it (files expanded, folded by a click), each
//!   file's diff read when its row first comes into view; Files as a tree
//!   read folder by folder, a file shown in the panel with Back.
//! - Cost: the panel follows the open session only while it is open;
//!   reopening it reads the session again (one call).

mod actions;
mod actions_view;
mod appear;
mod change_rows;
mod changes;
mod download;
mod editing;
mod editing_view;
mod editor_status;
mod elsewhere;
mod files;
mod found;
mod frame;
mod line_syntax;
mod search;
mod shown_source;
mod source_acquisition;
mod source_answer;
mod source_follow;
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod source_tests;
mod tree;
mod viewer;
mod watch;

use std::collections::{HashMap, HashSet};

use vessel_app_kit::ui::{Context, Window};
use vessel_app_kit::{ComboboxState, VirtualList};
use vessel_app_view::workspace::changes::{ChangeRow, FileState, ScopeKind, change_rows};
use vessel_app_view::workspace::file_tree::{Listing, TreeRow, tree_rows};
use vessel_app_view::workspace::problem::Problem;

use vessel_proto::SessionId;
use vessel_proto::workspace::{DiffSummary, RelPath, WorkspaceRoot};

pub(crate) use elsewhere::FindRefused;
pub(crate) use frame::{code_beside, code_toggle, frame_row};
#[cfg(test)]
pub(crate) use shown_source::{ShownChangesContext, ShownDiffContext};

use crate::shell::Shell;
use vessel_app_link::workspace::Ask;

/// Which tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Changes,
    Files,
}

/// Where the open session runs, as far as the panel knows.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Place {
    /// `session.get` is out.
    Reading,
    /// In a project's folder or a worktree.
    Root(WorkspaceRoot),
    /// In no project: nothing to show.
    Outside,
    /// The read failed.
    Failed(String),
}

/// The session the panel shows.
struct Target {
    machine: String,
    session: SessionId,
    place: Place,
    /// The session's folder on the machine, for "Copy path".
    cwd: Option<std::path::PathBuf>,
    receipt: Option<shown_source::RootReceipt>,
}

/// A comparison's files.
enum Summary {
    Loading,
    Ready(DiffSummary),
    Failed(String),
}

/// The Changes tab.
struct Changes {
    scope: ScopeKind,
    summary: Summary,
    /// Files folded by the person.
    closed: HashSet<RelPath>,
    states: HashMap<RelPath, FileState>,
    rows: Vec<ChangeRow>,
    list: VirtualList,
    picker: ComboboxState,
    sources: shown_source::Sources,
}

/// A file shown in the Files tab.
struct Viewer {
    path: RelPath,
    text: Shown,
    list: VirtualList,
    /// The line to bring to the top once read (from 1: a search's hit).
    goto: Option<u32>,
    /// The lines chosen for the composer (from 0, first and last).
    chosen: Option<(usize, usize)>,
    /// The file edited in place (a desktop's width, at most 1 MiB of
    /// text); `None`: shown read-only by the lines above.
    edit: Option<editing::Editing>,
    /// Its watch while shown (protocol 27, ADR 0105).
    watch: vessel_app_view::workspace::watch::Watch,
}

/// What a file shows.
enum Shown {
    Loading,
    Text {
        lines: Vec<String>,
        states: Vec<vessel_app_view::workspace::syntax::LineState>,
        more: bool,
    },
    Binary,
    Failed(String),
}

/// The Files tab.
struct Files {
    listings: HashMap<RelPath, Listing>,
    open: HashSet<RelPath>,
    rows: Vec<TreeRow>,
    list: VirtualList,
    viewer: Option<Viewer>,
    search: search::Search,
    actions: actions::Actions,
    /// Whether the tab was last drawn at a phone's width, where a file is
    /// shown read-only (Paseo edits on a desktop only); set as it draws.
    narrow: std::cell::Cell<bool>,
}

/// The panel's state.
pub(crate) struct CodePanel {
    /// Whether it is out.
    pub(crate) open: bool,
    tab: Tab,
    target: Option<Target>,
    changes: Changes,
    files: Files,
    /// Its left edge, dragged to resize it (`frame::PANEL`).
    edge: crate::areas::edge::PanelEdge,
    /// The width of the row of conversation and panel, last frame.
    row: std::rc::Rc<std::cell::Cell<Option<f32>>>,
    /// Tabs whose content a git write changed while they were not shown:
    /// read again when next shown.
    stale: Stale,
    /// Each session's root for finds from other areas (`code_find_files`).
    finder: elsewhere::Finder,
    tickets: vessel_app_view::workspace::file_ticket::FileTickets,
}

/// Which tabs are to be read again when shown.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Stale {
    changes: bool,
    files: bool,
}

impl CodePanel {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            open: false,
            tab: Tab::Changes,
            target: None,
            changes: Changes {
                scope: ScopeKind::Uncommitted,
                summary: Summary::Loading,
                closed: HashSet::new(),
                states: HashMap::new(),
                rows: Vec::new(),
                list: VirtualList::new(false),
                picker: ComboboxState::new(window, cx),
                sources: shown_source::Sources::default(),
            },
            files: Files {
                listings: HashMap::new(),
                open: HashSet::new(),
                rows: Vec::new(),
                list: VirtualList::new(false),
                viewer: None,
                search: search::Search::new(window, cx),
                actions: actions::Actions::new(cx),
                narrow: std::cell::Cell::new(false),
            },
            edge: crate::areas::edge::PanelEdge::new(),
            row: std::rc::Rc::new(std::cell::Cell::new(None)),
            stale: Stale::default(),
            finder: elsewhere::Finder::default(),
            tickets: vessel_app_view::workspace::file_ticket::FileTickets::default(),
        }
    }

    /// The machine was renamed in the book: the panel goes on under the
    /// new name.
    pub(in crate::areas::workspace) fn machine_renamed(&mut self, from: &str, to: &str) {
        self.finder.machine_gone(from);
        if let Some(target) = self.target.as_mut().filter(|target| target.machine == from) {
            target.receipt = None;
            self.changes.sources.invalidate();
            to.clone_into(&mut target.machine);
        }
    }

    /// The machine was forgotten: nothing of it is shown or read again.
    pub(in crate::areas::workspace) fn machine_forgot(&mut self, name: &str) {
        self.finder.machine_gone(name);
        if self
            .target
            .as_ref()
            .is_some_and(|target| target.machine == name)
        {
            self.changes.sources.invalidate();
            self.target = None;
            self.changes.summary = Summary::Loading;
            self.changes.states.clear();
            self.changes.relist();
            self.files.listings.clear();
            self.files.viewer = None;
            self.files.relist();
        }
    }

    /// The session's folder on the machine, when it said.
    fn cwd(&self) -> Option<&std::path::Path> {
        self.target.as_ref()?.cwd.as_deref()
    }

    /// The root shown, when the session is in one.
    fn root(&self) -> Option<(&str, &WorkspaceRoot)> {
        let target = self.target.as_ref()?;
        match &target.place {
            Place::Root(root) => Some((target.machine.as_str(), root)),
            Place::Reading | Place::Outside | Place::Failed(_) => None,
        }
    }
}

impl Changes {
    /// The rows again, after the summary, a fold or a diff changed.
    fn relist(&mut self) {
        self.rows = match &self.summary {
            Summary::Ready(summary) => change_rows(&summary.files, &self.closed, &self.states),
            Summary::Loading | Summary::Failed(_) => Vec::new(),
        };
        self.list.reset(self.rows.len());
    }
}

impl Files {
    fn relist(&mut self) {
        self.rows = tree_rows(&self.listings, &self.open);
        self.list.reset(self.row_count());
    }

    /// The list's rows: the tree's, and the row a name is typed in.
    fn row_count(&self) -> usize {
        let extra = self
            .actions
            .editing
            .as_ref()
            .and_then(|editing| actions::edit_slot(&self.rows, &editing.kind))
            .is_some_and(|(_, replaces)| !replaces);
        self.rows.len().saturating_add(usize::from(extra))
    }

    /// What list row `index` draws: the typed name's row (at its depth),
    /// or the tree's row at an index.
    fn slot(&self, index: usize) -> Slot {
        let Some((at, replaces)) = self
            .actions
            .editing
            .as_ref()
            .and_then(|editing| actions::edit_slot(&self.rows, &editing.kind))
        else {
            return Slot::Row(index);
        };
        if index == at {
            let depth = self.rows.get(at).map_or(0, TreeRow::depth);
            return Slot::Edit(depth);
        }
        if index > at && !replaces {
            return Slot::Row(index.saturating_sub(1));
        }
        Slot::Row(index)
    }
}

/// What a row of the tree's list draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    /// The tree's row at this index.
    Row(usize),
    /// The row a name is typed in, at this depth.
    Edit(usize),
}

impl Shell {
    /// The session the panel should show: the open one.
    fn code_session(&self) -> Option<(String, SessionId)> {
        let open = self.areas.session.open.as_ref()?;
        Some((open.machine.clone(), open.session))
    }

    /// Open or close the panel; opening it says the search field's words
    /// in the person's language (the field is made before the language is
    /// known). The bar's toggle and track S2's "toggle inspector"
    /// shortcut both call this; with no session open it does nothing (the
    /// panel is a session's), as the toggle is not drawn then. At a
    /// phone's width the panel it opens takes the whole screen, as from the
    /// toggle.
    /// Something was put in the session's composer from the panel: at a
    /// phone's width the panel covers the composer, so it closes and the
    /// composer is seen with what was added (Paseo's compact layout closes
    /// its Explorer sheet when a choice goes to the main pane); beside the
    /// conversation it stays.
    fn code_handed_to_composer(&mut self, window: &Window) {
        if vessel_app_kit::breakpoint::phone(window) {
            self.areas.workspace.code.changes.sources.invalidate();
            self.areas.workspace.code.open = false;
        }
    }

    pub(crate) fn toggle_code(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.areas.session.open.is_none() {
            return;
        }
        let panel = &mut self.areas.workspace.code;
        panel.changes.sources.invalidate();
        panel.open = !panel.open;
        if panel.open {
            self.search_words(window, cx);
            self.code_follow();
            let _asked = self.code_reread_shown();
            self.code_watch();
        }
        cx.notify();
    }

    /// Track G1's call after a git write it made succeeded (a commit, a
    /// switch, a stash, a discard): when the panel shows `root` on
    /// `machine`, the tab shown is read again, what it shows staying until
    /// the new answers replace it (no flash of an empty panel); a tab not
    /// shown, or the whole panel while closed, is read again when it is
    /// next shown. Another machine or root: nothing.
    pub(crate) fn code_git_changed(
        &mut self,
        machine: &str,
        root: &WorkspaceRoot,
        cx: &mut Context<'_, Self>,
    ) {
        let panel = &mut self.areas.workspace.code;
        if panel.root() != Some((machine, root)) {
            return;
        }
        panel.changes.sources.invalidate();
        panel.stale = Stale {
            changes: true,
            files: true,
        };
        if panel.open {
            let _asked = self.code_reread_shown();
        }
        cx.notify();
    }

    /// Track G1's read of what the Changes tab shows: the machine and the
    /// root, only while the panel is out on Changes (G1 refreshes git's
    /// status while it is). No reading, no change.
    pub(crate) fn code_git_context(&self) -> Option<(&str, &WorkspaceRoot)> {
        let panel = &self.areas.workspace.code;
        if panel.open && panel.tab == Tab::Changes {
            panel.root()
        } else {
            None
        }
    }

    /// Track G1's read of the panel's width beside the conversation, for
    /// how much of the git toolbar's label fits.
    pub(crate) fn code_git_width(&self) -> Option<vessel_app_kit::ui::Pixels> {
        let panel = &self.areas.workspace.code;
        // The width the frame draws (word 29's edge): the dragged or kept
        // one, held inside the row last measured.
        let room = panel.row.get().unwrap_or(f32::INFINITY);
        panel
            .open
            .then(|| vessel_app_kit::ui::px(frame::PANEL.width(self, room)))
    }

    /// Read the shown tab again if a git write changed it while hidden;
    /// `true` when it was asked again.
    fn code_reread_shown(&mut self) -> bool {
        let panel = &mut self.areas.workspace.code;
        match panel.tab {
            Tab::Changes if panel.stale.changes => {
                panel.stale.changes = false;
                self.reread_changes();
                true
            }
            Tab::Files if panel.stale.files => {
                panel.stale.files = false;
                self.reread_files();
                true
            }
            Tab::Changes | Tab::Files => false,
        }
    }

    /// Read what the chosen tab shows, if not read yet, or again when a
    /// git write changed it while it was hidden.
    fn code_read_tab(&mut self) {
        let reread = self.code_reread_shown();
        match self.areas.workspace.code.tab {
            // Asked again already: a second ask would answer twice.
            Tab::Changes if reread => {}
            Tab::Changes => self.read_changes(),
            // A root never listed is not among the folders read again.
            Tab::Files => self.read_folder(&RelPath::root()),
        }
        self.code_watch();
    }
}
