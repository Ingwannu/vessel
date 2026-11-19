//! The sidebar's projects section (the slot `areas::sidebar::projects`
//! leaves for this track): Paseo's section header ("Projects", 12 and
//! muted, an add button at its end) and its project rows (36 high, 8
//! inside, radius 8, the project's mark 16, the name 14 and muted, the
//! sidebar's hover surface).
//!
//! [Decision Log]
//! - Purpose: reach a project from anywhere in two presses: a row starts a
//!   new chat in it (the composer opens with the project in its tray).
//! - Constraint: Paseo's rows open a project's workspaces; Vessel has no
//!   workspace screen, and a project's sessions are listed under their
//!   machine already.
//! - Alternatives: a row opening the project's page in Settings (a place to
//!   manage, not to work); rows that fold out the project's worktrees
//!   (sessions twice in one sidebar).
//! - Choice: a row is "a new chat here"; a project not ready (cloning, a
//!   failed clone, its folder gone) opens its page in Settings, where the
//!   reason and what to do are; the header's plus opens Settings > Projects
//!   with its add menu.
//! - Cost: the section grows with the projects: past [`SHOWN`] rows the
//!   rest are behind one row, "All projects (N)", which unfolds them in
//!   place ("Show fewer" folds them back).
//! - Why in place (2026-11-15, the coordinator's find on a fleet of two
//!   machines): the row counted every machine's projects but opened
//!   Settings > Projects, which shows one machine (`build-server`, "No
//!   projects yet", while the fourteen were on `laptop`). Paseo's sidebar
//!   lists every host's projects in the sidebar itself (its Display menu
//!   filters them; there is no "all" screen), so the count and what the
//!   press shows are now the same rows.
//! - The row as Paseo's project row (TRACKER 662, `ProjectHeaderRow`,
//!   `ProjectLeadingVisual`): the mark in a slot as tall as the title's
//!   line, the project's status as a badge on its corner (`kit::
//!   status_mark`, the most urgent of its sessions), an 8 spinner in its
//!   place while the project is removed; at its end, shown on hover and
//!   always on a phone, "New workspace" (a new chat in a new worktree, for
//!   a git project) and the project's menu (`actions`); the sidebar's
//!   fills (Paseo's dark theme hovers on `surface1`, not `surface3`).
//!   Paseo never fills a project header as selected; neither does this.
//!   The mark is TRACKER 809's (`project_icons::mark`, 16, 20 on a
//!   phone), the badge hanging from its box.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::loading::{Points, Size, activity_indicator};
use vessel_app_kit::status_mark::{Knockout, SLOT_HEIGHT, icon_box, marked, status_badge};
use vessel_app_kit::theme::SidebarSurfaces;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, SharedString, Stateful, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, glyph};
use vessel_app_view::settings::Section;
use vessel_proto::ProjectId;
use vessel_proto::workspace::{ProjectInfo, ProjectState};

use super::PlacePick;
use crate::routes::Route;
use crate::shell::Shell;

mod actions;
pub(crate) mod tree;

pub(crate) use actions::{Origin, ProjectRows, layers};

/// Rows shown before "All projects".
const SHOWN: usize = 8;

/// One row's facts.
struct Entry {
    machine: String,
    project: ProjectId,
    name: String,
    /// Why it cannot be started in, in words (`None`: ready).
    state: Option<String>,
    /// Whether it is a git repository (it can have worktrees).
    git: bool,
    /// Its icon, or its initial on a tile (`project_icons`).
    mark: AnyElement,
}

impl Entry {
    fn of(shell: &Shell, machine: &str, project: &ProjectInfo, compact: bool) -> Self {
        let name = project.name.to_string();
        Self {
            machine: machine.to_owned(),
            project: project.id,
            mark: super::project_icons::mark(shell, machine, project.id, &name, compact),
            name,
            git: project.git,
            state: match project.state {
                ProjectState::Ready => None,
                ProjectState::Cloning { .. }
                | ProjectState::CloneFailed { .. }
                | ProjectState::Missing
                | ProjectState::Unknown => shell.words().project_state(&project.state),
            },
        }
    }
}

impl Shell {
    /// Start a new chat in `project` on `machine`: the composer, the
    /// project in its tray.
    fn chat_in_project(
        &mut self,
        machine: String,
        project: ProjectId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.draft.machine.as_deref() != Some(machine.as_str()) {
            self.choose_machine(machine, window, cx);
        }
        self.choose_place(PlacePick::Project(project), window, cx);
        self.go(Route::Draft, window, cx);
    }

