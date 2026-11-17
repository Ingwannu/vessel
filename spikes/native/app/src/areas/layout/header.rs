//! The pane header's actions, as Paseo's `WorkspacePaneToolbarActions`
//! (pinned workspace-desktop-tabs-row.tsx 225-388): "+" opens the new-tab
//! menu, the maximise/restore toggle shows with two or more panes, and "…"
//! opens the split menu.
//!
//! [Decision Log]
//! - Purpose: TRACKER 776 (coordinator, 2026-11-16). Vessel drew two direct
//!   split buttons after a "+" that opened a passive New tab, and an X on an
//!   empty pane; Paseo's row is + | maximise/restore | … with both "+" and
//!   "…" opening menus.
//! - Reference: "…" is a `ToolbarButton kind="menu"` labelled
//!   `workspace.git.actions.moreActions`, its `DropdownMenu` below, aligned to
//!   the end, 4 down, 220 wide, with "Split pane right" (`Columns2`) and
//!   "Split pane down" (`Rows2`) and each one's bound chord. "+" is a
//!   `ToolbarButton kind="menu"` with the new-tab chord in its tooltip; its
//!   `WorkspaceNewTabMenuContent` opens below, aligned to the start, 4 down,
//!   at least 200 wide, and lists the launch catalog in the pane's order.
//! - Choice: both menus are the kit menu under the button (`open_start`,
//!   `open_end`), drawn by `menu_overlay`; the list is the view's launch
//!   `catalog` (`LaunchPurpose::Primary`, as Paseo's pane row). Each choice
//!   runs only while the layout it was opened for is still current.
//! - Launch availability: only Agent can start from here today; it opens the
//!   new-agent page, as the passive New tab's own button does, because a
//!   workspace draft tab cannot be created outside tests yet (W2's seed
//!   bridge). The other launches are listed disabled, as Paseo's
//!   `item.disabled`, until their panels render in a pane: Terminal (the
//!   terminal panel), Changes, Diff and Files (the explorer panels), Browser
//!   (row 509) and Pull request. Plugin panels and terminal profiles (Paseo's
//!   other two groups) are row 336.
//! - Chords: shown only for actions Vessel performs (New tab, both splits).
//!   The launch shortcuts (`TabAgent`, `NewTerminal`, `TabChanges`,
//!   `TabFiles`, `TabBrowser`) are bound but not yet performed, so their
//!   chords would promise nothing; they appear when the actions do.
//! - Not drawn: an empty pane's X. Paseo removes a pane when its last tab
//!   leaves (`removeTabFromTree`), so no empty pane needs one; the view now
//!   does the same. Paseo's X in this row is Exit focus mode, and Vessel's
//!   focus mode is the maximised pane, whose way back is the selected
//!   Restore pane toggle (the W12 adoption).
use super::{mode, with_layout};
use crate::areas::shortcuts::caps::caps;
use crate::routes::Route;
use crate::shell::Shell;
use std::rc::Rc;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::menu::{Content, menu_with};
use vessel_app_kit::toolbar_button::{
    Disclosure, Selection, ToolbarButton, ToolbarConfig, ToolbarIcon, ToolbarInk,
};
#[cfg(test)]
use vessel_app_kit::ui::InteractiveElement as _;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{ButtonState, ControlSizing, MenuHandlers, MenuItem, Tokens, anchor_probe};
use vessel_app_view::layout::{
    Availability, Launch, LaunchPurpose, Layout, LayoutAction, LayoutActionContext, LayoutKey,
    LayoutMode, Pane, PaneId, Position, catalog,
};
use vessel_app_view::shortcuts::Action;

