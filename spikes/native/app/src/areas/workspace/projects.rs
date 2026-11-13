//! Settings > Projects (Paseo's projects in its settings, agent-
//! orchestrator's add-project flow; the page's `projects-section.tsx`): a
//! machine's projects as rows, the way to add one at the heading's right (a
//! folder, a clone, a new one), the machine's git when it cannot be used,
//! and the chosen project's detail under the list.
//!
//! Different from the page, for now: the add menu and the add form open in
//! place under the heading (the kit's menu and dialog are track C's and not
//! in yet); the row's "…" menu is a Remove button for the same reason; a
//! success is said in the frame's foot (the app has no toasts yet).

use vessel_app_kit::segmented::{SegmentSize, segment, segmented};
use vessel_app_kit::ui::{
    Context, Div, ElementId, InteractiveElement as _, ParentElement as _, Stateful,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens, Type, TypeExt as _, button};
use vessel_app_link::workspace::{Ask, Form, ProjectAsk};
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::projects::{ProjectsView, repository_name};
use vessel_app_view::workspace::words::AddForm;
use vessel_proto::ProjectId;
use vessel_proto::workspace::{AddProject, CloneProject, InitProject, ProjectInfo, ProjectState};

use super::detail::{self, Detail};
use super::widgets::{Tone, badge, banner, divided, empty, page, row, rows, section, title};
use crate::shell::Shell;

/// The page's state.
pub(crate) struct Page {
    /// The machine shown (`None`: the first online one).
    pub(crate) machine: Option<String>,
    /// The project whose detail is open.
    pub(crate) detail: Option<Detail>,
    /// Whether the add menu is open.
    menu: bool,
    /// The add form open, if one.
    adding: Option<AddForm>,
    /// The form's fields.
    path: TextField,
    url: TextField,
    /// Whether the form's call is out.
    busy: bool,
    /// Why the form's last call failed.
    error: Option<String>,
    /// A removal the node refused, until dismissed.
    failure: Option<String>,
}

impl Page {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            machine: None,
            detail: None,
            menu: false,
            adding: None,
            path: super::field(window, cx),
            url: super::field(window, cx),
            busy: false,
            error: None,
            failure: None,
        }
    }
}

/// Arriving: settle the machine shown and read its projects again (a
/// project added from the CLI shows here).
pub(crate) fn arrive(shell: &mut Shell) {
    if let Some(machine) = shell.projects_machine() {
        shell.reload_projects(&machine);
    }
}

impl Shell {
    /// The machine Settings > Projects shows: the one chosen if online,
    /// else the composer's, else the first online.
    pub(crate) fn projects_machine(&self) -> Option<String> {
        let online = |name: &str| self.fleet.online().any(|machine| machine.name == name);
        self.areas
            .workspace
            .page
            .machine
            .clone()
            .filter(|name| online(name))
            .or_else(|| self.areas.draft.machine.clone().filter(|name| online(name)))
            .or_else(|| {
                self.fleet
                    .online()
                    .next()
                    .map(|machine| machine.name.clone())
            })
    }

    fn choose_projects_machine(&mut self, machine: String, cx: &mut Context<'_, Self>) {
        self.reload_projects(&machine);
        let page = &mut self.areas.workspace.page;
        page.machine = Some(machine);
        page.detail = None;
        page.adding = None;
        page.menu = false;
        cx.notify();
    }

    fn open_add(&mut self, form: AddForm, window: &mut Window, cx: &mut Context<'_, Self>) {
        let labels = self.words().labels();
        let page = &mut self.areas.workspace.page;
        page.menu = false;
        page.adding = Some(form);
        page.error = None;
        page.path.clear(window, cx);
        page.url.clear(window, cx);
        let placeholder = match form {
            AddForm::Clone => labels.parent_placeholder,
            AddForm::Folder | AddForm::New => labels.folder_placeholder,
        };
        page.path.set_placeholder(placeholder, window, cx);
        page.url.set_placeholder(labels.url_placeholder, window, cx);
        match form {
            AddForm::Clone => page.url.focus(window, cx),
            AddForm::Folder | AddForm::New => page.path.focus(window, cx),
        }
        cx.notify();
    }

