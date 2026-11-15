//! The Files tab: Paseo's files panel in the Explorer. The session's
//! folder as a tree (a folder read when first opened, 12 per level, the
//! folder or file glyph, what git ignores dimmed), and a file chosen shown
//! in the panel with its lines numbered and coloured by the page's lexer,
//! Back to the tree.

use vessel_app_kit::Tokens;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_view::workspace::file_location::FileLocation;
use vessel_app_view::workspace::file_tree::Listing;
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::syntax::{language_of, line_states};
use vessel_proto::workspace::{DirListing, FileContent, FileRead, ListDir, ReadFile, RelPath};

use super::Shown;
use crate::shell::Shell;
use vessel_app_link::workspace::FileAsk;

impl Shell {
    /// List `dir`, unless it is listed or being listed.
    pub(super) fn read_folder(&mut self, dir: &RelPath) {
        let Some(scope) = self.areas.workspace.code.scope() else {
            return;
        };
        let panel = &mut self.areas.workspace.code;
        let Some(root) = panel.root().map(|(_, root)| *root) else {
            return;
        };
        if matches!(
            panel.files.listings.get(dir),
            Some(Listing::Listed(_) | Listing::Loading)
        ) {
            return;
        }
        let Some(ticket) = self.next_file_ticket() else {
            return;
        };
        let panel = &mut self.areas.workspace.code;
        let _older = panel.files.listing_tickets.insert(dir.clone(), ticket);
        let _was = panel.files.listings.insert(dir.clone(), Listing::Loading);
        panel.files.relist();
        self.request_directory(
            scope,
            ListDir {
                root,
                dir: dir.clone(),
                ignored: true,
            },
            ticket,
        );
    }

