//! The Files tab: Paseo's files panel in the Explorer. The session's
//! folder as a tree (a folder read when first opened, 12 per level, the
//! folder or file glyph, what git ignores dimmed), and a file chosen shown
//! in the panel with its lines numbered and coloured by the page's lexer,
//! Back to the tree.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, ClickEvent, Context, Div, FluentBuilder as _, HighlightStyle,
    InteractiveElement as _, IntoElement as _, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, StyledText, Window, div, px,
};
use vessel_app_kit::{CodeTokens, Tokens, Type, TypeExt as _, VirtualList, glyph};
use vessel_app_view::workspace::file_tree::{Listing, TreeRow};
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::syntax::{TokenKind, language_of, line_states, tokenize};
use vessel_proto::workspace::{
    DirListing, EntryKind, FileContent, FileRead, ListDir, ReadFile, RelPath,
};

use super::frame::note;
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
        return viewer_element(shell, viewer, tokens, window, cx);
    }
    let body = super::search::results(shell, tokens, cx).unwrap_or_else(|| {
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
            failed(labels.list_failed, message, indented(*depth), tokens).into_any_element()
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

/// The file shown: Back and its path, then its lines.
fn viewer_element(
    shell: &Shell,
    viewer: &Viewer,
    tokens: &Tokens,
    _window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let labels = shell.words().labels();
    let hover = neutral.surface2;
    let head = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .min_h(px(36.0))
        .px(px(8.0))
        .border_b_1()
        .border_color(neutral.border)
        .child(
            div()
                .id("code-back")
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .h(px(28.0))
                .px(px(8.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .type_(Type::Body)
                .text_color(neutral.foreground_muted)
                .child(glyph(Glyph::ChevronLeft, 14.0, neutral.foreground_muted))
                .child(labels.back)
                .on_click(cx.listener(|this, _event, _window, cx| {
                    this.areas.workspace.code.files.viewer = None;
                    cx.notify();
                })),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .type_(Type::Mono)
                .text_color(neutral.foreground_muted)
                .child(viewer.path.as_str().to_owned()),
        )
        .children(viewer.chosen.map(|chosen| {
            vessel_app_kit::button(
                "code-add-to-chat",
                super::search::chosen_words(shell, chosen),
                true,
                tokens,
            )
            .on_click(cx.listener(|this, _event, window, cx| {
                this.add_lines_to_chat(window, cx);
            }))
        }));
    let body: AnyElement = match &viewer.text {
        Shown::Loading => note(labels.loading, tokens).into_any_element(),
        Shown::Binary => note(labels.binary_file, tokens).into_any_element(),
        Shown::Failed(why) => failed(labels.read_failed, why, 16.0, tokens).into_any_element(),
        Shown::Text { .. } => viewer
            .list
            .element(
                cx.processor(|shell: &mut Shell, index: usize, _window, cx| {
                    let tokens = vessel_app_kit::tokens(cx);
                    code_line(shell, index, &tokens, cx)
                }),
            )
            .into_any_element(),
    };
    div()
        .flex_1()
        .min_h(px(0.0))
        .flex()
        .flex_col()
        .child(head)
        .child(div().flex_1().min_h(px(0.0)).child(body))
        .into_any_element()
}

/// Line `index` of the file shown: its number, its text coloured.
fn code_line(
    shell: &Shell,
    index: usize,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let Some(Shown::Text {
        lines,
        states,
        more,
    }) = shell
        .areas
        .workspace
        .code
        .files
        .viewer
        .as_ref()
        .map(|viewer| &viewer.text)
    else {
        return div().into_any_element();
    };
    let neutral = tokens.neutral;
    let Some(text) = lines.get(index) else {
        return if *more {
            note(shell.words().labels().file_continues, tokens).into_any_element()
        } else {
            div().into_any_element()
        };
    };
    let path = shell
        .areas
        .workspace
        .code
        .files
        .viewer
        .as_ref()
        .map_or("", |viewer| viewer.path.as_str());
    let language = language_of(path);
    let state = states.get(index).copied().unwrap_or_default();
    let (runs, _) = tokenize(text, language, state);
    let colours = CodeTokens::of(tokens.scheme);
    let highlights = runs.into_iter().filter_map(|token| {
        let colour = match token.kind? {
            TokenKind::Keyword | TokenKind::Tag => colours.keyword,
            TokenKind::String => colours.string,
            TokenKind::Comment => colours.comment,
            TokenKind::Number => colours.number,
            TokenKind::Constant => colours.constant,
            TokenKind::Function => colours.function,
        };
        Some((
            token.range,
            HighlightStyle {
                color: Some(colour),
                ..HighlightStyle::default()
            },
        ))
    });
    let styled = StyledText::new(SharedString::from(text.clone())).with_highlights(highlights);
    let chosen = shell
        .areas
        .workspace
        .code
        .files
        .viewer
        .as_ref()
        .and_then(|viewer| viewer.chosen)
        .is_some_and(|(first, last)| (first..=last).contains(&index));
    line_frame(index, chosen, tokens, cx)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_color(neutral.foreground)
                .child(styled),
        )
        .into_any_element()
}

/// A line's frame: the number column (muted, right-aligned; a click
/// chooses the line, Shift extends the choice), mono 12 on 20, wrapped; a
/// chosen line on the second surface.
fn line_frame(index: usize, chosen: bool, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let neutral = tokens.neutral;
    div()
        .w_full()
        .flex()
        .flex_row()
        .gap(px(12.0))
        .px(px(8.0))
        .type_(Type::Mono)
        .when(chosen, |line| line.bg(neutral.surface2))
        .child(
            div()
                .id(("code-line", index))
                .w(px(36.0))
                .flex_none()
                .text_right()
                .cursor_pointer()
                .text_color(if chosen {
                    neutral.foreground
                } else {
                    neutral.foreground_extra_muted
                })
                .hover(move |style| style.text_color(neutral.foreground))
                .child(index.saturating_add(1).to_string())
                .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
                    this.choose_line(index, event.modifiers().shift, cx);
                })),
        )
}

/// A read that failed: the panel's sentence, the node's reason under it.
fn failed(sentence: &'static str, reason: &str, inset: f32, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .pl(px(inset))
        .pr(px(16.0))
        .py(px(12.0))
        .child(
            div()
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .child(sentence),
        )
        .child(
            div()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(reason.to_owned()),
        )
}

impl Shell {
    /// Choose line `index` of the viewed file; with Shift, from the line
    /// chosen first to this one; a second click on the only line chosen
    /// lets it go.
    fn choose_line(&mut self, index: usize, extend: bool, cx: &mut Context<'_, Self>) {
        if let Some(viewer) = self.areas.workspace.code.files.viewer.as_mut() {
            viewer.chosen = match viewer.chosen {
                Some((first, _)) if extend => Some((first.min(index), first.max(index))),
                Some((first, last)) if first == index && last == index => None,
                Some(_) | None => Some((index, index)),
            };
        }
        cx.notify();
    }
}
