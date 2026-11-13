//! Side calls and a second, independently owned session stream. Request
//! identities include the machine and parent; late answers cannot change
//! another panel. Nothing here replaces the main session's stream.
//!
//! [Decision Log]
//! - Purpose: GOAT's SideChatSurface opens and resets its own conversation
//!   while the original keeps running; read-only clients may discover it.
//! - Constraints: the link's main task must keep refreshing the fleet;
//!   untrusted error strings must not reach UI text or logs.
//! - Choice: typed asks and safe failures, bounded background calls, one
//!   owned side stream. Closing only drops that subscription.
//! - Cost: a timed-out write may have happened. No question is retried
//!   automatically; the person checks the conversation before sending again.

pub use vessel_app_view::side_question::Failure;
use vessel_app_view::side_question::{Target, Ticket};
use vessel_proto::automation::PromptText;
use vessel_proto::{EventRecord, EventSeq, SessionId, SessionInfo};

#[cfg(not(target_arch = "wasm32"))]
mod call;
#[cfg(not(target_arch = "wasm32"))]
mod follow;

/// One operation in a panel scoped to an original conversation.
#[derive(Debug)]
pub enum Ask {
    /// Read the parent and discover its newest unarchived side; no writes.
    Discover,
    /// Open/reuse through the node's serialized creation path.
    Open,
    /// Replace only the side that the person saw.
    Reset(SessionId),
    /// One Start-only question, with the node's refreshed parent context.
    Prompt(SessionId, PromptText),
    /// Explicitly resume an ended side without sending a question.
    Resume(SessionId),
    /// Stop this side's turn, leaving the original running.
    Stop(SessionId),
}

/// The operation kind retained when a call times out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Read-only discovery.
    Discover,
    /// Open or reuse.
    Open,
    /// Serialized expected-id reset.
    Reset,
    /// A question, never retried automatically.
    Prompt,
    /// Explicit resume.
    Resume,
    /// Cancel this side's turn.
    Stop,
}

impl Ask {
    /// Which operation this is, without retaining or cloning its question.
    #[must_use]
    pub const fn kind(&self) -> Kind {
        match self {
            Self::Discover => Kind::Discover,
            Self::Open => Kind::Open,
            Self::Reset(_) => Kind::Reset,
            Self::Prompt(..) => Kind::Prompt,
            Self::Resume(_) => Kind::Resume,
            Self::Stop(_) => Kind::Stop,
        }
    }
}

/// A read-only discovery keeps the original's complete metadata available.
#[derive(Debug)]
pub struct Discovered {
    /// The original conversation.
    pub parent: Box<SessionInfo>,
    /// None means a writer must open one; an ended side is still returned.
    pub side: Option<Box<SessionInfo>>,
}

/// The successful result of an operation.
#[derive(Debug)]
pub enum Outcome {
    /// Read-only discovery.
    Discovered(Discovered),
    /// Open, reset or explicit resume returned this side.
    Session(Box<SessionInfo>),
    /// The question or cancel was accepted.
    Done,
}

/// An answer is scoped to the exact request that is still on screen.
#[derive(Debug)]
pub struct Answer {
    /// Validated machine name and node-local parent.
    pub target: Target,
    /// Request identity, not a session id.
    pub ticket: Ticket,
    /// The action whose adjacent notice should receive a failure.
    pub kind: Kind,
    /// Safe result; no raw peer error crosses this boundary.
    pub result: Result<Outcome, Failure>,
}

/// The bounded link command carries its complete target and identity.
#[derive(Debug)]
pub enum Request {
    /// A single operation; its reply cannot be applied to another request.
    Call {
        target: Target,
        ticket: Ticket,
        ask: Ask,
    },
    /// Replace only this area's subscription, leaving the main one intact.
    Watch {
        target: Target,
        ticket: Ticket,
        side: SessionId,
        from: EventSeq,
    },
    /// Close only the subscription with this identity.
    Close(Ticket),
}

/// One side-stream item, scoped separately from the main conversation.
#[derive(Debug)]
pub enum Notice {
    /// One bounded operation completed.
    Answer(Answer),
    /// A record from the independently followed side.
    Record {
        target: Target,
        ticket: Ticket,
        side: SessionId,
        record: Box<EventRecord>,
    },
    /// Retention pruned an earlier range; following continues at its end.
    Pruned {
        target: Target,
        ticket: Ticket,
        side: SessionId,
        expected: EventSeq,
        resumed_at: EventSeq,
    },
    /// The stream ended normally; resuming remains explicit.
    Ended {
        target: Target,
        ticket: Ticket,
        side: SessionId,
    },
    /// The stream stopped; the person can retry reading from its cursor.
    Lost {
        target: Target,
        ticket: Ticket,
        side: SessionId,
        why: Failure,
    },
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use follow::Hub;

impl crate::Link {
    /// Enqueue an operation or stream change, settling a full-queue failure
    /// immediately so the panel never remains stuck waiting for a reply.
    ///
    /// # Errors
    /// No queue room, a closed link, or the browser's unsupported demo.
    pub fn request_side_question(&self, request: Request) -> Result<(), Failure> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.commands
                .try_send(crate::Command::SideQuestion(request))
                .map_err(|error| match error {
                    tokio::sync::mpsc::error::TrySendError::Full(_) => Failure::Busy,
                    tokio::sync::mpsc::error::TrySendError::Closed(_) => Failure::Offline,
                })
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _request = request;
            Err(Failure::Refused(vessel_proto::ErrorCode::Unsupported))
        }
    }
}