    /// Ask again for every folder listed and the file shown, what is shown
    /// kept until each answers (`code_git_changed`).
    pub(super) fn reread_files(&mut self) {
        if let Some(key) = self.areas.workspace.code.file_key() {
            if let Some(viewer) = self.areas.workspace.code.viewer_for(&key) {
                viewer.watch.restart();
            }
            self.begin_file_watch(&key);
        }
        let Some(scope) = self.areas.workspace.code.scope() else {
            return;
        };
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
            let Some(ticket) = self.next_file_ticket() else {
                return;
            };
            let _older = self
                .areas
                .workspace
                .code
                .files
                .listing_tickets
                .insert(dir.clone(), ticket);
            self.request_directory(
                scope.clone(),
                ListDir {
                    root,
                    dir,
                    ignored: true,
                },
                ticket,
            );
        }
        if self
            .areas
            .workspace
            .code
            .file_key()
            .is_some_and(|key| self.areas.workspace.code.file_mutating(&key))
        {
            return;
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
            if let vessel_app_view::workspace::editor::Step::Read { sequence } =
                editing.model.reconcile()
                && let Some(path) = shown
            {
                self.send_editor_read(root, path, sequence);
            }
            return;
        }
        if let Some(path) = shown {
            self.read_viewer(
                &machine,
                ReadFile {
                    root,
                    path,
                    offset: 0,
                    limit: None,
                },
            );
        }
    }

    /// A folder's entries arrived.
    pub(in crate::areas::workspace) fn folder_read(
        &mut self,
        machine: &str,
        ask: &ListDir,
        ticket: vessel_app_view::workspace::file_ticket::FileTicket,
        result: Result<DirListing, Problem>,
    ) {
        let words = self.words();
        let panel = &mut self.areas.workspace.code;
        if panel.root() != Some((machine, &ask.root))
            || !panel.files.listings.contains_key(&ask.dir)
        {
            return;
        }
        if panel.files.listing_tickets.get(&ask.dir) != Some(&ticket) {
            return;
        }
        let _settled = panel.files.listing_tickets.remove(&ask.dir);
        panel
            .files
            .modified
            .retain(|path, _| path.parent().as_ref() != Some(&ask.dir));
        let listing = match result {
            Ok(listing) => Listing::Listed(listing),
            // The sentence is the panel's (`labels.list_failed`); the
            // node's reason is shown under it.
            Err(problem) => Listing::Failed(words.problem(&problem)),
        };
        let _was = panel.files.listings.insert(ask.dir.clone(), listing);
        panel.files.relist();
        self.restore_tree_children(&ask.dir);
    }

    pub(super) fn toggle_folder(&mut self, path: &RelPath, cx: &mut Context<'_, Self>) {
        let files = &mut self.areas.workspace.code.files;
        if !files.open.remove(path) {
            let _new = files.open.insert(path.clone());
            self.read_folder(path);
        }
        self.areas.workspace.code.files.relist();
        self.remember_file_tree();
        cx.notify();
    }

    pub(super) fn open_file(&mut self, path: RelPath, cx: &mut Context<'_, Self>) {
        self.open_file_at(path, None, cx);
    }

    /// Show `path` from an answer (a file just made), with no frame to
    /// notify: the answer's own handling repaints.
    pub(super) fn open_file_at_quietly(&mut self, path: RelPath) {
        self.flush_file();
        if !self.park_file() {
            return;
        }
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
        let location = line.map(|line| {
            FileLocation::line(std::num::NonZeroU32::new(line).unwrap_or(std::num::NonZeroU32::MIN))
        });
        self.open_file_location(path, location, cx);
    }

    pub(super) fn open_file_location(
        &mut self,
        path: RelPath,
        location: Option<FileLocation>,
        cx: &mut Context<'_, Self>,
    ) {
        let panel = &mut self.areas.workspace.code;
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        if self
            .areas
            .workspace
            .code
            .scope()
            .map(|scope| super::buffers::FileKey::new(&scope, path.clone()))
            .is_some_and(|key| self.areas.workspace.code.file_mutating(&key))
        {
            self.areas.workspace.code.files.actions.refused =
                Some(self.words().labels().file_busy.to_owned());
            cx.notify();
            return;
        }
        self.flush_file();
        if !self.park_file() {
            cx.notify();
            return;
        }
        self.show_file(&machine, root, path, location);
        cx.notify();
    }

    pub(super) fn file_read_answered_for(
        &mut self,
        key: &super::buffers::FileKey,
        ask: &ReadFile,
        ticket: vessel_app_view::workspace::file_ticket::FileTicket,
        result: Result<FileRead, Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(viewer) = self.areas.workspace.code.viewer_for(key) else {
            return;
        };
        if viewer
            .reading
            .take_if(|pending| *pending == ticket)
            .is_none()
        {
            return;
        }
        self.file_read_for(key, ask, result, cx);
        self.reconcile_watched_file(key, cx);
    }

    /// A file's first page arrived.
    pub(super) fn file_read_for(
        &mut self,
        key: &super::buffers::FileKey,
        ask: &ReadFile,
        result: Result<FileRead, Problem>,
        _cx: &mut Context<'_, Self>,
    ) {
        let words = self.words();
        if let Ok(read) = &result
            && read.version.size <= vessel_app_view::workspace::source::MAX_SOURCE_BYTES
            && let FileContent::Text { text, .. } = &read.content
            && !self.reserve_readonly_source(
                key,
                vessel_proto::workspace::panel_files::FileSize(
                    u64::try_from(text.len()).unwrap_or(u64::MAX),
                ),
            )
        {
            if let Some(viewer) = self.areas.workspace.code.viewer_for(key) {
                viewer.text = Shown::Failed(words.labels().editor_buffers_full.to_owned());
                viewer.list.reset(0);
            }
            return;
        }
        let Some(viewer) = self.areas.workspace.code.viewer_for(key) else {
            return;
        };
        if let Some(preview) = &mut viewer.markdown {
            preview.invalidate();
        }
        viewer.text = match result {
            Ok(read)
                if read.version.size > vessel_app_view::workspace::source::MAX_SOURCE_BYTES =>
            {
                Shown::TooLarge
            }
            Ok(read) => match read.content {
                FileContent::Text { text, next, .. } => {
                    let language = language_of(ask.path.as_str());
                    let states = if read.version.size
                        <= vessel_app_view::workspace::source::MAX_HIGHLIGHTED_BYTES
                    {
                        line_states(text.lines(), language)
                    } else {
                        Vec::new()
                    };
                    match vessel_app_view::workspace::source::SourceText::new(std::sync::Arc::from(
                        text,
                    )) {
                        Ok(source) => Shown::Text {
                            source,
                            states,
                            more: next.is_some(),
                        },
                        Err(_oversize) => Shown::TooLarge,
                    }
                }
                FileContent::Binary | FileContent::Unknown => Shown::Binary,
            },
            // The sentence is the panel's (`labels.read_failed`); the
            // node's reason is shown under it.
            Err(problem) => Shown::Failed(words.problem(&problem)),
        };
        let rows = match &viewer.text {
            Shown::Text { source, more, .. } => {
                source.line_count().saturating_add(usize::from(*more))
            }
            Shown::Loading | Shown::Binary | Shown::TooLarge | Shown::Failed(_) => 0,
        };
        viewer.list.reset(rows);
        if let Some(location) = viewer.goto.take() {
            let range = location.range();
            let line_count = if let Shown::Text { source, .. } = &viewer.text {
                source.line_count()
            } else {
                0
            };
            let last = u32::try_from(line_count).unwrap_or(u32::MAX).max(1);
            let start = range.start.line.get().min(last);
            let end = range.end.line.get().min(last).max(start);
            viewer.chosen = location.selects().then(|| {
                (
                    usize::try_from(start.saturating_sub(1)).unwrap_or(usize::MAX),
                    usize::try_from(end.saturating_sub(1)).unwrap_or(usize::MAX),
                )
            });
            super::search::reveal_line(&viewer.list, start);
        }
    }
}

impl Shell {
    /// An editor response belongs to its original node, file and ticket.
    pub(super) fn editor_read_answered_for(
        &mut self,
        key: &super::buffers::FileKey,
        ticket: vessel_app_view::workspace::file_ticket::FileTicket,
        ask: &ReadFile,
        result: Result<FileRead, Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        if self.editing_read(key, ticket, &result, cx) {
            return;
        }
        if let Some(viewer) = self.areas.workspace.code.viewer_for(key) {
            viewer.edit = None;
        }
        if matches!(&result, Ok(read) if read.version.size <= vessel_app_view::workspace::source::MAX_SOURCE_BYTES
            && matches!(read.content, FileContent::Text { next: Some(_), .. }))
        {
            self.read_viewer_for(
                key,
                ReadFile {
                    offset: 0,
                    ..ask.clone()
                },
            );
            return;
        }
        self.file_read_for(key, ask, result, cx);
        self.reconcile_watched_file(key, cx);
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
        .child(super::tree_toolbar::toolbar(shell, tokens, cx))
        .child(super::search::bar(shell, tokens, cx))
        .children(super::actions_view::refused_note(shell, tokens))
        .child(div().flex_1().min_h(px(0.0)).child(body))
        .children(super::actions_view::overlays(shell, tokens, window, cx))
        .into_any_element()
}
