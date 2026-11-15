//! The Files tab: Paseo's files panel in the Explorer. The session's
//! folder as a tree (a folder read when first opened, 12 per level, the
//! folder or file glyph, what git ignores dimmed), and a file chosen shown
//! in the panel with its lines numbered and coloured by the page's lexer,
//! Back to the tree.

use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, VirtualList};
use vessel_app_view::workspace::file_tree::Listing;
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::syntax::{language_of, line_states};
use vessel_proto::workspace::{
    DirListing, FileContent, FileRead, ListDir, ReadFile, RelPath, WorkspaceRoot,
};

use super::{Shown, Viewer};
use crate::shell::Shell;
use vessel_app_link::workspace::{Ask, FileAsk};

impl Shell {
    /// List `dir`, unless it is listed or being listed.
    pub(super) fn read_folder(&mut self, dir: &RelPath) {
        let panel = &mut self.areas.workspace.code;
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        if matches!(
            panel.files.listings.get(dir),
            Some(Listing::Listed(_) | Listing::Loading)
        ) {
            return;
        }
        let _was = panel.files.listings.insert(dir.clone(), Listing::Loading);
        panel.files.relist();
        self.ask(
            &machine,
            Ask::File(FileAsk::List(ListDir {
                root,
                dir: dir.clone(),
                ignored: true,
            })),
        );
    }

    /// Ask again for every folder listed and the file shown, what is shown
    /// kept until each answers (`code_git_changed`).
    pub(super) fn reread_files(&mut self) {
        let panel = &mut self.areas.workspace.code;
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        let folders: Vec<RelPath> = panel.files.listings.keys().cloned().collect();
        let shown = panel
            .files
            .viewer
            .as_ref()
            .map(|viewer| viewer.path.clone());
        for dir in folders {
            self.ask(
                &machine,
                Ask::File(FileAsk::List(ListDir {
                    root,
                    dir,
                    ignored: true,
                })),
            );
        }
        let edited = self
            .areas
            .workspace
            .code
            .files
            .viewer
            .as_mut()
            .and_then(|viewer| viewer.edit.as_mut());
        if let Some(editing) = edited {
            // The editor meets the disk: a clean buffer follows it, a
            // modified one shows the conflict (Paseo's `reconcile`).
            if let vessel_app_view::workspace::editor::Step::Read { .. } = editing.model.reconcile()
                && let Some(path) = shown
            {
                self.ask(
                    &machine,
                    Ask::File(FileAsk::Read(super::editing::whole_read(root, path))),
                );
            }
            return;
        }
        if let Some(path) = shown {
            self.ask(
                &machine,
                Ask::File(FileAsk::Read(ReadFile {
                    root,
                    path,
                    offset: 0,
                    limit: None,
                })),
            );
        }
    }

    /// A folder's entries arrived.
    pub(in crate::areas::workspace) fn folder_read(
        &mut self,
        ask: &ListDir,
        result: Result<DirListing, Problem>,
    ) {
        let words = self.words();
        let panel = &mut self.areas.workspace.code;
        if panel.root().map(|(_, root)| root) != Some(&ask.root) {
            return;
        }
        let listing = match result {
            Ok(listing) => Listing::Listed(listing),
            // The sentence is the panel's (`labels.list_failed`); the
            // node's reason is shown under it.
            Err(problem) => Listing::Failed(words.problem(&problem)),
        };
        let _was = panel.files.listings.insert(ask.dir.clone(), listing);
        panel.files.relist();
    }

    pub(super) fn toggle_folder(&mut self, path: &RelPath, cx: &mut Context<'_, Self>) {
        let files = &mut self.areas.workspace.code.files;
        if !files.open.remove(path) {
            let _new = files.open.insert(path.clone());
            self.read_folder(path);
        }
        self.areas.workspace.code.files.relist();
        cx.notify();
    }

    pub(super) fn open_file(&mut self, path: RelPath, cx: &mut Context<'_, Self>) {
        self.open_file_at(path, None, cx);
    }

    /// Show `path` from an answer (a file just made), with no frame to
    /// notify: the answer's own handling repaints.
    pub(super) fn open_file_at_quietly(&mut self, path: RelPath) {
        let panel = &mut self.areas.workspace.code;
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        self.show_file(&machine, root, path, None);
    }

