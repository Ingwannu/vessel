//! Correlated usage operations carry source facts and sanitized errors only.
use vessel_app_view::usage::{
    Inventory, InventoryRefresh, InventorySurface, LedgerSelection, ReadRequest, RequestFailure,
    SessionSelection,
};
use vessel_app_view::{
    connection::ConnectionContext,
    devices::{MachineKey, Ticket},
};
use vessel_proto::{
    limits::{ExternalLookup, LimitRefresh, UsageLimits},
    usage::UsageReport,
};

/// Each operation carries the identity reserved before enqueueing.
#[derive(Debug, Clone)]
pub enum Ask {
    /// Configured accounts and independent cached limits; only an explicit target refreshes.
    Accounts {
        /// Independent consumer lifetime.
        surface: InventorySurface,
        /// Machine and read lifetime.
        request: ReadRequest<ConnectionContext>,
        /// Exact configured binding explicitly selected for a vendor refresh.
        refresh: InventoryRefresh,
    },
    /// Retained reported token/cost totals, independent of vendor quota.
    Ledger(ReadRequest<LedgerSelection>),
    /// Persisted credential-use policy; never automatically retried.
    Lookup {
        /// Parsed machine key.
        machine: ConnectionContext,
        /// Mutation correlation.
        ticket: Ticket,
        /// Explicit administrator choice.
        policy: ExternalLookup,
    },
    /// Accept/revoke one explicitly displayed source disclosure; never starts a fetch.
    SourceLookup {
        /// Captured destination.
        machine: ConnectionContext,
        /// Non-reused mutation correlation.
        ticket: Ticket,
        /// Typed registered source, permission and exact displayed revision.
        request: vessel_proto::usage_reports::SetUsageSourceLookup,
    },
    /// The node resolves the session's immutable launch association.
    Session {
        /// Session, machine and observed binding epoch.
        request: ReadRequest<SessionSelection>,
        /// Cached read or explicitly requested vendor refresh.
        refresh: LimitRefresh,
    },
}

/// Reply identity is preserved even when a queue or connection refuses the operation.
#[derive(Debug)]
pub enum Answer {
    /// A real fresh connection invalidates prior publishing rights, even for the same node.
    Connected {
        /// Parsed destination book identity.
        machine: MachineKey,
        /// Handshake identity, never a display name.
        node: vessel_proto::NodeId,
    },
    /// Whole configured inventory; individual accounts may have failed independently.
    Accounts {
        /// Independent consumer lifetime.
        surface: InventorySurface,
        /// Original machine and lifetime.
        request: ReadRequest<ConnectionContext>,
        /// Cache-only account outcomes, or an inventory read failure.
        result: Result<Inventory, RequestFailure>,
    },
    /// Ledger totals for exactly one rolling range.
    Ledger {
        /// Original selection and lifetime.
        request: ReadRequest<LedgerSelection>,
        /// Node accounting, or safe failure.
        result: Result<UsageReport, RequestFailure>,
    },
    /// Acknowledged policy, or an uncertain/refused mutation that must be read back.
    Lookup {
        /// Original machine.
        machine: ConnectionContext,
        /// Mutation correlation.
        ticket: Ticket,
        /// What the node acknowledged; no raw error text.
        result: Result<ExternalLookup, RequestFailure>,
    },
    /// The source owner acknowledged its persisted grant, or requires a cache readback.
    SourceLookup {
        /// Captured destination.
        machine: ConnectionContext,
        /// Original mutation identity.
        ticket: Ticket,
        /// No raw node/credential/provider text crosses the event boundary.
        result: Result<vessel_proto::usage_reports::UsageSourceDescriptor, RequestFailure>,
    },
    /// Saved account facts for one session's current launch association.
    Session {
        /// Original session scope and lifetime.
        request: ReadRequest<SessionSelection>,
        /// Factual account limits; never a diagnosis of why a turn stopped.
        result: Result<UsageLimits, RequestFailure>,
    },
}
impl Ask {
    /// All variants settle their own exact correlation when not admitted.
    #[must_use]
    pub fn refused(self, reason: RequestFailure) -> Answer {
        match self {
            Self::Accounts {
                surface, request, ..
            } => Answer::Accounts {
                surface,
                request,
                result: Err(reason),
            },
            Self::Ledger(request) => Answer::Ledger {
                request,
                result: Err(reason),
            },
            Self::Lookup {
                machine, ticket, ..
            } => Answer::Lookup {
                machine,
                ticket,
                result: Err(reason),
            },
            Self::SourceLookup {
                machine, ticket, ..
            } => Answer::SourceLookup {
                machine,
                ticket,
                result: Err(reason),
            },
            Self::Session { request, .. } => Answer::Session {
                request,
                result: Err(reason),
            },
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn connection(&self) -> &ConnectionContext {
        match self {
            Self::Accounts { request, .. } => &request.selection,
            Self::Ledger(request) => &request.selection.machine,
            Self::Lookup { machine, .. } | Self::SourceLookup { machine, .. } => machine,
            Self::Session { request, .. } => &request.selection.machine,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod calls;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use calls::Hub;

#[cfg(test)]
mod tests;
