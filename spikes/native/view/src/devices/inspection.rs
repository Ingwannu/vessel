//! Public device metadata on touch, and one selection rule for details
//! and revocation. No credential or node call belongs in this model.
//!
//! [Decision Log]
//! - Purpose: the complete grants and paired date must be readable without
//!   hover, while list rows remain one title, hint and trailing action.
//! - Constraint: a stale row must never select a revoked/missing device or
//!   replace a request already being sent. Caller authority stays unknown.
//! - Alternatives: another hint line per fact, or a touch-only tooltip.
//! - Choice: select one public record by id and expose read-only metadata;
//!   use the same eligibility rule before opening revoke confirmation.
//! - Why: one rule is testable without a toolkit or connection. A later
//!   successful list refreshes or closes the selected public snapshot.
//! - Cost: failed reads cannot establish newer facts; navigation drops it.
use super::{Listing, Page, Revoking, Target, time::exact_time};
use crate::Lang;
use chrono::FixedOffset;
use vessel_proto::{DeviceId, DeviceInfo};

/// A stale control's refusal, before any focus change or RPC is attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SelectionError {
    /// The page no longer has an online target.
    #[error("the selected machine is unavailable")]
    TargetUnavailable,
    /// Safe confirmation identities are exhausted.
    #[error("confirmation identities exhausted")]
    IdentitiesExhausted,
    /// The row was drawn for a retired connection.
    #[error("the selected connection changed")]
    ConnectionChanged,
    /// This admitted caller has no currently confirmed management grant.
    #[error("management access is not confirmed for this connection")]
    AccessNotConfirmed,
    /// No successful list is available for this target.
    #[error("the device list is not ready")]
    ListUnavailable,
    /// The existing confirmation must be settled before another selection.
    #[error("the revoke confirmation is already open")]
    ConfirmationOpen,
    /// A confirmed revoke is already awaiting its response.
    #[error("a revocation is already being sent")]
    RevocationPending,
    /// The id is absent or its access was already removed.
    #[error("the device no longer has access")]
    DeviceUnavailable,
}

impl Page {
    /// Select only a row drawn for the current authenticated admission.
    ///
    /// # Errors
    /// The context or active-list selection is stale.
    pub fn inspect_bound(
        &mut self,
        context: &super::ConnectionContext,
        id: DeviceId,
    ) -> Result<(), SelectionError> {
        self.check_context(context)
            .map_err(|_changed| SelectionError::ConnectionChanged)?;
        self.inspect_device(id)
    }
    /// Open one uniquely identified confirmation on this admitted connection.
    ///
    /// # Errors
    /// The context, list or selection is stale, or request identities exhausted.
    pub fn choose_bound(
        &mut self,
        context: &super::ConnectionContext,
        id: DeviceId,
    ) -> Result<(), SelectionError> {
        self.check_context(context)
            .map_err(|_changed| SelectionError::ConnectionChanged)?;
        if !self.authority.may_manage() {
            return Err(SelectionError::AccessNotConfirmed);
        }
        self.choose_revoke(id)
    }

    fn selectable(&self, id: DeviceId) -> Result<&DeviceInfo, SelectionError> {
        if !matches!(self.target, Target::Online(_)) {
            return Err(SelectionError::TargetUnavailable);
        }
        match self.revoking {
            Revoking::Confirm(_) => return Err(SelectionError::ConfirmationOpen),
            Revoking::Sending { .. } => return Err(SelectionError::RevocationPending),
            Revoking::Empty | Revoking::Failed(_) => {}
        }
        let Listing::Ready(devices) = &self.listing else {
            return Err(SelectionError::ListUnavailable);
        };
        devices
            .iter()
            .find(|device| device.id == id && device.revoked_at.is_none())
            .ok_or(SelectionError::DeviceUnavailable)
    }

    /// Open metadata only for an active row of the current successful list.
    ///
    /// # Errors
    /// The row is stale, its target is unavailable, or a revoke is pending.
    pub(super) fn inspect_device(&mut self, id: DeviceId) -> Result<(), SelectionError> {
        self.inspection = Some(self.selectable(id)?.clone());
        Ok(())
    }

    /// The selected public snapshot; no access credential is representable.
    #[must_use]
    pub const fn inspected_device(&self) -> Option<&DeviceInfo> {
        self.inspection.as_ref()
    }

    /// Close details without changing the listing or any pending operation.
    pub fn close_inspection(&mut self) {
        self.inspection = None;
    }

    /// Reconcile public metadata after a successful list, never from a failed
    /// read's absence. A revoked/removed record closes its details.
    pub fn reconcile_inspection(&mut self) {
        let Some(selected) = self.inspection.as_ref().map(|device| device.id) else {
            return;
        };
        if let Listing::Ready(devices) = &self.listing {
            self.inspection = devices
                .iter()
                .find(|device| device.id == selected && device.revoked_at.is_none())
                .cloned();
        }
    }

    /// Select a device for the existing confirmation, closing its details.
    /// No request is sent until `Revoking::confirm` consumes that selection.
    ///
    /// # Errors
    /// The row is stale, its target is unavailable, or a revoke is pending.
    pub(super) fn choose_revoke(&mut self, id: DeviceId) -> Result<(), SelectionError> {
        let device = self.selectable(id)?.clone();
        let context = self
            .context()
            .cloned()
            .ok_or(SelectionError::ConnectionChanged)?;
        let intent = match self.tickets.next_ticket() {
            Ok(ticket) => super::RevokeIntent(ticket),
            Err(why) => {
                self.revoking = Revoking::Failed(why);
                return Err(SelectionError::IdentitiesExhausted);
            }
        };
        self.inspection = None;
        self.revoking = Revoking::Confirm(super::Confirmation {
            context,
            intent,
            device,
        });
        Ok(())
    }
}

/// Display text only: exact local dates and every independent grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceMetadata {
    /// Pairing time with seconds and the viewer's UTC offset.
    pub paired_at: String,
    /// Exact last connection time, or a complete absent-time value.
    pub last_seen: String,
    /// All grants, never merely the list row's primary access summary.
    pub grants: String,
}
impl DeviceMetadata {
    /// Reuse the same local-time and scope wording as the exact tooltip.
    #[must_use]
    pub fn of(lang: Lang, device: &DeviceInfo, offset: FixedOffset) -> Self {
        let words = lang.devices();
        Self {
            paired_at: exact_time(device.created_at, offset),
            last_seen: device.last_seen.map_or_else(
                || words.never_connected.to_owned(),
                |seen| exact_time(seen, offset),
            ),
            grants: words.granted_scopes(&device.scopes),
        }
    }
}

#[cfg(test)]
mod tests;
