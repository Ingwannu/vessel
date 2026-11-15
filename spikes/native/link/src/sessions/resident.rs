//! Staged typed session ingress. The connection owner alone mints delivery guards;
//! the resident owner alone validates attempts and settles operation tickets.
//! No event registration, raw fallback, registry or identity allocator lives here.
use crate::{ActFailed, Acted, DeliveryGuard, connections::SelectedConnection};
use std::sync::Arc;
use vessel_app_view::{
    connection::{ConnectionContext, Refusal},
    residents::{FollowBinding, OperationKind, OperationTicket, ResidentProblem},
};
use vessel_proto::{Conversation, EventRecord, SessionInfo};

/// One effect's opaque original owner admission, echoed without reminting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Operation {
    /// Canonical concrete owner namespace.
    pub kind: OperationKind,
    /// Single-use ticket minted by the sole Residents table.
    pub ticket: OperationTicket,
}
/// Immutable attempt facts and weak physical validity, captured once per work item.
#[derive(Clone)]
pub(crate) struct Scope {
    context: Arc<ConnectionContext>,
    binding: FollowBinding,
    delivery: DeliveryGuard,
}
impl Scope {
    pub(crate) fn capture(
        selected: &SelectedConnection,
        context: &ConnectionContext,
        binding: FollowBinding,
    ) -> Result<Self, Refusal> {
        let delivery = selected.delivery_guard();
        delivery.validate(context)?;
        if context.node() != binding.resident.address.node {
            return Err(Refusal::Changed);
        }
        Ok(Self {
            context: Arc::new(context.clone()),
            binding,
            delivery,
        })
    }
    pub(crate) fn envelope(&self, payload: Payload) -> Envelope {
        Envelope {
            scope: self.clone(),
            payload,
        }
    }
}
/// Data is exposed only after actual delivery validity and canonical admission.
pub enum Payload {
    /// Current metadata must name the originally requested session.
    Metadata(Box<SessionInfo>),
    /// Current pending requests and conversation state.
    Conversation(Box<Conversation>),
    /// Ordered records from this exact attach attempt.
    Records(Vec<EventRecord>),
    /// Retires this exact follow, never the shared connection by itself.
    Lost(ActFailed),
    /// Stream exhaustion does not retire the shared lease or settle a write.
    Eof,
    /// An owner must consume this ticket before it clears input or closes a tab.
    Completion {
        /// Original owner kind and single-use ticket.
        operation: Operation,
        /// Typed RPC outcome; uncertain transport failures are handled separately.
        result: Box<Result<Acted, ActFailed>>,
    },
}
impl std::fmt::Debug for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResidentPayload(<opaque>)")
    }
}
/// Invalid ingress never returns its peer payload or applies its mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// Original physical admission expired or closed after queueing.
    Delivery(Refusal),
    /// Canonical resident/attempt/ticket validation refused installation.
    Resident(ResidentProblem),
    /// A faulty metadata response named a different session.
    WrongSession,
}
/// Queued answers hold only weak delivery validity and immutable typed payload.
#[must_use]
pub struct Envelope {
    scope: Scope,
    payload: Payload,
}
impl std::fmt::Debug for Envelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResidentEnvelope(<opaque>)")
    }
}
impl Envelope {
    /// Original attempt for safe matching retirement if delivery is rejected.
    #[must_use]
    pub const fn binding(&self) -> FollowBinding {
        self.scope.binding
    }
    /// Public original admission facts, never a latest-alias lookup.
    #[must_use]
    pub fn context(&self) -> &ConnectionContext {
        &self.scope.context
    }
    /// Inspect correlation only; the peer result remains sealed until install.
    #[must_use]
    pub fn operation(&self) -> Option<Operation> {
        match &self.payload {
            Payload::Completion { operation, .. } => Some(*operation),
            Payload::Metadata(_)
            | Payload::Conversation(_)
            | Payload::Records(_)
            | Payload::Lost(_)
            | Payload::Eof => None,
        }
    }
    /// The synchronous callback must enter canonical receive/settle before data
    /// mutation. It must not await, refresh a view context or use selected globals.
    /// # Errors
    /// Refuses old delivery, metadata identity or the canonical owner's admission.
    pub fn install(
        self,
        apply: impl FnOnce(FollowBinding, Payload) -> Result<(), ResidentProblem>,
    ) -> Result<(), Problem> {
        self.scope
            .delivery
            .validate(&self.scope.context)
            .map_err(Problem::Delivery)?;
        if let Payload::Metadata(info) = &self.payload
            && info.id != self.scope.binding.resident.address.session
        {
            return Err(Problem::WrongSession);
        }
        apply(self.scope.binding, self.payload).map_err(Problem::Resident)
    }
}

/// A failed actor preflight can prove no effect; an attempted write cannot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailurePhase {
    /// Actor selection failed before work could be polled.
    NotSent,
    /// Any started call lacks no-send proof; retain input and never replay.
    Uncertain,
}
/// Control-only downgrade. Refusal never fabricates a delivery guard or success.
pub struct CorrelatedRefusal {
    context: ConnectionContext,
    binding: FollowBinding,
    operation: Option<Operation>,
    phase: FailurePhase,
    reason: Refusal,
}
impl std::fmt::Debug for CorrelatedRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CorrelatedRefusal(<opaque>)")
    }
}
impl CorrelatedRefusal {
    pub(crate) fn new(
        context: ConnectionContext,
        binding: FollowBinding,
        operation: Option<Operation>,
        phase: FailurePhase,
        reason: Refusal,
    ) -> Self {
        Self {
            context,
            binding,
            operation,
            phase,
            reason,
        }
    }
    /// Match exact original context/binding/ticket before changing pending UI.
    /// The callback cannot clear input, grant success or retire a newer binding.
    /// A follow error alone is not evidence of shared connection retirement.
    /// # Errors
    /// Canonical current-pending validation refused the obsolete downgrade.
    pub fn apply(
        self,
        apply: impl FnOnce(
            &ConnectionContext,
            FollowBinding,
            Option<Operation>,
            FailurePhase,
            Refusal,
        ) -> Result<(), ResidentProblem>,
    ) -> Result<(), ResidentProblem> {
        if self.context.node() != self.binding.resident.address.node {
            return Err(ResidentProblem::OwnerConflict);
        }
        apply(
            &self.context,
            self.binding,
            self.operation,
            self.phase,
            self.reason,
        )
    }
}
#[cfg(test)]
mod tests;
