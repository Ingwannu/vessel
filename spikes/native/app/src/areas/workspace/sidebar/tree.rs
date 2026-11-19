//! A project's parts of the sidebar grouped by project (TRACKER 682, the
//! list itself is `areas::sidebar::projects`): the section's heading, a
//! project's header (the project row, folding) and the "New workspace"
//! row under a project with no session.
//!
//! [Decision Log]
//! - Purpose: Paseo's project tree (`sidebar-workspace-list.tsx`
//!   `ProjectBlock`, `ProjectHeaderRow`, `NewWorkspaceGhostRow`) on the
//!   project rows TRACKER 662 drew, so one row is a project in both lists.
//! - The header: the project row as it is, its press folding the project
//!   (Paseo's) instead of starting a chat (the row's press in the projects
//!   section, which this list replaces); on hover the chevron takes the
//!   mark's place (`ProjectInlineChevron`: down while open, right while
//!   folded, 14 in Paseo's literal grey). A row not ready keeps opening its
//!   page in Settings, where the reason is, and draws no chevron.
//! - The fold is kept (`Preferences::folded_projects`), as Paseo keeps its
//!   collapsed projects; the document the page shares is unchanged until a
//!   project is folded.
//! - The ghost row (Paseo's `rowModel.trailingAction`, `new_workspace`):
//!   for a ready git project, where the header offers New workspace too.
//! - The heading: the projects section's ("Projects", the plus) with the
//!   Display trigger, since this list replaces both the section and the
//!   list's own heading. Paseo's says "Workspaces" over the same list (its
//!   rows are workspaces; a Vessel row is a session): a difference, named.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::status_mark::{SLOT_HEIGHT, icon_box};
use vessel_app_kit::theme::{SidebarSurfaces, spacing};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, InteractiveElement as _, IntoElement as _,
    ParentElement as _, SharedString, Stateful, StatefulInteractiveElement as _, Styled as _, div,
    px, svg,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, glyph};
use vessel_app_link::Command;
use vessel_app_view::sidebar::tree::SidebarProject;
use vessel_proto::ProjectId;
use vessel_proto::workspace::ProjectState;

use super::{Entry, Row, actions, add_project, project_row, section_header};
use crate::shell::Shell;

/// The chevron (Paseo's `size={14}`, on a phone too).
const CHEVRON: f32 = 14.0;
/// The ghost row (Paseo's `newWorkspaceGhostRow`): a session row's box,
/// on the grouped rows' rail (16 from the left, its plus in the status
/// slot's place); its plus at the `sm` icon size.
const GHOST: f32 = 36.0;
const GHOST_COMPACT: f32 = 44.0;
const GHOST_LEFT: f32 = 16.0;
const GHOST_RIGHT: f32 = 12.0;
const GHOST_Y: f32 = 8.0;
const GHOST_BELOW: f32 = 2.0;
const GHOST_GAP: f32 = 8.0;
const GHOST_CORNER: f32 = 8.0;
const PLUS: f32 = 14.0;
const PLUS_COMPACT: f32 = 18.0;

impl Shell {
    /// Fold `project` on `machine` when open, open it when folded; kept
    /// with the preferences.
    pub(crate) fn fold_project(
        &mut self,
        machine: &str,
        project: ProjectId,
        cx: &mut Context<'_, Self>,
    ) {
        self.prefs.settings.folded_projects.toggle(machine, project);
        self.command(Command::SavePrefs(Box::new(self.prefs.clone())));
        cx.notify();
    }
}

/// The list's heading: "Projects", the Display menu's trigger, the plus.
pub(crate) fn heading(
    shell: &Shell,
    trigger: AnyElement,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let labels = shell.words().labels();
    section_header(labels.projects, tokens).child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::S0_5))
            .child(trigger)
            .child(add_project(labels.add_project, tokens, cx)),
    )
}

/// `project`'s header; `index` keeps its element apart from the others'.
pub(crate) fn header(
    project: &SidebarProject<'_>,
    index: usize,
    several_machines: bool,
    compact: bool,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let row = Row {
        index,
        show_machine: several_machines,
        compact,
        fold: Some(project.folded),
    };
    project_row(
        row,
        Entry::of(shell, project.machine, project.project, compact),
        shell,
        tokens,
        cx,
    )
}

/// `leading` giving way to the chevron while the row (`group`) is hovered.
pub(super) fn with_chevron(
    leading: AnyElement,
    folded: bool,
    group: &SharedString,
    compact: bool,
) -> AnyElement {
    let chevron = if folded {
        Glyph::ChevronRight
    } else {
        Glyph::ChevronDown
    };
    div()
        .relative()
        .flex_none()
        .child(
            div()
                .group_hover(group.clone(), |style| style.opacity(0.0))
                .child(leading),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .w(px(icon_box(compact)))
                .h(px(SLOT_HEIGHT))
                .flex()
                .items_center()
                .justify_center()
                .opacity(0.0)
                .group_hover(group.clone(), |style| style.opacity(1.0))
                .child(glyph(chevron, CHEVRON, SidebarSurfaces::chevron())),
        )
        .into_any_element()
}

/// Paseo's `NewWorkspaceGhostRow` under `project` when it has no session:
/// a new chat in a new worktree of it; `None` unless it is a ready git
/// project.
pub(crate) fn ghost(
    project: &SidebarProject<'_>,
    compact: bool,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<Stateful<Div>> {
    let info = project.project;
    if !info.git || !matches!(info.state, ProjectState::Ready) {
        return None;
    }
    let words = shell.t.lang.sidebar();
    let name = info.name.to_string();
    let label = words.create_workspace_for(&name);
    let target = actions::Target {
        machine: project.machine.to_owned(),
        project: info.id,
        name,
    };
    let group: SharedString = format!("sidebar-ghost-{}-{}", project.machine, info.id).into();
    let fills = SidebarSurfaces::of(tokens);
    let neutral = tokens.neutral;
    let plus = if compact { PLUS_COMPACT } else { PLUS };
    Some(
        div()
            .id(ElementId::Name(group.clone()))
            .group(group.clone())
            .min_h(px(if compact { GHOST_COMPACT } else { GHOST }))
            .mb(px(GHOST_BELOW))
            .py(px(GHOST_Y))
            .pl(px(GHOST_LEFT))
            .pr(px(GHOST_RIGHT))
            .rounded(px(GHOST_CORNER))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(GHOST_GAP))
            .cursor_pointer()
            // The words take the row's own hover (a group hover's text
            // colour is fixed when the words are laid out).
            .text_color(neutral.foreground_muted)
            .hover(move |style| style.bg(fills.hover).text_color(neutral.foreground))
            .active(move |style| style.bg(fills.pressed).text_color(neutral.foreground))
            .role(gpui::Role::Button)
            .aria_label(label)
            .child(
                div()
                    .size(px(icon_box(compact)))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        svg()
                            .path(Glyph::Plus.path())
                            .size(px(plus))
                            .flex_none()
                            .text_color(neutral.foreground_muted)
                            .group_hover(group, move |style| {
                                style.text_color(neutral.foreground)
                            }),
                    ),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .type_(Type::Body)
                    .child(words.new_workspace),
            )
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.new_workspace_in(&target, window, cx);
            })),
    )
}
