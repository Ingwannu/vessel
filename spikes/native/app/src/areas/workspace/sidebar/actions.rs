//! A project row's actions, as Paseo's (`sidebar-workspace-list.tsx`:
//! `ProjectRowTrailingActions`, `NewWorktreeButton`, `ProjectKebabMenu`,
//! `ProjectMenuItems`, `handleRemoveProject`): "New workspace" and the
//! project's menu at the row's end, the menu's Open project settings and
//! Remove project, and the question before removing.
//!
//! [Decision Log] (TRACKER 662)
//! - Purpose: what Paseo's project row offers, where the person sees the
//!   project, without going to Settings first.
//! - Existing: Settings › Projects removes a project at once (its page's
//!   button) and opens a project's detail by a press on its row; the draft
//!   tray places a new chat in a project, local or in a new worktree.
//! - Choice: the plus (a 24 square, the plus 15, muted, the foreground on
//!   hover; Paseo's tooltip "New workspace" under it, at once) starts a new
//!   chat in a new worktree of the project, offered for a ready git
//!   project as Paseo offers it where a host can make a worktree; the
//!   kebab (24, 32 on a phone; `MoreVertical`; muted, the foreground on
//!   hover, `surface2` behind it) opens Paseo's menu (220 wide, a sheet
//!   titled "Project actions" on a phone). Both show while the row is
//!   hovered, always on a phone, and the kebab while its menu is open.
//!   Remove asks Paseo's question first (destructive Remove, Cancel), then
//!   the node removes the project (its files stay); while it does, the
//!   row's mark is a spinner and the menu's row says "Removing...". A
//!   refusal is a toast in the app's words. A removal from Settings shows
//!   the same spinner, its refusal stays on its page.
//! - Differences, named: Paseo's Open in new window and Open in file manager
//!   are its desktop shell's (a local folder in a new window or the system's
//!   file manager); not offered. Paseo's shortcut hint in the plus's tooltip
//!   (its new-worktree key) has no binding here.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::menu::{Content, ItemStatus, MenuHandlers, MenuItem, MenuState, menu_with};
use vessel_app_kit::theme::{SidebarSurfaces, radius};
use vessel_app_kit::tooltip::{Configuration, Delay, Offset, Place, Side};
use vessel_app_kit::ui::{
    AnyElement, App, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, MouseButton, MouseDownEvent, ParentElement as _, Pixels, Point, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, px, svg,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, ControlSizing, DialogContent, DialogSize, DialogState, Tokens,
    Tooltips, Type, TypeExt as _, breakpoint, button, button_with_kind, dialog,
};
use vessel_app_link::workspace::{Ask, ProjectAsk};
use vessel_app_view::toasts::ToastTone;
use vessel_app_view::workspace::place::PlaceMode;
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::ProjectId;

use super::super::PlacePick;
use super::super::detail::Detail;
use crate::shell::Shell;

/// The plus and the kebab's square on a wide window, the kebab's on a
/// phone (Paseo's `projectIconActionButton`, `projectKebabButton`).
const ACTION: f32 = 24.0;
const ACTION_COMPACT: f32 = 32.0;
/// The plus's glyph (Paseo's 15).
const PLUS: f32 = 15.0;
/// The kebab's glyph (`sm` density: 14, 18 on a phone).
const KEBAB: f32 = 14.0;
const KEBAB_COMPACT: f32 = 18.0;
/// Between the two (Paseo's 2).
const ACTIONS_GAP: f32 = 2.0;
/// The actions pulled through the kebab's unpainted edge onto the row's
/// rail (Paseo's `marginRight: -6`). Negative literals: sizes take no
/// arithmetic.
const ACTIONS_BACK: f32 = -6.0;
/// On a phone the 32 kebab hangs out by 4 so the row keeps its 44
/// (`marginRight: -4`, `marginVertical: -4`).
const KEBAB_BACK: f32 = -4.0;
/// The menu's width (`DropdownMenuContent width={220}`).
const MENU_WIDTH: f32 = 220.0;

/// The project an action is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Target {
    pub(super) machine: String,
    pub(super) project: ProjectId,
    pub(super) name: String,
}

/// Where a removal was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    /// The row's menu: a refusal is a toast.
    Sidebar,
    /// Settings › Projects: a refusal stays on its page.
    Page,
}

/// The project rows' menu, question, removals and tooltips.
pub(crate) struct ProjectRows {
    menu: MenuState,
    /// The project the menu is open for.
    target: Option<Target>,
    /// The project the remove question asks about.
    confirm: Option<Target>,
    dialog: DialogState,
    /// Removals the node has not answered yet.
    removing: HashMap<(String, ProjectId), Origin>,
    tips: Tooltips,
    places: RefCell<HashMap<String, Place>>,
}

