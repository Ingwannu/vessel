//! One UI lifetime owns cached reads; only explicit actions request external work.
use super::{PolicyPending, State};
use crate::{routes::Route, shell::Shell};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{
    Command,
    usage::{Answer, Ask},
};
use vessel_app_view::{
    Reach,
    devices::{MachineKey, Target},
    settings::Section,
    usage::{LedgerSelection, Range, RequestFailure},
};
use vessel_proto::limits::{ExternalLookup, LimitAccount};

impl Shell {
    pub(crate) fn settle_usage(&mut self, window: &Window) {
        self.settle_session_usage();
        let visible = self.route == Route::Settings
            && self.areas.settings.section == Section::Usage
            && (!vessel_app_kit::breakpoint::phone(window) || self.areas.settings.pushed);
        let target = if !visible {
            Target::Hidden
        } else if !self.loaded {
            Target::Loading
        } else {
            match super::super::settings::chosen_machine(self) {
                None => Target::NoMachines,
                Some(machine) => {
                    let key = MachineKey::from_book(&machine.name);
                    match machine.reach {
                        Reach::Online => Target::Online(key),
                        Reach::Unreachable(_) => Target::Offline(key),
                    }
                }
            }
        };
        let state = &mut self.areas.usage;
        if state.target != target {
            state.close();
            state.target = target;
        }
        let Target::Online(machine) = &state.target else {
            return;
        };
        let machine = machine.clone();
        if state.policy_pending.is_none() && state.inventory.selection() != Some(&machine) {
            self.usage_accounts(None);
        }
        let state = &mut self.areas.usage;
        let selection = LedgerSelection {
            machine,
            range: state.range,
        };
        if state.ledger.selection() != Some(&selection) {
            if let Some(request) = state.ledger.request(selection) {
                self.command(Command::Usage(Ask::Ledger(request)));
            }
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
        let Target::Online(machine) = &state.target else {
            return;
        };
        if let Some(request) = state.ledger.request(LedgerSelection {
            machine: machine.clone(),
            range: state.range,
        }) {
            self.command(Command::Usage(Ask::Ledger(request)));
        }
    }
    fn usage_accounts(&mut self, refresh: Option<LimitAccount>) {
        let state = &mut self.areas.usage;
        let Target::Online(machine) = &state.target else {
            return;
        };
        state.refresh_account = refresh;
        if let Some(request) = state.inventory.request(machine.clone()) {
            let refresh = state.refresh_account.take();
            self.command(Command::Usage(Ask::Accounts { request, refresh }));
        }
    }
    pub(crate) fn usage_refresh_account(&mut self, account: LimitAccount) {
        self.usage_accounts(Some(account));
    }
    pub(crate) fn usage_set_policy(&mut self, policy: ExternalLookup) {
        let state = &mut self.areas.usage;
        if state.policy_pending.is_some() {
            return;
        }
        let Target::Online(machine) = &state.target else {
            return;
        };
        let Ok(ticket) = state.tickets.next_ticket() else {
            state.policy_error = Some(RequestFailure::Exhausted);
            return;
        };
        let machine = machine.clone();
        // Disabling immediately revokes old snapshots' publishing rights in this UI.
        state.inventory.close();
        state.session.close();
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
    pub(crate) fn usage_answer(&mut self, answer: Answer, _cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.usage;
        let next = match answer {
            Answer::Accounts { request, result } => state
                .inventory
                .answer(request.ticket, result)
                .map(|request| Ask::Accounts {
                    request,
                    refresh: state.refresh_account.take(),
                }),
            Answer::Ledger { request, result } => {
                state.ledger.answer(request.ticket, result).map(Ask::Ledger)
            }
            Answer::Lookup {
                machine,
                ticket,
                result,
            } => {
                let Some(pending) = &state.policy_pending else {
                    return;
                };
                if pending.machine != machine || pending.ticket != ticket {
                    return;
                }
                state.policy_error = match result {
                    Ok(policy) if policy == pending.policy => None,
                    Ok(_) => Some(RequestFailure::InvalidReply),
                    Err(reason) => Some(reason),
                };
                state.policy_pending = None;
                // Even an uncertain mutation must be reread, never retried automatically.
                self.usage_accounts(None);
                return;
            }
            Answer::Session { request, result } => state
                .session
                .answer(request.ticket, result)
                .map(|request| Ask::Session {
                    request,
                    refresh: vessel_proto::limits::LimitRefresh::Cached,
                }),
        };
        if let Some(ask) = next {
            self.command(Command::Usage(ask));
        }
    }
}
impl State {
    fn close(&mut self) {
        self.ledger.close();
        self.inventory.close();
        self.refresh_account = None;
        self.policy_pending = None;
        self.policy_error = None;
    }
}
