//! Correlated draft creation over the existing session calls.
//! No new wire method is invented here. Workspace receipts replace this
//! legacy two-call path when the centrally reviewed node contract is available.

#[cfg(not(target_arch = "wasm32"))]
mod run;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use run::create as perform;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

use std::sync::Arc;

use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::creation::{Failure, Stage, Ticket};
use vessel_app_view::machine_book::Name;
use vessel_proto::{NodeId, SessionId, SessionInfo, SessionSpec};

/// A first-message retry can never accidentally register another resource.
#[derive(Debug, Clone)]
pub enum Target {
    /// Create once with the selected launch specification.
    New(SessionSpec),
    /// A committed session whose first message was explicitly refused.
    Registered(SessionId),
}

/// Files/context are delivered by their owner only after capabilities arrive.
#[derive(Clone, PartialEq, Eq)]
pub enum FirstInput {
    /// Attachment owner will deliver the full prepared payload.
    Deferred,
    /// A plain chat message, preserving the person's original whitespace.
    Text(Arc<str>),
}

impl std::fmt::Debug for FirstInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Deferred => "Deferred",
            Self::Text(_) => "Text([redacted])",
        })
    }
}

/// A validated machine/spec and one local submission's identity.
#[derive(Debug, Clone)]
pub struct Ask {
    /// The payload owner; tickets are local to this exact draft.
    pub source: DraftKey,
    /// Original actor admission, including reconnect/credential retirement.
    pub context: ConnectionContext,
    /// Local correlation, not a connection's request counter.
    pub ticket: Ticket,
    /// Canonical first-input identity retained across preparation and child retry.
    pub message: vessel_proto::timeline::ClientMessageId,
    /// Existing node-book entry, parsed once by the app.
    pub machine: Name,
    /// Pin the welcome identity, so reusing a book name cannot retarget effects.
    pub expected_node: NodeId,
    /// Existing node creation contract; placement remains node-authoritative.
    pub target: Target,
    /// First-message owner, independent of resource registration.
    pub input: FirstInput,
}

/// The created resource is announced even when its first message later fails.
#[derive(Debug, Clone)]
pub enum Answer {
    /// Creation committed; failures from here belong to this resource.
    Registered {
        /// Exact draft whose local ticket this reply settles.
        source: DraftKey,
        /// Local submission identity.
        ticket: Ticket,
        /// Node-book identity, retained even after navigation.
        machine: Name,
        /// Original admission, retained for the eventual resident handoff.
        context: ConnectionContext,
        /// Welcome identity of the client that actually registered the resource.
        node: NodeId,
        /// Exact session returned by the existing create call.
        session: Box<SessionInfo>,
    },
    /// Progress of the matching payload.
    Progress {
        /// Exact draft whose local ticket this reply settles.
        source: DraftKey,
        /// Local submission identity.
        ticket: Ticket,
        /// Side effect currently awaited.
        stage: Stage,
    },
    /// The legacy flow finished; deferred files remain their owner's concern.
    Complete(DraftKey, Ticket),
    /// A known refusal or uncertain side effect, without raw server text.
    Failed {
        /// Exact draft whose local ticket this reply settles.
        source: DraftKey,
        /// Local submission identity.
        ticket: Ticket,
        /// Last attempted side effect.
        stage: Stage,
        /// Safe typed presentation reason.
        failure: Failure,
    },
}

impl Answer {
    /// A reply cannot settle another draft with the same local ticket.
    #[must_use]
    pub const fn source(&self) -> DraftKey {
        match self {
            Self::Registered { source, .. }
            | Self::Progress { source, .. }
            | Self::Failed { source, .. }
            | Self::Complete(source, _) => *source,
        }
    }
}

impl Ask {
    /// Settle unsent/unsupported commands with the same correlation as a reply.
    #[must_use]
    pub fn refused(self, failure: Failure) -> Answer {
        let stage = match self.target {
            Target::New(_) => Stage::Resource,
            Target::Registered(_) => Stage::InitialInput,
        };
        Answer::Failed {
            source: self.source,
            ticket: self.ticket,
            stage,
            failure,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    /// A slow creation must not hold fleet refresh or commands for another node.
    pub(crate) fn create(&self, ask: Ask) {
        let client = self.selected_connection(&ask.context);
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            match client {
                Ok(client) => run::create(&client, ask, &events).await,
                Err(_changed) => {
                    let _closed = events
                        .send(crate::Event::Creation(
                            ask.refused(Failure::Refused(vessel_proto::ErrorCode::Conflict)),
                        ))
                        .await;
                }
            }
        }));
    }
}
