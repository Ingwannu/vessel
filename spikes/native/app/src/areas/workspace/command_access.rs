//! Settings confirmations use the producing read's immutable connection/digest.
//! The model is area-lived; closing a detail cannot reuse an old receipt number.
use crate::routes::Route;
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::workspace::{
    Ask,
    command_access::{Answer, Ask as Request, Operation},
};
use vessel_app_view::settings::Section;
use vessel_app_view::workspace::command_access::{Target, Ticket};
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::ProjectId;

impl Shell {
    pub(super) fn command_owner_is_current(&self, target: &Target) -> bool {
        self.areas.workspace.command_access.target.as_ref() == Some(target)
            && self
                .fleet
                .machines
                .iter()
                .find(|machine| machine.name == target.context.machine().as_str())
                .and_then(vessel_app_view::Machine::connection_context)
                .as_ref()
                == Some(&target.context)
            && self
                .areas
                .workspace
                .page
                .detail
                .as_ref()
                .is_some_and(|detail| detail.project == target.project)
    }
    pub(super) fn command_ticket_is_current(&self, ticket: &Ticket) -> bool {
        self.command_owner_is_current(&ticket.target)
            && self.route == Route::Settings
            && self.areas.settings.pushed
            && self.areas.settings.section == Section::Projects
            && self.projects_machine().as_deref() == Some(ticket.target.context.machine().as_str())
            && self.areas.workspace.command_access.ticket().as_ref() == Some(ticket)
    }
    /// This is an explicit read, so it can capture a newly admitted connection.
    pub(super) fn read_project_commands(
        &mut self,
        machine: &str,
        project: ProjectId,
        reopening: bool,
    ) {
        let context = self
            .fleet
            .machines
            .iter()
            .find(|entry| entry.name == machine)
            .and_then(vessel_app_view::Machine::connection_context);
        let Some(context) = context else {
            self.areas.workspace.command_access.select(None);
            self.areas.workspace.command_access.failure = Some(Problem::Gone(String::new()));
            return;
        };
        let target = Target { context, project };
        let model = &mut self.areas.workspace.command_access;
        if reopening {
            model.select(Some(target.clone()));
        }
        let Some(key) = model.begin_read(target.clone()) else {
            return;
        };
        self.ask(
            target.context.machine().as_str(),
            Ask::CommandAccess(Request {
                target: target.clone(),
                key,
                operation: Operation::Read,
            }),
        );
    }
    pub(super) fn command_access_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        if !self.command_owner_is_current(&answer.ask.target) {
            return;
        }
        let Request {
            target,
            key,
            operation,
        } = answer.ask;
        let mutation = matches!(operation, Operation::Approve { .. });
        let model = &mut self.areas.workspace.command_access;
        let accepted = match operation {
            Operation::Read => model.accept_read(&target, key, answer.result),
            Operation::Approve { digest, approved } => {
                model.accept_write(&target, key, &digest, approved, answer.result)
            }
        };
        if accepted {
            if mutation
                && let Some(detail) = self
                    .areas
                    .workspace
                    .page
                    .detail
                    .as_mut()
                    .filter(|detail| detail.project == target.project)
            {
                detail.reviewing = false;
            }
            cx.notify();
        }
    }
    pub(super) fn approve_project_commands(
        &mut self,
        ticket: &Ticket,
        approved: bool,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !window.is_window_active() || !self.command_ticket_is_current(ticket) {
            return;
        }
        let Some(key) = self
            .areas
            .workspace
            .command_access
            .begin_write(ticket, approved)
        else {
            cx.notify();
            return;
        };
        self.ask(
            ticket.target.context.machine().as_str(),
            Ask::CommandAccess(Request {
                target: ticket.target.clone(),
                key,
                operation: Operation::Approve {
                    digest: ticket.digest.clone(),
                    approved,
                },
            }),
        );
        cx.notify();
    }
    pub(super) fn review_project_commands(
        &mut self,
        ticket: &Ticket,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !window.is_window_active() || !self.command_ticket_is_current(ticket) {
            return;
        }
        if let Some(detail) = self.areas.workspace.page.detail.as_mut() {
            detail.reviewing = true;
        }
        self.read_project_commands(
            ticket.target.context.machine().as_str(),
            ticket.target.project,
            false,
        );
        cx.notify();
    }
    pub(super) fn refresh_command_card(
        &mut self,
        machine: &str,
        project: ProjectId,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !window.is_window_active()
            || self.route != Route::Settings
            || self.areas.settings.section != Section::Projects
            || self.projects_machine().as_deref() != Some(machine)
            || self
                .areas
                .workspace
                .page
                .detail
                .as_ref()
                .is_none_or(|detail| detail.project != project)
        {
            return;
        }
        self.read_project_commands(machine, project, false);
        cx.notify();
    }
    pub(super) fn cancel_command_review(&mut self, target: &Target, cx: &mut Context<'_, Self>) {
        if self.command_owner_is_current(target)
            && let Some(detail) = self.areas.workspace.page.detail.as_mut()
        {
            detail.reviewing = false;
            cx.notify();
        }
    }
}
