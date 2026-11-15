//! Readonly code acquisitions retain their original context, ticket and guard.
//! This adds no root resolver, node method, source controller or authority.
use crate::DeliveryGuard;
use vessel_app_view::{
    connection::ConnectionContext,
    workspace::{file_scope::FileScope, file_ticket::FileTicket, problem::Problem},
};
use vessel_proto::{
    SessionId, SessionInfo,
    workspace::{DiffSummary, FileDiff, GetChanges, GetFileDiff},
};
#[cfg(not(target_arch = "wasm32"))]
mod run;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use run::Hub;

/// Existing readonly calls with their owner-allocated original correlation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Resolve the explicitly selected session, never cwd or another alias.
    Root {
        /// Original admission.
        context: ConnectionContext,
        /// Original session.
        session: SessionId,
        /// Root acquisition identity.
        ticket: FileTicket,
    },
    /// Read the comparison selected by this root acquisition.
    Summary {
        /// Original root facts.
        scope: FileScope,
        /// Root acquisition identity.
        root_ticket: FileTicket,
        /// Summary identity.
        ticket: FileTicket,
        /// Existing typed request.
        ask: GetChanges,
    },
    /// Read one page belonging to this exact summary acquisition.
    Diff {
        /// Original root facts.
        scope: FileScope,
        /// Root acquisition identity.
        root_ticket: FileTicket,
        /// Parent summary identity.
        summary: FileTicket,
        /// Page identity.
        ticket: FileTicket,
        /// Existing typed request.
        ask: GetFileDiff,
    },
}
/// A typed body, never re-associated with a later selected connection.
#[derive(Debug)]
pub enum Body {
    /// Session metadata supplies the existing physical root projection.
    Root(Result<Box<SessionInfo>, Problem>),
    /// The actual summary that will be drawn.
    Summary(Result<DiffSummary, Problem>),
    /// The actual next page that will be appended.
    Diff(Result<FileDiff, Problem>),
}
/// A result's weak guard is minted only from its actual selected connection.
#[derive(Debug)]
pub struct Answer {
    /// Exact originating request, including root and monotonic ticket.
    pub request: Request,
    /// None for local refusal; never yields admitted shown data.
    pub guard: Option<DeliveryGuard>,
    /// Existing typed readonly payload or redacted failure.
    pub body: Body,
}
impl Request {
    /// Original admission, not a lookup by node or current route.
    #[must_use]
    pub const fn context(&self) -> &ConnectionContext {
        match self {
            Self::Root { context, .. } => context,
            Self::Summary { scope, .. } | Self::Diff { scope, .. } => scope.connection(),
        }
    }
    /// Refusal preserves the exact request and cannot manufacture a guard.
    #[must_use]
    pub fn refused(self, problem: Problem) -> Answer {
        let body = match &self {
            Self::Root { .. } => Body::Root(Err(problem)),
            Self::Summary { .. } => Body::Summary(Err(problem)),
            Self::Diff { .. } => Body::Diff(Err(problem)),
        };
        Answer {
            request: self,
            guard: None,
            body,
        }
    }
}
