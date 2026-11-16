//! The Changes tab: Paseo's diff pane in the Explorer. A toolbar (track
//! G1's git actions first, through [`git_head`]; then the comparison as a
//! label with a caret that opens its list, Paseo's `DiffModeMenu`; the
//! files and lines counted at the end), then every changed file with its
//! diff under it, folded by a click on its header. A file's diff is read
//! when its row first comes into view, so a hundred changed files cost
//! the calls of the ones looked at.

use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ComboHandlers, ComboHeader, ComboRow, PillParts, Side, Tokens, Type, TypeExt as _, combobox,
    glyph,
};
use vessel_app_view::workspace::changes::{FileState, ScopeKind, totals};
use vessel_app_view::workspace::diff_rows::add_page;
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::workspace::{
    DiffSummary, FileDiff, GetChanges, GetFileDiff, RelPath, WorkspaceRoot,
};

use super::{Summary, frame::note};
use crate::shell::Shell;
use vessel_app_link::workspace::{Ask, GitAsk};

/// Track G1's slot at the head of the Changes tab: the session's git
/// actions (commit, push, pull, sync, branch, stash, discard), drawn by
/// its area, `crate::areas::git`; `None` draws nothing.
pub(crate) fn git_head(
    shell: &Shell,
    machine: &str,
    root: &WorkspaceRoot,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    crate::areas::git::slot(shell, machine, root, tokens, window, cx)
}

impl Shell {
    /// Read the comparison's files, unless they are read.
    pub(super) fn read_changes(&mut self) {
        if self
            .areas
            .workspace
            .code
            .target
            .as_ref()
            .is_some_and(|target| target.receipt.is_some())
        {
            let panel = &self.areas.workspace.code;
            if panel.source().is_none() && panel.changes.sources.requested.is_none() {
                self.source_summary();
            }
            return;
        }
        let panel = &mut self.areas.workspace.code;
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        if matches!(panel.changes.summary, Summary::Ready(_)) {
            return;
        }
        panel.changes.summary = Summary::Loading;
        let scope = panel.changes.scope.scope();
        self.ask(
            &machine,
            Ask::Git(GitAsk::Changes(GetChanges {
                root,
                scope,
                ignore_whitespace: false,
            })),
        );
    }

    /// Ask for the comparison again, what is shown kept until it answers
    /// (`code_git_changed`).
    pub(super) fn reread_changes(&mut self) {
        if self
            .areas
            .workspace
            .code
            .target
            .as_ref()
            .is_some_and(|target| target.receipt.is_some())
        {
            self.source_summary();
            return;
        }
        let panel = &mut self.areas.workspace.code;
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        let scope = panel.changes.scope.scope();
        self.ask(
            &machine,
            Ask::Git(GitAsk::Changes(GetChanges {
                root,
                scope,
                ignore_whitespace: false,
            })),
        );
    }

    /// The comparison's files arrived.
    pub(in crate::areas::workspace) fn changes_read(
        &mut self,
        ask: &GetChanges,
        result: Result<DiffSummary, Problem>,
    ) {
        if self
            .areas
            .workspace
            .code
            .target
            .as_ref()
            .is_some_and(|target| target.receipt.is_some())
        {
            return;
        }
        self.install_changes_read(ask, result);
    }
    pub(super) fn install_changes_read(
        &mut self,
        ask: &GetChanges,
        result: Result<DiffSummary, Problem>,
    ) {
        let words = self.words();
        let panel = &mut self.areas.workspace.code;
        if panel.root().map(|(_, root)| root) != Some(&ask.root)
            || panel.changes.scope.scope() != ask.scope
        {
            return;
        }
        let changes = &mut panel.changes;
        changes.summary = match result {
            Ok(summary) => Summary::Ready(summary),
            Err(problem) => Summary::Failed(words.problem(&problem)),
        };
        changes.states.clear();
        changes.relist();
    }

