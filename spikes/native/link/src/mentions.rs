//! Mention lookups reuse existing file.find plus file.list for one parent.
//! ADR 0083 owns recursive directory suggestions; this adapter does not
//! invent that pending call. At most two current composer lookups run.

use vessel_app_view::composer::mentions::{Failure, Request, Suggestion};

/// A correlated reply carries its full destination rather than today's route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// The immutable request which produced this result.
    pub request: Request,
    /// Empty success stays distinct from any refusal.
    pub result: Result<Vec<Suggestion>, Failure>,
}

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::Hub;

/// Resolve an existing session's authoritative project/worktree once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootAsk {
    /// UI lookup generation, independent of a transport request id.
    pub ticket: vessel_app_view::composer::mentions::Ticket,
    /// Validated node book key.
    pub machine: vessel_app_view::machine_book::Name,
    /// The conversation whose creation info owns its workspace.
    pub session: vessel_proto::SessionId,
}
/// A folder outside registered roots has no suggestions, not a fabricated root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootAnswer {
    /// Captured destination, checked before installing the result.
    pub ask: RootAsk,
    /// The existing session.get projection or a safe refusal.
    pub result: Result<Option<vessel_proto::workspace::WorkspaceRoot>, Failure>,
}
