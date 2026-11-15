//! The Files tab's search: a file by its name as it is typed (`file.find`,
//! the node's fuzzy index, a newer search replacing an older one by the
//! field's key), or lines inside files on Enter (`file.grep`); a result
//! opens the file at its line. And a viewed file's lines sent to the
//! session's composer (agent-orchestrator's "Add to chat", the page's
//! `reference.ts`): click a line's number, Shift-click another, then "Add
//! to chat".
//!
//! [Decision Log]
//! - Purpose: open by name and search in files, as Paseo's Cmd+P and
//!   agent-orchestrator's file search do, inside the panel.
//! - Constraint: a search in files costs the machine a walk of the tree;
//!   a find is cheap and the node stops an older one with the same key.
//! - Choice: the name searched as typed; the content searched on Enter,
//!   the two kinds chosen by a segmented control beside the field.
//! - Cost: a content search waits for Enter (Paseo's searches as typed
//!   with a pause; the node's grep has no key to stop an older one).

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::segmented::{SegmentSize, segment, segmented};
use vessel_app_kit::ui::{
    Context, Div, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Reveal, TextField, Tokens, VirtualList, glyph};
use vessel_app_link::workspace::{Ask, FileAsk};
use vessel_app_view::workspace::find::SearchKeys;
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::reference::{file_reference, with_reference};
use vessel_proto::workspace::{
    FindFiles, FoundFiles, GrepCase, GrepMode, GrepResult, RelPath, SearchFiles, SearchKey,
};

use crate::shell::Shell;

/// The most files a search by name asks for (the best first): past it, a
/// person types more of the name.
pub(super) const FIND_LIMIT: u32 = 100;

/// Which search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Name,
    Content,
}

/// What the search shows.
pub(super) enum Results {
    /// Nothing typed: the tree.
    Idle,
    Loading,
    Found(FoundFiles),
    Grepped(GrepResult),
    /// The machine could not search: its reason.
    Failed(String),
    /// The query could not be sent as typed.
    Refused,
}

/// The search's state.
pub(super) struct Search {
    pub(super) field: TextField,
    pub(super) mode: Mode,
    key: SearchKey,
    pub(super) results: Results,
    pub(super) list: VirtualList,
    /// How many characters the search in files shown looked for: each
    /// line's mark runs that far from the node's column.
    pub(super) width: u32,
    /// The query last sent, `None` once the field was emptied, refused or
    /// switched: an answer to any other query is late and dropped (the
    /// node replaces an older find only when a newer one is sent, so a
    /// search refused or cleared on this side still gets its answer).
    asked: Option<String>,
}

impl Search {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        let mut keys = SearchKeys::default();
        Self {
            field: TextField::with_change(
                "",
                |shell: &mut Shell, text, _window, cx| shell.search_submitted(&text, cx),
                |shell: &mut Shell, text, cx| shell.search_typed(&text, cx),
                window,
                cx,
            ),
            mode: Mode::Name,
            key: keys.next(),
            results: Results::Idle,
            list: VirtualList::new(false),
            width: 0,
            asked: None,
        }
    }

    fn rows(&self) -> usize {
        match &self.results {
            Results::Found(found) => found.matches.len(),
            Results::Grepped(grepped) => grepped.matches.len(),
            Results::Idle | Results::Loading | Results::Failed(_) | Results::Refused => 0,
        }
    }
}

impl Shell {
    fn search_typed(&mut self, text: &str, cx: &mut Context<'_, Self>) {
        let search = &mut self.areas.workspace.code.files.search;
        if text.trim().is_empty() {
            search.results = Results::Idle;
            search.asked = None;
            search.list.reset(0);
        } else if search.mode == Mode::Name {
            self.run_search(text);
        }
        cx.notify();
    }

    fn search_submitted(&mut self, text: &str, cx: &mut Context<'_, Self>) {
        if !text.trim().is_empty() {
            self.run_search(text);
        }
        cx.notify();
    }

