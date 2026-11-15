//! Resolve stable session addresses on authenticated connections, never by title.
//! A lookup reads session.get only; opening history never resumes a provider.

mod access;
pub mod lifecycle;
pub use access::{Access, Binding, Read};

use vessel_app_view::Card;
use vessel_app_view::machine_book::Name as NodeName;
use vessel_app_view::sessions::{NavigationTarget, NavigationTicket, SessionAddress};
use vessel_proto::ErrorCode;

/// One explicitly requested navigation or copy-link lookup.
#[derive(Debug, Clone)]
pub struct Ask {
    /// The UI ignores answers to a superseded request.
    pub ticket: NavigationTicket,
    /// Stable address, or a validated saved-host name from a visible row.
    pub target: NavigationTarget,
}
/// Metadata fetched without mutating runtime state.
#[derive(Debug, Clone)]
pub struct Located {
    /// Authenticated destination, suitable for a credential-free link.
    pub address: SessionAddress,
    /// Current saved name used by the app's existing session follow path.
    pub host: NodeName,
    /// The node's authoritative session projection.
    pub card: Card,
}
/// Safe feedback without forwarding arbitrary transport or credential text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// The address has no connected saved host.
    Offline,
    /// A node returned a structured refusal.
    Refused(ErrorCode),
    /// The bounded metadata lookup did not complete.
    TimedOut,
    /// The local command queue did not accept the request.
    Busy,
    /// A saved host name violated the validated book contract.
    InvalidHost,
    /// A faulty peer returned another session for this exact lookup.
    WrongSession,
}
/// One result remains tied to its originating UI request.
#[derive(Debug, Clone)]
pub struct Answer {
    /// The matching pending request.
    pub ticket: NavigationTicket,
    /// Session metadata or a safe failure.
    pub result: Result<Located, Failure>,
}
impl Ask {
    /// Queue refusal must settle navigation exactly as a remote refusal does.
    #[must_use]
    pub fn refused(self, failure: Failure) -> Answer {
        Answer {
            ticket: self.ticket,
            result: Err(failure),
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
mod navigation;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use navigation::Hub;

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests;