impl ProjectRows {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            menu: MenuState::new(cx),
            target: None,
            confirm: None,
            dialog: DialogState::new(cx),
            removing: HashMap::new(),
            tips: Tooltips::new(),
            places: RefCell::new(HashMap::new()),
        }
    }

    /// Whether `project` on `machine` is being removed.
    pub(super) fn is_removing(&self, machine: &str, project: ProjectId) -> bool {
        self.removing.contains_key(&(machine.to_owned(), project))
    }

    fn menu_is_for(&self, target: &Target) -> bool {
        self.menu.is_open() && self.target.as_ref() == Some(target)
    }

    /// `key`'s place for its tooltip, kept across frames.
    fn place(&self, key: &str) -> Place {
        Rc::clone(
            self.places
                .borrow_mut()
                .entry(key.to_owned())
                .or_insert_with(|| Rc::new(std::cell::Cell::new(None))),
        )
    }
}

impl Shell {
    fn project_menu_open(
        &mut self,
        target: Target,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let rows = &mut self.areas.workspace.sidebar;
        rows.target = Some(target);
        rows.menu.open(at, window, cx);
        cx.notify();
    }

    fn project_menu_chose(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(target) = self.areas.workspace.sidebar.target.take() else {
            return;
        };
        // The menu's rows: Open project settings, then Remove project.
        if index == 0 {
            self.show_project_settings(&target, window, cx);
        } else {
            self.areas.workspace.sidebar.confirm = Some(target);
            self.areas.workspace.sidebar.dialog.focus(window, cx);
        }
        cx.notify();
    }

    /// Settings › Projects on the project's machine, its detail open.
    fn show_project_settings(
        &mut self,
        target: &Target,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(info) = self
            .areas
            .workspace
            .projects
            .get(&target.machine)
            .and_then(|view| view.project(&target.project))
            .cloned()
        else {
            return;
        };
        self.projects_page(Some(target.machine.clone()), false, window, cx);
        self.areas.workspace.page.detail = Some(Detail::new(info.id));
        self.load_detail(&target.machine, &info);
    }

    /// A new chat in a new worktree of the project (Paseo's new workspace).
    fn new_workspace_in(
        &mut self,
        target: &Target,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.chat_in_project(target.machine.clone(), target.project, window, cx);
        self.choose_place(PlacePick::Isolation(PlaceMode::Worktree), window, cx);
    }

    fn project_remove_dismiss(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.workspace.sidebar.confirm = None;
        cx.notify();
    }

    fn project_remove_confirmed(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(target) = self.areas.workspace.sidebar.confirm.take() else {
            return;
        };
        self.remove_project_from(&target.machine, target.project, Origin::Sidebar);
        cx.notify();
    }

    /// Ask `machine` to forget `project`, its row spinning until it
    /// answers.
    pub(crate) fn remove_project_from(
        &mut self,
        machine: &str,
        project: ProjectId,
        origin: Origin,
    ) {
        let _was = self
            .areas
            .workspace
            .sidebar
            .removing
            .insert((machine.to_owned(), project), origin);
        self.ask(machine, Ask::Project(ProjectAsk::Remove(project)));
    }

    /// The node answered a removal: the row stops spinning; the sidebar's
    /// says nothing when done and a toast when refused, the page's says
    /// both on its page.
    pub(crate) fn project_removal_answered(
        &mut self,
        machine: &str,
        project: ProjectId,
        result: Result<(), Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        let origin = self
            .areas
            .workspace
            .sidebar
            .removing
            .remove(&(machine.to_owned(), project));
        match (origin, result) {
            (Some(Origin::Sidebar), Err(problem)) => {
                let words = self.words();
                let name = self
                    .areas
                    .workspace
                    .projects
                    .get(machine)
                    .and_then(|view| view.project(&project))
                    .map_or_else(|| project.to_string(), |info| info.name.to_string());
                let said = words.not_removed(&name, &words.problem(&problem));
                self.toast(said, ToastTone::Error, cx);
            }
            (Some(Origin::Sidebar), Ok(())) => {
                // Paseo's row just goes; the page's "forgot" line at the
                // frame's foot is the page's.
                let notice = self.notice.clone();
                self.forgot_project(machine, project, Ok(()));
                self.notice = notice;
            }
            (Some(Origin::Page) | None, result) => self.forgot_project(machine, project, result),
        }
    }
}

