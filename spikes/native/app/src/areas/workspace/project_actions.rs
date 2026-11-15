//! Project action correlation and explicit destructive confirmation.
//! Late replies trigger a fresh authoritative read without mutating a newer draft.

use crate::shell::Shell;
use std::path::PathBuf;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    Context, Div, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button_with_kind};
use vessel_app_link::workspace::{ProjectChange, ProjectFailure};
use vessel_app_view::machine_book::Name;
use vessel_app_view::workspace::projects::actions::{
    ProjectAction, ProjectActionError, ProjectActionKey, ProjectChanged, ProjectReply,
    ProjectSettlement,
};
use vessel_proto::{NodeId, ProjectId, workspace::ProjectName};

/// The confirmation retains exact host/id even if Settings selection changes behind it.
pub(super) struct Removing {
    host: Name,
    project: ProjectId,
    node: NodeId,
    name: ProjectName,
    path: PathBuf,
    context: vessel_app_view::connection::ConnectionContext,
    pending: Option<ProjectActionKey>,
    error: Option<ProjectFailure>,
}

impl Shell {
    pub(super) fn project_changed(
        &mut self,
        machine: &str,
        key: &ProjectActionKey,
        result: Result<ProjectChanged, ProjectFailure>,
    ) {
        let settlement = match &result {
            Ok(_) => ProjectSettlement::Succeeded,
            Err(_) => ProjectSettlement::Failed,
        };
        if !self.accept_project_reply(machine, key, settlement) {
            return;
        }
        match result {
            Ok(ProjectChanged::Registered(project)) => {
                if let Some(view) = self.areas.workspace.projects.get_mut(machine) {
                    view.took(project);
                }
                self.areas.workspace.page.adding = None;
                self.areas.workspace.page.form_host = None;
                self.areas.workspace.page.form_node = None;
                self.areas.workspace.page.form_context = None;
            }
            Ok(ProjectChanged::Removed(project)) => {
                self.forgot_project(machine, project, Ok(()));
                if self
                    .areas
                    .workspace
                    .page
                    .removing
                    .as_ref()
                    .is_some_and(|remove| remove.pending.as_ref() == Some(key))
                {
                    self.areas.workspace.page.removing = None;
                }
            }
            Err(failure) => {
                let said = project_failure(self, &failure);
                self.areas.workspace.page.error = Some(said);
                if let Some(remove) = self.areas.workspace.page.removing.as_mut()
                    && remove.pending.as_ref() == Some(key)
                {
                    remove.pending = None;
                    remove.error = Some(failure);
                }
            }
        }
        self.read_projects(machine);
    }

    /// Settlement never grants a rebound alias authority to hydrate an old node's payload.
    fn accept_project_reply(
        &mut self,
        machine: &str,
        key: &ProjectActionKey,
        settlement: ProjectSettlement,
    ) -> bool {
        if machine != key.host().as_str() {
            return false;
        }
        let current =
            self.areas.workspace.page.actions.finish(key, settlement) == ProjectReply::Current;
        let changed_node = self
            .fleet
            .machines
            .iter()
            .find(|host| host.name == machine)
            .and_then(|host| host.node)
            .is_some_and(|node| node != key.node());
        if changed_node {
            if current {
                self.project_host_detached(machine);
            }
            return false;
        }
        if !current {
            // A book alias may now name a different node: never hydrate from an old reply.
            if self.areas.workspace.projects.contains_key(machine) {
                self.read_projects(machine);
            }
            return false;
        }
        true
    }

    /// Capture the shared actor admission from the same public machine snapshot.
    pub(super) fn project_connection(
        &self,
        host: &str,
    ) -> Option<vessel_app_view::connection::ConnectionContext> {
        self.fleet
            .online()
            .find(|machine| machine.name == host)?
            .connection_context()
    }

    /// Only an authenticated online welcome can provide a new mutation's node identity.
    pub(super) fn project_node(&self, host: &str) -> Option<NodeId> {
        self.fleet
            .online()
            .find(|machine| machine.name == host)
            .and_then(|machine| machine.node)
    }

    /// A forgotten/renamed alias cannot retain mutation authority for its former target.
    pub(super) fn project_host_detached(&mut self, host: &str) {
        let page = &mut self.areas.workspace.page;
        if page
            .actions
            .pending()
            .is_some_and(|key| key.host().as_str() == host)
        {
            page.actions.cancel();
        }
        if page
            .form_host
            .as_ref()
            .is_some_and(|name| name.as_str() == host)
        {
            page.form_host = None;
            page.form_node = None;
            page.form_context = None;
            page.adding = None;
        }
        if page
            .removing
            .as_ref()
            .is_some_and(|remove| remove.host.as_str() == host)
        {
            page.removing = None;
        }
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
        let Some(node) = self.project_node(&machine) else {
            self.areas.workspace.page.error =
                Some(self.words.projects().host_unavailable().to_owned());
            cx.notify();
            return;
        };
        let Some(context) = self.project_connection(host.as_str()) else {
            return;
        };
        if !self.project_cache_current(&host)
            || self.areas.workspace.project_reads.node(&host) != Some(node)
            || self.areas.workspace.project_reads.pending(&host, node)
        {
            self.areas.workspace.page.error = Some(self.words().problem(&Problem::Unsent));
            cx.notify();
            return;
        }
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
            node,
            context,
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
        let node = remove.node;
        let context = remove.context.clone();
        if self.project_connection(host.as_str()).as_ref() != Some(&context) {
            self.project_host_detached(host.as_str());
            cx.notify();
            return;
        }
        let Ok(key) = self.areas.workspace.page.actions.begin(host.clone(), node) else {
            return;
        };
        if let Some(remove) = self.areas.workspace.page.removing.as_mut() {
            remove.pending = Some(key.clone());
            remove.error = None;
        }
        self.command(vessel_app_link::Command::Projects(
            vessel_app_link::workspace::project_requests::Request {
                context,
                operation: vessel_app_link::workspace::project_requests::Operation::Change(
                    ProjectChange {
                        key,
                        action: ProjectAction::Remove(project),
                    },
                ),
            },
        ));
        cx.notify();
    }

    pub(super) fn project_action_refused(&mut self, error: ProjectActionError) {
        match error {
            ProjectActionError::Busy => {}
            ProjectActionError::ReconciliationFull => {
                self.areas.workspace.page.error =
                    Some(self.words.projects().reconciliation_full().to_owned());
            }
            ProjectActionError::Exhausted => {
                self.areas.workspace.page.error =
                    Some(self.words.projects().identifiers_exhausted().to_owned());
            }
        }
    }
}

pub(super) fn project_failure(shell: &Shell, failure: &ProjectFailure) -> String {
    match failure {
        ProjectFailure::Call(problem) => shell.words().problem(problem),
        ProjectFailure::HostChanged => shell.words.projects().host_unavailable().to_owned(),
        ProjectFailure::TimedOut | ProjectFailure::OutcomeUnknown => {
            shell.words.projects().outcome_unknown().to_owned()
        }
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
