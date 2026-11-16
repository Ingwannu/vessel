//! A Devices page is scoped to one already-parsed node-book entry.
//! Changing its scope drops all pending results, including credentials.

use super::{
    ConnectionContext, DeviceIdentity, IdentityChange, Inviting, Listing, MachineKey, Revoking,
    Ticket, Tickets,
};

/// The selected machine's availability, without inventing a live-device
/// status the node does not report.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Target {
    /// This page is not shown (pending credentials are discarded).
    #[default]
    Hidden,
    /// The node book has not arrived yet.
    Loading,
    /// The book contains no machine.
    NoMachines,
    /// The selected machine is unavailable.
    Offline(MachineKey),
    /// Requests can be sent through this connection.
    Online(MachineKey),
}

/// One confirmation, distinct from the eventual network request identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevokeIntent(pub(super) Ticket);
impl std::fmt::Display for RevokeIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

/// State whose lifetime is the selected machine, plus a ticket source
/// whose lifetime is the app: an A-to-B-to-A switch cannot reuse tickets.
#[derive(Debug, Default)]
pub struct Page {
    identity: DeviceIdentity,
    pub(super) unconfirmed: Option<super::interrupted::Interrupted>,
    /// This admitted caller's explicitly reported effective grant.
    pub(super) authority: super::Authority,
    // A selected public record, never an invitation or a credential. Kept
    // while a read is pending; a successful list reconciles it by id.
    pub(super) inspection: Option<vessel_proto::DeviceInfo>,
    /// What the page is about.
    pub target: Target,
    /// The device list's state.
    pub listing: Listing,
    /// The in-memory invitation.
    pub inviting: Inviting,
    /// Revocation state, from selection through confirmed response.
    pub revoking: Revoking,
    /// Request identities retained across target changes.
    pub tickets: Tickets,
}

impl Page {
    pub(super) fn clear_owned(&mut self) {
        self.authority = super::Authority::Unchecked;
        self.listing = Listing::Empty;
        self.inviting = Inviting::Empty;
        self.revoking = Revoking::Empty;
        self.inspection = None;
    }
    /// Reconcile authenticated facts before drawing or accepting a callback.
    /// Scope is not inferred from a session read or display-name match.
    pub fn bind(&mut self, context: Option<ConnectionContext>) -> IdentityChange {
        let context = context.filter(|context| {
            matches!(&self.target,
            Target::Online(machine) if machine == context.machine())
        });
        if self.context() != context.as_ref() {
            self.interrupted();
        }
        let change = self.identity.observe(context);
        if change == IdentityChange::Replaced {
            self.clear_owned();
        }
        change
    }
    /// Public identity to capture in callbacks, request envelopes and element ids.
    #[must_use]
    pub const fn context(&self) -> Option<&ConnectionContext> {
        self.identity.current()
    }
    /// Refuse old callbacks and replies without interpreting an alias again.
    ///
    /// # Errors
    /// This admission is no longer the one selected by this page.
    pub fn check_context(&self, context: &ConnectionContext) -> Result<(), super::StaleContext> {
        self.identity.check(context)
    }
    /// Current confirmation intent; cancel/reopen never reuses it.
    #[must_use]
    pub const fn revoke_intent(&self) -> Option<RevokeIntent> {
        self.revoking.intent()
    }
    /// Consume only the confirmation the pressed button was drawn for.
    /// Repeated or stale callbacks send nothing.
    pub fn confirm_revoke(
        &mut self,
        context: &ConnectionContext,
        intent: RevokeIntent,
        request: Ticket,
    ) -> Option<vessel_proto::DeviceId> {
        if self.check_context(context).is_err()
            || !self.authority.may_manage()
            || self.revoking.intent() != Some(intent)
        {
            return None;
        }
        self.revoking.confirm(context, intent, request)
    }
    /// A cancelled older dialog cannot dismiss a newly opened confirmation.
    pub fn cancel_revoke(&mut self, context: &ConnectionContext, intent: RevokeIntent) {
        if self.check_context(context).is_ok()
            && self.revoking.intent() == Some(intent)
            && matches!(self.revoking, Revoking::Confirm(_))
        {
            self.revoking = Revoking::Empty;
        }
    }

    /// Replace scope, dropping everything belonging to the previous one.
    pub fn target(&mut self, target: Target) {
        if self.target != target {
            self.interrupted();
            self.clear_owned();
            let _retired = self.identity.observe(None);
            self.target = target;
        }
    }
}