/// Paseo's new-tab menu `minWidth`.
const NEW_TAB_MIN: f32 = 200.0;
/// Paseo's split menu `width`.
const SPLIT_WIDTH: f32 = 220.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Control {
    New,
    Focus,
    More,
}
impl Control {
    fn glyph(self, maximized: bool) -> Glyph {
        match self {
            Self::New => Glyph::Plus,
            Self::Focus if maximized => Glyph::PanelMinimize,
            Self::Focus => Glyph::PanelMaximize,
            Self::More => Glyph::More,
        }
    }
    fn label(self, shell: &Shell, maximized: bool) -> &str {
        let words = shell.words.layout();
        match self {
            Self::New => words.new_tab(),
            Self::Focus if maximized => words.restore(),
            Self::Focus => words.maximize(),
            Self::More => words.more_actions(),
        }
    }
    /// The menu it opens, if it opens one.
    const fn menu(self) -> Option<MenuKind> {
        match self {
            Self::New => Some(MenuKind::NewTab),
            Self::More => Some(MenuKind::Split),
            Self::Focus => None,
        }
    }
}

/// The header's two menus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuKind {
    NewTab,
    Split,
}

/// The open header menu: which one, for which pane, opened on which layout.
#[derive(Debug, Clone)]
pub(super) struct HeaderMenu {
    kind: MenuKind,
    pane: PaneId,
    /// The workspace whose layout the menu was opened on.
    pub(super) workspace: LayoutKey,
    context: LayoutActionContext,
}

/// What a header control or menu row does to its pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaneOp {
    ToggleMaximize,
    Split(Position),
    Launch(Launch),
}

pub(super) fn controls(
    shell: &Shell,
    layout: &Layout,
    pane: &Pane,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let mut row = div();
    // Paseo: "+" always; maximise with more than one pane; "…" while panes
    // can split.
    let mut controls = vec![Control::New];
    if mode(window) == LayoutMode::SplitPanes {
        if layout.root().panes().len() > 1 {
            controls.push(Control::Focus);
        }
        controls.push(Control::More);
    }
    let (Ok(workspace), Some(context)) = (super::key(shell), shell.layout_action_context()) else {
        return row;
    };
    let pane_id = pane.id();
    let maximized = shell
        .areas
        .layout
        .maximized
        .as_ref()
        .is_some_and(|(key, pane)| *key == workspace && *pane == pane_id);
    let sizing = ControlSizing::from_compact(mode(window) == LayoutMode::Compact);
    for control in controls {
        let place = Rc::clone(
            shell
                .areas
                .layout
                .tip_places
                .borrow_mut()
                .entry((pane_id, control))
                .or_insert_with(|| Rc::new(std::cell::Cell::new(None))),
        );
        let opened = HeaderMenu {
            kind: MenuKind::NewTab,
            pane: pane_id,
            workspace: workspace.clone(),
            context: context.clone(),
        };
        let button = button(shell, control, (pane_id, maximized, sizing), tokens)
            .on_click(cx.listener({
                let place = Rc::clone(&place);
                move |shell, event: &vessel_app_kit::ui::ClickEvent, window, cx| match control
                    .menu()
                {
                    Some(kind) => open_menu(
                        shell,
                        HeaderMenu {
                            kind,
                            ..opened.clone()
                        },
                        place.get(),
                        event.position(),
                        window,
                        cx,
                    ),
                    None => PaneAction {
                        op: PaneOp::ToggleMaximize,
                        pane: pane_id,
                        context: opened.context.clone(),
                    }
                    .run(shell, window, cx),
                }
            }))
            .tooltip(&shell.areas.layout.tips, &place);
        // The menus open under the button, measured whether or not a
        // tooltip measures it too (touch layouts install none).
        row = row.child(
            div()
                .relative()
                .flex_none()
                .child(button)
                .child(anchor_probe(&place)),
        );
    }
    #[cfg(test)]
    let row = row.debug_selector(move || {
        if pane_id == PaneId::MAIN {
            "layout-main-pane-controls".to_owned()
        } else {
            format!("layout-pane-controls-{}", pane_id.get())
        }
    });
    row
}

