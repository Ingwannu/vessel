//! A tree entry's actions (Paseo's `components/file-actions-menu.tsx`, opened
//! by a right click on a row as Paseo's `ContextMenuTrigger`): its groups
//! in Paseo's order with a hairline between them (open; create; reference;
//! manage; destructive), a new file's, a new folder's or a new name typed
//! in a row of the tree in place (Paseo's inline entry), and a delete
//! confirmed in the kit's dialog with Paseo's words.
//!
//! [Decision Log]
//! - Purpose: make, rename and delete files and folders, and hand a path
//!   to the conversation, from the tree, as Paseo's Files does.
//! - Constraint: the node moves and deletes inside the root only and
//!   never over an existing path (`file.rename`, `file.delete`); a new
//!   file is a write that expects nothing there.
//! - Choice: Paseo's menu items that the node can do; Paseo's "Open in
//!   editor", "Reveal in", "Download", "Duplicate" and "Open to side" are
//!   left out (no editor or file manager on the machine is reachable
//!   from here; additional file panes await shared scope admission). "Add to chat" puts the
//!   path, as `@path`, at the end of the session's draft (Paseo attaches
//!   a chip; the draft here is text).
//! - Cost: a rename or a new entry is typed in the tree, so the tree's
//!   rows carry one extra row while it is typed.

use vessel_app_kit::dialog::DialogState;
use vessel_app_kit::ui::{ClipboardItem, Context, Pixels, Point, Window};
use vessel_app_kit::{MenuState, TextField};
use vessel_app_link::workspace::PathChange;
use vessel_app_view::workspace::file_scope::FileScope;
use vessel_app_view::workspace::file_ticket::FileTicket;
use vessel_app_view::workspace::file_tree::TreeRow;
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::workspace::{CreateDir, DeletePath, RelPath, RenamePath};

use crate::shell::Shell;

/// What is typed in the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum EditKind {
    /// A new file in this folder.
    NewFile(RelPath),
    /// A new folder in this folder.
    NewFolder(RelPath),
    /// A new name for this entry.
    Rename(RelPath),
}

/// A name being typed.
pub(super) struct Editing {
    pub(super) opening: EntryOpening,
    pub(super) kind: EditKind,
    pub(super) field: TextField,
}

/// The entry a menu was opened on.
#[derive(Debug, Clone)]
pub(super) struct Target {
    pub(super) opening: EntryOpening,
    pub(super) path: RelPath,
    pub(super) folder: bool,
    pub(super) open: bool,
}

/// One action of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    OpenFile,
    CollapseFolder,
    NewFile,
    NewFolder,
    CopyPath,
    CopyRelativePath,
    AddToChat,
    Rename,
    Delete,
}

/// A menu/form capability is tied to its original root and one opening.
#[derive(Debug, Clone)]
pub(super) struct EntryOpening {
    pub(super) scope: FileScope,
    pub(super) ticket: FileTicket,
}
impl EntryOpening {
    pub(super) fn current(&self, panel: &super::CodePanel) -> bool {
        panel.scope().as_ref() == Some(&self.scope)
    }
}
#[derive(Debug, Clone, Copy)]
pub(super) enum RemovalDepth {
    File,
    DirectoryTree,
}
pub(super) struct DeletePrompt {
    pub(super) opening: EntryOpening,
    pub(super) path: RelPath,
    pub(super) depth: RemovalDepth,
}

/// The tree's actions' state.
pub(super) struct Actions {
    pub(super) menu: MenuState,
    pub(super) target: Option<Target>,
    pub(super) editing: Option<Editing>,
    /// A delete waiting for its confirm: the entry and whether a folder.
    pub(super) deleting: Option<DeletePrompt>,
    pub(super) dialog: DialogState,
    /// Why the last change was refused (the machine's reason).
    pub(super) refused: Option<String>,
}

impl Actions {
    pub(super) fn invalidate(&mut self) {
        self.menu.close();
        self.target = None;
        self.editing = None;
        self.deleting = None;
    }

    pub(super) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            menu: MenuState::new(cx),
            target: None,
            editing: None,
            deleting: None,
            dialog: DialogState::new(cx),
            refused: None,
        }
    }
}

/// Where the row being typed sits among the tree's rows: its index and
/// whether it takes the place of the row there (a rename) or comes
/// before it (a new entry, first in its folder).
pub(super) fn edit_slot(rows: &[TreeRow], kind: &EditKind) -> Option<(usize, bool)> {
    let entry_at = |path: &RelPath| {
        rows.iter().position(|row| match row {
            TreeRow::Entry { path: at, .. } => at == path,
            TreeRow::Loading { .. }
            | TreeRow::Failed { .. }
            | TreeRow::More { .. }
            | TreeRow::Empty { .. } => false,
        })
    };
    match kind {
        EditKind::Rename(path) => entry_at(path).map(|index| (index, true)),
        EditKind::NewFile(dir) | EditKind::NewFolder(dir) => {
            if dir.is_root() {
                Some((0, false))
            } else {
                entry_at(dir).map(|index| (index.saturating_add(1), false))
            }
        }
    }
}

