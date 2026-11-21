//! Schedule replies retain the originating admission until the app applies
//! them. A queued answer cannot restore stale rows after a node reconnects.
use crate::DeliveryGuard;
use vessel_app_view::schedules::{
    load::Failure,
    page::{Request, Snapshot, WriteValue},
};

#[cfg(not(target_arch = "wasm32"))]
mod calls;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use calls::Hub;

/// List facts or a confirmed write, detached from the transport.
#[derive(Debug)]
pub enum Value {
    /// List and independently fallible metadata.
    Listed(Box<Snapshot>),
    /// The node's typed mutation receipt.
    Written(WriteValue),
}

/// Reply including the physical connection and ticket of its request.
#[derive(Debug)]
pub struct Answer {
    request: Request,
    result: Result<Value, Failure>,
    guard: Option<DeliveryGuard>,
}

impl Answer {
    /// Settle a command that did not reach the node, with its original ticket.
    #[must_use]
    pub const fn refused(request: Request, failure: Failure) -> Self {
        Self {
            request,
            result: Err(failure),
            guard: None,
        }
    }

    /// Validate queued replies immediately before the app applies them.
    pub fn settle(self) -> (Request, Result<Value, Failure>) {
        let result = match self.guard {
            Some(guard) if guard.validate(self.request.connection()).is_err() => {
                Err(interrupted(self.request.ask, Failure::ConnectionChanged))
            }
            Some(_) | None => self.result,
        };
        (self.request, result)
    }
}

const fn interrupted(ask: vessel_app_view::schedules::page::Ask, read: Failure) -> Failure {
    use vessel_app_view::schedules::page::Ask;
    match ask {
        Ask::List => read,
        Ask::Pause(_) | Ask::Resume(_) | Ask::Run(_) | Ask::Delete(_) => Failure::Unconfirmed,
    }
}
