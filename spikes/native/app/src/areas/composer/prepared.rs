//! Creation owns navigation and its ticket; the attachment area owns bytes.
//!
//! Preparing changes no file identity or upload reference. A failed or stale
//! creation cannot take the next draft's tray, and a retry can keep the same
//! source files until a real session exists for the node's upload contract.

use super::attach::Starting;
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_view::{Card, composer::drafts::DraftKey, creation::Ticket};

/// Proof that this ticket's first input stays in the attachment pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PreparedSubmission {
    ticket: Ticket,
}

impl PreparedSubmission {
    pub(crate) fn ticket(self) -> Ticket {
        self.ticket
    }
}

pub(crate) use vessel_app_view::composer::attach::PrepareProblem;

impl Shell {
    /// Creation retains one canonical identity before preparing its first payload.
    pub(crate) fn prepare_draft_input(
        &mut self,
        ticket: Ticket,
        message: vessel_proto::timeline::ClientMessageId,
        expected_node: vessel_proto::NodeId,
        machine: &str,
        text: &str,
    ) -> Result<Option<PreparedSubmission>, PrepareProblem> {
        if self.composer_attachments_blocked(DraftKey::New) {
            return Err(PrepareProblem::Busy);
        }
        let _name =
            vessel_app_view::machine_book::Name::parse(machine).map_err(PrepareProblem::Machine)?;
        if !self.draft_has_files() {
            return Ok(None);
        }
        let connection = self
            .fleet
            .machines
            .iter()
            .find(|entry| entry.name == machine && entry.node == Some(expected_node))
            .and_then(vessel_app_view::Machine::connection_context)
            .ok_or(vessel_app_view::composer::attach::TargetProblem::Offline)?;
        self.prepare_draft_input_on(ticket, message, connection, text)
    }

    /// A1 may pass the exact context it captured before selecting/creating a child.
    pub(crate) fn prepare_draft_input_on(
        &mut self,
        ticket: Ticket,
        message: vessel_proto::timeline::ClientMessageId,
        connection: vessel_app_view::connection::ConnectionContext,
        text: &str,
    ) -> Result<Option<PreparedSubmission>, PrepareProblem> {
        let machine = connection.machine().as_str();
        let expected_node = connection.node();
        if self.composer_attachments_blocked(DraftKey::New) {
            return Err(PrepareProblem::Busy);
        }
        let machine =
            vessel_app_view::machine_book::Name::parse(machine).map_err(PrepareProblem::Machine)?;
        if !self.draft_has_files() {
            return Ok(None);
        }
        if self
            .areas
            .composer
            .sends
            .len()
            .saturating_add(self.areas.composer.workspace_starting.len())
            >= vessel_app_view::composer::trays::Trays::MAX
        {
            return Err(PrepareProblem::Capacity);
        }
        if !self.creation_has_attachment_room(DraftKey::New) {
            return Err(PrepareProblem::Capacity);
        }
        let mut held = self.areas.composer.starting.borrow_mut();
        if held.is_some() {
            return Err(PrepareProblem::Busy);
        }
        let context = self
            .areas
            .composer
            .contexts
            .get_mut(DraftKey::New)
            .map(vessel_app_view::composer::context::ContextTray::prepare)
            .transpose()?
            .flatten();
        *held = Some(Starting {
            connection,
            message,
            context,
            machine,
            expected_node,
            text: text.to_owned(),
            ticket: Some(ticket),
            files: self
                .areas
                .composer
                .trays
                .get(DraftKey::New)
                .cloned()
                .unwrap_or_default(),
        });
        Ok(Some(PreparedSubmission { ticket }))
    }

    pub(crate) fn started_draft_files(
        &mut self,
        ticket: Ticket,
        card: &Card,
        cx: &mut Context<'_, Self>,
    ) {
        let matches = self
            .areas
            .composer
            .starting
            .borrow()
            .as_ref()
            .is_some_and(|held| {
                held.ticket == Some(ticket) && held.machine.as_str() == card.machine
            });
        if matches {
            let retained = self.areas.composer.starting.borrow().as_ref().map(|held| {
                (
                    held.machine.clone(),
                    held.expected_node,
                    held.connection.clone(),
                )
            });
            let Some((machine, expected_node, connection)) = retained else {
                return;
            };
            self.composer_started(card, cx);
            if let Err(why) = self.read_creation_capabilities(
                ticket,
                machine,
                expected_node,
                connection,
                card.session,
            ) {
                self.fail_creation_preparation(card.session, &why, cx);
            }
        }
    }

    pub(crate) fn rejected_draft_files(&mut self, ticket: Ticket, cx: &mut Context<'_, Self>) {
        let mut held = self.areas.composer.starting.borrow_mut();
        if held
            .as_ref()
            .is_some_and(|held| held.ticket == Some(ticket))
        {
            if let Some(starting) = held.take()
                && let Some(context) = &starting.context
                && let Some(tray) = self.areas.composer.contexts.get_mut(DraftKey::New)
            {
                let _settled = tray.settle(
                    context,
                    vessel_app_view::composer::context::SubmissionOutcome::Failed,
                );
            }
            cx.notify();
        }
    }
}

#[cfg(test)]
pub(super) mod tests;

mod workspace;
#[cfg_attr(
    not(test),
    expect(
        unused_imports,
        reason = "W3 keyed creation request consumer is staged next"
    )
)]
pub(crate) use workspace::WorkspaceInput;
