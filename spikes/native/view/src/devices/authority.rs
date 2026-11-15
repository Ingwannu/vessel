//! Caller facts belong to the selected admission, never a device label.
//!
//! [Decision Log]
//! - Purpose: show management only for an explicitly reported Admin grant.
//! - Constraint: session Read is independent of Admin; failure is unknown.
//! - Alternatives: infer authority from Reach, a list or a matching name.
//! - Choice: one Page-owned self query, correlated by context and ticket.
//! - Why: the existing page identity already retires every owned surface.
//! - Cost: older nodes need explicit recovery; they receive no default grant.
use super::{ConnectionContext, Page, Problem, Ticket};
use vessel_proto::{DeviceId, DeviceSelf, Scope};

/// Only a successful query establishes this caller's effective grant.
#[derive(Debug, Default)]
pub enum Authority {
    /// No facts have been requested for this admission.
    #[default]
    Unchecked,
    /// Refreshing also hides the previous grant and caller identity.
    Loading(Ticket),
    /// Node-reported effective scopes, including a known empty grant.
    Known(DeviceSelf),
    /// A failed read establishes neither denial nor permission.
    Failed(Problem),
}

impl Authority {
    /// Device listing, invitation and revocation all require Admin.
    #[must_use]
    pub fn may_manage(&self) -> bool {
        matches!(self, Self::Known(caller) if caller.scopes.contains(&Scope::Admin))
    }

    /// Owner/agent/delegated callers may report no device id, even as Admin.
    #[must_use]
    pub const fn device(&self) -> Option<DeviceId> {
        match self {
            Self::Known(caller) => caller.device,
            Self::Unchecked | Self::Loading(_) | Self::Failed(_) => None,
        }
    }

    fn answer(&mut self, ticket: Ticket, result: Result<DeviceSelf, Problem>) -> bool {
        if !matches!(self, Self::Loading(expected) if *expected == ticket) {
            return false;
        }
        *self = match result {
            Ok(caller) if valid(&caller) => Self::Known(caller),
            Ok(_invalid) => Self::Failed(Problem::InvalidReply),
            Err(why) => Self::Failed(why),
        };
        true
    }
}

fn valid(caller: &DeviceSelf) -> bool {
    // The wire contract is a canonical, duplicate-free set of at most ALL.
    // Refuse malformed facts rather than truncate them into a plausible grant.
    caller.scopes.len() <= Scope::ALL.len()
        && caller
            .scopes
            .iter()
            .enumerate()
            .all(|(index, scope)| !caller.scopes[..index].contains(scope))
}

impl Page {
    /// Refreshing authority invalidates previous management intent immediately.
    /// A stale callback cannot clear a newer admission's surfaces.
    pub fn checking_authority(&mut self, context: &ConnectionContext, ticket: Ticket) -> bool {
        if self.check_context(context).is_err() {
            return false;
        }
        self.clear_owned();
        self.authority = Authority::Loading(ticket);
        true
    }

    /// A reply cannot grant permissions or a caller id to another admission.
    pub fn authority_answer(
        &mut self,
        context: &ConnectionContext,
        ticket: Ticket,
        result: Result<DeviceSelf, Problem>,
    ) -> bool {
        self.check_context(context).is_ok() && self.authority.answer(ticket, result)
    }
}

#[cfg(test)]
mod tests;
