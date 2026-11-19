//! The sidebar grouped by project (the Display menu's "Project", the
//! native default since TRACKER 682): every reachable machine's projects
//! by name, each heading its sessions (`Fleet::sidebar_tree`), then each
//! machine's sessions in no listed project under its name and state.
//!
//! [Decision Log]
//! - Purpose: Paseo's default sidebar (`sidebar-workspace-list.tsx`):
//!   work found by its project, a project folded away when it is not
//!   needed.
//! - Before: the projects section listed projects to start a chat in, and
//!   the sessions were listed again by machine under it; a session's
//!   project was not on screen.
//! - Choice: one list. A project block is Paseo's `ProjectBlock`: its
//!   header (`workspace::sidebar_tree::header`), its sessions flush with
//!   the header (Paseo's project rows are not indented), Show more / Show
//!   less when the limit hides any (Paseo's `SidebarGroupToggleRow`), or
//!   the New workspace row when it has none; 12 under an open block, so a
//!   column of folded headers closes up. A machine's sessions in no listed
//!   project follow as the machine grouping draws them (`machines::group`,
//!   the coordinator's ruling: an unreachable machine stays in sight).
//! - Show more is not kept across launches (Paseo keeps it in the row's
//!   state); a fold is (`workspace::sidebar_tree::fold_project`).
//! - Differences, named: Paseo's dragged project order is TRACKER 811
//!   (projects sort by name until then); a project on two machines has a
//!   header on each; the trailing machine groups are Vessel's (Paseo has
//!   no session outside a workspace).

use std::collections::HashSet;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::status_mark::icon_box;
use vessel_app_kit::theme::{SidebarSurfaces, spacing};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, SharedString, Stateful, StatefulInteractiveElement as _,
    Styled as _, div, px, svg,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::Card;
use vessel_app_view::prefs::SidebarView;
use vessel_app_view::sidebar::tree::{SidebarProject, SidebarTree, TreeState};
use vessel_proto::ProjectId;

use super::layout::EndTarget;
use super::session_row::{RowExtras, session_row};
use super::{Numbers, machines};
use crate::areas::workspace::sidebar_tree;
use crate::routes::Route;
use crate::shell::Shell;

/// The toggle row (Paseo's `SidebarGroupToggleRow`, a session row's box):
/// its least height, then its padding, corners, gap and chevron.
const TOGGLE: f32 = 36.0;
const TOGGLE_TOUCH: f32 = 44.0;
const TOGGLE_BELOW: f32 = 2.0;
const TOGGLE_Y: f32 = 8.0;
const TOGGLE_LEFT: f32 = 8.0;
const TOGGLE_RIGHT: f32 = 12.0;
const TOGGLE_CORNER: f32 = 8.0;
const TOGGLE_GAP: f32 = 8.0;
const CHEVRON: f32 = 14.0;
const CHEVRON_TOUCH: f32 = 18.0;

/// The projects showing every session (Show more pressed), by machine.
pub(super) type ShownAll = HashSet<(String, ProjectId)>;

/// What the tree is drawn with from `shell`.
fn tree_state(shell: &Shell) -> TreeState<'_> {
    let open = shell
        .areas
        .session
        .open
        .as_ref()
        .filter(|_| shell.route == Route::Session)
        .map(|open| (open.machine.as_str(), open.session));
    TreeState {
        lists: &shell.areas.workspace.projects,
        folded: &shell.prefs.settings.folded_projects,
        unfolded_more: &shell.areas.sidebar.shown_all,
        open,
    }
}

/// The tree as `shell` shows it.
fn tree(shell: &Shell, view: SidebarView) -> SidebarTree<'_> {
    shell.fleet.sidebar_tree(view, tree_state(shell))
}

/// The sessions as the tree draws them: what the shortcut numbers count.
pub(super) fn order(shell: &Shell) -> Vec<&Card> {
    tree(shell, shell.prefs.settings.sidebar).order()
}

/// The list: the projects' blocks, then the machines' groups.
pub(super) fn groups(
    view: SidebarView,
    numbers: &Numbers,
    compact: bool,
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let tree = tree(shell, view);
    let mut list = div().flex().flex_col();
    let final_project = tree
        .machines
        .is_empty()
        .then(|| tree.projects.len().checked_sub(1))
        .flatten();
    for (index, project) in tree.projects.iter().enumerate() {
        let drawn = Block {
            index,
            several_machines: tree.several_machines,
            compact,
            touch,
        };
        let block = block(project, drawn, numbers, shell, tokens, cx);
        list = list
            .child(EndTarget::at(index, final_project).wrap(block, &shell.areas.sidebar.layout));
    }
    if !tree.machines.is_empty() {
        let mut loose = div().flex().flex_col().gap(px(spacing::S3_5));
        let final_machine = tree.machines.len().checked_sub(1);
        for (index, listed) in tree.machines.iter().enumerate() {
            loose = loose.child(machines::group(
                listed,
                EndTarget::at(index, final_machine),
                numbers,
                shell,
                tokens,
                touch,
                cx,
            ));
        }
        list = list.child(loose.when(!tree.projects.is_empty(), |loose| loose.pt(px(spacing::S3))));
    }
    list
}

