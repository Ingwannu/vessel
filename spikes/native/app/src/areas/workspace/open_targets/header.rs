//! The session header's Open (Paseo's `WorkspaceOpenInEditorButton`,
//! `workspace/open-in-editor/button.tsx`, mounted icon-only in its
//! workspace header): a split button whose primary opens the workspace,
//! or the file the Files tab shows at its line, in the preferred target,
//! and whose caret lists the targets ("Choose editor") with a check on the
//! preferred one. Choosing one only makes it the preferred, as Paseo's.
//!
//! Geometry is Paseo's: 26 points tall (28 on a phone), its radius 6, a
//! hairline in `borderAccent` around it and between its parts, each part
//! as wide as tall, `surface2` while hovered or open; the target's icon at
//! 16, the caret's chevron at 16 in the extra-muted ink.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::{
    ComboHandlers, ComboHeader, ComboRow, Leading, Side, combobox,
};
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    px,
};
use vessel_app_kit::{Tokens, agent_mark, glyph};
use vessel_app_view::open_targets::{Icon, Request, TargetId};
use vessel_proto::ProjectId;

use super::{Origin, artwork};
use crate::shell::Shell;

/// Paseo's `HEADER_CONTROL_HEIGHT` and, on a phone, `CONTROL_HEIGHTS.sm`.
const HEIGHT: f32 = 26.0;
const HEIGHT_NARROW: f32 = 28.0;

/// Paseo's `borderRadius.md`.
const RADIUS: f32 = 6.0;

/// Paseo's menu bounds (`minWidth` 148, `maxWidth` 176): the list's width.
const LIST_WIDTH: f32 = 176.0;

/// What the header's Open would open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Shown {
    /// The open session's machine.
    pub(crate) machine: String,
    /// Its project, which keeps the preferred target.
    pub(crate) project: Option<ProjectId>,
    /// Its folder on this computer.
    pub(crate) root: PathBuf,
    /// The file the Files tab shows, and the line it was opened at.
    pub(crate) file: Option<(PathBuf, Option<u32>)>,
}

impl Shell {
    /// The open session's workspace, when its machine is this computer and
    /// its folder is an absolute path.
    pub(crate) fn shown_workspace(&self) -> Option<Shown> {
        let open = self.areas.session.open.as_ref()?;
        if !self.machine_here(&open.machine) {
            return None;
        }
        let card = self
            .fleet
            .machines
            .iter()
            .find(|machine| machine.name == open.machine)?
            .sessions
            .iter()
            .find(|card| card.session == open.session)?;
        let root = PathBuf::from(card.detail.cwd.as_deref()?);
        if !root.is_absolute() {
            return None;
        }
        Some(Shown {
            machine: open.machine.clone(),
            project: card.detail.project,
            root,
            file: self.code_shown_file(),
        })
    }

    /// The primary pressed: open `shown` in `target`.
    fn open_shown(&self, shown: Shown, target: TargetId, cx: &mut Context<'_, Self>) {
        let (file, line) = shown.file.unzip();
        let request = Request {
            folder: shown.root.clone(),
            root: shown.root,
            file,
            line: line.flatten(),
            column: None,
        };
        self.open_in_target(target, request, Origin::Editor, cx);
    }
}