    fn submit_add(&mut self, cx: &mut Context<'_, Self>) {
        let Some(machine) = self.projects_machine() else {
            return;
        };
        let page = &self.areas.workspace.page;
        let Some(form) = page.adding else {
            return;
        };
        let path = page.path.text(cx).trim().to_owned();
        let url = page.url.text(cx).trim().to_owned();
        if page.busy || path.is_empty() {
            return;
        }
        let ask = match form {
            AddForm::Folder => ProjectAsk::Add(AddProject {
                path: path.into(),
                name: None,
            }),
            AddForm::New => ProjectAsk::Init(InitProject {
                path: path.into(),
                name: None,
            }),
            AddForm::Clone => {
                if repository_name(&url).is_none() {
                    return;
                }
                match url.parse() {
                    Ok(url) => ProjectAsk::Clone(CloneProject {
                        url,
                        parent: path.into(),
                        folder: None,
                        name: None,
                    }),
                    Err(refused) => {
                        self.areas.workspace.page.error = Some(format!("{refused}"));
                        cx.notify();
                        return;
                    }
                }
            }
        };
        self.ask(&machine, Ask::Project(ask));
        let page = &mut self.areas.workspace.page;
        page.busy = true;
        page.error = None;
        cx.notify();
    }

    /// The node answered the add form.
    pub(super) fn took_project(
        &mut self,
        machine: &str,
        _form: Form,
        result: Result<ProjectInfo, Problem>,
    ) {
        let words = self.words();
        let page = &mut self.areas.workspace.page;
        page.busy = false;
        match result {
            Ok(project) => {
                page.adding = None;
                if let Some(view) = self.areas.workspace.projects.get_mut(machine) {
                    view.took(project);
                }
            }
            Err(problem) => page.error = Some(words.problem(&problem)),
        }
    }

    pub(super) fn forget_project(&mut self, project: ProjectId, cx: &mut Context<'_, Self>) {
        if let Some(machine) = self.projects_machine() {
            self.ask(&machine, Ask::Project(ProjectAsk::Remove(project)));
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
pub(crate) fn render(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words();
    let labels = words.labels();
    let mut column = page(labels.projects_lead, tokens);
    let Some(machine) = shell.projects_machine() else {
        return scrolled(column.child(empty(labels.no_machines, tokens)));
    };
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
            .text_color(tokens.text_secondary)
            .cursor_pointer()
            .hover(|style| style.text_color(tokens.text_primary))
            .child(format!("\u{2039} {}", words.projects_on(&machine)))
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.areas.workspace.page.detail = None;
                cx.notify();
            }));
        return scrolled(
            column
                .child(back)
                .child(detail::render(shell, &machine, project, open, tokens, cx)),
        );
    }
    let mut action = div().flex().flex_row().items_center().gap(px(8.0));
    let online: Vec<String> = shell
        .fleet
        .online()
        .map(|machine| machine.name.clone())
        .collect();
    if online.len() > 1 {
        let mut picker = segmented(SegmentSize::Small);
        for (index, name) in online.into_iter().enumerate() {
            let chosen = name == machine;
            let choice = name.clone();
            picker = picker.child(
                segment(
                    ElementId::Name(format!("projects-machine-{index}").into()),
                    name,
                    chosen,
                    true,
                    SegmentSize::Small,
                    tokens,
                )
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    this.choose_projects_machine(choice.clone(), cx);
                })),
            );
        }
        action = action.child(picker);
    }
    if view.and_then(ProjectsView::list).is_some() {
        action = action.child(
            button("add-project", labels.add_project, false, tokens).on_click(cx.listener(
                |this, _event, _window, cx| {
                    let page = &mut this.areas.workspace.page;
                    page.menu = !page.menu;
                    cx.notify();
                },
            )),
        );
    }
    let body = list_body(shell, &machine, view, tokens, cx);
    column = column.child(section(
        words.projects_on(&machine),
        Some(action),
        body,
        tokens,
    ));
    scrolled(column)
}

