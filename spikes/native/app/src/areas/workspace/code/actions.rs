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
//!   editor", "Reveal in" and "Open to side" are left out (no editor or
//!   file manager on the machine is reachable from here, and there is no
//!   second pane). "Duplicate" (`file.duplicate`) and a file's "Download"
//!   (`file.read_bytes`, `download.rs`) are offered only by a machine with
//!   the file panels' calls (ADR 0105), as Paseo hides Duplicate without
//!   `fsEntryDuplicateEnabled`; a duplicate's folder is listed again
//!   (Paseo also selects the copy; this tree has no selected row). "Add to
//!   chat" puts the path, as `@path`, at the end of the session's draft
//!   (Paseo attaches a chip; the draft here is text).
//! - Cost: a rename or a new entry is typed in the tree, so the tree's
//!   rows carry one extra row while it is typed.

use vessel_app_kit::dialog::DialogState;
use vessel_app_kit::ui::{ClipboardItem, Context, Pixels, Point, Window};
use vessel_app_kit::{MenuState, TextField};
use vessel_app_link::workspace::{Ask, FileAsk, PathChange};
use vessel_app_view::workspace::file_panels::offered;
use vessel_app_view::workspace::file_tree::TreeRow;
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::workspace::panel_files::{DuplicateFileEntry, DuplicatedFileEntry};
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
    pub(super) kind: EditKind,
    pub(super) field: TextField,
}

/// The entry a menu was opened on.
#[derive(Debug, Clone)]
pub(super) struct Target {
    pub(super) path: RelPath,
    pub(super) folder: bool,
    pub(super) open: bool,
    /// Its machine has the file panels' calls (`file.duplicate`).
    pub(super) panels: bool,
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
    Download,
    AddToChat,
    Rename,
    Duplicate,
    Delete,
}

/// The tree's actions' state.
pub(super) struct Actions {
    pub(super) menu: MenuState,
    pub(super) target: Option<Target>,
    pub(super) editing: Option<Editing>,
    /// A delete waiting for its confirm: the entry and whether a folder.
    pub(super) deleting: Option<(RelPath, bool)>,
    pub(super) dialog: DialogState,
    /// Why the last change was refused (the machine's reason).
    pub(super) refused: Option<String>,
}

impl Actions {
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
    items.extend([Action::CopyPath, Action::CopyRelativePath]);
    if target.panels && !target.folder {
        items.push(Action::Download);
    }
    items.extend([Action::AddToChat, Action::Rename]);
    if target.panels {
        items.push(Action::Duplicate);
    }
    items.push(Action::Delete);
    items
}

