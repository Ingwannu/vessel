//! Settings > Projects (Paseo's projects in its settings; the page's
//! `projects-section.tsx`): a machine's projects as rows, Add project at
//! the heading's right (Paseo's sheet, `add_flow`), the machine's git when
//! it cannot be used, and the chosen project's detail under the list.
//!
//! Different from the page, for now: the row's "…" menu is a Remove button
//! (the kit's menu is track C's and not in yet); a removal is said in the
//! frame's foot (the app has no toasts yet).

use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Stateful, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, button};
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::projects::ProjectsView;
use vessel_proto::ProjectId;
use vessel_proto::workspace::{ProjectInfo, ProjectState};

use super::detail::{self, Detail};
use super::widgets::{Tone, badge, banner, divided, empty, row, rows, section, title};
use crate::shell::Shell;

/// The page's state.
pub(crate) struct Page {
    /// The project whose detail is open.
    pub(crate) detail: Option<Detail>,
    /// A removal the node refused, until dismissed.
    failure: Option<String>,
}

impl Page {
    pub(crate) const fn new() -> Self {
        Self {
            detail: None,
            failure: None,
        }
    }
}

impl Shell {
    /// The machine Settings > Projects shows: the one Settings' machine
    /// picker chose, else the first in the book (Settings' own rule).
    pub(crate) fn projects_machine(&self) -> Option<String> {
        let chosen = self.areas.settings.machine.as_deref();
        self.fleet
            .machines
            .iter()
            .find(|machine| Some(machine.name.as_str()) == chosen)
            .or_else(|| self.fleet.machines.first())
            .map(|machine| machine.name.clone())
    }

    /// Translate the Add project sheet's field placeholder without
    /// changing what it holds (the sheet keeps its page across a language
    /// change).
    pub(crate) fn refresh_project_words(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.refresh_add_flow_words(window, cx);
    }

    pub(super) fn forget_project(&mut self, project: ProjectId, cx: &mut Context<'_, Self>) {
        if let Some(machine) = self.projects_machine() {
            self.remove_project_from(&machine, project, super::sidebar::Origin::Page);
        }
        cx.notify();
    }

    /// The node answered a removal.
    pub(super) fn forgot_project(
        &mut self,
        machine: &str,
        project: ProjectId,
        result: Result<(), Problem>,
    ) {
        let words = self.words();
        let name = self
            .areas
            .workspace
            .projects
            .get(machine)
            .and_then(|view| view.project(&project))
            .map_or_else(|| project.to_string(), |info| info.name.to_string());
        match result {
            Ok(()) => {
                if let Some(view) = self.areas.workspace.projects.get_mut(machine) {
                    view.forgot(&project);
                }
                let page = &mut self.areas.workspace.page;
                if page
                    .detail
                    .as_ref()
                    .is_some_and(|open| open.project == project)
                {
                    page.detail = None;
                }
                self.notice = Some(words.forgot(&name));
            }
            Err(problem) => {
                self.areas.workspace.page.failure =
                    Some(words.not_removed(&name, &words.problem(&problem)));
            }
        }
    }

    fn open_project(&mut self, project: &ProjectInfo, cx: &mut Context<'_, Self>) {
        let page = &mut self.areas.workspace.page;
        if page
            .detail
            .as_ref()
            .is_some_and(|open| open.project == project.id)
        {
            page.detail = None;
        } else {
            page.detail = Some(Detail::new(project.id));
            if let Some(machine) = self.projects_machine() {
                self.load_detail(&machine, project);
            }
        }
        cx.notify();
    }
}

/// The facts under a project's name: its folder, then its state or its
/// default branch.
fn facts(shell: &Shell, project: &ProjectInfo) -> Vec<String> {
    let words = shell.words();
    let labels = words.labels();
    let second = words.project_state(&project.state).unwrap_or_else(|| {
        if project.git {
            project
                .default_branch
                .as_ref()
                .map_or_else(|| labels.no_commit_yet.to_owned(), ToString::to_string)
        } else {
            labels.no_git.to_owned()
        }
    });
    vec![project.root.display().to_string(), second]
}

