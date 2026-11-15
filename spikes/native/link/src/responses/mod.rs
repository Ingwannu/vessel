//! Answers bypass offline queues and retain their exact request/ticket identity.
//! Rich ADR 0084 responses extend this port; unsupported edits never downgrade.

use vessel_app_view::responses::{Pending, Problem, Request};
use vessel_proto::{PermissionId, PermissionOptionId, QuestionAnswer, QuestionId};

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::Hub;

/// Existing typed answer paths, without unrelated prompt-message fallbacks.
#[derive(Clone)]
pub enum Response {
    /// Preserve the provider's exact selected option id.
    Permission {
        /// The exact waiting permission, never a session-wide default.
        id: PermissionId,
        /// An option actually offered by that request's provider.
        option: PermissionOptionId,
    },
    /// Preserve the typed form and its request id.
    Question {
        /// The exact form whose typed values are being returned.
        id: QuestionId,
        /// Validated field values, or the person's explicit decline.
        answer: QuestionAnswer,
    },
}

// Answers may contain credentials or private prose. Keep useful correlation in
// actor diagnostics without printing their payload, even via Command::Debug.
impl std::fmt::Debug for Response {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Response")
            .field("request", &self.request())
            .finish_non_exhaustive()
    }
}

impl Response {
    /// Which request the existing wire input resolves.
    #[must_use]
    pub const fn request(&self) -> Request {
        match self {
            Self::Permission { id, .. } => Request::Permission(*id),
            Self::Question { id, .. } => Request::Question(*id),
        }
    }
}

/// A checked correspondence between admission and the answer to send.
#[derive(Debug, Clone)]
pub struct Ask {
    pending: Pending,
    response: Response,
}

impl Ask {
    /// Validate the complete response identity before selecting a connection.
    /// # Errors
    /// Mismatched question/permission identities perform no work.
    pub fn new(pending: Pending, response: Response) -> Result<Self, Problem> {
        if pending.request != response.request() {
            return Err(Problem::Invalid);
        }
        Ok(Self { pending, response })
    }

    /// The selected connection key, validated at the book edge.
    #[must_use]
    pub fn machine(&self) -> &str {
        self.pending.target.connection.machine().as_str()
    }

    /// Local or preflight refusal retains the exact pending identity.
    #[must_use]
    pub fn refused(self, problem: Problem) -> Answer {
        Answer {
            pending: self.pending,
            result: Err(problem),
        }
    }
}

/// The caller may settle only the exact request this response names.
#[derive(Debug, Clone)]
pub struct Answer {
    /// Original target, request and monotonic ticket.
    pub pending: Pending,
    /// Confirmed response or explicit safe refusal/uncertainty.
    pub result: Result<(), Problem>,
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(crate) async fn respond(&self, ask: Ask) {
        let selected = self.selected_connection(&ask.pending.target.connection);
        self.responses.ask(selected, ask, self.events.clone()).await;
    }
}

#[cfg(test)]
mod tests;
