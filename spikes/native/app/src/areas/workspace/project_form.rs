//! Project registration form interactions and rendering, separate from the host list.
//! Node registration creates neither a workspace nor an agent; existing link adapters own IO.

use crate::shell::Shell;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, button};
use vessel_app_link::workspace::{Ask, Form, ProjectAsk};
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::projects::repository_name;
use vessel_app_view::workspace::words::AddForm;
use vessel_proto::workspace::{AddProject, CloneProject, InitProject, ProjectInfo};

impl Shell {
    /// Translate labels retained by an open form without changing its values.
    pub(crate) fn refresh_project_words(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let page = &self.areas.workspace.page;
        let labels = self.words().labels();
        let placeholder = match page.adding {
            Some(AddForm::Clone) => labels.parent_placeholder,
            Some(AddForm::Folder | AddForm::New) | None => labels.folder_placeholder,
        };
        page.path.set_placeholder(placeholder, window, cx);
        page.url.set_placeholder(labels.url_placeholder, window, cx);
    }

    pub(super) fn open_add(
        &mut self,
        form: AddForm,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
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
}

/// The add menu's three ways, in place under the heading.
pub(super) fn add_menu(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let labels = shell.words().labels();
    let mut menu = div().flex().flex_row().flex_wrap().gap(px(spacing::S1_5));
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
pub(super) fn add_form(
    shell: &Shell,
    machine: &str,
    form: AddForm,
    tokens: &Tokens,
    compact: bool,
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
        .gap(px(spacing::S1_5 + spacing::S1))
        .p(px(spacing::S3_5))
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .border_1()
        .border_color(tokens.neutral.border)
        .bg(tokens.neutral.surface1)
        .child(div().type_(Type::Title).child(heading));
    if form == AddForm::Clone {
        let url_field = state.url.element(tokens, compact, cx);
        card = card.child(labelled(labels.repository_url, url_field, tokens));
    }
    let path_field = state.path.element(tokens, compact, cx);
    card = card
        .child(labelled(folder_label, path_field, tokens))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.add_hint(form, machine, target)),
        );
    if let Some(error) = &state.error {
        card = card.child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.destructive)
                .child(error.clone()),
        );
    }
    card.child(
        div()
            .flex()
            .flex_row()
            .justify_end()
            .gap(px(spacing::S2))
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
        .gap(px(spacing::S1))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(label),
        )
        .child(field)
}
