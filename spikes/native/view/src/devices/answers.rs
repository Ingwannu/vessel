//! Replies belong to the admission and request that started them. Checking
//! here prevents a later screen from forgetting one of the two identities.
//!
//! [Decision Log]
//! - Purpose: a reply must never populate a replacement connection's page.
//! - Constraint: tickets and admission identity guard different lifetimes.
//! - Alternatives: repeat the checks in each toolkit event arm.
//! - Choice: pure reducers require both facts before consuming the result.
//! - Why: the same rule drops credentials and public data without drawing.
//! - Cost: three small adapters; untrusted error text still never enters view.
use super::{ConnectionContext, Listing, Page, Pairing, Problem, Ticket};
use vessel_proto::DeviceInfo;

impl Page {
    /// Apply only this admission's outstanding read and reconcile its details.
    pub fn listed(
        &mut self,
        context: &ConnectionContext,
        ticket: Ticket,
        result: Result<Vec<DeviceInfo>, Problem>,
    ) -> bool {
        if self.check_context(context).is_err()
            || !matches!(self.listing, Listing::Loading(expected) if expected == ticket)
        {
            return false;
        }
        self.listing.answer(ticket, result);
        self.reconcile_inspection();
        true
    }
    /// Drop a replaced connection's credential without ever exposing it.
    pub fn invited(
        &mut self,
        context: &ConnectionContext,
        ticket: Ticket,
        result: Result<Pairing, Problem>,
    ) -> bool {
        self.check_context(context).is_ok() && self.inviting.answer(ticket, result)
    }
    /// A late revoke result cannot refresh or modify a replacement's list.
    pub fn revoked(
        &mut self,
        context: &ConnectionContext,
        ticket: Ticket,
        result: Result<(), Problem>,
    ) -> bool {
        self.check_context(context).is_ok() && self.revoking.answer(ticket, result)
    }
}

#[cfg(test)]
mod tests;
