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
    /// The creation flow supplies the node it admitted before any asynchronous work.
    pub(crate) fn prepare_draft_files_on(
        &mut self,
        ticket: Ticket,
        expected_node: vessel_proto::NodeId,
        machine: &str,
        text: &str,
    ) -> Result<Option<PreparedSubmission>, PrepareProblem> {
        let machine =
            vessel_app_view::machine_book::Name::parse(machine).map_err(PrepareProblem::Machine)?;
        if !self.composer_has_files(DraftKey::New) {
            return Ok(None);
        }
        if self.areas.composer.sends.len() >= vessel_app_view::composer::trays::Trays::MAX {
            return Err(PrepareProblem::Capacity);
        }
        let mut held = self.areas.composer.starting.borrow_mut();
        if held.is_some() {
            return Err(PrepareProblem::Busy);
        }
        *held = Some(Starting {
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
            let retained = self
                .areas
                .composer
                .starting
                .borrow()
                .as_ref()
                .map(|held| (held.machine.clone(), held.expected_node));
            let Some((machine, expected_node)) = retained else {
                return;
            };
            self.composer_started(card, cx);
            if let Err(why) =
                self.read_creation_capabilities(ticket, machine, expected_node, card.session)
            {
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
            *held = None;
            cx.notify();
        }
    }
}

#[cfg(test)]
mod tests;
