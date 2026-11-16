//! Directory-only marks; the original selected lease, not captured UI facts, authorizes calls.
pub use vessel_app_view::session_marks::Ask;
use vessel_proto::SessionInfo;
mod delivery;
pub use delivery::Answer;

/// Safe bounded feedback, never an arbitrary server message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// The command queue or bounded call capacity did not admit this request.
    Unsent,
    /// Exact original admission was unavailable before issuing a call.
    Changed,
    /// This peer/platform cannot make this mark; no call was issued.
    Unsupported,
    /// A dispatched update may have completed; refresh, never replay it.
    Uncertain,
    /// The peer answered with another session or failed the requested postcondition.
    InvalidReply,
    /// A dispatched call refused; its code is safe but does not prove no effect.
    Refused(vessel_proto::ErrorCode),
}
impl Failure {
    /// Only definite local pre-effect rejection is a no-send observation.
    #[must_use]
    pub const fn may_have_completed(self) -> bool {
        matches!(
            self,
            Self::Uncertain | Self::InvalidReply | Self::Refused(_)
        )
    }
}
/// Payload exposed only after the answer checked its original weak delivery guard.
#[derive(Debug)]
pub struct Reply {
    ask: Ask,
    result: Result<Box<SessionInfo>, Failure>,
}
impl Reply {
    /// Inspect local UI correlation; never a resident operation ticket.
    #[must_use]
    pub const fn ask(&self) -> &Ask {
        &self.ask
    }
    /// Apply immediately, without await or a latest-alias lookup.
    #[must_use]
    pub fn into_parts(self) -> (Ask, Result<Box<SessionInfo>, Failure>) {
        (self.ask, self.result)
    }
}

/// The browser has no authenticated transport, so it cannot issue a mark.
#[cfg(target_arch = "wasm32")]
pub(crate) fn unsupported(ask: Ask) -> crate::Event {
    crate::Event::SessionMarked(Answer::refused(ask, Failure::Unsupported))
}

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::Hub;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