/// One header control in the shared toolbar chrome; a menu's button keeps
/// its highlight while its menu is open (Paseo's `kind="menu"`).
fn button(
    shell: &Shell,
    control: Control,
    (pane, maximized, sizing): (PaneId, bool, ControlSizing),
    tokens: &Tokens,
) -> ToolbarButton {
    let open = control.menu().is_some_and(|kind| {
        shell.areas.layout.header_menu.is_open()
            && shell
                .areas
                .layout
                .header_menu_for
                .as_ref()
                .is_some_and(|menu| menu.kind == kind && menu.pane == pane)
    });
    let config = ToolbarConfig::new(sizing, ButtonState::Enabled)
        .selected(if control == Control::Focus && maximized {
            Selection::Selected
        } else {
            Selection::Unselected
        })
        .disclosure(if open {
            Disclosure::Open
        } else {
            Disclosure::Closed
        });
    let shortcut = match control {
        Control::New => caps(Action::NewTab, shell),
        Control::Focus | Control::More => Vec::new(),
    };
    ToolbarButton::new(
        ElementId::Name(format!("layout-control-{}-{control:?}", pane.get()).into()),
        control.label(shell, maximized).to_owned(),
        ToolbarIcon::new(control.glyph(maximized), ToolbarInk::ExtraMuted),
        config,
        tokens,
    )
    .shortcut(shortcut)
}

/// Open `menu` under its button (the click's point when the button was not
/// measured yet).
fn open_menu(
    shell: &mut Shell,
    menu: HeaderMenu,
    bounds: Option<vessel_app_kit::ui::Bounds<vessel_app_kit::ui::Pixels>>,
    at: vessel_app_kit::ui::Point<vessel_app_kit::ui::Pixels>,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if !shell.layout_action_context_is_current(&menu.context) {
        return;
    }
    let state = &mut shell.areas.layout.header_menu;
    match (menu.kind, bounds) {
        (MenuKind::NewTab, Some(bounds)) => state.open_start(bounds, window, cx),
        (MenuKind::Split, Some(bounds)) => state.open_end(bounds, window, cx),
        (MenuKind::NewTab | MenuKind::Split, None) => state.open(at, window, cx),
    }
    shell.areas.layout.header_menu_for = Some(menu);
    cx.notify();
}

/// The open header menu, over the layout.
pub(super) fn menu_overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let open = shell.areas.layout.header_menu_for.clone()?;
    if !shell.areas.layout.header_menu.is_open()
        || super::key(shell).as_ref() != Ok(&open.workspace)
    {
        return None;
    }
    let rows = rows(shell, open.kind, window);
    let items: Vec<MenuItem> = rows.iter().map(|(_, item)| item.clone()).collect();
    let ops: Vec<Option<PaneOp>> = rows.iter().map(|(op, _)| *op).collect();
    let content = match open.kind {
        MenuKind::NewTab => {
            Content::new("layout-new-tab-menu", &items).with_min_width(px(NEW_TAB_MIN))
        }
        MenuKind::Split => Content::new("layout-split-menu", &items).with_width(px(SPLIT_WIDTH)),
    };
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.layout.header_menu,
        chose: Rc::new(move |shell: &mut Shell, index, window, cx| {
            let Some(Some(op)) = ops.get(index) else {
                return;
            };
            PaneAction {
                op: *op,
                pane: open.pane,
                context: open.context.clone(),
            }
            .run(shell, window, cx);
        }),
    };
    menu_with(
        content,
        &shell.areas.layout.header_menu,
        &handlers,
        tokens,
        vessel_app_kit::breakpoint::phone(window),
        cx,
    )
}

