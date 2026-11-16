//! Correlated lifecycle requests prevent double actions and late navigation.
use crate::machine_book::Name;
use vessel_proto::SessionId;
/// A lifecycle result is correlated independently from navigation lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleTicket(u64);
/// One target keeps the saved host and session together at every boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleTarget {
    /// Validated saved-host identity for the current link connection.
    pub host: Name,
    /// Durable node-local session ID.
    pub session: SessionId,
    /// Retained authenticated identity for a deep-link/pane destination.
    /// A normal saved-host action has no stable identity before hydration.
    pub node: Option<vessel_proto::NodeId>,
}
/// The user's intended effect after the authoritative node result arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleAction {
    /// Keep the conversation available in history.
    Archive,
    /// Restore visibility without starting a provider.
    Unarchive,
    /// Continue the same provider conversation.
    Resume,
    /// Permanently delete an ended session, behind confirmation.
    Delete,
    /// Archive before removing the visible session.
    Exit,
    /// Archive before opening a new draft with the same launch configuration.
    Fresh,
}
/// A second action cannot overwrite an in-flight destructive request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LifecycleBusy {
    /// The first request has not settled yet.
    #[error("a session lifecycle operation is already pending")]
    Pending,
    /// Request IDs are never recycled in the same window.
    #[error("session lifecycle request IDs are exhausted")]
    Exhausted,
}
/// Retain target and action so an answer cannot apply to a different session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingLifecycle {
    /// Correlation identity carried by the link response.
    pub ticket: LifecycleTicket,
    /// The exact target acted on.
    pub target: LifecycleTarget,
    /// The requested postcondition.
    pub action: LifecycleAction,
}
/// One operation per window avoids duplicate archive/delete clicks.
#[derive(Debug, Default)]
pub struct Lifecycle {
    issued: u64,
    pending: Option<PendingLifecycle>,
}
impl Lifecycle {
    /// Preserve an existing pending operation until its own response arrives.
    /// # Errors
    /// Busy or identity exhaustion; no state is changed on refusal.
    pub fn begin(
        &mut self,
        target: LifecycleTarget,
        action: LifecycleAction,
    ) -> Result<PendingLifecycle, LifecycleBusy> {
        if self.pending.is_some() {
            return Err(LifecycleBusy::Pending);
        }
        let next = self.issued.checked_add(1).ok_or(LifecycleBusy::Exhausted)?;
        self.issued = next;
        let pending = PendingLifecycle {
            ticket: LifecycleTicket(next),
            target,
            action,
        };
        self.pending = Some(pending.clone());
        Ok(pending)
    }
    /// A stale acknowledgement never clears a later request.
    pub fn finish(&mut self, ticket: LifecycleTicket) -> Option<PendingLifecycle> {
        if self
            .pending
            .as_ref()
            .is_none_or(|pending| pending.ticket != ticket)
        {
            return None;
        }
        self.pending.take()
    }
    /// Pending controls stay disabled until a matching result settles them.
    #[must_use]
    pub const fn pending(&self) -> Option<&PendingLifecycle> {
        self.pending.as_ref()
    }
}

/// A typed success response is insufficient until its lifecycle state agrees.
#[must_use]
pub const fn satisfies_lifecycle(
    action: LifecycleAction,
    info: &vessel_proto::SessionInfo,
) -> bool {
    match action {
        LifecycleAction::Archive | LifecycleAction::Exit | LifecycleAction::Fresh => {
            info.marks.archived_at.is_some() && !info.status.is_live()
        }
        LifecycleAction::Unarchive => info.marks.archived_at.is_none(),
        LifecycleAction::Resume => info.marks.archived_at.is_none() && info.status.is_live(),
        LifecycleAction::Delete => false,
    }
}