    /// Settings > Projects on `machine`, its add menu open when `add`.
    fn projects_page(
        &mut self,
        machine: Option<String>,
        add: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if machine.is_some() {
            self.areas.settings.machine = machine;
        }
        self.open_settings(Some(Section::Projects), window, cx);
        if add {
            self.open_add_menu();
        }
    }
}

/// The section, when a machine online has projects to list or can have
/// them (its list read).
pub(crate) fn projects(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let words = shell.words();
    let mut entries = Vec::new();
    let mut machines_with = 0_usize;
    let mut any_list = false;
    for machine in shell.fleet.online() {
        let Some(list) = shell
            .areas
            .workspace
            .projects
            .get(&machine.name)
            .and_then(|view| view.list())
        else {
            continue;
        };
        any_list = true;
        if !list.projects.is_empty() {
            machines_with = machines_with.saturating_add(1);
        }
        for project in &list.projects {
            entries.push(Entry::of(shell, &machine.name, project, compact));
        }
    }
    if !any_list {
        return None;
    }
    let labels = words.labels();
    let total = entries.len();
    let mut section = div()
        .flex()
        .flex_col()
        .child(header(labels.projects, tokens, cx));
    if entries.is_empty() {
        section = section.child(
            div()
                .px(px(8.0))
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(labels.no_projects),
        );
    }
    let unfolded = shell.areas.workspace.projects_unfolded;
    let shown = if unfolded { total } else { SHOWN };
    for (index, entry) in entries.into_iter().take(shown).enumerate() {
        let row = Row {
            index,
            show_machine: machines_with > 1,
            compact,
            fold: None,
        };
        section = section.child(project_row(row, entry, shell, tokens, cx));
    }
    if total > SHOWN {
        let words = if unfolded {
            labels.fewer_projects.to_owned()
        } else {
            format!("{} ({total})", labels.all_projects)
        };
        section = section.child(
            base_row("sidebar-all-projects", compact, tokens)
                .child(
                    div()
                        .flex_1()
                        .type_(Type::Body)
                        .text_color(tokens.neutral.foreground_muted)
                        .child(words),
                )
                .on_click(cx.listener(|this, _event, _window, cx| {
                    let workspace = &mut this.areas.workspace;
                    workspace.projects_unfolded = !workspace.projects_unfolded;
                    cx.notify();
                })),
        );
    }
    Some(section.into_any_element())
}

/// Paseo's sidebar hover surface (`surfaceSidebarHover`).
const fn hover(tokens: &Tokens) -> vessel_app_kit::ui::Hsla {
    SidebarSurfaces::of(tokens).hover
}

/// A row's least height, on a wide window and on a phone (Paseo's
/// `density.rowHeight`).
const ROW: f32 = 36.0;
const ROW_COMPACT: f32 = 44.0;

/// Paseo's section header: the title at 8 from the edge, 12 and muted;
/// its action a 24 square at the end.
fn header(title: &'static str, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    section_header(title, tokens).child(add_project(tokens, cx))
}

/// The section header's frame and title, its actions to follow.
fn section_header(title: &'static str, tokens: &Tokens) -> Div {
    div()
        .min_h(px(36.0))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .pl(px(8.0))
        .pr(px(4.0))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(title),
        )
}

