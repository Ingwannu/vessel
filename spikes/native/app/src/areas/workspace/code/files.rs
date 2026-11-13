//! The Files tab: Paseo's files panel in the Explorer. The session's
//! folder as a tree (a folder read when first opened, 12 per level, the
//! folder or file glyph, what git ignores dimmed), and a file chosen shown
//! in the panel with its lines numbered and coloured by the page's lexer,
//! Back to the tree.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, VirtualList, glyph};
use vessel_app_view::workspace::file_tree::{Listing, TreeRow};
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::syntax::{language_of, line_states};
use vessel_proto::workspace::{
    DirListing, EntryKind, FileContent, FileRead, ListDir, ReadFile, RelPath,
};

use super::{Shown, Viewer};
use crate::shell::Shell;
use vessel_app_link::workspace::{Ask, FileAsk};

/// A level's indent.
const INDENT: f32 = 12.0;

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

    fn toggle_folder(&mut self, path: &RelPath, cx: &mut Context<'_, Self>) {
        let files = &mut self.areas.workspace.code.files;
        if !files.open.remove(path) {
            let _new = files.open.insert(path.clone());
            self.read_folder(path);
        }
        self.areas.workspace.code.files.relist();
        cx.notify();
    }

    fn open_file(&mut self, path: RelPath, cx: &mut Context<'_, Self>) {
        self.open_file_at(path, None, cx);
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
        panel.files.viewer = Some(Viewer {
            path: path.clone(),
            text: Shown::Loading,
            list: VirtualList::new(false),
            goto: line,
            chosen: None,
        });
        self.ask(
            &machine,
            Ask::File(FileAsk::Read(ReadFile {
                root,
                path,
                offset: 0,
                limit: None,
            })),
        );
        cx.notify();
    }

    /// A file's first page arrived.
    pub(in crate::areas::workspace) fn file_read(
        &mut self,
        ask: &ReadFile,
        result: Result<FileRead, Problem>,
    ) {
        let words = self.words();
        let panel = &mut self.areas.workspace.code;
        if panel.root().map(|(_, root)| root) != Some(&ask.root) {
            return;
        }
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
                FileContent::Text { text, next } => {
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
    if let Some(viewer) = &files.viewer {
        return super::viewer::viewer_element(shell, viewer, tokens, window, cx);
    }
    let body = super::found::results(shell, tokens, cx).unwrap_or_else(|| {
        files
            .list
            .element(
                cx.processor(|shell: &mut Shell, index: usize, _window, cx| {
                    let tokens = vessel_app_kit::tokens(cx);
                    tree_row(shell, index, &tokens, cx)
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
        .child(div().flex_1().min_h(px(0.0)).child(body))
        .into_any_element()
}

fn indented(depth: usize) -> f32 {
    // Whole levels, at most 32 deep: an f32 from a small integer.
    let levels = u16::try_from(depth.min(32)).unwrap_or(32);
    // `mul_add`: the lints deny arithmetic operators on floats.
    f32::from(levels).mul_add(INDENT, 8.0)
}

/// A row of the tree.
fn tree_row(
    shell: &Shell,
    index: usize,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let files = &shell.areas.workspace.code.files;
    let neutral = tokens.neutral;
    let labels = shell.words().labels();
    let Some(row) = files.rows.get(index) else {
        return div().into_any_element();
    };
    let quiet = |depth: usize, text: String| {
        div()
            .pl(px(indented(depth)))
            .pr(px(8.0))
            .py(px(4.0))
            .type_(Type::Meta)
            .text_color(neutral.foreground_muted)
            .child(text)
            .into_any_element()
    };
    match row {
        TreeRow::Loading { depth, .. } => quiet(*depth, labels.loading.to_owned()),
        TreeRow::Failed { message, depth, .. } => {
            super::viewer::failed(labels.list_failed, message, indented(*depth), tokens)
                .into_any_element()
        }
        TreeRow::More { depth, .. } => quiet(*depth, labels.more_entries.to_owned()),
        TreeRow::Empty { depth, .. } => quiet(*depth, labels.empty_folder.to_owned()),
        TreeRow::Entry {
            dir,
            index: entry,
            path,
            depth,
            open,
        } => {
            let Some(Listing::Listed(listing)) = files.listings.get(dir) else {
                return div().into_any_element();
            };
            let Some(entry) = listing.entries.get(*entry) else {
                return div().into_any_element();
            };
            let folder = entry.kind == EntryKind::Dir;
            let path = path.clone();
            let hover = neutral.surface2;
            div()
                .id(("code-tree", index))
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .min_h(px(28.0))
                .pl(px(indented(*depth)))
                .pr(px(8.0))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .child(div().w(px(12.0)).flex_none().when(folder, |cell| {
                    cell.child(glyph(
                        if *open {
                            Glyph::ChevronDown
                        } else {
                            Glyph::ChevronRight
                        },
                        12.0,
                        neutral.foreground_muted,
                    ))
                }))
                .child(glyph(
                    if folder { Glyph::Folder } else { Glyph::File },
                    14.0,
                    neutral.foreground_muted,
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .type_(Type::Body)
                        .text_color(if entry.ignored {
                            neutral.foreground_extra_muted
                        } else {
                            neutral.foreground
                        })
                        .child(entry.name.clone()),
                )
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    if folder {
                        this.toggle_folder(&path, cx);
                    } else {
                        this.open_file(path.clone(), cx);
                    }
                }))
                .into_any_element()
        }
    }
}
