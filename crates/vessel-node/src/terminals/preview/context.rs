//! Retain canonical connection observers, never the context lifetime guard.

use tokio_util::sync::CancellationToken;
use vessel_proto::{SessionId, WorkspaceId};

use crate::connection_life::{ConnectionLife, Owner};

/// Authenticated node code derives this query scope; UUIDs alone grant nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PreviewScope {
    pub(crate) workspace: WorkspaceId,
    pub(crate) opening_session: SessionId,
}

/// Bind later operations to the actual connection without prolonging it.
#[derive(Debug, Clone)]
pub(crate) struct PreviewContext {
    pub(super) scope: PreviewScope,
    pub(super) owner: Owner,
    pub(super) closed: CancellationToken,
    pub(super) transport_closed: CancellationToken,
}

impl PreviewContext {
    /// Real adapters also observe Outbound::closed while a call retains context.
    /// Authentication, protocol and session policy remain mandatory above this API.
    pub(crate) fn from_observers(
        scope: PreviewScope,
        life: &ConnectionLife,
        transport_closed: CancellationToken,
    ) -> Self {
        Self {
            scope,
            owner: life.owner(),
            closed: life.closed(),
            transport_closed,
        }
    }

    pub(super) fn ended(&self) -> bool {
        self.closed.is_cancelled() || self.transport_closed.is_cancelled()
    }
}
