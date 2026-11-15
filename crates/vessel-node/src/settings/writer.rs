//! Cancellation-safe ownership of a settings write through runtime publication.
//! Caller cancellation drops a waiter, never the serialization guard or writer.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use tokio::sync::OwnedMutexGuard;
use vessel_proto::settings::NodeSettings;
use vessel_store::{SavedNodeSettings, SetAside};

use super::{SettingsError, State};

/// Keep the concrete join cause without publishing a task's panic payload.
/// The public error chain ends here; this does not sanitize the panic hook.
pub(crate) struct TaskFailure(tokio::task::JoinError);

impl From<tokio::task::JoinError> for TaskFailure {
    fn from(cause: tokio::task::JoinError) -> Self {
        Self(cause)
    }
}

impl fmt::Debug for TaskFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for TaskFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_cancelled() {
            formatter.write_str("settings task was cancelled")
        } else {
            formatter.write_str("settings task panicked")
        }
    }
}

impl std::error::Error for TaskFailure {}

impl State {
    /// The consumer owns the accepted intent before this guard is acquired.
    /// Keep both across real I/O and publication, independent of the caller.
    pub(super) async fn persist(
        &self,
        saved: OwnedMutexGuard<SavedNodeSettings>,
        next: SavedNodeSettings,
    ) -> Result<NodeSettings, SettingsError> {
        if next == *saved {
            return Ok(self.in_force(saved.public_settings()));
        }
        if let Some(reason) = &self.kept {
            return Err(SettingsError::Kept(Arc::clone(reason)));
        }
        self.write_and_apply(saved, next).await
    }

    async fn write_and_apply(
        &self,
        mut saved: OwnedMutexGuard<SavedNodeSettings>,
        next: SavedNodeSettings,
    ) -> Result<NodeSettings, SettingsError> {
        let data_dir = self.data_dir.clone();
        let later = self.later.load(Ordering::Relaxed);
        #[cfg(test)]
        let gate = self.gate.take();
        let (next, aside) = tokio::task::spawn_blocking(move || {
            #[cfg(test)]
            if let Some(gate) = gate {
                gate.pause();
            }
            let aside = later
                .then(|| data_dir.set_aside_settings(chrono::Utc::now(), SetAside::Later))
                .transpose()?;
            data_dir.write_settings(&next).map(|()| (next, aside))
        })
        .await
        .map_err(TaskFailure::from)
        .map_err(SettingsError::Lost)?
        .map_err(SettingsError::Write)?;
        if let Some(aside) = aside {
            tracing::info!(aside = %aside.display(), "settings.json held a later build's settings; a copy was kept before this change");
            self.later.store(false, Ordering::Relaxed);
        }
        *saved = next;
        let in_force = self.in_force(saved.public_settings());
        self.runtime.set_park_after(in_force.park_after.duration());
        self.runtime.set_drivers(in_force.drivers);
        Ok(in_force)
    }
}