    /// A page of a file's diff arrived.
    pub(in crate::areas::workspace) fn diff_read(
        &mut self,
        ask: &GetFileDiff,
        result: Result<FileDiff, Problem>,
    ) {
        if self
            .areas
            .workspace
            .code
            .target
            .as_ref()
            .is_some_and(|target| target.receipt.is_some())
        {
            return;
        }
        self.install_diff_read(ask, result);
    }
    pub(super) fn install_diff_read(
        &mut self,
        ask: &GetFileDiff,
        result: Result<FileDiff, Problem>,
    ) {
        let words = self.words();
        let panel = &mut self.areas.workspace.code;
        if panel.root().map(|(_, root)| root) != Some(&ask.root)
            || panel.changes.scope.scope() != ask.scope
        {
            return;
        }
        let changes = &mut panel.changes;
        let before = match changes.states.get(&ask.path) {
            Some(FileState::Loading(before)) => before.clone(),
            Some(FileState::Read(_) | FileState::Unread | FileState::Failed(_)) | None => None,
        };
        let state = match result {
            Ok(page) => FileState::Read(add_page(before.as_ref(), &page)),
            Err(problem) => FileState::Failed(words.problem(&problem)),
        };
        let _was = changes.states.insert(ask.path.clone(), state);
        changes.relist();
    }

    /// Read a page of `path`'s diff (`from`: the next page's hunk).
    pub(super) fn read_diff(&mut self, path: &RelPath, from: Option<u32>) {
        if self
            .areas
            .workspace
            .code
            .target
            .as_ref()
            .is_some_and(|target| target.receipt.is_some())
        {
            self.source_diff(path, from);
            return;
        }
        let panel = &mut self.areas.workspace.code;
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        let changes = &mut panel.changes;
        let before = match changes.states.get(path) {
            Some(FileState::Read(body)) => Some(body.clone()),
            Some(FileState::Loading(_)) => return,
            Some(FileState::Unread | FileState::Failed(_)) | None => None,
        };
        let _was = changes
            .states
            .insert(path.clone(), FileState::Loading(before));
        let scope = changes.scope.scope();
        self.ask(
            &machine,
            Ask::Git(GitAsk::Diff(GetFileDiff {
                root,
                scope,
                path: path.clone(),
                ignore_whitespace: false,
                from_hunk: from,
            })),
        );
    }

    pub(super) fn fold_file(&mut self, path: RelPath, cx: &mut Context<'_, Self>) {
        let changes = &mut self.areas.workspace.code.changes;
        if !changes.closed.remove(&path) {
            let _new = changes.closed.insert(path);
        }
        changes.relist();
        cx.notify();
    }

    fn choose_scope(&mut self, kind: ScopeKind, cx: &mut Context<'_, Self>) {
        let changes = &mut self.areas.workspace.code.changes;
        changes.picker.close();
        if changes.scope != kind {
            changes.sources.invalidate();
            changes.scope = kind;
            changes.summary = Summary::Loading;
            changes.closed.clear();
            changes.states.clear();
            changes.relist();
            self.read_changes();
        }
        cx.notify();
    }
}

/// The tab.
pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let panel = &shell.areas.workspace.code;
    let changes = &panel.changes;
    let words = shell.words();
    let labels = words.labels();
    let body: AnyElement = match &changes.summary {
        Summary::Loading => note(labels.loading, tokens).into_any_element(),
        Summary::Failed(why) => note(why.clone(), tokens).into_any_element(),
        Summary::Ready(summary) if summary.files.is_empty() => {
            note(words.scope_empty(&changes.scope.scope()), tokens).into_any_element()
        }
        Summary::Ready(_) => changes
            .list
            .element(cx.processor(|shell: &mut Shell, index: usize, window, cx| {
                let tokens = vessel_app_kit::tokens(cx);
                super::change_rows::row(shell, index, &tokens, window, cx)
            }))
            .into_any_element(),
    };
    div()
        .flex_1()
        .min_h(px(0.0))
        .flex()
        .flex_col()
        .child(toolbar(shell, tokens, window, cx))
        .child(div().flex_1().min_h(px(0.0)).child(body))
        .children(scope_list(shell, tokens, window, cx))
        .into_any_element()
}