/// The row's end: the plus (when `new_workspace`), then the kebab.
pub(super) fn trailing(
    target: &Target,
    new_workspace: Option<&Target>,
    group: &SharedString,
    compact: bool,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let rows = &shell.areas.workspace.sidebar;
    let menu_open = rows.menu_is_for(target);
    let hidden = |element: vessel_app_kit::ui::Stateful<Div>, keep: bool| {
        element.when(!(compact || keep), |element| {
            element
                .opacity(0.0)
                .group_hover(group.clone(), |style| style.opacity(1.0))
        })
    };
    let words = shell.t.lang.sidebar();
    let plus = new_workspace.map(|target| {
        let pressed = target.clone();
        let key = format!("{group}-new");
        let button = icon_button(
            ElementId::Name(key.clone().into()),
            Glyph::Plus,
            PLUS,
            ACTION,
            SidebarSurfaces::of(tokens).hover,
            tokens,
        )
        .role(gpui::Role::Button)
        .aria_label(words.create_workspace_for(&target.name))
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .on_click(cx.listener(move |this, _event, window, cx| {
            cx.stop_propagation();
            this.new_workspace_in(&pressed, window, cx);
        }));
        let button = rows.tips.on_configured(
            button,
            &rows.place(&key),
            words.new_workspace,
            Configuration::new(Side::Bottom, Offset::EIGHT, Delay::IMMEDIATE),
            ControlSizing::from_compact(compact),
        );
        hidden(button, false)
    });
    let opened = target.clone();
    let (side, mark) = if compact {
        (ACTION_COMPACT, KEBAB_COMPACT)
    } else {
        (ACTION, KEBAB)
    };
    let kebab = icon_button(
        ElementId::Name(format!("{group}-menu").into()),
        Glyph::MoreVertical,
        mark,
        side,
        tokens.neutral.surface2,
        tokens,
    )
    .when(compact, |kebab| kebab.mr(px(KEBAB_BACK)).my(px(KEBAB_BACK)))
    .role(gpui::Role::Button)
    .aria_label(words.project_actions)
    .on_mouse_down(
        MouseButton::Left,
        cx.listener(move |this, event: &MouseDownEvent, window, cx| {
            cx.stop_propagation();
            this.project_menu_open(opened.clone(), event.position, window, cx);
        }),
    )
    .on_click(|_event, _window, cx| {
        cx.stop_propagation();
    });
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(ACTIONS_GAP))
        .flex_none()
        .mr(px(ACTIONS_BACK))
        .children(plus)
        .child(hidden(kebab, menu_open))
}

/// A square action: `glyph` muted, the foreground and `wash` behind it on
/// hover.
fn icon_button(
    id: ElementId,
    glyph: Glyph,
    size: f32,
    square: f32,
    wash: vessel_app_kit::ui::Hsla,
    tokens: &Tokens,
) -> vessel_app_kit::ui::Stateful<Div> {
    let neutral = tokens.neutral;
    let group: SharedString = format!("{id}").into();
    div()
        .id(id)
        .group(group.clone())
        .size(px(square))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(radius::MD))
        .cursor_pointer()
        .hover(move |style| style.bg(wash))
        .child(
            svg()
                .path(glyph.path())
                .size(px(size))
                .flex_none()
                .text_color(neutral.foreground_muted)
                .group_hover(group, move |style| style.text_color(neutral.foreground)),
        )
}

/// The menu while open, the remove question while asked, and the plus's
/// tooltip.
pub(crate) fn layers(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let rows = &shell.areas.workspace.sidebar;
    let words = shell.t.lang.sidebar();
    let phone = breakpoint::phone(window);
    let mut drawn = Vec::new();
    if let Some(target) = rows.target.as_ref() {
        let removing = rows.is_removing(&target.machine, target.project);
        let mut remove = MenuItem::new(words.remove_project).with_leading(Glyph::Trash2);
        if removing {
            remove.status = ItemStatus::Pending(words.removing_project.into());
        }
        let items = [
            MenuItem::new(words.open_project_settings).with_leading(Glyph::Gear),
            remove,
        ];
        let handlers = MenuHandlers {
            state: |shell: &mut Shell| &mut shell.areas.workspace.sidebar.menu,
            chose: Rc::new(|shell: &mut Shell, index, window, cx| {
                shell.project_menu_chose(index, window, cx);
            }),
        };
        let content = Content::new("sidebar-project-menu", &items)
            .with_width(px(MENU_WIDTH))
            .with_sheet(words.project_actions);
        drawn.extend(menu_with(content, &rows.menu, &handlers, tokens, phone, cx));
    }
    if let Some(target) = rows.confirm.as_ref() {
        let footer = vec![
            button("project-remove-cancel", words.cancel, false, tokens)
                .on_click(cx.listener(|shell, _event, window, cx| {
                    shell.project_remove_dismiss(window, cx);
                }))
                .into_any_element(),
            button_with_kind(
                "project-remove-confirm",
                words.remove_confirm,
                ButtonKind::Destructive,
                ButtonState::Enabled,
                tokens,
            )
            .on_click(cx.listener(|shell, _event, window, cx| {
                shell.project_remove_confirmed(window, cx);
            }))
            .into_any_element(),
        ];
        drawn.push(dialog(
            &rows.dialog,
            DialogContent {
                title: words.remove_project_title.into(),
                body: div()
                    .type_(Type::Body)
                    .child(words.remove_project_body(&target.name))
                    .into_any_element(),
                footer,
                size: DialogSize::Normal,
            },
            tokens,
            window,
            phone,
            Shell::project_remove_dismiss,
            cx,
        ));
    }
    drawn.extend(rows.tips.render(tokens));
    drawn
}

#[cfg(test)]
#[path = "actions_tests.rs"]
mod tests;
