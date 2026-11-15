//! Cancellation-safe ownership of settings mutations through publication.
//! File writes and explicit reload share admission, guard lifetime and shutdown.

use std::fmt;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use tokio::sync::OwnedMutexGuard;
use tokio_util::task::task_tracker::TaskTrackerToken;
use vessel_proto::settings::{NodeSettings, SetNodeSettings};
use vessel_store::SetAside;

use super::{Settings, SettingsError};

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

/// Admission is counted before close can decide that no mutation remains.
#[derive(Debug)]
pub(super) struct Mutation(TaskTrackerToken);

impl Mutation {
    /// Keep the actual serialization guard with either asynchronous operation.
    ///
    /// # Errors
    /// Preserve the operation's error or the independent task's join failure.
    pub(super) async fn run<T, F, Operation>(
        self,
        saved: OwnedMutexGuard<SetNodeSettings>,
        operation: Operation,
    ) -> Result<T, SettingsError>
    where
        T: Send + 'static,
        F: Future<Output = Result<T, SettingsError>> + Send + 'static,
        Operation: FnOnce(OwnedMutexGuard<SetNodeSettings>) -> F + Send + 'static,
    {
        tokio::spawn(async move {
            let _mutation = self.0;
            let result = operation(saved).await;
            if let Err(error) = &result {
                tracing::warn!(%error, "node settings writer failed");
            }
            result
        })
        .await
        .map_err(|error| SettingsError::Lost(error.into()))?
    }
}

impl Settings {
    /// A token precedes the closed check, so drain cannot miss admitted work.
    ///
    /// # Errors
    /// Once closed, no new file read/write or runtime publication is admitted.
    pub(super) fn admit(&self) -> Result<Mutation, SettingsError> {
        let mutation = Mutation(self.writers.token());
        if self.writers.is_closed() {
            return Err(SettingsError::Stopping);
        }
        Ok(mutation)
    }

    /// Keep the already-acquired guard through I/O and application, even when
    /// the caller stops awaiting the result.
    ///
    /// # Errors
    /// Closed admission, preservation/write failures and task failure refuse.
    /// A tracker token precedes the admission check, so a concurrent close
    /// either sees this writer or causes refusal before I/O starts.
    pub(super) async fn persist(
        self: &Arc<Self>,
        saved: OwnedMutexGuard<SetNodeSettings>,
        next: SetNodeSettings,
    ) -> Result<NodeSettings, SettingsError> {
        let mutation = self.admit()?;
        if next == *saved {
            return Ok(self.in_force(*saved));
        }
        if let Some(reason) = &self.kept {
            return Err(SettingsError::Kept(Arc::clone(reason)));
        }
        let settings = Arc::clone(self);
        // Dropping this waiter never drops the independently owned guard.
        mutation
            .run(saved, move |saved| async move {
                settings.write_and_apply(saved, next).await
            })
            .await
    }

    async fn write_and_apply(
        &self,
        mut saved: OwnedMutexGuard<SetNodeSettings>,
        next: SetNodeSettings,
    ) -> Result<NodeSettings, SettingsError> {
        let data_dir = self.data_dir.clone();
        let later = self.later.load(Ordering::Relaxed);
        #[cfg(test)]
        let gate = self.gate.take();
        let aside = tokio::task::spawn_blocking(move || {
            #[cfg(test)]
            if let Some(gate) = gate {
                gate.pause();
            }
            let aside = later
                .then(|| data_dir.set_aside_settings(chrono::Utc::now(), SetAside::Later))
                .transpose()?;
            data_dir.write_settings(&next).map(|()| aside)
        })
        .await
        .map_err(|error| SettingsError::Lost(error.into()))?
        .map_err(SettingsError::Write)?;
        if let Some(aside) = aside {
            tracing::info!(aside = %aside.display(), "settings.json held a later build's settings; a copy was kept before this change");
            self.later.store(false, Ordering::Relaxed);
        }
        *saved = next;
        let in_force = self.in_force(*saved);
        self.apply(&in_force);
        Ok(in_force)
    }

    /// After RPC callers finish, close admission and join every retained mutation.
    pub(crate) async fn shutdown(&self) {
        let _first_close = self.writers.close();
        self.writers.wait().await;
    }
}
