//! Closed source choices keep synthetic authority out of product configuration.

use std::sync::Arc;

use tokio::time::Instant;
use vessel_proc::ProcessLifetime;

use super::{MissingPreviewProof, PreviewError, PreviewOrigin, PreviewScope, budget::FRESH_FOR};

#[derive(Clone)]
pub(super) enum Source {
    Unavailable,
    #[cfg(test)]
    Fixture(super::fixtures::Fixture),
}

/// Only a trusted source can construct this binding; UUIDs are not its witness.
#[derive(Clone)]
pub(super) struct Binding {
    pub(super) scope: PreviewScope,
    pub(super) origin: PreviewOrigin,
    pub(super) run: ProcessLifetime,
    pub(super) observed: Instant,
    pub(super) authority: Arc<Authority>,
}

/// Prototype witness only; real canonical authority is an unwired dependency.
pub(super) struct Authority;

impl Binding {
    pub(super) fn validate(&self) -> Result<(), PreviewError> {
        if !self.run.is_live() {
            return Err(PreviewError::Revoked);
        }
        let now = Instant::now();
        if self.observed > now {
            return Err(PreviewError::Clock);
        }
        if now.duration_since(self.observed) >= FRESH_FOR {
            return Err(PreviewError::Expired);
        }
        Ok(())
    }

    pub(super) fn same_binding(&self, other: &Self) -> bool {
        self.scope == other.scope
            && self.origin == other.origin
            && self.run.same_run(&other.run)
            && Arc::ptr_eq(&self.authority, &other.authority)
    }
}

impl Source {
    pub(super) fn current(&self) -> Result<Binding, PreviewError> {
        match self {
            Self::Unavailable => Err(PreviewError::Unavailable(
                MissingPreviewProof::CanonicalAuthority,
            )),
            #[cfg(test)]
            Self::Fixture(fixture) => fixture.current(),
        }
    }

    pub(super) async fn changed(&mut self) {
        match self {
            Self::Unavailable => std::future::pending().await,
            #[cfg(test)]
            Self::Fixture(fixture) => fixture.changed().await,
        }
    }
}