/// The section's body: the add menu and form, the list, the banners.
fn list_body(
    shell: &Shell,
    machine: &str,
    view: Option<&ProjectsView>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words();
    let labels = words.labels();
    let state = &shell.areas.workspace.page;
    let mut body = div().flex().flex_col().gap(px(10.0));
    if state.menu {
        body = body.child(add_menu(shell, tokens, cx));
    }
    if let Some(form) = state.adding {
        body = body.child(add_form(shell, machine, form, tokens, cx));
    }
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
                list_rows = list_rows.child(project_row(shell, index, project, tokens, cx));
            }
            list_rows
        }
    });
    for project in view
        .and_then(ProjectsView::list)
        .map_or(&[][..], |list| list.projects.as_slice())
    {
        if let ProjectState::CloneFailed { reason, message } = &project.state {
            body = body.child(banner(
                format!(
                    "{}: {} {message}",
                    project.name,
                    words.clone_failure(*reason)
                ),
                true,
                None,
                tokens,
            ));
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

/// The page in a scrolled frame.
fn scrolled(column: Div) -> Div {
    div().size_full().child(
        div()
            .id("projects-page")
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .items_center()
            .child(column),
    )
}

fn project_row(
    shell: &Shell,
    index: usize,
    project: &ProjectInfo,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let words = shell.words();
    let opened = project.clone();
    let name = project.name.to_string();
    // Paseo's project row: the project's mark (its initial on a tile, 16),
    // its name, its state as a badge, a chevron; the whole row opens the
    // project's page.
    let initial = name
        .chars()
        .next()
        .map_or_else(|| "?".to_owned(), |first| first.to_uppercase().collect());
    let title_line = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .size(px(16.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .bg(tokens.bg_active)
                .type_(Type::Tiny)
                .text_color(tokens.text_secondary)
                .child(initial),
        )
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
            .text_color(tokens.text_tertiary)
            .child("\u{203a}"),
    );
    let hover = tokens.bg_hover;
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

/// The add menu's three ways, in place under the heading.
fn add_menu(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let labels = shell.words().labels();
    let mut menu = div().flex().flex_row().flex_wrap().gap(px(6.0));
    for (form, label) in [
        (AddForm::Folder, labels.add_folder),
        (AddForm::Clone, labels.clone_repository),
        (AddForm::New, labels.start_new),
    ] {
        menu = menu.child(
            vessel_app_kit::choice(
                ElementId::Name(format!("add-{form:?}").into()),
                label,
                false,
                tokens,
            )
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.open_add(form, window, cx);
            })),
        );
    }
    menu
}

/// The add form, in place: a clone's URL, the folder, the help line, the
/// node's refusal, Cancel and the form's verb.
fn add_form(
    shell: &Shell,
    machine: &str,
    form: AddForm,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words();
    let labels = words.labels();
    let state = &shell.areas.workspace.page;
    let (heading, verb, folder_label) = match form {
        AddForm::Folder => (labels.title_add_folder, labels.add, labels.folder),
        AddForm::Clone => (labels.title_clone, labels.clone, labels.clone_into),
        AddForm::New => (labels.title_new, labels.create, labels.folder),
    };
    let url = state.url.text(cx);
    let path = state.path.text(cx);
    let folder = path.trim().trim_end_matches(['/', '\\']);
    let name = repository_name(&url);
    let target = name
        .filter(|_| !folder.is_empty())
        .map(|name| (folder, name));
    let ready = !state.busy && !folder.is_empty() && (form != AddForm::Clone || name.is_some());
    let mut card = div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .p(px(14.0))
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .border_1()
        .border_color(tokens.border_subtle)
        .bg(tokens.bg_elevated)
        .child(div().type_(Type::Title).child(heading));
    if form == AddForm::Clone {
        card = card.child(labelled(labels.repository_url, state.url.element(), tokens));
    }
    card = card
        .child(labelled(folder_label, state.path.element(), tokens))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.text_tertiary)
                .child(words.add_hint(form, machine, target)),
        );
    if let Some(error) = &state.error {
        card = card.child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.danger)
                .child(error.clone()),
        );
    }
    card.child(
        div()
            .flex()
            .flex_row()
            .justify_end()
            .gap(px(8.0))
            .child(
                button("add-cancel", labels.cancel, false, tokens).on_click(cx.listener(
                    |this, _event, _window, cx| {
                        this.areas.workspace.page.adding = None;
                        cx.notify();
                    },
                )),
            )
            .child(
                button("add-submit", verb, true, tokens)
                    .when(!ready, |button| button.opacity(0.5))
                    .on_click(cx.listener(|this, _event, _window, cx| this.submit_add(cx))),
            ),
    )
}

/// A field with its label above it.
fn labelled(
    label: &'static str,
    field: impl vessel_app_kit::ui::IntoElement,
    tokens: &Tokens,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.text_secondary)
                .child(label),
        )
        .child(field)
}

use vessel_app_kit::ui::FluentBuilder as _;
