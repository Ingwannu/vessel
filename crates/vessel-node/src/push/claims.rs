//! Connected local admission reuses Web Push's presence policy. The writer
//! persists only the once-only source admission; it never elects a recipient.
use super::{PresenceId, Push, policy};
use chrono::Utc;
use std::sync::PoisonError;
use vessel_proto::notifications::NotificationClaim;
use vessel_proto::{SessionId, WireError};
use vessel_store::{LocalAdmission, NotificationAuthority, PushAttention};

impl Push {
    /// Stable ordering breaks equal-activity ties identically on every claim,
    /// unlike HashMap iteration. No lock is held across a store operation.
    pub(crate) fn recipient(&self, caller: PresenceId, session: SessionId) -> NotificationClaim {
        self.recipient_for(caller, vessel_proto::push::PushFocus::Session { session })
    }

    pub(crate) fn recipient_for(
        &self,
        caller: PresenceId,
        target: vessel_proto::push::PushFocus,
    ) -> NotificationClaim {
        let states = self.presence.lock().unwrap_or_else(PoisonError::into_inner);
        let mut ordered: Vec<_> = states.iter().collect();
        ordered.sort_unstable_by_key(|(id, _)| **id);
        let snapshots: Vec<_> = ordered.iter().map(|(_, presence)| **presence).collect();
        match policy::plan_for(&snapshots, target, Utc::now()) {
            policy::Plan::Suppressed => NotificationClaim::Suppressed,
            policy::Plan::Push => NotificationClaim::NoPresentClient,
            policy::Plan::Local(index) => {
                if ordered.get(index).is_some_and(|(id, _)| **id == caller) {
                    NotificationClaim::Admitted
                } else {
                    NotificationClaim::NotRecipient
                }
            }
        }
    }

    pub(crate) async fn claim_session(
        &self,
        caller: PresenceId,
        authority: NotificationAuthority,
        source: PushAttention,
    ) -> Result<NotificationClaim, WireError> {
        let recipient = self.recipient(caller, source.key.session);
        if recipient != NotificationClaim::Admitted {
            return Ok(recipient);
        }
        let admission = self
            .store
            .claim_notification(authority, source)
            .await
            .map_err(|error| {
                WireError::new(
                    error.code(),
                    "notification admission could not be committed; keep the inbox open",
                )
            })?;
        match admission {
            LocalAdmission::NoAttention => return Ok(NotificationClaim::NoAttention),
            LocalAdmission::AlreadyClaimed => return Ok(NotificationClaim::AlreadyClaimed),
            LocalAdmission::Admitted => {}
        }
        // A focus/read/revocation change while awaiting commit may spend this
        // best-effort opportunity, but must never authorize stale OS content.
        if !self
            .store
            .notification_current(authority, source)
            .await
            .map_err(|error| {
                WireError::new(
                    error.code(),
                    "notification attention could not be rechecked; keep the inbox open",
                )
            })?
        {
            return Ok(NotificationClaim::NoAttention);
        }
        Ok(self.recipient(caller, source.key.session))
    }
}
