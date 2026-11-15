//! One definition per Preview admission bound; permits follow actual owners.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::PreviewError;

pub(super) const MAX_PREVIEW_LEASES: usize = 32;
pub(super) const MAX_PREVIEW_STREAMS: usize = 64;
pub(super) const MAX_LEASE_STREAMS: usize = 8;
pub(super) const MAX_PREVIEW_PREPARATIONS: usize = 4;
pub(super) const PREVIEW_OPERATION_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const FRESH_FOR: Duration = super::super::ports::PERIOD.saturating_mul(2);

/// Distinguish the exhausted owner without putting identifiers in diagnostics.
#[derive(Debug, Clone, Copy)]
pub(crate) enum PreviewBudget {
    Leases,
    Connections,
    LeaseConnections,
    Preparation,
}

impl PreviewBudget {
    pub(super) const fn limit(self) -> usize {
        match self {
            Self::Leases => MAX_PREVIEW_LEASES,
            Self::Connections => MAX_PREVIEW_STREAMS,
            Self::LeaseConnections => MAX_LEASE_STREAMS,
            Self::Preparation => MAX_PREVIEW_PREPARATIONS,
        }
    }
}

impl std::fmt::Display for PreviewBudget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Leases => "lease",
            Self::Connections => "connection",
            Self::LeaseConnections => "per-lease connection",
            Self::Preparation => "preparation",
        })
    }
}

pub(super) fn take(
    pool: &Arc<Semaphore>,
    budget: PreviewBudget,
) -> Result<OwnedSemaphorePermit, PreviewError> {
    Arc::clone(pool)
        .try_acquire_owned()
        .map_err(|source| PreviewError::Busy { budget, source })
}
