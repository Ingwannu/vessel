//! Staged Preview admission owns connection flow, never invents kernel proof.
//! The real backend refuses until canonical registry, run, peer and scheme
//! producers are wired and qualified with the browser adapter's package gate.

mod budget;
mod context;
mod error;
#[cfg(test)]
mod fixtures;
mod lease;
mod origin;
mod source;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio_util::{sync::CancellationToken, task::TaskTracker};

pub(crate) use budget::PreviewBudget;
pub(crate) use context::{PreviewContext, PreviewScope};
pub(crate) use error::{MissingPreviewProof, PreviewError};
pub(crate) use lease::{PreviewDrain, PreviewLease};
pub(crate) use origin::{PreviewOrigin, PreviewScheme};

use budget::MAX_PREVIEW_LEASES;
use source::Source;

/// One node service shares the lease budget; connections are a later stage.
#[derive(Clone)]
pub(crate) struct PreviewService {
    source: Source,
    runtime: Arc<Runtime>,
}

struct Runtime {
    shutdown: CancellationToken,
    tasks: TaskTracker,
    leases: Arc<Semaphore>,
}

impl PreviewService {
    /// Missing canonical producers are a refusal, never a network fallback.
    pub(crate) fn unavailable(shutdown: CancellationToken, tasks: TaskTracker) -> Self {
        Self::new(Source::Unavailable, shutdown, tasks)
    }

    fn new(source: Source, shutdown: CancellationToken, tasks: TaskTracker) -> Self {
        Self {
            source,
            runtime: Arc::new(Runtime {
                shutdown,
                tasks,
                leases: Arc::new(Semaphore::new(MAX_PREVIEW_LEASES)),
            }),
        }
    }

    /// Authorization above this boundary must derive the exact trusted context.
    pub(crate) async fn admit(
        &self,
        context: PreviewContext,
        origin: PreviewOrigin,
    ) -> Result<PreviewLease, PreviewError> {
        if self.runtime.shutdown.is_cancelled() || context.ended() {
            return Err(PreviewError::Revoked);
        }
        let binding = self.source.current()?;
        binding.validate()?;
        if binding.scope != context.scope {
            return Err(PreviewError::Scope);
        }
        if binding.origin != origin {
            return Err(PreviewError::Origin);
        }
        PreviewLease::new(self, context, binding)
    }
}

impl std::fmt::Debug for PreviewService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewService").finish_non_exhaustive()
    }
}
