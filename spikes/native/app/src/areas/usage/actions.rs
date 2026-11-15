//! One UI lifetime owns cached reads; only explicit actions request external work.
use super::{PolicyPending, State};
use crate::{routes::Route, shell::Shell};
use vessel_app_kit::ui::Window;
use vessel_app_link::{Command, usage::Ask};
use vessel_app_view::{
    connection::ConnectionContext,
    devices::{MachineKey, Target},
    settings::Section,
    usage::{InventoryRefresh, InventorySurface, LedgerSelection, Range, RequestFailure},
};
use vessel_proto::limits::{ExternalLookup, LimitAccount};

impl Shell {
    pub(super) fn usage_connection_current(&self, expected: &ConnectionContext) -> bool {
        self.fleet.machines.iter().any(|machine| {
            machine.name == expected.machine().as_str()
                && machine.connection_context().as_ref() == Some(expected)
        })
    }
    pub(crate) fn settle_usage(&mut self, window: &Window) {
        self.settle_session_usage();
        self.settle_summary_usage();
        let visible = self.route == Route::Settings
            && self.areas.settings.section == Section::Usage
            && (!vessel_app_kit::breakpoint::phone(window) || self.areas.settings.pushed);
        let target = if self.areas.usage.modal.current().is_some() {
            super::modal::target(self)
        } else if !visible {
            Target::Hidden
        } else if !self.loaded {
            Target::Loading
        } else {
            match super::super::settings::chosen_machine(self) {
                None => Target::NoMachines,
                Some(machine) => {
                    let key = MachineKey::from_book(&machine.name);
                    if machine.connection_context().is_some() {
                        Target::Online(key)
                    } else {
                        Target::Offline(key)
                    }
                }
            }
        };
        let connection = match &target {
            Target::Online(machine) => self
                .fleet
                .machines
                .iter()
                .find(|candidate| candidate.name == machine.as_str())
                .and_then(vessel_app_view::Machine::connection_context),
            _ => None,
        };
        let state = &mut self.areas.usage;
        if state.target != target || state.connection != connection {
            state.close();
            state.target = target;
            state.connection = connection;
        }
        let Some(machine) = &state.connection else {
            return;
        };
        let machine = machine.clone();
        if state.policy_pending.is_none()
            && state.source_pending.is_none()
            && state.inventory.selection() != Some(&machine)
        {
            self.usage_accounts(InventoryRefresh::Cached);
        }
        let state = &mut self.areas.usage;
        let selection = LedgerSelection {
            machine,
            range: state.range,
        };
        if state.ledger.selection() != Some(&selection)
            && let Some(request) = state.ledger.request(selection)
        {
            self.command(Command::Usage(Ask::Ledger(request)));
        }
    }
    pub(crate) fn usage_range(&mut self, range: Range) {
        if self.areas.usage.range != range {
            self.areas.usage.range = range;
            self.usage_refresh();
        }
    }
    pub(crate) fn usage_refresh(&mut self) {
        let state = &mut self.areas.usage;
        let Some(machine) = &state.connection else {
            return;
        };
        if let Some(request) = state.ledger.request(LedgerSelection {
            machine: machine.clone(),
            range: state.range,
        }) {
            self.command(Command::Usage(Ask::Ledger(request)));
        }
    }
    pub(super) fn usage_accounts(&mut self, refresh: InventoryRefresh) {
        let state = &mut self.areas.usage;
        let Some(machine) = &state.connection else {
            return;
        };
        state.refresh_account = refresh;
        if let Some(request) = state.inventory.request(machine.clone()) {
            let refresh = std::mem::take(&mut state.refresh_account);
            self.command(Command::Usage(Ask::Accounts {
                surface: InventorySurface::Accounts,
                request,
                refresh,
            }));
        }
    }
    pub(super) fn usage_refresh_all(&mut self, machine: ConnectionContext) {
        if !self.usage_connection_current(&machine)
            || self.areas.usage.connection.as_ref() != Some(&machine)
        {
            return;
        }
        self.usage_accounts(InventoryRefresh::All);
    }
    pub(crate) fn usage_refresh_account(
        &mut self,
        machine: ConnectionContext,
        account: LimitAccount,
    ) {
        if !self.usage_connection_current(&machine)
            || self.areas.usage.connection.as_ref() != Some(&machine)
        {
            return;
        }
        self.usage_accounts(InventoryRefresh::Account(account));
    }
    pub(crate) fn usage_set_policy(&mut self, expected: ConnectionContext, policy: ExternalLookup) {
        if !self.usage_connection_current(&expected) {
            return;
        }
        let state = &mut self.areas.usage;
        if state.policy_pending.is_some() || state.source_pending.is_some() {
            return;
        }
        let Some(machine) = &state.connection else {
            return;
        };
        if machine != &expected {
            return;
        }
        let Ok(ticket) = state.tickets.next_ticket() else {
            state.policy_error = Some(RequestFailure::Exhausted);
            return;
        };
        let machine = machine.clone();
        // Disabling immediately revokes old snapshots' publishing rights in this UI.
        state.inventory.close();
        state.source_tips = super::source_tips::SourceTips::default();
        state.session.close();
        state.summary.close();
        state.policy_error = None;
        state.policy_pending = Some(PolicyPending {
            ticket,
            machine: machine.clone(),
            policy,
        });
        self.command(Command::Usage(Ask::Lookup {
            machine,
            ticket,
            policy,
        }));
    }
}
impl State {
    fn close(&mut self) {
        self.source_tips = super::source_tips::SourceTips::default();
        self.ledger.close();
        self.inventory.close();
        self.refresh_account = InventoryRefresh::Cached;
        self.policy_error = None;
    }
}
