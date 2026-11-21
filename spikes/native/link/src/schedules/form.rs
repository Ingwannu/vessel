//! Typed editor answers live on both targets. Native Link sets a delivery
//! guard after dispatch, and the receiving form checks the original ticket.
use crate::DeliveryGuard;
use vessel_app_view::schedules::{
    editor::io::{Request, Value},
    load::Failure,
};

/// The form request and its terminal result.
#[derive(Debug)]
pub struct Answer {
    /// Exact original request associated with the result.
    pub(super) request: Request,
    /// Result after the RPC and shape-specific decoding.
    pub(super) result: Result<Value, Failure>,
    /// Only a dispatched native result has an admission to recheck.
    pub(super) guard: Option<DeliveryGuard>,
}

impl Answer {
    /// A command rejected before a call can issue has a known outcome.
    #[must_use]
    pub const fn refused(request: Request, reason: Failure) -> Self {
        Self {
            request,
            result: Err(reason),
            guard: None,
        }
    }

    /// Recheck the admitted connection when a queued reply reaches the window.
    #[must_use]
    pub fn settle(self) -> (Request, Result<Value, Failure>) {
        let result = match self.guard {
            Some(guard) if guard.validate(&self.request.connection).is_err() => {
                Err(self.request.ask.interrupted(Failure::ConnectionChanged))
            }
            Some(_) | None => self.result,
        };
        (self.request, result)
    }
}