/// A menu's rows and what each one does (`None`: listed, not chosen).
fn rows(shell: &Shell, kind: MenuKind, window: &Window) -> Vec<(Option<PaneOp>, MenuItem)> {
    let words = shell.words.layout();
    match kind {
        MenuKind::Split => vec![
            (
                Some(PaneOp::Split(Position::Right)),
                MenuItem::new(words.split_right().to_owned())
                    .with_leading(Glyph::Columns2)
                    .with_shortcut(caps(Action::SplitRight, shell)),
            ),
            (
                Some(PaneOp::Split(Position::Bottom)),
                MenuItem::new(words.split_down().to_owned())
                    .with_leading(Glyph::Rows2)
                    .with_shortcut(caps(Action::SplitDown, shell)),
            ),
        ],
        MenuKind::NewTab => catalog(LaunchPurpose::Primary, mode(window), availability)
            .into_iter()
            .map(|(launch, availability)| {
                let mut item = MenuItem::new(launch_label(shell, launch).to_owned())
                    .with_leading(launch_glyph(launch));
                item.disabled = availability != Availability::Ready;
                (
                    (availability == Availability::Ready).then_some(PaneOp::Launch(launch)),
                    item,
                )
            })
            .collect(),
    }
}

/// Which launches start from the pane header today (see the decision log).
const fn availability(launch: Launch) -> Availability {
    match launch {
        Launch::Agent => Availability::Ready,
        Launch::Terminal
        | Launch::Changes
        | Launch::Diff
        | Launch::Files
        | Launch::Browser
        | Launch::PullRequest => Availability::Disabled,
    }
}

/// Paseo's launch item words (`workspace.tabs.fallback.*` and the panels'
/// labels).
fn launch_label(shell: &Shell, launch: Launch) -> &str {
    let words = shell.words.layout();
    match launch {
        Launch::Agent => words.launch_agent(),
        Launch::Terminal => words.launch_terminal(),
        Launch::Changes => words.changes(),
        Launch::Diff => words.launch_diff(),
        Launch::Files => words.files(),
        Launch::Browser => words.launch_browser(),
        Launch::PullRequest => words.launch_pull_request(),
    }
}

/// Paseo's launch item icons (`SquarePen`, `SquareTerminal`, the diff
/// panels' `FileDiff`, `Files`, `Globe`, `GitPullRequest`).
const fn launch_glyph(launch: Launch) -> Glyph {
    match launch {
        Launch::Agent => Glyph::SquarePen,
        Launch::Terminal => Glyph::SquareTerminal,
        Launch::Changes | Launch::Diff => Glyph::FileDiff,
        Launch::Files => Glyph::Files,
        Launch::Browser => Glyph::Globe,
        Launch::PullRequest => Glyph::PullRequest,
    }
}

// Keep the captured owner selection rather than following whatever tab happens
// to be selected when a delayed callback reaches the event queue.
struct PaneAction {
    op: PaneOp,
    pane: PaneId,
    context: LayoutActionContext,
}
impl PaneAction {
    fn run(&self, shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
        if !shell.layout_action_context_is_current(&self.context) {
            return;
        }
        let pane_id = self.pane;
        match self.op {
            PaneOp::Split(position) => shell.layout_split(pane_id, position, window, cx),
            PaneOp::ToggleMaximize => {
                let result = with_layout(shell, |layout| layout.focus(pane_id));
                if result.is_ok() {
                    if let Some(context) = shell.layout_action_context() {
                        let _handled = shell.run_layout_action_at(
                            &context,
                            LayoutAction::ToggleFocus,
                            window,
                            cx,
                        );
                    }
                } else {
                    shell.layout_result(result, cx);
                }
            }
            // The new-agent page, as the passive New tab's own button opens.
            PaneOp::Launch(Launch::Agent) => shell.go(Route::Draft, window, cx),
            PaneOp::Launch(
                Launch::Terminal
                | Launch::Changes
                | Launch::Diff
                | Launch::Files
                | Launch::Browser
                | Launch::PullRequest,
            ) => {}
        }
    }
}

#[cfg(test)]
mod tests;
