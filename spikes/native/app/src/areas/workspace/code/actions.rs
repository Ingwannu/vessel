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
//!   from here, and there is no second pane). "Add to chat" puts the
//!   path, as `@path`, at the end of the session's draft (Paseo attaches
//!   a chip; the draft here is text).
//! - Cost: a rename or a new entry is typed in the tree, so the tree's
//!   rows carry one extra row while it is typed.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::dialog::{DialogContent, DialogSize, DialogState, dialog};
use vessel_app_kit::ui::{
    AnyElement, ClipboardItem, Context, Div, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Pixels, Point, StatefulInteractiveElement as _, Styled as _, Window, div,
    px,
};
use vessel_app_kit::{
    MenuHandlers, MenuItem, MenuState, TextField, Tokens, Type, TypeExt as _, breakpoint, button,
    glyph, menu,
};
use vessel_app_link::workspace::{Ask, FileAsk, PathChange};
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
    pub(super) kind: EditKind,
    pub(super) field: TextField,
}

/// The entry a menu was opened on.
#[derive(Debug, Clone)]
struct Target {
    path: RelPath,
    folder: bool,
    open: bool,
}

/// One action of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
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

/// The tree's actions' state.
pub(super) struct Actions {
    pub(super) menu: MenuState,
    target: Option<Target>,
    pub(super) editing: Option<Editing>,
    /// A delete waiting for its confirm: the entry and whether a folder.
    deleting: Option<(RelPath, bool)>,
    dialog: DialogState,
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
fn items_for(target: &Target) -> Vec<Action> {
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
const fn group(action: Action) -> u8 {
    match action {
        Action::OpenFile | Action::CollapseFolder => 0,
        Action::NewFile | Action::NewFolder => 1,
        Action::CopyPath | Action::CopyRelativePath | Action::AddToChat => 2,
        Action::Rename => 3,
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
        let actions = &mut self.areas.workspace.code.files.actions;
        actions.target = Some(Target { path, folder, open });
        actions.menu.open(at, window, cx);
        cx.notify();
    }

    fn file_action(&mut self, action: Action, window: &mut Window, cx: &mut Context<'_, Self>) {
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
                    | Action::AddToChat
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

    fn cancel_edit(&mut self, cx: &mut Context<'_, Self>) {
        let files = &mut self.areas.workspace.code.files;
        files.actions.editing = None;
        files.relist();
        cx.notify();
    }

    fn confirm_delete(&mut self, cx: &mut Context<'_, Self>) {
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
}

/// The row being typed, at `depth`: the entry's glyph and the field.
pub(super) fn edit_row(
    shell: &Shell,
    depth: usize,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let Some(editing) = shell.areas.workspace.code.files.actions.editing.as_ref() else {
        return div().into_any_element();
    };
    let neutral = tokens.neutral;
    let folder = matches!(editing.kind, EditKind::NewFolder(_));
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .py(px(4.0))
        .pl(px(super::files::indented(depth)))
        .pr(px(8.0))
        .child(
            div()
                .size(px(16.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(glyph(
                    if folder { Glyph::Folder } else { Glyph::File },
                    16.0,
                    neutral.foreground_muted,
                )),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .h(px(24.0))
                .px(px(6.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(neutral.border_accent)
                .bg(neutral.surface1)
                .child(editing.field.input(tokens, cx)),
        )
        .child(
            div()
                .id("code-edit-cancel")
                .size(px(24.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .cursor_pointer()
                .hover(move |style| style.bg(neutral.surface2))
                .child(glyph(Glyph::Close, 12.0, neutral.foreground_muted))
                .on_click(cx.listener(|this, _event, _window, cx| this.cancel_edit(cx))),
        )
        .into_any_element()
}

/// The menu and the delete's dialog, over everything, when open.
pub(super) fn overlays(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let actions = &shell.areas.workspace.code.files.actions;
    let labels = shell.words().labels();
    let narrow = breakpoint::phone(window);
    let mut drawn = Vec::new();
    if let Some(target) = &actions.target {
        let chosen = items_for(target);
        let mut last_group = None;
        let items: Vec<MenuItem> = chosen
            .iter()
            .map(|action| {
                let label = match action {
                    Action::OpenFile => labels.open_file,
                    Action::CollapseFolder => labels.collapse_folder,
                    Action::NewFile => labels.new_file,
                    Action::NewFolder => labels.new_folder,
                    Action::CopyPath => labels.copy_path,
                    Action::CopyRelativePath => labels.copy_relative_path,
                    Action::AddToChat => labels.add_to_chat,
                    Action::Rename => labels.rename,
                    Action::Delete => labels.delete,
                };
                let item = if *action == Action::Delete {
                    MenuItem::danger(label)
                } else {
                    MenuItem::new(label)
                };
                let starts = last_group.is_some_and(|last| last != group(*action));
                last_group = Some(group(*action));
                if starts && *action != Action::Delete {
                    item.grouped()
                } else {
                    item
                }
            })
            .collect();
        let handlers = MenuHandlers {
            state: |shell: &mut Shell| &mut shell.areas.workspace.code.files.actions.menu,
            chose: std::rc::Rc::new(move |shell: &mut Shell, index, window, cx| {
                if let Some(action) = chosen.get(index).copied() {
                    shell.file_action(action, window, cx);
                }
            }),
        };
        drawn.extend(menu(
            "code-file-actions",
            &actions.menu,
            &items,
            &handlers,
            tokens,
            narrow,
            cx,
        ));
    }
    if let Some((path, folder)) = &actions.deleting {
        let name = path.file_name().unwrap_or_default().to_owned();
        let content = DialogContent {
            title: if *folder {
                labels.delete_folder_title
            } else {
                labels.delete_file_title
            }
            .into(),
            body: div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .child(labels.delete_message)
                .child(
                    div()
                        .type_(Type::Mono)
                        .text_color(tokens.neutral.foreground)
                        .child(name),
                )
                .into_any_element(),
            footer: vec![
                button("code-delete-cancel", labels.cancel, false, tokens)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.areas.workspace.code.files.actions.deleting = None;
                        cx.notify();
                    }))
                    .into_any_element(),
                vessel_app_kit::button_with_kind(
                    "code-delete-confirm",
                    labels.delete,
                    vessel_app_kit::ButtonKind::Destructive,
                    vessel_app_kit::ButtonState::Enabled,
                    tokens,
                )
                .on_click(cx.listener(|this, _event, _window, cx| this.confirm_delete(cx)))
                .into_any_element(),
            ],
            size: DialogSize::Normal,
        };
        drawn.push(dialog(
            &actions.dialog,
            content,
            tokens,
            window,
            narrow,
            |shell: &mut Shell, _window, cx| {
                shell.areas.workspace.code.files.actions.deleting = None;
                cx.notify();
            },
            cx,
        ));
    }
    drawn
}

/// The machine's refusal of the last change, over the tree.
pub(super) fn refused_note(shell: &Shell, tokens: &Tokens) -> Option<Div> {
    let reason = shell.areas.workspace.code.files.actions.refused.as_ref()?;
    Some(super::viewer::failed(
        shell.words().labels().change_refused,
        reason,
        12.0,
        tokens,
    ))
}