/// Paseo's group of an action (a hairline starts each group).
pub(super) const fn group(action: Action) -> u8 {
    match action {
        Action::OpenFile | Action::CollapseFolder => 0,
        Action::NewFile | Action::NewFolder => 1,
        Action::CopyPath | Action::CopyRelativePath | Action::Download | Action::AddToChat => 2,
        Action::Rename | Action::Duplicate => 3,
        Action::Delete => 4,
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
        let panels = self
            .areas
            .workspace
            .code
            .root()
            .is_some_and(|(machine, _)| {
                offered(
                    self.fleet
                        .machines
                        .iter()
                        .find(|known| known.name == machine)
                        .and_then(|known| known.protocol),
                )
            });
        let actions = &mut self.areas.workspace.code.files.actions;
        actions.target = Some(Target {
            path,
            folder,
            open,
            panels,
        });
        actions.menu.open(at, window, cx);
        cx.notify();
    }

    pub(super) fn file_action(
        &mut self,
        action: Action,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(target) = self.areas.workspace.code.files.actions.target.take() else {
            return;
        };
        self.areas.workspace.code.files.actions.menu.close();
        let labels = self.words().labels();
        match action {
            Action::OpenFile => self.open_file_at(target.path, None, cx),
            Action::CollapseFolder => self.toggle_folder(&target.path, cx),
            Action::NewFile | Action::NewFolder | Action::Rename => {
                let kind = match action {
                    Action::NewFile => EditKind::NewFile(target.path.clone()),
                    Action::NewFolder => EditKind::NewFolder(target.path.clone()),
                    Action::Rename
                    | Action::OpenFile
                    | Action::CollapseFolder
                    | Action::CopyPath
                    | Action::CopyRelativePath
                    | Action::Download
                    | Action::AddToChat
                    | Action::Duplicate
                    | Action::Delete => EditKind::Rename(target.path.clone()),
                };
                if !matches!(kind, EditKind::Rename(_)) && !target.open {
                    self.toggle_folder(&target.path, cx);
                }
                let field = TextField::new(
                    labels.name_placeholder,
                    |shell: &mut Shell, text, _window, cx| shell.finish_edit(&text, cx),
                    window,
                    cx,
                );
                if let EditKind::Rename(path) = &kind {
                    field.set_text(path.file_name().unwrap_or_default(), window, cx);
                }
                field.focus(window, cx);
                let files = &mut self.areas.workspace.code.files;
                files.actions.refused = None;
                files.actions.editing = Some(Editing { kind, field });
                files.relist();
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
            Action::Download => self.choose_download(target.path, cx),
            Action::Duplicate => {
                let root = self
                    .areas
                    .workspace
                    .code
                    .root()
                    .map(|(machine, root)| (machine.to_owned(), *root));
                if let Some((machine, root)) = root {
                    let ask = DuplicateFileEntry {
                        root,
                        path: target.path,
                    };
                    self.ask(&machine, Ask::File(FileAsk::Duplicate(ask)));
                }
            }
            Action::Delete => {
                let actions = &mut self.areas.workspace.code.files.actions;
                actions.deleting = Some((target.path, target.folder));
                actions.dialog.focus(window, cx);
            }
        }
        cx.notify();
    }

    /// The name typed: make the entry or rename it.
    fn finish_edit(&mut self, text: &str, cx: &mut Context<'_, Self>) {
        let name = text.trim();
        let name_refused = self.words().labels().name_refused;
        let panel = &mut self.areas.workspace.code;
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        let Some(editing) = panel.files.actions.editing.as_ref() else {
            return;
        };
        let target = match &editing.kind {
            EditKind::NewFile(dir) | EditKind::NewFolder(dir) => dir.child(name),
            EditKind::Rename(path) => path.parent().unwrap_or_else(RelPath::root).child(name),
        };
        let Ok(target) = target else {
            panel.files.actions.refused = Some(name_refused.to_owned());
            cx.notify();
            return;
        };
        let change = match &editing.kind {
            EditKind::NewFile(_) => PathChange::CreateFile { root, path: target },
            EditKind::NewFolder(_) => PathChange::CreateDir(CreateDir { root, path: target }),
            EditKind::Rename(path) => PathChange::Rename(RenamePath {
                root,
                from: path.clone(),
                to: target,
            }),
        };
        self.ask(&machine, Ask::File(FileAsk::Path(change)));
        cx.notify();
    }

    pub(super) fn cancel_edit(&mut self, cx: &mut Context<'_, Self>) {
        let files = &mut self.areas.workspace.code.files;
        files.actions.editing = None;
        files.relist();
        cx.notify();
    }

    pub(super) fn confirm_delete(&mut self, cx: &mut Context<'_, Self>) {
        let panel = &mut self.areas.workspace.code;
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        let Some((path, folder)) = panel.files.actions.deleting.take() else {
            return;
        };
        self.ask(
            &machine,
            Ask::File(FileAsk::Path(PathChange::Delete(DeletePath {
                root,
                path,
                recursive: folder,
            }))),
        );
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
        if files
            .viewer
            .as_ref()
            .is_some_and(|viewer| touched.contains(&viewer.path))
            && !matches!(change, PathChange::CreateFile { .. })
        {
            files.viewer = None;
        }
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

    /// A duplicate answered: the copy's folder is listed again, or the
    /// machine's reason is shown over the tree.
    pub(in crate::areas::workspace) fn entry_duplicated(
        &mut self,
        ask: &DuplicateFileEntry,
        result: Result<DuplicatedFileEntry, Problem>,
    ) {
        let words = self.words();
        let panel = &mut self.areas.workspace.code;
        if panel.root().map(|(_, root)| root) != Some(&ask.root) {
            return;
        }
        let actions = &mut panel.files.actions;
        let copy = match result {
            Ok(copy) => copy,
            Err(problem) => {
                actions.refused = Some(words.problem(&problem));
                return;
            }
        };
        actions.refused = None;
        let dir = copy.path.parent().unwrap_or_else(RelPath::root);
        let _was = panel.files.listings.remove(&dir);
        self.read_folder(&dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(folder: bool, panels: bool) -> Target {
        Target {
            path: "src/main.rs".parse().unwrap(),
            folder,
            open: false,
            panels,
        }
    }

    #[test]
    fn a_file_s_download_comes_before_add_to_chat_in_the_reference_group() {
        let items = items_for(&target(false, true));
        let relative = items
            .iter()
            .position(|item| *item == Action::CopyRelativePath);
        let download = items.iter().position(|item| *item == Action::Download);
        assert_eq!(download, relative.map(|at| at.saturating_add(1)));
        assert_eq!(
            items.get(download.unwrap_or_default().saturating_add(1)),
            Some(&Action::AddToChat)
        );
        assert_eq!(group(Action::Download), group(Action::CopyPath));
        assert!(!items_for(&target(true, true)).contains(&Action::Download));
        assert!(!items_for(&target(false, false)).contains(&Action::Download));
    }

    #[test]
    fn duplicate_follows_rename_in_paseo_s_manage_group_on_a_machine_with_it() {
        for folder in [false, true] {
            let items = items_for(&target(folder, true));
            let rename = items.iter().position(|item| *item == Action::Rename);
            let duplicate = items.iter().position(|item| *item == Action::Duplicate);
            assert_eq!(duplicate, rename.map(|at| at.saturating_add(1)));
            assert_eq!(items.last(), Some(&Action::Delete));
            assert_eq!(group(Action::Duplicate), group(Action::Rename));
            assert!(!items_for(&target(folder, false)).contains(&Action::Duplicate));
        }
    }
}