    fn run_search(&mut self, text: &str) {
        let panel = &mut self.areas.workspace.code;
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        let search = &mut panel.files.search;
        // What is sent, trimmed for a name (a find's spaces mean nothing)
        // and as typed for a search in files (its spaces are searched).
        let sent = match search.mode {
            Mode::Name => text.trim(),
            Mode::Content => text,
        };
        let ask = match search.mode {
            Mode::Name => sent.parse().map(|query| {
                FileAsk::Find(FindFiles {
                    root,
                    query,
                    key: Some(search.key),
                    limit: Some(FIND_LIMIT),
                })
            }),
            Mode::Content => sent.parse().map(|query| {
                FileAsk::Grep(SearchFiles {
                    root,
                    query,
                    mode: GrepMode::Literal,
                    case: GrepCase::Smart,
                    dir: None,
                    limit: Some(200),
                })
            }),
        };
        let Ok(ask) = ask else {
            search.results = Results::Refused;
            search.asked = None;
            search.list.reset(0);
            return;
        };
        search.asked = Some(sent.to_owned());
        search.results = Results::Loading;
        search.list.reset(0);
        self.ask(&machine, Ask::File(ask));
    }

    /// The field's placeholder for its search, in the person's language.
    pub(super) fn search_words(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let labels = self.words().labels();
        let search = &self.areas.workspace.code.files.search;
        search.field.set_placeholder(
            match search.mode {
                Mode::Name => labels.go_to_file,
                Mode::Content => labels.search_in_files,
            },
            window,
            cx,
        );
    }

    fn search_mode(&mut self, mode: Mode, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.workspace.code.files.search.mode = mode;
        self.search_words(window, cx);
        let search = &mut self.areas.workspace.code.files.search;
        let text = search.field.text(cx);
        search.results = Results::Idle;
        search.asked = None;
        search.list.reset(0);
        if mode == Mode::Name && !text.trim().is_empty() {
            self.run_search(&text);
        }
        search_focus(self, window, cx);
        cx.notify();
    }

    /// Files found by name.
    pub(in crate::areas::workspace) fn files_found(
        &mut self,
        ask: &FindFiles,
        result: Result<FoundFiles, Problem>,
    ) {
        if ask.key != Some(self.areas.workspace.code.files.search.key) {
            // Another field's find (track P's `@`, `code_find_files`).
            super::elsewhere::found_elsewhere(self, ask, result);
            return;
        }
        let words = self.words();
        let search = &mut self.areas.workspace.code.files.search;
        if search.mode != Mode::Name || search.asked.as_deref() != Some(ask.query.as_str()) {
            return;
        }
        search.results = match result {
            Ok(found) if found.superseded => return,
            Ok(found) => Results::Found(found),
            Err(problem) => Results::Failed(words.problem(&problem)),
        };
        let rows = search.rows();
        search.list.reset(rows);
    }

    /// Lines found in files.
    pub(in crate::areas::workspace) fn files_grepped(
        &mut self,
        ask: &SearchFiles,
        result: Result<GrepResult, Problem>,
    ) {
        let words = self.words();
        let search = &mut self.areas.workspace.code.files.search;
        if search.mode != Mode::Content || search.asked.as_deref() != Some(ask.query.as_str()) {
            return;
        }
        search.width = u32::try_from(ask.query.as_str().chars().count()).unwrap_or(u32::MAX);
        search.results = match result {
            Ok(grepped) => Results::Grepped(grepped),
            Err(problem) => Results::Failed(words.problem(&problem)),
        };
        let rows = search.rows();
        search.list.reset(rows);
    }

