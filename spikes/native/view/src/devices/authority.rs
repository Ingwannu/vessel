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

/// A bounded, duplicate-free node report; it cannot be constructed unchecked.
/// Context and ticket admission remain the Page reducer's separate responsibility.
#[derive(Debug)]
pub struct ReportedAuthority(DeviceSelf);
impl ReportedAuthority {
    fn parse(caller: DeviceSelf) -> Result<Self, Problem> {
        if valid(&caller) {
            Ok(Self(caller))
        } else {
            Err(Problem::InvalidReply)
        }
    }
    /// Every effective grant, including an explicitly empty set.
    #[must_use]
    pub fn scopes(&self) -> &[Scope] {
        &self.0.scopes
    }
    /// Only the node-reported direct caller id, absent for other principals.
    #[must_use]
    pub const fn device(&self) -> Option<DeviceId> {
        self.0.device
    }
}

/// Only a successful query establishes this caller's effective grant.
#[derive(Debug, Default)]
pub enum Authority {
    /// No facts have been requested for this admission.
    #[default]
    Unchecked,
    /// Refreshing also hides the previous grant and caller identity.
    Loading(Ticket),
    /// Node-reported effective scopes, including a known empty grant.
    Known(ReportedAuthority),
    /// A failed read establishes neither denial nor permission.
    Failed(Problem),
}

impl Authority {
    /// Device listing, invitation and revocation all require Admin.
    #[must_use]
    pub fn may_manage(&self) -> bool {
        matches!(self, Self::Known(caller) if caller.scopes().contains(&Scope::Admin))
    }

    /// Owner/agent/delegated callers may report no device id, even as Admin.
    #[must_use]
    pub const fn device(&self) -> Option<DeviceId> {
        match self {
            Self::Known(caller) => caller.device(),
            Self::Unchecked | Self::Loading(_) | Self::Failed(_) => None,
        }
    }

    fn answer(&mut self, ticket: Ticket, result: Result<DeviceSelf, Problem>) -> bool {
        if !matches!(self, Self::Loading(expected) if *expected == ticket) {
            return false;
        }
        *self = match result {
            Ok(caller) => match ReportedAuthority::parse(caller) {
                Ok(caller) => Self::Known(caller),
                Err(why) => Self::Failed(why),
            },
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
            .all(|(index, scope)| !caller.scopes.iter().take(index).any(|prior| prior == scope))
}

impl Page {
    /// Read only the current admission's authority; updates require correlation.
    #[must_use]
    pub const fn authority(&self) -> &Authority {
        &self.authority
    }

    /// Refreshing authority invalidates previous management intent immediately.
    /// A stale callback cannot clear a newer admission's surfaces.
    #[must_use]
    pub fn checking_authority(&mut self, context: &ConnectionContext, ticket: Ticket) -> bool {
        if self.check_context(context).is_err() {
            return false;
        }
        self.clear_owned();
        self.authority = Authority::Loading(ticket);
        true
    }

    /// Local inability to start a fresh query also drops previously owned facts.
    /// Refuse a stale control without clearing a newer admission.
    #[must_use]
    pub fn cannot_check_authority(&mut self, context: &ConnectionContext, why: Problem) -> bool {
        if self.check_context(context).is_err() {
            return false;
        }
        self.clear_owned();
        self.authority = Authority::Failed(why);
        true
    }

    /// A reply cannot grant permissions or a caller id to another admission.
    #[must_use]
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