/// Settings > Projects.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words();
    let labels = words.labels();
    let mut column = div().flex().flex_col();
    let Some(machine) = shell.projects_machine() else {
        return column
            .child(empty(labels.no_machines, tokens))
            .into_any_element();
    };
    // A machine that did not answer has no list to wait for: say so in
    // the sidebar's words (the lead's fix of 2026-11-13: the transport's
    // English text and the endpoint were on a Korean screen), the
    // transport's text only in the line's tooltip.
    if let Some(vessel_app_view::Reach::Unreachable(why)) = shell
        .fleet
        .machines
        .iter()
        .find(|known| known.name == machine)
        .map(|known| &known.reach)
    {
        return column
            .child(
                shell
                    .areas
                    .sidebar
                    .unreachable_on_page(&machine, why, shell.t, tokens, compact),
            )
            .into_any_element();
    }
    let view = shell.areas.workspace.projects.get(&machine);
    if let Some(git) = view
        .and_then(ProjectsView::list)
        .and_then(|list| words.git_support(&list.git))
    {
        column = column.child(banner(git, false, None, tokens));
    }
    let state = &shell.areas.workspace.page;
    // A project open is its own page, as Paseo opens a project's settings:
    // a way back to the list, then the project.
    let chosen = state.detail.as_ref().and_then(|open| {
        view.and_then(|view| view.project(&open.project))
            .map(|project| (open, project))
    });
    if let Some((open, project)) = chosen {
        let back = div()
            .id("projects-back")
            .flex()
            .flex_row()
            .items_center()
            .gap(px(4.0))
            .mb(px(12.0))
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .cursor_pointer()
            .hover(|style| style.text_color(tokens.neutral.foreground))
            .child(format!("\u{2039} {}", words.projects_on(&machine)))
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.areas.workspace.page.detail = None;
                cx.notify();
            }));
        return column
            .child(back)
            .child(detail::render(shell, &machine, project, open, tokens, cx))
            .into_any_element();
    }
    let mut action = div().flex().flex_row().items_center().gap(px(8.0));
    if view.and_then(ProjectsView::list).is_some() {
        let target = machine.clone();
        action = action.child(
            button("add-project", labels.add_project, false, tokens).on_click(cx.listener(
                move |this, _event, window, cx| {
                    this.open_add_flow(Some(target.clone()), window, cx);
                },
            )),
        );
    }
    let body = list_body(shell, &machine, view, tokens, compact, cx);
    column = column.child(section(
        words.projects_on(&machine),
        Some(action),
        body,
        tokens,
    ));
    column.into_any_element()
}

/// The section's body: the list, the banners.
fn list_body(
    shell: &Shell,
    machine: &str,
    view: Option<&ProjectsView>,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words();
    let labels = words.labels();
    let state = &shell.areas.workspace.page;
    let mut body = div().flex().flex_col().gap(px(10.0));
    body = body.child(match view {
        None | Some(ProjectsView::Loading) => empty(labels.loading, tokens),
        Some(ProjectsView::Older) => empty(words.older_machine(machine), tokens),
        Some(ProjectsView::Failed(problem)) => empty(words.problem(problem), tokens),
        Some(ProjectsView::Ready(list)) if list.projects.is_empty() => div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .py(px(16.0))
            .child(title(labels.no_projects, tokens))
            .child(empty(words.no_projects_body(machine), tokens)),
        Some(ProjectsView::Ready(list)) => {
            let mut list_rows = rows(tokens);
            for (index, project) in list.projects.iter().enumerate() {
                list_rows =
                    list_rows.child(project_row(shell, index, project, tokens, compact, cx));
            }
            list_rows
        }
    });
    for project in view
        .and_then(ProjectsView::list)
        .map_or(&[][..], |list| list.projects.as_slice())
    {
        if let ProjectState::CloneFailed {
            reason, message, ..
        } = &project.state
        {
            let said = format!(
                "{}: {} {message}",
                project.name,
                words.clone_failure(*reason)
            );
            body = body.child(banner(said, true, None, tokens));
        }
    }
    if let Some(failure) = &state.failure {
        body = body.child(banner(
            failure.clone(),
            true,
            Some(div().child(
                button("dismiss-failure", labels.dismiss, false, tokens).on_click(cx.listener(
                    |this, _event, _window, cx| {
                        this.areas.workspace.page.failure = None;
                        cx.notify();
                    },
                )),
            )),
            tokens,
        ));
    }
    body
}

fn project_row(
    shell: &Shell,
    index: usize,
    project: &ProjectInfo,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let words = shell.words();
    let opened = project.clone();
    let name = project.name.to_string();
    // Paseo's project row: the project's mark (`project_icons::mark`, 16),
    // its name, its state as a badge, a chevron; the whole row opens the
    // project's page.
    let title_line = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .child(super::project_icons::mark(
            shell,
            &shell.projects_machine().unwrap_or_default(),
            project.id,
            &name,
            compact,
        ))
        .child(title(name, tokens));
    let mut controls = div();
    match &project.state {
        ProjectState::CloneFailed { .. } => {
            controls = controls.child(badge(
                words.project_state(&project.state).unwrap_or_default(),
                Tone::Danger,
                tokens,
            ));
        }
        ProjectState::Missing => {
            controls = controls.child(badge(
                words.project_state(&project.state).unwrap_or_default(),
                Tone::Warn,
                tokens,
            ));
        }
        ProjectState::Ready | ProjectState::Cloning { .. } | ProjectState::Unknown => {}
    }
    controls = controls.child(
        div()
            .type_(Type::Label)
            .text_color(tokens.neutral.foreground_muted)
            .child("\u{203a}"),
    );
    let hover = tokens.neutral.surface2;
    div()
        .id(ElementId::Name(format!("project-{index}").into()))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .child(divided(
            row(title_line, &facts(shell, project), Some(controls), tokens),
            index,
            tokens,
        ))
        .on_click(cx.listener(move |this, _event, _window, cx| this.open_project(&opened, cx)))
}