/// A file's name for the button's words.
fn file_name(file: &Path) -> String {
    file.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A target's icon at 16: its mark in its own colours, else Paseo's
/// symbol in the muted ink.
fn icon(target: TargetId, tokens: &Tokens) -> AnyElement {
    let muted = tokens.neutral.foreground_muted;
    match target.icon() {
        Icon::Mark(mark) => agent_mark::agent_mark(
            Some(artwork(mark)),
            agent_mark::Size::Sixteen,
            agent_mark::Space::Compact,
            tokens,
        )
        .into_any_element(),
        Icon::Folder => glyph(Glyph::Folder, 16.0, muted).into_any_element(),
        Icon::Terminal => glyph(Glyph::SquareTerminal, 16.0, muted).into_any_element(),
    }
}

/// The header's Open, when the open session's machine is this computer
/// and something is listed for its folder.
pub(crate) fn open_button(
    shell: &Shell,
    tokens: &Tokens,
    narrow: bool,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let shown = shell.shown_workspace()?;
    let targets = shell.open_targets(&shown.root, cx)?.to_vec();
    let primary = shell.preferred_target(&shown.machine, shown.project, &targets)?;
    let words = shell.t.lang.open_targets();
    let neutral = tokens.neutral;
    let size = if narrow { HEIGHT_NARROW } else { HEIGHT };
    let hover = neutral.surface2;
    let name = match &shown.file {
        Some((file, _)) => words.open_file_in(&file_name(file), primary),
        None => words.open_workspace_in(primary),
    };
    let chooser = &shell.areas.workspace.open.chooser;
    let opened = shown.clone();
    let main = div()
        .id("open-in-editor-primary")
        .size(px(size))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .debug_selector(move || format!("open-in-editor-primary:{name}"))
        .child(icon(primary, tokens))
        .on_click(cx.listener(move |shell, _event, _window, cx| {
            shell.open_shown(opened.clone(), primary, cx);
        }));
    let mut button = div()
        .flex()
        .flex_row()
        .flex_shrink_0()
        .h(px(size))
        .rounded(px(RADIUS))
        .border_1()
        .border_color(neutral.border_accent)
        .overflow_hidden()
        .child(main);
    if targets.len() > 1 {
        let caret = div()
            .id("open-in-editor-caret")
            .relative()
            .h_full()
            .w(px(size))
            .flex()
            .items_center()
            .justify_center()
            .border_l_1()
            .border_color(neutral.border_accent)
            .cursor_pointer()
            .when_some(chooser.is_open().then_some(hover), |caret, fill| caret.bg(fill))
            .hover(move |style| style.bg(hover))
            .debug_selector(|| "open-in-editor-caret".to_owned())
            .child(glyph(Glyph::ChevronDown, 16.0, neutral.foreground_extra_muted))
            .child(chooser.anchor())
            .on_click(cx.listener(|shell, _event, window, cx| {
                let chooser = &mut shell.areas.workspace.open.chooser;
                if chooser.is_open() {
                    chooser.close();
                } else {
                    chooser.open(None, window, cx);
                }
                cx.notify();
            }));
        button = button.child(caret);
    }
    let list = chooser_list(shell, &shown, &targets, primary, tokens, window, cx);
    Some(div().flex_shrink_0().child(button).children(list).into_any_element())
}

/// "Choose editor": the targets with their icons, a check on the
/// preferred; choosing one keeps it and closes the list.
#[expect(
    clippy::too_many_arguments,
    reason = "the list is drawn from the shown workspace, its targets and the frame"
)]
fn chooser_list(
    shell: &Shell,
    shown: &Shown,
    targets: &[TargetId],
    primary: TargetId,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let chooser = &shell.areas.workspace.open.chooser;
    if !chooser.is_open() {
        return None;
    }
    let rows: Vec<ComboRow> = targets
        .iter()
        .map(|target| {
            let mut row = ComboRow::new(target.label());
            row.leading = Some(match target.icon() {
                Icon::Mark(mark) => Leading::Artwork(artwork(mark)),
                Icon::Folder => Leading::Glyph(Glyph::Folder),
                Icon::Terminal => Leading::Glyph(Glyph::SquareTerminal),
            });
            row.checked = *target == primary;
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
    let chosen = targets.to_vec();
    let machine = shown.machine.clone();
    let project = shown.project;
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.workspace.open.chooser,
        chose: Rc::new(move |shell: &mut Shell, index, _window: &mut Window, cx: &mut Context<'_, Shell>| {
            if let Some(target) = chosen.get(index).copied() {
                shell.choose_target(&machine, project, target, cx);
            }
            shell.areas.workspace.open.chooser.close();
            cx.notify();
        }),
        back: None,
    };
    combobox(
        "open-in-editor-menu",
        chooser,
        &header,
        &rows,
        &handlers,
        LIST_WIDTH,
        tokens,
        window,
        cx,
    )
}
