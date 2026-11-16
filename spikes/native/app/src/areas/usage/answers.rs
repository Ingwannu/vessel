//! Every reader settles only its own surface, selection and non-reused ticket.
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::{
    Command,
    usage::{Answer, Ask},
};
use vessel_app_view::{
    connection::ConnectionContext,
    devices::{MachineKey, Ticket},
    usage::{Inventory, InventorySurface, ReadRequest, RequestFailure},
};
use vessel_proto::limits::{ExternalLookup, LimitRefresh};
impl Shell {
    pub(crate) fn usage_answer(&mut self, answer: Answer, _cx: &mut Context<'_, Self>) {
        let next = match answer {
            Answer::Connected { machine, node: _ } => {
                self.usage_connected(&machine);
                None
            }
            Answer::Accounts {
                surface,
                request,
                result,
            } => {
                self.usage_accounts_answer(surface, request, result);
                None
            }
            Answer::Ledger { request, result } => self
                .areas
                .usage
                .ledger
                .answer(request.ticket, result)
                .map(Ask::Ledger),
            Answer::Lookup {
                machine,
                ticket,
                result,
            } => {
                self.usage_policy_answer(&machine, ticket, result);
                None
            }
            Answer::SourceLookup {
                machine,
                ticket,
                result,
            } => {
                self.usage_source_answer(&machine, ticket, result);
                None
            }
            Answer::Session { request, result } => self
                .areas
                .usage
                .session
                .answer(request.ticket, result)
                .map(|request| Ask::Session {
                    request,
                    refresh: LimitRefresh::Cached,
                }),
        };
        if let Some(ask) = next {
            self.command(Command::Usage(ask));
        }
    }
    fn usage_connected(&mut self, machine: &MachineKey) {
        let state = &mut self.areas.usage;
        if state
            .inventory
            .selection()
            .is_some_and(|context| context.machine() == machine)
        {
            state.inventory.close();
            state.source_tips = super::source_tips::SourceTips::default();
            state.ledger.close();
        }
        if state
            .summary
            .selection()
            .is_some_and(|context| context.machine() == machine)
        {
            state.summary.close();
        }
        if state
            .session
            .selection()
            .is_some_and(|selected| selected.machine.machine() == machine)
        {
            state.session.close();
        }
    }
    fn usage_accounts_answer(
        &mut self,
        surface: InventorySurface,
        request: ReadRequest<ConnectionContext>,
        result: Result<Inventory, RequestFailure>,
    ) {
        let machine = request.selection;
        let state = &mut self.areas.usage;
        let previous = state.inventory.observation_ticket();
        let next = match surface {
            InventorySurface::Accounts => {
                state
                    .inventory
                    .answer(request.ticket, result)
                    .map(|request| Ask::Accounts {
                        surface,
                        request,
                        refresh: std::mem::take(&mut state.refresh_account),
                    })
            }
            InventorySurface::Summary => {
                state
                    .summary
                    .answer(request.ticket, result)
                    .map(|request| Ask::Accounts {
                        surface,
                        request,
                        refresh: vessel_app_view::usage::InventoryRefresh::Cached,
                    })
            }
        };
        // Retire the overlay before any root render can clone its old content.
        // A stale answer cannot change the reader's accepted observation ticket.
        if surface == InventorySurface::Accounts && previous != state.inventory.observation_ticket()
        {
            state.source_tips = super::source_tips::SourceTips::default();
        }
        if let Some(ask) = next {
            self.command(Command::Usage(ask));
        }
        if surface == InventorySurface::Accounts {
            self.refresh_usage_summary(&machine);
        }
    }
    fn usage_policy_answer(
        &mut self,
        machine: &ConnectionContext,
        ticket: Ticket,
        result: Result<ExternalLookup, RequestFailure>,
    ) {
        let current = self.usage_connection_current(machine);
        let state = &mut self.areas.usage;
        let Some(pending) = &state.policy_pending else {
            return;
        };
        if &pending.machine != machine || pending.ticket != ticket {
            return;
        }
        let problem = match result {
            Ok(policy) if policy == pending.policy => None,
            Ok(_) => Some(RequestFailure::InvalidReply),
            Err(reason) => Some(reason),
        };
        state.policy_error = if current && state.connection.as_ref() == Some(machine) {
            problem
        } else {
            None
        };
        state.policy_pending = None;
        self.usage_accounts(vessel_app_view::usage::InventoryRefresh::Cached);
    }
}
