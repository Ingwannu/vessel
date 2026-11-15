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

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, SharedString, Stateful, StatefulInteractiveElement as _, Styled as _,
    Window, div, px,
};
use vessel_app_kit::{Scheme, Tokens, Type, TypeExt as _, glyph, initial_of, initial_tile};
use vessel_app_view::settings::Section;
use vessel_proto::ProjectId;
use vessel_proto::workspace::ProjectState;

use super::PlacePick;
use crate::routes::Route;
use crate::shell::Shell;

/// Rows shown before "All projects".
const SHOWN: usize = 8;

/// One row's facts.
struct Entry {
    machine: String,
    project: ProjectId,
    name: String,
    /// Why it cannot be started in, in words (`None`: ready).
    state: Option<String>,
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
            entries.push(Entry {
                machine: machine.name.clone(),
                project: project.id,
                name: project.name.to_string(),
                state: match project.state {
                    ProjectState::Ready => None,
                    ProjectState::Cloning { .. }
                    | ProjectState::CloneFailed { .. }
                    | ProjectState::Missing
                    | ProjectState::Unknown => words.project_state(&project.state),
                },
            });
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
        section = section.child(project_row(index, entry, machines_with > 1, tokens, cx));
    }
    if total > SHOWN {
        let words = if unfolded {
            labels.fewer_projects.to_owned()
        } else {
            format!("{} ({total})", labels.all_projects)
        };
        section = section.child(
            base_row("sidebar-all-projects", tokens)
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

/// Paseo's sidebar hover surface (`surfaceSidebarHover`: the third
/// surface in the dark, the first in the light).
const fn hover(tokens: &Tokens) -> vessel_app_kit::ui::Hsla {
    match tokens.scheme {
        Scheme::Dark => tokens.neutral.surface3,
        Scheme::Light => tokens.neutral.surface1,
    }
}

/// Paseo's section header: the title at 8 from the edge, 12 and muted;
/// its action a 24 square at the end.
fn header(title: &'static str, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let neutral = tokens.neutral;
    let wash = hover(tokens);
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
                .text_color(neutral.foreground_muted)
                .child(title),
        )
        .child(
            div()
                .id("sidebar-add-project")
                .size(px(24.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.0))
                .cursor_pointer()
                .hover(move |style| style.bg(wash))
                .child(glyph(Glyph::Plus, 14.0, neutral.foreground_muted))
                .on_click(cx.listener(|this, _event, window, cx| {
                    let machine = this.projects_machine();
                    this.projects_page(machine, true, window, cx);
                })),
        )
}

/// A row's frame: Paseo's `projectRow`.
fn base_row(id: impl Into<vessel_app_kit::ui::ElementId>, tokens: &Tokens) -> Stateful<Div> {
    let wash = hover(tokens);
    let pressed = tokens.neutral.surface2;
    div()
        .id(id)
        .min_h(px(36.0))
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

fn project_row(
    index: usize,
    entry: Entry,
    show_machine: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let Entry {
        machine,
        project,
        name,
        state,
    } = entry;
    let ready = state.is_none();
    let trailing: Option<SharedString> = state
        .map(Into::into)
        .or_else(|| show_machine.then(|| machine.clone().into()));
    base_row(
        vessel_app_kit::ui::ElementId::NamedInteger(
            "sidebar-project".into(),
            u64::try_from(index).unwrap_or(u64::MAX),
        ),
        tokens,
    )
    .child(initial_tile(initial_of(&name), tokens))
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
    .on_click(cx.listener(move |this, _event, window, cx| {
        if ready {
            this.chat_in_project(machine.clone(), project, window, cx);
        } else {
            this.projects_page(Some(machine.clone()), false, window, cx);
        }
    }))
    .when(!ready, |row| row.opacity(0.8))
}