    /// Send the viewed file's chosen lines to the session's composer.
    pub(super) fn add_lines_to_chat(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(viewer) = self.areas.workspace.code.files.viewer.as_ref() else {
            return;
        };
        let Some((first, last)) = viewer.chosen else {
            return;
        };
        let super::Shown::Text { lines, .. } = &viewer.text else {
            return;
        };
        let chosen: Vec<String> = lines
            .get(first..=last)
            .map(<[String]>::to_vec)
            .unwrap_or_default();
        let start = u32::try_from(first.saturating_add(1)).unwrap_or(u32::MAX);
        let path = viewer.path.clone();
        if let Some(viewer) = self.areas.workspace.code.files.viewer.as_mut() {
            viewer.chosen = None;
        }
        self.send_lines(&path, start, &chosen, window, cx);
    }

    /// The editor's selected lines (from 1, first and last) to the
    /// session's composer.
    pub(super) fn add_editor_lines(
        &mut self,
        (first, last): (u32, u32),
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(viewer) = self.areas.workspace.code.files.viewer.as_ref() else {
            return;
        };
        let Some(editor) = viewer.edit.as_ref().and_then(|edit| edit.editor.as_ref()) else {
            return;
        };
        let text = editor.text(cx);
        let skip = usize::try_from(first.saturating_sub(1)).unwrap_or(usize::MAX);
        let take = usize::try_from(last.saturating_sub(first).saturating_add(1)).unwrap_or(0);
        let chosen: Vec<String> = text
            .lines()
            .skip(skip)
            .take(take)
            .map(str::to_owned)
            .collect();
        let path = viewer.path.clone();
        self.send_lines(&path, first, &chosen, window, cx);
    }

    /// Lines of `path` from line `start` (from 1) at the end of the
    /// session's draft, as agent-orchestrator's reference.
    fn send_lines(
        &mut self,
        path: &RelPath,
        start: u32,
        lines: &[String],
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let chosen: Vec<&str> = lines.iter().map(String::as_str).collect();
        let fence = path
            .as_str()
            .rsplit_once('.')
            .map_or("", |(_, extension)| extension);
        let reference = file_reference(path.as_str(), start, &chosen, fence);
        let composer = &self.areas.session.composer;
        let draft = composer.text(cx);
        composer.set_text(&with_reference(&draft, &reference), window, cx);
        composer.focus(window, cx);
        self.code_handed_to_composer(window);
        cx.notify();
    }
}

fn search_focus(shell: &Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    shell
        .areas
        .workspace
        .code
        .files
        .search
        .field
        .focus(window, cx);
}

/// The field and the kind of search, over the tree or the results.
pub(super) fn bar(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let search = &shell.areas.workspace.code.files.search;
    let labels = shell.words().labels();
    let neutral = tokens.neutral;
    let size = SegmentSize::Small;
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(8.0))
        .py(px(6.0))
        .border_b_1()
        .border_color(neutral.border)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .h(px(28.0))
                .px(px(8.0))
                .rounded(px(6.0))
                .bg(neutral.surface1)
                .child(glyph(Glyph::Search, 14.0, neutral.foreground_muted))
                .child(search.field.input(tokens, cx)),
        )
        .child(
            segmented(size)
                .child(
                    segment(
                        "code-search-name",
                        labels.by_name,
                        search.mode == Mode::Name,
                        true,
                        size,
                        tokens,
                    )
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.search_mode(Mode::Name, window, cx);
                    })),
                )
                .child(
                    segment(
                        "code-search-content",
                        labels.by_content,
                        search.mode == Mode::Content,
                        true,
                        size,
                        tokens,
                    )
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.search_mode(Mode::Content, window, cx);
                    })),
                ),
        )
}

/// Bring line `line` (from 1) of the viewed file to the top, once read.
pub(super) fn reveal_line(list: &VirtualList, line: u32) {
    let row = usize::try_from(line.saturating_sub(1)).unwrap_or(0);
    list.reveal(row, Reveal::Top);
}

/// The viewed file's chosen lines, in words, for its button.
pub(super) fn chosen_words(shell: &Shell, chosen: (usize, usize)) -> String {
    let (first, last) = chosen;
    shell
        .words()
        .add_lines(first.saturating_add(1), last.saturating_add(1))
}
