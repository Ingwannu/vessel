//! Typed node boundary; provider tasks never become managed session operations.

use super::{ChildRead, DetachChoice, Host, Target, TrackProblem, TrackRead};
use crate::devices::Ticket;
use vessel_proto::{
    SessionInfo, SessionRef,
    subagents::{DetachedSession, ProviderChildPage, ProviderChildren},
};

/// Every request captures the original authenticated identity and generation.
#[derive(Debug, Clone)]
pub enum SubagentAsk {
    /// Read current parentage for a single opened session.
    Lookup {
        /// Exact managed address.
        target: Target,
        /// Correlation generation.
        ticket: Ticket,
    },
    /// Hydrate one host's metadata for cross-host child selection.
    Managed {
        /// Authenticated host and its route.
        host: Host,
        /// Exact managed parent whose track requested this snapshot.
        parent: SessionRef,
        /// Correlation generation.
        ticket: Ticket,
    },
    /// List bounded provider descriptors.
    Providers(TrackRead),
    /// Read a canonical provider child page.
    Timeline(ChildRead),
    /// Explicit confirmed managed-child detach.
    Detach {
        /// Parent and child captured before confirmation.
        choice: DetachChoice,
        /// Correlation generation.
        ticket: Ticket,
    },
}
/// Replies retain request scope, never the currently painted target.
#[derive(Debug, Clone)]
pub enum SubagentReply {
    /// Authoritative current-parent metadata.
    LookedUp {
        /// Requested target.
        target: Target,
        /// Original generation.
        ticket: Ticket,
        /// Validated metadata or typed failure.
        result: Result<Box<SessionInfo>, TrackProblem>,
    },
    /// One hydrated host, separate from other hosts' success/failure.
    Managed {
        /// Requested host.
        host: Host,
        /// Original full parent address.
        parent: SessionRef,
        /// Original generation.
        ticket: Ticket,
        /// Metadata only; registry/action owners supply visibility/eligibility.
        result: Result<Vec<SessionInfo>, TrackProblem>,
    },
    /// Bounded descriptor list and explicit coverage.
    Providers {
        /// Original request.
        read: TrackRead,
        /// Facts or failure.
        result: Result<ProviderChildren, TrackProblem>,
    },
    /// Bounded canonical child page with parent cursor.
    Timeline {
        /// Original request.
        read: ChildRead,
        /// Facts or failure.
        result: Result<ProviderChildPage, TrackProblem>,
    },
    /// Original durable detach receipt, including an idempotent repeat.
    Detached {
        /// Confirmed request identities.
        choice: DetachChoice,
        /// Original generation.
        ticket: Ticket,
        /// Durable result or explicit uncertainty.
        result: Result<DetachedSession, TrackProblem>,
    },
}
impl SubagentAsk {
    /// Submission failure settles exactly the request that failed to enter the link.
    #[must_use]
    pub fn refused(self, problem: TrackProblem) -> SubagentReply {
        match self {
            Self::Lookup { target, ticket } => SubagentReply::LookedUp {
                target,
                ticket,
                result: Err(problem),
            },
            Self::Managed {
                host,
                parent,
                ticket,
            } => SubagentReply::Managed {
                host,
                parent,
                ticket,
                result: Err(problem),
            },
            Self::Providers(read) => SubagentReply::Providers {
                read,
                result: Err(problem),
            },
            Self::Timeline(read) => SubagentReply::Timeline {
                read,
                result: Err(problem),
            },
            Self::Detach { choice, ticket } => SubagentReply::Detached {
                choice,
                ticket,
                result: Err(problem),
            },
        }
    }
    /// Immutable actor-owned admission selected before the request was drawn.
    #[must_use]
    pub fn context(&self) -> &crate::connection::ConnectionContext {
        match self {
            Self::Lookup { target, .. } => &target.context,
            Self::Managed { host, .. } => &host.context,
            Self::Providers(read) => &read.target.context,
            Self::Timeline(read) => &read.query.target.context,
            Self::Detach { choice, .. } => &choice.child.context,
        }
    }
    /// Parsed machine routing key; the adapter separately verifies welcome identity.
    #[must_use]
    pub fn machine(&self) -> &crate::devices::MachineKey {
        match self {
            Self::Lookup { target, .. } => &target.machine,
            Self::Managed { host, .. } => &host.machine,
            Self::Providers(read) => &read.target.machine,
            Self::Timeline(read) => &read.query.target.machine,
            Self::Detach { choice, .. } => &choice.child.machine,
        }
    }
    /// Node identity captured before choosing/confirming the action.
    #[must_use]
    pub const fn node(&self) -> vessel_proto::NodeId {
        match self {
            Self::Lookup { target, .. } => target.address.node,
            Self::Managed { host, .. } => host.node,
            Self::Providers(read) => read.target.address.node,
            Self::Timeline(read) => read.query.target.address.node,
            Self::Detach { choice, .. } => choice.child.address.node,
        }
    }
}

impl SubagentReply {
    /// The same original per-hop admission captured by the request.
    #[must_use]
    pub fn context(&self) -> &crate::connection::ConnectionContext {
        match self {
            Self::LookedUp { target, .. } => &target.context,
            Self::Managed { host, .. } => &host.context,
            Self::Providers { read, .. } => &read.target.context,
            Self::Timeline { read, .. } => &read.query.target.context,
            Self::Detached { choice, .. } => &choice.child.context,
        }
    }
    /// A stale queued mutation receipt cannot prove a repeat safe. Read payloads
    /// are discarded before projection and retain only their original correlation.
    #[must_use]
    pub fn invalidated(self, read_problem: TrackProblem) -> Self {
        match self {
            Self::LookedUp { target, ticket, .. } => Self::LookedUp {
                target,
                ticket,
                result: Err(read_problem),
            },
            Self::Managed {
                host,
                parent,
                ticket,
                ..
            } => Self::Managed {
                host,
                parent,
                ticket,
                result: Err(read_problem),
            },
            Self::Providers { read, .. } => Self::Providers {
                read,
                result: Err(read_problem),
            },
            Self::Timeline { read, .. } => Self::Timeline {
                read,
                result: Err(read_problem),
            },
            Self::Detached { choice, ticket, .. } => Self::Detached {
                choice,
                ticket,
                result: Err(TrackProblem::Uncertain),
            },
        }
    }
}