/// The menu's items for `target`, in Paseo's groups and order.
pub(super) fn items_for(target: &Target) -> Vec<Action> {
    let mut items = Vec::new();
    if target.folder {
        items.extend([Action::NewFile, Action::NewFolder]);
        if target.open {
            items.push(Action::CollapseFolder);
        }
    } else {
        items.push(Action::OpenFile);
    }
    items.extend([
        Action::CopyPath,
        Action::CopyRelativePath,
        Action::AddToChat,
        Action::Rename,
        Action::Delete,
    ]);
    items
}

/// Paseo's group of an action (a hairline starts each group).
pub(super) const fn group(action: Action) -> u8 {
    match action {
        Action::OpenFile | Action::CollapseFolder => 0,
        Action::NewFile | Action::NewFolder => 1,
        Action::CopyPath | Action::CopyRelativePath | Action::AddToChat => 2,
        Action::Rename => 3,
        Action::Delete => 4,
    }
}

impl Shell {
    pub(super) fn entry_opening(&mut self) -> Option<EntryOpening> {
        let scope = self.areas.workspace.code.scope()?;
        Some(EntryOpening {
            scope,
            ticket: self.next_file_ticket()?,
        })
    }
}

impl Shell {
    /// Open the menu on an entry, at the pointer.
    pub(super) fn open_file_actions(
        &mut self,
        path: RelPath,
        folder: bool,
        open: bool,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(opening) = self.entry_opening() else {
            return;
        };
        let actions = &mut self.areas.workspace.code.files.actions;
        actions.target = Some(Target {
            opening,
            path,
            folder,
            open,
        });
        actions.menu.open(at, window, cx);
        cx.notify();
    }

    pub(super) fn file_action(
        &mut self,
        ticket: FileTicket,
        action: Action,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self
            .areas
            .workspace
            .code
            .files
            .actions
            .target
            .as_ref()
            .is_none_or(|target| {
                target.opening.ticket != ticket
                    || !target.opening.current(&self.areas.workspace.code)
            })
        {
            return;
        }
        let Some(target) = self.areas.workspace.code.files.actions.target.take() else {
            return;
        };
        self.areas.workspace.code.files.actions.menu.close();
        match action {
            Action::OpenFile => self.open_file_at(target.path, None, cx),
            Action::CollapseFolder => self.toggle_folder(&target.path, cx),
            Action::NewFile => {
                self.start_entry_form(EditKind::NewFile(target.path.clone()), target, window, cx);
            }
            Action::NewFolder => {
                self.start_entry_form(EditKind::NewFolder(target.path.clone()), target, window, cx);
            }
            Action::Rename => {
                self.start_entry_form(EditKind::Rename(target.path.clone()), target, window, cx);
            }
            Action::CopyPath | Action::CopyRelativePath => {
                let relative = target.path.as_str().to_owned();
                let text = match (action, self.areas.workspace.code.cwd()) {
                    (Action::CopyPath, Some(cwd)) => cwd.join(&relative).display().to_string(),
                    _ => relative,
                };
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            Action::AddToChat => {
                let composer = &self.areas.session.composer;
                let draft = composer.text(cx);
                let mention = format!("@{} ", target.path.as_str());
                let text = if draft.is_empty() || draft.ends_with(char::is_whitespace) {
                    format!("{draft}{mention}")
                } else {
                    format!("{draft} {mention}")
                };
                composer.set_text(&text, window, cx);
                composer.focus(window, cx);
                self.code_handed_to_composer(window);
            }
            Action::Delete => {
                let actions = &mut self.areas.workspace.code.files.actions;
                actions.deleting = Some(DeletePrompt {
                    opening: target.opening,
                    path: target.path,
                    depth: if target.folder {
                        RemovalDepth::DirectoryTree
                    } else {
                        RemovalDepth::File
                    },
                });
                actions.dialog.focus(window, cx);
            }
        }
        cx.notify();
    }

    /// A change of a path answered: list its folder again (and the old
    /// one's, for a move), or say why not.
    pub(in crate::areas::workspace) fn path_changed(
        &mut self,
        change: &PathChange,
        result: Result<(), Problem>,
    ) {
        let words = self.words();
        let files = &mut self.areas.workspace.code.files;
        if let Err(problem) = result {
            files.actions.refused = Some(words.problem(&problem));
            return;
        }
        files.actions.refused = None;
        files.actions.editing = None;
        let touched: Vec<RelPath> = match change {
            PathChange::CreateDir(CreateDir { path, .. })
            | PathChange::CreateFile { path, .. }
            | PathChange::Delete(DeletePath { path, .. }) => vec![path.clone()],
            PathChange::Rename(RenamePath { from, to, .. }) => vec![from.clone(), to.clone()],
        };
        let folders: Vec<RelPath> = touched
            .iter()
            .map(|path| path.parent().unwrap_or_else(RelPath::root))
            .collect();
        for dir in &folders {
            let _was = files.listings.remove(dir);
        }
        for dir in &folders {
            self.read_folder(dir);
        }
        if let PathChange::CreateFile { path, .. } = change {
            self.open_file_at_quietly(path.clone());
        }
    }
}
