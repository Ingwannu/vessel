//! Project action correlation and explicit destructive confirmation.
//! Late replies reconcile the host cache without settling a newer draft.

use crate::shell::Shell;
use std::path::PathBuf;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    Context, Div, InteractiveElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button_with_kind};
use vessel_app_link::workspace::{Ask, ProjectAsk, ProjectChange, ProjectFailure};
use vessel_app_view::machine_book::Name;
use vessel_app_view::workspace::projects::actions::{
    ProjectAction, ProjectActionError, ProjectActionKey, ProjectChanged, ProjectReply,
};
use vessel_proto::{ProjectId, workspace::ProjectName};

/// The confirmation retains exact host/id even if Settings selection changes behind it.
pub(super) struct Removing {
    host: Name,
    project: ProjectId,
    name: ProjectName,
    path: PathBuf,
    pending: Option<ProjectActionKey>,
    error: Option<ProjectFailure>,
}

impl Shell {
    pub(super) fn project_changed(
        &mut self,
        machine: &str,
        key: ProjectActionKey,
        result: Result<ProjectChanged, ProjectFailure>,
    ) {
        if machine != key.host().as_str() {
            return;
        }
        let current = self.areas.workspace.page.actions.settle(&key) == ProjectReply::Current;
        match result {
            Ok(ProjectChanged::Registered(project)) => {
                if let Some(view) = self.areas.workspace.projects.get_mut(machine) {
                    view.took(project);
                }
                if current {
                    self.areas.workspace.page.adding = None;
                    self.areas.workspace.page.form_host = None;
                }
            }
            Ok(ProjectChanged::Removed(project)) => {
                self.forgot_project(machine, project, Ok(()));
                if self
                    .areas
                    .workspace
                    .page
                    .removing
                    .as_ref()
                    .is_some_and(|remove| remove.pending.as_ref() == Some(&key))
                {
                    self.areas.workspace.page.removing = None;
                }
            }
            Err(failure) => {
                let said = project_failure(self, &failure);
                if current {
                    self.areas.workspace.page.error = Some(said);
                }
                if let Some(remove) = self.areas.workspace.page.removing.as_mut()
                    && remove.pending.as_ref() == Some(&key)
                {
                    remove.pending = None;
                    remove.error = Some(failure);
                }
            }
        }
        self.ask(machine, Ask::Project(ProjectAsk::List));
    }

    /// Only the confirmation performs a destructive call; opening it is read-only.
    pub(super) fn confirm_project_removal(
        &mut self,
        project: ProjectId,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(machine) = self.projects_machine() else {
            return;
        };
        let Ok(host) = Name::parse(&machine) else {
            return;
        };
        let Some(info) = self
            .areas
            .workspace
            .projects
            .get(&machine)
            .and_then(|view| view.project(&project))
        else {
            return;
        };
        self.areas.workspace.page.removing = Some(Removing {
            host,
            project,
            name: info.name.clone(),
            path: info.root.clone(),
            pending: None,
            error: None,
        });
        cx.notify();
    }

    fn remove_confirmed(&mut self, cx: &mut Context<'_, Self>) {
        let Some(remove) = self.areas.workspace.page.removing.as_ref() else {
            return;
        };
        let host = remove.host.clone();
        let project = remove.project;
        let Ok(key) = self.areas.workspace.page.actions.begin(host.clone()) else {
            return;
        };
        if let Some(remove) = self.areas.workspace.page.removing.as_mut() {
            remove.pending = Some(key.clone());
            remove.error = None;
        }
        self.ask(
            host.as_str(),
            Ask::ProjectChange(ProjectChange {
                key,
                action: ProjectAction::Remove(project),
            }),
        );
        cx.notify();
    }

    pub(super) fn project_action_refused(&mut self, error: ProjectActionError) {
        match error {
            ProjectActionError::Busy => {}
            ProjectActionError::Exhausted => {
                self.areas.workspace.page.error =
                    Some(self.words.projects().identifiers_exhausted().to_owned())
            }
        }
    }
}

pub(super) fn project_failure(shell: &Shell, failure: &ProjectFailure) -> String {
    match failure {
        ProjectFailure::Call(problem) => shell.words().problem(problem),
        ProjectFailure::TimedOut => shell.words.projects().outcome_unknown().to_owned(),
    }
}

pub(super) fn removal(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Option<Div> {
    let remove = shell.areas.workspace.page.removing.as_ref()?;
    let words = shell.words.projects();
    let busy = remove.pending.is_some();
    let mut confirm = button_with_kind(
        "project-remove-confirm",
        words.remove().to_owned(),
        ButtonKind::Destructive,
        if busy {
            ButtonState::Disabled
        } else {
            ButtonState::Enabled
        },
        tokens,
    );
    if !busy {
        confirm =
            confirm.on_click(cx.listener(|this, _event, _window, cx| this.remove_confirmed(cx)));
    }
    let mut card = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .p(px(spacing::S4))
        .border_1()
        .border_color(tokens.neutral.border)
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .child(
            div()
                .type_(Type::Title)
                .child(words.remove_title().to_owned()),
        )
        .child(
            div()
                .type_(Type::Body)
                .child(words.remove_description().to_owned()),
        )
        .child(div().type_(Type::Body).child(remove.name.to_string()))
        .child(
            div()
                .type_(Type::Meta)
                .child(remove.host.as_str().to_owned()),
        )
        .child(
            div()
                .type_(Type::Meta)
                .child(remove.path.display().to_string()),
        );
    if let Some(error) = &remove.error {
        card = card.child(super::widgets::banner(
            project_failure(shell, error),
            true,
            None,
            tokens,
        ));
    }
    Some(
        card.child(
            div()
                .flex()
                .flex_wrap()
                .justify_end()
                .gap(px(spacing::S2))
                .child(
                    button_with_kind(
                        "project-remove-cancel",
                        words.cancel().to_owned(),
                        ButtonKind::Secondary,
                        ButtonState::Enabled,
                        tokens,
                    )
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.areas.workspace.page.removing = None;
                        this.areas.workspace.page.actions.cancel();
                        cx.notify();
                    })),
                )
                .child(confirm),
        ),
    )
}