/// G1's actions, the comparison, the counts.
fn toolbar(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let panel = &shell.areas.workspace.code;
    let changes = &panel.changes;
    let words = shell.words();
    let neutral = tokens.neutral;
    let git = panel
        .root()
        .and_then(|(machine, root)| git_head(shell, machine, root, tokens, window, cx));
    let pill = crate::areas::draft::pill_with(
        "code-scope",
        &changes.picker,
        PillParts {
            label: Some(words.scope_label(&changes.scope.scope()).into()),
            caret: true,
            ..PillParts::default()
        },
        false,
        tokens,
    )
    .on_click(cx.listener(|this, _event, window, cx| {
        let picker = &mut this.areas.workspace.code.changes.picker;
        if picker.is_open() {
            picker.close();
        } else {
            picker.open(None, window, cx);
        }
        cx.notify();
    }));
    let counts = match &changes.summary {
        Summary::Ready(summary) if !summary.files.is_empty() => {
            let total = totals(&summary.files);
            Some(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .type_(Type::Meta)
                    .text_color(neutral.foreground_muted)
                    .child(words.files(u64::try_from(summary.files.len()).unwrap_or(u64::MAX)))
                    .child(
                        div()
                            .text_color(tokens.st_ready)
                            .child(format!("+{}", total.added)),
                    )
                    .child(
                        div()
                            .text_color(neutral.destructive)
                            .child(format!("\u{2212}{}", total.removed)),
                    ),
            )
        }
        Summary::Ready(_) | Summary::Loading | Summary::Failed(_) => None,
    };
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(8.0))
        .min_h(px(40.0))
        .px(px(8.0))
        .py(px(4.0))
        .border_b_1()
        .border_color(neutral.border)
        .children(git)
        .child(pill)
        .child(div().flex_1())
        .children(counts)
        .child(
            // Paseo's diff pane's refresh: the comparison read again.
            div()
                .id("code-refresh")
                .size(px(28.0))
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .rounded(px(6.0))
                .cursor_pointer()
                .hover(move |style| style.bg(neutral.surface2))
                .child(glyph(Glyph::Refresh, 14.0, neutral.foreground_muted))
                .on_click(cx.listener(|this, _event, _window, cx| {
                    let target = this
                        .areas
                        .workspace
                        .code
                        .root()
                        .map(|(machine, root)| (machine.to_owned(), *root));
                    if let Some((machine, root)) = target {
                        this.code_git_changed(&machine, &root, cx);
                    }
                })),
        )
}

/// The comparison's list (Paseo's diff mode menu, the kit's list).
fn scope_list(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let changes = &shell.areas.workspace.code.changes;
    if !changes.picker.is_open() {
        return None;
    }
    let words = shell.words();
    let rows: Vec<ComboRow> = ScopeKind::ALL
        .into_iter()
        .map(|kind| {
            let mut row = ComboRow::new(words.scope_label(&kind.scope()));
            row.detail = Some(words.scope_detail(kind).into());
            row.checked = kind == changes.scope;
            row
        })
        .collect();
    let header = ComboHeader {
        title: None,
        back: false,
        search: false,
        empty: SharedString::default(),
        note: None,
        side: Side::Below,
    };
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.workspace.code.changes.picker,
        chose: Rc::new(
            |shell: &mut Shell, index, _window: &mut Window, cx: &mut Context<'_, Shell>| {
                if let Some(kind) = ScopeKind::ALL.get(index).copied() {
                    shell.choose_scope(kind, cx);
                }
            },
        ),
        back: None,
    };
    combobox(
        "code-scope-list",
        &changes.picker,
        &header,
        &rows,
        &handlers,
        300.0,
        tokens,
        window,
        cx,
    )
}
