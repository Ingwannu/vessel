//! Fresh secret lookup through the fleet owner, with bounded blocking work.
//!
//! Cancellation of a waiter cannot release a spawned read's permit or stream
//! lease. Presence shares this gate without pretending to be a recording.

use super::quota::{Lease, NODE_STREAMS};
use crate::fleet::{FleetFiles, SecretsError};
use std::sync::Arc;
use tokio::sync::{AcquireError, Semaphore};
use tokio_util::sync::CancellationToken;
use vessel_proto::{ResolvedValue, SecretName, dictation::KeyPresence};

/// Settings and provider admission retain causes behind their fixed public boundaries.
#[derive(thiserror::Error)]
pub(crate) enum ReadError {
    #[error("the dictation preparation stopped before its secret read began")]
    Cancelled,
    #[error("the dictation secret read gate is closed")]
    Closed(#[source] AcquireError),
    #[error("the dictation secret store could not be read")]
    Store(#[source] SecretsError),
    #[error("the dictation secret worker did not finish")]
    Worker(#[source] tokio::task::JoinError),
    #[cfg(test)]
    #[error("the synthetic read gate could not be entered")]
    HookPoison,
    #[cfg(test)]
    #[error("the synthetic read gate did not finish")]
    Hook(#[source] std::sync::mpsc::RecvTimeoutError),
}

/// A retiring stream cancels admission, but never abandons already-started I/O.
pub(super) enum Admission<'a> {
    Presence,
    Preparing {
        lease: Arc<Lease>,
        stopped: &'a CancellationToken,
    },
}
impl std::fmt::Debug for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

/// Share the existing fleet reader and gate across bounded preparation callers.
#[derive(Clone)]
pub(crate) struct Reader {
    files: FleetFiles,
    slots: Arc<Semaphore>,
    #[cfg(test)]
    hook: Option<Arc<tests::Hook>>,
}
impl Reader {
    /// Reuse fleet storage ownership without a second format or credential cache.
    pub(crate) fn new(files: FleetFiles) -> Self {
        Self {
            files,
            slots: Arc::new(Semaphore::new(NODE_STREAMS)),
            #[cfg(test)]
            hook: None,
        }
    }
    /// Real blocking work retains its permits even when an async waiter is dropped.
    pub(super) async fn resolve(
        &self,
        name: &SecretName,
        admission: Admission<'_>,
    ) -> Result<Option<ResolvedValue>, ReadError> {
        let acquiring = Arc::clone(&self.slots).acquire_owned();
        let permit = match &admission {
            Admission::Presence => acquiring.await,
            Admission::Preparing { stopped, .. } => tokio::select! {
                biased;
                () = stopped.cancelled() => return Err(ReadError::Cancelled),
                permit = acquiring => permit,
            },
        }
        .map_err(ReadError::Closed)?;
        let lease = match admission {
            Admission::Presence => None,
            Admission::Preparing { lease, .. } => Some(lease),
        };
        let files = self.files.clone();
        let name = name.clone();
        #[cfg(test)]
        let hook = self.hook.clone();
        tokio::task::spawn_blocking(move || {
            let _read_permit = permit;
            let _stream_lease = lease;
            #[cfg(test)]
            if let Some(hook) = hook {
                hook.wait()?;
            }
            files.resolved_secret(&name).map_err(ReadError::Store)
        })
        .await
        .map_err(ReadError::Worker)?
    }
    /// An absent reference avoids disk work; supported settings share start capacity.
    pub(crate) async fn presence(
        &self,
        name: Option<&SecretName>,
    ) -> Result<KeyPresence, ReadError> {
        let Some(name) = name else {
            return Ok(KeyPresence::Absent);
        };
        Ok(match self.resolve(name, Admission::Presence).await? {
            Some(value) if !value.expose().trim().is_empty() => KeyPresence::Present,
            Some(_) | None => KeyPresence::Absent,
        })
    }
}

#[cfg(test)]
mod tests;