    /// Show `path`, at line `line` (from 1) when given.
    pub(super) fn open_file_at(
        &mut self,
        path: RelPath,
        line: Option<u32>,
        cx: &mut Context<'_, Self>,
    ) {
        let panel = &mut self.areas.workspace.code;
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        if panel.files.viewer.is_some() {
            self.leave_file(cx);
        }
        self.show_file(&machine, root, path, line);
        cx.notify();
    }

    /// Show `path`: edited in place at a desktop's width (read whole, in
    /// one page of at most 1 MiB), read-only at a phone's (Paseo's rule).
    fn show_file(&mut self, machine: &str, root: WorkspaceRoot, path: RelPath, line: Option<u32>) {
        let files = &mut self.areas.workspace.code.files;
        let editable = !files.narrow.get();
        files.viewer = Some(Viewer {
            path: path.clone(),
            text: Shown::Loading,
            list: VirtualList::new(false),
            goto: line,
            chosen: None,
            edit: editable.then(super::editing::Editing::new),
        });
        let read = if editable {
            super::editing::whole_read(root, path)
        } else {
            ReadFile {
                root,
                path,
                offset: 0,
                limit: None,
            }
        };
        self.ask(machine, Ask::File(FileAsk::Read(read)));
    }

    /// A file's first page arrived.
    pub(in crate::areas::workspace) fn file_read(
        &mut self,
        ask: &ReadFile,
        result: Result<FileRead, Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        let words = self.words();
        let panel = &mut self.areas.workspace.code;
        if panel.root().map(|(_, root)| root) != Some(&ask.root) {
            return;
        }
        let edited = panel
            .files
            .viewer
            .as_ref()
            .is_some_and(|viewer| viewer.path == ask.path && viewer.edit.is_some());
        if edited {
            if self.editing_read(&result, cx) {
                return;
            }
            // Not editable (binary, or more than a page): shown read-only.
            if let Some(viewer) = self.areas.workspace.code.files.viewer.as_mut() {
                viewer.edit = None;
            }
        }
        let panel = &mut self.areas.workspace.code;
        let Some(viewer) = panel
            .files
            .viewer
            .as_mut()
            .filter(|viewer| viewer.path == ask.path)
        else {
            return;
        };
        viewer.text = match result {
            Ok(read) => match read.content {
                FileContent::Text { text, next, .. } => {
                    let lines: Vec<String> = text.lines().map(str::to_owned).collect();
                    let language = language_of(ask.path.as_str());
                    let states = line_states(lines.iter().map(String::as_str), language);
                    Shown::Text {
                        lines,
                        states,
                        more: next.is_some(),
                    }
                }
                FileContent::Binary | FileContent::Unknown => Shown::Binary,
            },
            // The sentence is the panel's (`labels.read_failed`); the
            // node's reason is shown under it.
            Err(problem) => Shown::Failed(words.problem(&problem)),
        };
        let rows = match &viewer.text {
            Shown::Text { lines, more, .. } => lines.len().saturating_add(usize::from(*more)),
            Shown::Loading | Shown::Binary | Shown::Failed(_) => 0,
        };
        viewer.list.reset(rows);
        if let Some(line) = viewer.goto.take() {
            super::search::reveal_line(&viewer.list, line);
        }
    }
}

/// The tab: the file shown, else the tree.
pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let files = &shell.areas.workspace.code.files;
    files.narrow.set(vessel_app_kit::breakpoint::phone(window));
    if let Some(viewer) = &files.viewer {
        return super::viewer::viewer_element(shell, viewer, tokens, window, cx);
    }
    let body = super::found::results(shell, tokens, cx).unwrap_or_else(|| {
        files
            .list
            .element(
                cx.processor(|shell: &mut Shell, index: usize, _window, cx| {
                    let tokens = vessel_app_kit::tokens(cx);
                    super::tree::tree_row(shell, index, &tokens, cx)
                }),
            )
            .into_any_element()
    });
    div()
        .flex_1()
        .min_h(px(0.0))
        .flex()
        .flex_col()
        .child(super::search::bar(shell, tokens, cx))
        .children(super::actions_view::refused_note(shell, tokens))
        .child(div().flex_1().min_h(px(0.0)).child(body))
        .children(super::actions_view::overlays(shell, tokens, window, cx))
        .into_any_element()
}