/// The header's plus: Settings > Projects with its add menu.
fn add_project(tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Stateful<Div> {
    let wash = hover(tokens);
    div()
        .id("sidebar-add-project")
        .size(px(24.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(move |style| style.bg(wash))
        .child(glyph(Glyph::Plus, 14.0, tokens.neutral.foreground_muted))
        .on_click(cx.listener(|this, _event, window, cx| {
            let machine = this.projects_machine();
            this.projects_page(machine, true, window, cx);
        }))
}

/// A row's frame: Paseo's `projectRow`.
fn base_row(id: impl Into<ElementId>, compact: bool, tokens: &Tokens) -> Stateful<Div> {
    let wash = hover(tokens);
    let pressed = tokens.neutral.surface2;
    div()
        .id(id)
        .min_h(px(if compact { ROW_COMPACT } else { ROW }))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(8.0))
        .py(px(8.0))
        .mb(px(4.0))
        .rounded(px(8.0))
        .cursor_pointer()
        .hover(move |style| style.bg(wash))
        .active(move |style| style.bg(pressed))
}

/// Where a row sits and how it is drawn.
#[derive(Debug, Clone, Copy)]
struct Row {
    index: usize,
    /// More than one machine has projects: a ready row names its machine.
    show_machine: bool,
    /// A phone's width.
    compact: bool,
    /// In the project tree (TRACKER 682): whether it is folded. A ready
    /// header's press folds it, its mark gives way to the chevron on hover.
    fold: Option<bool>,
}

fn project_row(
    row: Row,
    entry: Entry,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let Entry {
        machine,
        project,
        name,
        state,
        git,
        mark,
    } = entry;
    let ready = state.is_none();
    let trailing: Option<SharedString> = state
        .map(Into::into)
        .or_else(|| row.show_machine.then(|| machine.clone().into()));
    let group: SharedString = format!("sidebar-project-{machine}-{project}").into();
    let target = actions::Target {
        machine: machine.clone(),
        project,
        name: name.clone(),
    };
    let label: SharedString = name.clone().into();
    let removing = shell.areas.workspace.sidebar.is_removing(&machine, project);
    let leading = if removing {
        archiving(&group, row.compact, tokens)
    } else {
        let words = shell.t.lang.sidebar();
        let knockout = Knockout {
            group: group.clone(),
            selected: false,
        };
        let badge = shell
            .fleet
            .project_bucket(&machine, project, shell.prefs.settings.sidebar)
            .and_then(|bucket| {
                status_badge(
                    ElementId::Name(format!("{group}-status").into()),
                    crate::areas::sidebar::status_mark(bucket),
                    words.bucket(bucket),
                    &knockout,
                    tokens,
                )
            });
        marked(mark, row.compact, badge).into_any_element()
    };
    // In the tree a ready header folds; its chevron takes the mark's place
    // on hover (Paseo's `ProjectLeadingVisual` `showChevron`).
    let fold = row.fold.filter(|_| ready);
    let leading = match fold {
        Some(folded) => tree::with_chevron(leading, folded, &group, row.compact),
        None => leading,
    };
    let new_workspace = (ready && git).then(|| target.clone());
    base_row(
        ElementId::NamedInteger(
            "sidebar-project".into(),
            u64::try_from(row.index).unwrap_or(u64::MAX),
        ),
        row.compact,
        tokens,
    )
    .group(group.clone())
    .child(leading)
    .child(
        div()
            .flex_1()
            .min_w_0()
            .truncate()
            .type_(Type::Body)
            .text_color(neutral.foreground_muted)
            .child(name),
    )
    .children(trailing.map(|trailing| {
        div()
            .flex_none()
            .type_(Type::Tiny)
            .text_color(neutral.foreground_extra_muted)
            .child(trailing)
    }))
    .child(actions::trailing(
        &target,
        new_workspace.as_ref(),
        &group,
        row.compact,
        shell,
        tokens,
        cx,
    ))
    // A button named by the project wherever it is drawn (TRACKER 813, the
    // app reviewer's note on 682): a ready header in the tree folds and says
    // whether it is open; elsewhere a press starts a chat or, not ready,
    // opens its page in Settings.
    .role(gpui::Role::Button)
    .aria_label(label)
    .when_some(fold, |row, folded| row.aria_expanded(!folded))
    .on_click(cx.listener(move |this, _event, window, cx| {
        if fold.is_some() {
            this.fold_project(&machine, project, cx);
        } else if ready {
            this.chat_in_project(machine.clone(), project, window, cx);
        } else {
            this.projects_page(Some(machine.clone()), false, window, cx);
        }
    }))
    .when(!ready, |row| row.opacity(0.8))
}

/// Paseo's 8 activity indicator in the mark's slot while the project is
/// removed (`ProjectLeadingVisual` `isArchiving`).
fn archiving(group: &SharedString, compact: bool, tokens: &Tokens) -> AnyElement {
    div()
        .w(px(icon_box(compact)))
        .h(px(SLOT_HEIGHT))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(activity_indicator(
            ElementId::Name(format!("{group}-removing").into()),
            Size::Numeric(Points::EIGHT),
            tokens.neutral.foreground_muted,
        ))
        .into_any_element()
}
