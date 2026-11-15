//! Staged owned resources for W12's one `SessionResources` bundle, never a registry.
//! The application Store/writer and window preview stay services; these resources
//! keep their actual Origin leases and immutable pending payload through moves.
use super::{attach::Waiting, capabilities, transmission::Phase};
use std::num::NonZeroUsize;
use vessel_app_view::{
    composer::{context::ContextTray, tray::Tray},
    residents::{
        FollowBinding, FollowRetirement, HoldReason, OperationKind, OperationTicket,
        OwnerReadiness, ResidentToken, SessionContext,
    },
};

#[derive(Default)]
enum Submission {
    #[default]
    Idle,
    Prepared(Box<Waiting>),
    Active {
        binding: FollowBinding,
        ticket: OperationTicket,
        waiting: Box<Waiting>,
    },
    Retired {
        binding: FollowBinding,
        ticket: OperationTicket,
        waiting: Box<Waiting>,
    },
}
impl Submission {
    fn operation(&self) -> Option<(FollowBinding, OperationTicket)> {
        match self {
            Self::Idle | Self::Prepared(_) => None,
            Self::Active {
                binding, ticket, ..
            }
            | Self::Retired {
                binding, ticket, ..
            } => Some((*binding, *ticket)),
        }
    }
    fn waiting(&self) -> Option<&Waiting> {
        match self {
            Self::Idle => None,
            Self::Prepared(waiting)
            | Self::Active { waiting, .. }
            | Self::Retired { waiting, .. } => Some(waiting),
        }
    }
}
#[derive(Debug, Default, PartialEq, Eq)]
enum Recovery {
    #[default]
    Confirmed,
    Unconfirmed,
}
#[derive(Debug, Default, PartialEq, Eq)]
enum Preview {
    #[default]
    Released,
    Held,
}
/// Fields move from their current sole owner at coordinated cutover; no Clone.
pub(crate) struct SessionAttachments {
    owner: ResidentToken,
    tray: Tray,
    context: ContextTray,
    submission: Submission,
    capability: Option<capabilities::State>,
    acquiring: Option<NonZeroUsize>,
    recovery: Recovery,
    preview: Preview,
}
impl std::fmt::Debug for SessionAttachments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionAttachments")
            .field("files", &self.tray.items().len())
            .field("context_retained", &self.context.has_retained())
            .field("pending", &self.submission.waiting().is_some())
            .field("operation", &self.submission.operation().is_some())
            .field("readiness", &self.readiness())
            .finish_non_exhaustive()
    }
}
/// A concrete moved value, including leases; it is neither serde nor metadata.
#[derive(Debug)]
pub(crate) struct AttachmentSnapshot(SessionAttachments);
/// Refusal returns the original resource rather than dropping protected input.
#[derive(Debug)]
pub(crate) enum SnapshotAttempt {
    Ready(AttachmentSnapshot),
    Held {
        resources: SessionAttachments,
        reason: HoldReason,
    },
}
/// Another resource/window cannot take ownership merely by sharing a `SessionId`.
#[derive(Debug)]
pub(crate) enum RestoreAttempt {
    Ready(SessionAttachments),
    Refused(AttachmentSnapshot),
}
impl SessionAttachments {
    #[must_use]
    pub(crate) fn new(context: &SessionContext) -> Self {
        Self {
            owner: context.resident,
            tray: Tray::default(),
            context: ContextTray::default(),
            submission: Submission::Idle,
            capability: None,
            acquiring: None,
            recovery: Recovery::Confirmed,
            preview: Preview::Released,
        }
    }
    #[must_use]
    pub(crate) fn readiness(&self) -> OwnerReadiness {
        if self.recovery == Recovery::Unconfirmed
            || matches!(self.submission, Submission::Retired { .. })
        {
            return OwnerReadiness::Hold(HoldReason::UncertainEffect);
        }
        if let Some(waiting) = self.submission.waiting() {
            let reason = match &waiting.phase {
                Phase::ResumeUncertain => HoldReason::UncertainEffect,
                Phase::Retained { problem, .. } => match problem {
                    vessel_app_link::attachments::SubmitFailure::NotSent
                    | vessel_app_link::attachments::SubmitFailure::Target(_) => {
                        HoldReason::DirtyInput
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    vessel_app_link::attachments::SubmitFailure::Uncertain(_)
                    | vessel_app_link::attachments::SubmitFailure::TimedOut => {
                        HoldReason::UncertainEffect
                    }
                    #[cfg(target_arch = "wasm32")]
                    vessel_app_link::attachments::SubmitFailure::TimedOut => {
                        HoldReason::UncertainEffect
                    }
                },
                Phase::Preparing | Phase::Submitting(_) => HoldReason::PendingEffect,
            };
            return OwnerReadiness::Hold(reason);
        }
        if self.acquiring.is_some()
            || matches!(self.capability, Some(capabilities::State::Reading(_)))
        {
            return OwnerReadiness::Hold(HoldReason::Loading);
        }
        if self.preview == Preview::Held {
            return OwnerReadiness::Hold(HoldReason::Overlay);
        }
        if !self.tray.is_empty() || self.context.has_retained() {
            return OwnerReadiness::Hold(HoldReason::DirtyInput);
        }
        OwnerReadiness::Ready
    }
    /// Registry invalidation precedes this callback. No RPC, retry, clear or
    /// fabricated transport error is needed to retain the original ownership.
    #[must_use]
    pub(crate) fn follow_retired(&mut self, notice: &FollowRetirement) -> bool {
        let Submission::Active {
            binding, ticket, ..
        } = &self.submission
        else {
            return false;
        };
        if binding.resident != self.owner
            || notice.binding() != *binding
            || !notice
                .operations()
                .iter()
                .any(|op| op.kind() == OperationKind::Attachment && op.ticket() == *ticket)
        {
            return false;
        }
        let previous = std::mem::take(&mut self.submission);
        self.submission = match previous {
            Submission::Active {
                binding,
                ticket,
                waiting,
            } => Submission::Retired {
                binding,
                ticket,
                waiting,
            },
            Submission::Idle | Submission::Prepared(_) | Submission::Retired { .. } => previous,
        };
        true
    }
    #[must_use]
    pub(crate) fn into_snapshot(self) -> SnapshotAttempt {
        match self.readiness() {
            OwnerReadiness::Ready => SnapshotAttempt::Ready(AttachmentSnapshot(self)),
            OwnerReadiness::Hold(reason) => SnapshotAttempt::Held {
                resources: self,
                reason,
            },
        }
    }
}
impl AttachmentSnapshot {
    /// Pure resource transfer grants no fresh connection/follow/queue authority.
    #[must_use]
    pub(crate) fn restore(self, context: &SessionContext) -> RestoreAttempt {
        if self.0.owner.registry != context.resident.registry
            || self.0.owner.address != context.resident.address
        {
            return RestoreAttempt::Refused(self);
        }
        let mut resources = self.0;
        resources.owner = context.resident;
        RestoreAttempt::Ready(resources)
    }
}

#[cfg(test)]
mod tests;
