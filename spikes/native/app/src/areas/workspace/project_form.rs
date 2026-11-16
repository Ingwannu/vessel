//! Project registration form interactions and rendering, separate from the host list.
//! Node registration creates neither a workspace nor an agent; existing link adapters own IO.

use crate::shell::Shell;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    Context, Div, ElementId, ParentElement as _, StatefulInteractiveElement as _, Styled as _,
    Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button, button_with_kind,
};
use vessel_app_link::workspace::{Form, ProjectChange};
use vessel_app_view::machine_book::Name;
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::projects::actions::ProjectAction;
use vessel_app_view::workspace::projects::navigation::{NavigationRefusal, ProjectTarget};
use vessel_app_view::workspace::projects::repository_name;
use vessel_app_view::workspace::words::AddForm;
use vessel_proto::workspace::{AddProject, CloneProject, InitProject, ProjectInfo};

impl Shell {
    /// Onboarding retains its shared physical admission across asynchronous visits.
    ///
    /// # Errors
    /// A retired admission cannot open a draft on its same-node replacement.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "onboarding consumes this producer after source review"
        )
    )]
    pub(crate) fn open_add_for_connection(
        &mut self,
        form: AddForm,
        context: &vessel_app_view::connection::ConnectionContext,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), NavigationRefusal> {
        if self.project_connection(context.machine().as_str()).as_ref() != Some(context) {
            return Err(NavigationRefusal::HostUnavailable);
        }
        let host = Name::parse(context.machine().as_str())
            .map_err(|_invalid| NavigationRefusal::HostUnavailable)?;
        self.open_add_for_target(form, &ProjectTarget::new(host, context.node()), window, cx)
    }

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
        let host = self
            .projects_machine()
            .and_then(|host| Name::parse(&host).ok());
        let node = host
            .as_ref()
            .and_then(|name| self.project_node(name.as_str()));
        if let (Some(host), Some(node)) = (host.as_ref(), node) {
            let target = ProjectTarget::new(host.clone(), node);
            if self.open_add_for_target(form, &target, window, cx).is_ok() {
                return;
            }
        }
        self.prepare_add_form(form, host, node, window, cx);
    }

    /// Open the existing form on an explicit peer, independently of Settings' selection.
    /// A display target alone does not prove managed-local OS-path provenance.
    ///
    /// # Errors
    /// A missing, offline or rebound peer refuses without opening/retargeting a draft.
    pub(crate) fn open_add_for_target(
        &mut self,
        form: AddForm,
        target: &ProjectTarget,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), NavigationRefusal> {
        if self
            .project_connection(target.host().as_str())
            .is_none_or(|context| context.node() != target.node())
        {
            return Err(NavigationRefusal::HostUnavailable);
        }
        self.prepare_add_form(
            form,
            Some(target.host().clone()),
            Some(target.node()),
            window,
            cx,
        );
        Ok(())
    }

    fn prepare_add_form(
        &mut self,
        form: AddForm,
        host: Option<Name>,
        node: Option<vessel_proto::NodeId>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let context = host
            .as_ref()
            .and_then(|host| self.project_connection(host.as_str()));
        let labels = self.words().labels();
        let page = &mut self.areas.workspace.page;
        page.actions.cancel();
        page.form_host = host;
        page.form_node = node;
        page.form_context = context;
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
        let Some(host) = self.areas.workspace.page.form_host.clone() else {
            return;
        };
        let Some(node) = self
            .areas
            .workspace
            .page
            .form_node
            .filter(|node| self.project_node(host.as_str()) == Some(*node))
        else {
            self.areas.workspace.page.error =
                Some(self.words.projects().host_unavailable().to_owned());
            cx.notify();
            return;
        };
        let Some(context) = self
            .areas
            .workspace
            .page
            .form_context
            .clone()
            .filter(|context| self.project_connection(host.as_str()).as_ref() == Some(context))
        else {
            self.areas.workspace.page.error =
                Some(self.words.projects().host_unavailable().to_owned());
            cx.notify();
            return;
        };
        let page = &self.areas.workspace.page;
        let Some(form) = page.adding else {
            return;
        };
        let path = page.path.text(cx).trim().to_owned();
        let url = page.url.text(cx).trim().to_owned();
        if page.actions.pending().is_some() || path.is_empty() {
            return;
        }
        let ask = match form {
            AddForm::Folder => ProjectAction::Register(AddProject {
                path: path.into(),
                name: None,
            }),
            AddForm::New => ProjectAction::Initialize(InitProject {
                path: path.into(),
                name: None,
            }),
            AddForm::Clone => {
                if repository_name(&url).is_none() {
                    return;
                }
                match url.parse() {
                    Ok(url) => ProjectAction::Clone(CloneProject {
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
        let key = match self.areas.workspace.page.actions.begin(host.clone(), node) {
            Ok(key) => key,
            Err(error) => {
                self.project_action_refused(error);
                cx.notify();
                return;
            }
        };
        self.command(vessel_app_link::Command::Projects(
            vessel_app_link::workspace::project_requests::Request {
                context,
                operation: vessel_app_link::workspace::project_requests::Operation::Change(
                    ProjectChange { key, action: ask },
                ),
            },
        ));
        let page = &mut self.areas.workspace.page;
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

        match result {
            Ok(project) => {
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
    let state = &shell.areas.workspace.page;
    if let Some((host, node)) = state.form_host.as_ref().zip(state.form_node) {
        let target = ProjectTarget::new(host.clone(), node);
        if let Some(form) = for_target(shell, &target, tokens, compact, cx) {
            return form;
        }
    }
    draw_form(shell, machine, form, tokens, compact, cx)
}

/// Reuse the form renderer for a fixed-target onboarding visit, without rendering
/// Settings' project list or inferring a selected host from it.
#[must_use]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "onboarding consumes this producer after source review"
    )
)]
pub(crate) fn for_connection(
    shell: &Shell,
    context: &vessel_app_view::connection::ConnectionContext,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    if shell
        .project_connection(context.machine().as_str())
        .as_ref()
        != Some(context)
        || shell.areas.workspace.page.form_context.as_ref() != Some(context)
    {
        return None;
    }
    let host = Name::parse(context.machine().as_str()).ok()?;
    for_target(
        shell,
        &ProjectTarget::new(host, context.node()),
        tokens,
        compact,
        cx,
    )
}

/// Legacy local renderer; cross-area callers retain a full connection context.
#[must_use]
pub(crate) fn for_target(
    shell: &Shell,
    target: &ProjectTarget,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    let state = &shell.areas.workspace.page;
    if state.form_host.as_ref() != Some(target.host()) || state.form_node != Some(target.node()) {
        return None;
    }
    let form = state.adding?;
    Some(draw_form(
        shell,
        target.host().as_str(),
        form,
        tokens,
        compact,
        cx,
    ))
}

fn form_current(shell: &Shell) -> bool {
    let state = &shell.areas.workspace.page;
    state
        .form_host
        .as_ref()
        .zip(state.form_node)
        .is_some_and(|(host, node)| {
            shell.project_node(host.as_str()) == Some(node)
                && state.form_context.is_some()
                && state.form_context == shell.project_connection(host.as_str())
        })
}

fn draw_form(
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
    let bound = form_current(shell);
    let ready = bound
        && state.actions.pending().is_none()
        && !folder.is_empty()
        && (form != AddForm::Clone || name.is_some());
    let mut card = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
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
                .child(words.add_hint(
                    form,
                    state.form_host.as_ref().map_or(machine, Name::as_str),
                    target,
                )),
        );
    if let Some(error) = &state.error {
        card = card.child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.destructive)
                .child(error.clone()),
        );
    }
    if !bound {
        card = card.child(
            div()
                .type_(Type::Meta)
                .child(shell.words.projects().host_unavailable().to_owned()),
        );
    }
    let mut submit = button_with_kind(
        "add-submit",
        verb,
        ButtonKind::Primary,
        if ready {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        tokens,
    );
    if ready {
        submit = submit.on_click(cx.listener(|this, _event, _window, cx| this.submit_add(cx)));
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
                        let page = &mut this.areas.workspace.page;
                        page.adding = None;
                        page.form_host = None;
                        page.form_node = None;
                        page.form_context = None;
                        page.actions.cancel();
                        cx.notify();
                    },
                )),
            )
            .child(submit),
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
