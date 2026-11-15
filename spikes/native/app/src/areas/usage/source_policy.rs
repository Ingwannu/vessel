//! Source consent mutations retain their captured host/disclosure and never trigger vendor refresh.
use super::SourcePending;
use crate::shell::Shell;
use vessel_app_link::{Command, usage::Ask};
use vessel_app_view::{
    devices::{MachineKey, Target, Ticket},
    usage::{InventoryRefresh, RequestFailure},
};
use vessel_proto::{
    limits::ExternalLookup,
    usage_reports::{
        SetUsageSourceLookup, UsageSourceAvailability, UsageSourceConsent, UsageSourceDescriptor,
    },
};
impl Shell {
    pub(super) fn usage_source_policy(
        &mut self,
        machine: MachineKey,
        request: SetUsageSourceLookup,
    ) {
        let state = &mut self.areas.usage;
        if state.policy_pending.is_some()
            || state.source_pending.is_some()
            || !matches!(&state.target,Target::Online(current) if current==&machine)
        {
            return;
        }
        let Some(reports) = state
            .inventory
            .phase()
            .observation()
            .and_then(vessel_app_view::usage::Inventory::source_reports)
        else {
            return;
        };
        let Some(source) = reports
            .sources
            .as_slice()
            .iter()
            .find(|source| source.id == request.source)
        else {
            return;
        };
        if source.disclosure.version != request.disclosure_version
            || matches!(source.consent, UsageSourceConsent::InheritedGlobal)
            || source.availability == UsageSourceAvailability::Unsupported
        {
            return;
        }
        let Ok(ticket) = state.tickets.next_ticket() else {
            state.policy_error = Some(RequestFailure::Exhausted);
            return;
        };
        state.inventory.close();
        state.summary.close();
        state.session.close();
        state.policy_error = None;
        state.source_pending = Some(SourcePending {
            ticket,
            machine: machine.clone(),
            request: request.clone(),
        });
        self.command(Command::Usage(Ask::SourceLookup {
            machine,
            ticket,
            request,
        }));
    }
    pub(super) fn usage_refresh_report(
        &mut self,
        machine: MachineKey,
        id: vessel_proto::usage_reports::UsageReportId,
    ) {
        if !matches!(&self.areas.usage.target,Target::Online(current) if current==&machine) {
            return;
        }
        self.usage_accounts(InventoryRefresh::Report(id));
    }
    pub(super) fn usage_source_answer(
        &mut self,
        machine: MachineKey,
        ticket: Ticket,
        result: Result<UsageSourceDescriptor, RequestFailure>,
    ) {
        let state = &mut self.areas.usage;
        let Some(pending) = &state.source_pending else {
            return;
        };
        if pending.machine != machine || pending.ticket != ticket {
            return;
        }
        let failure = match result {
            Ok(source) => {
                let granted = match (pending.request.policy, source.consent) {
                    (ExternalLookup::Off, UsageSourceConsent::Off) => true,
                    (ExternalLookup::On, UsageSourceConsent::Granted { version }) => {
                        version == pending.request.disclosure_version
                    }
                    (
                        ExternalLookup::Off,
                        UsageSourceConsent::Granted { .. } | UsageSourceConsent::InheritedGlobal,
                    )
                    | (
                        ExternalLookup::On,
                        UsageSourceConsent::Off | UsageSourceConsent::InheritedGlobal,
                    ) => false,
                };
                (!granted
                    || source.id != pending.request.source
                    || source.disclosure.version != pending.request.disclosure_version)
                    .then_some(RequestFailure::InvalidReply)
            }
            Err(reason) => Some(reason),
        };
        state.policy_error = if matches!(&state.target,Target::Online(current) if current==&machine)
        {
            failure
        } else {
            None
        };
        state.source_pending = None;
        self.usage_accounts(InventoryRefresh::Cached);
    }
}