/// How a block is drawn.
#[derive(Debug, Clone, Copy)]
struct Block {
    index: usize,
    /// More than one machine has projects: a header names its machine.
    several_machines: bool,
    /// A phone's width (the project rows).
    compact: bool,
    /// The phone's sheet (the session rows).
    touch: bool,
}

/// A project's header and what it holds while open (Paseo's
/// `ProjectBlock`, a group named by the project).
fn block(
    project: &SidebarProject<'_>,
    drawn: Block,
    numbers: &Numbers,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let header = sidebar_tree::header(
        project,
        drawn.index,
        drawn.several_machines,
        drawn.compact,
        shell,
        tokens,
        cx,
    );
    let mut held: Vec<AnyElement> = Vec::new();
    if !project.folded {
        if project.total > 0 {
            let mut rows = div()
                .flex()
                .flex_col()
                .gap(px(spacing::S0_5))
                .pb(px(spacing::S0_5));
            for card in project.leading.iter().chain(&project.sessions) {
                let extras = RowExtras {
                    machine: None,
                    number: numbers.of(card),
                    flush: true,
                };
                rows = rows.child(session_row(card, extras, shell, tokens, drawn.touch, cx));
            }
            held.push(rows.into_any_element());
            if project.can_toggle {
                held.push(toggle(project, drawn.touch, shell, tokens, cx).into_any_element());
            }
        } else if let Some(ghost) = sidebar_tree::ghost(project, drawn.compact, shell, tokens, cx) {
            held.push(ghost.into_any_element());
        }
    }
    let name: SharedString = project.project.name.to_string().into();
    div()
        .id(ElementId::Name(
            format!("sidebar-tree-{}-{}", project.machine, project.project.id).into(),
        ))
        .role(gpui::Role::Group)
        .aria_label(name)
        .flex()
        .flex_col()
        .when(!held.is_empty(), |block| block.pb(px(spacing::S3)))
        .child(header)
        .children(held)
        .into_any_element()
}

/// Show more (every session of the project) or Show less (back to the
/// limit), as Paseo's toggle row: a session row's box, the chevron in the
/// status slot's place, both muted until hovered.
fn toggle(
    project: &SidebarProject<'_>,
    touch: bool,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let words = shell.t.lang.sidebar();
    let (label, chevron) = if project.expanded {
        (words.show_less, Glyph::ChevronUp)
    } else {
        (words.show_more, Glyph::ChevronDown)
    };
    let key = (project.machine.to_owned(), project.project.id);
    let group: SharedString =
        format!("sidebar-more-{}-{}", project.machine, project.project.id).into();
    let fills = SidebarSurfaces::of(tokens);
    let neutral = tokens.neutral;
    div()
        .id(ElementId::Name(group.clone()))
        .group(group.clone())
        .min_h(px(if touch { TOGGLE_TOUCH } else { TOGGLE }))
        .mb(px(TOGGLE_BELOW))
        .py(px(TOGGLE_Y))
        .pl(px(TOGGLE_LEFT))
        .pr(px(TOGGLE_RIGHT))
        .rounded(px(TOGGLE_CORNER))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(TOGGLE_GAP))
        .cursor_pointer()
        // The words take the row's own hover: a text colour set by a group
        // hover is fixed when the words are laid out, before the pointer is
        // known (the plus and the chevron are painted, so theirs follow).
        .text_color(neutral.foreground_muted)
        .hover(move |style| style.bg(fills.hover).text_color(neutral.foreground))
        .active(move |style| style.bg(fills.pressed).text_color(neutral.foreground))
        .role(gpui::Role::Button)
        .aria_label(label)
        .child(
            div()
                .size(px(icon_box(touch)))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    svg()
                        .path(chevron.path())
                        .size(px(if touch { CHEVRON_TOUCH } else { CHEVRON }))
                        .flex_none()
                        .text_color(neutral.foreground_muted)
                        .group_hover(group, move |style| {
                            style.text_color(neutral.foreground)
                        }),
                ),
        )
        .child(div().min_w_0().truncate().type_(Type::Body).child(label))
        .on_click(cx.listener(move |this, _event, _window, cx| {
            let shown = &mut this.areas.sidebar.shown_all;
            if !shown.remove(&key) {
                let _new = shown.insert(key.clone());
            }
            cx.notify();
        }))
}
