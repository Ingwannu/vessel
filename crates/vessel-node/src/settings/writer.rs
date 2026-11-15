//! Cancellation-safe ownership of a settings write through runtime publication.
//! Caller cancellation drops a waiter, never the serialization guard or writer.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use tokio::sync::OwnedMutexGuard;
use vessel_proto::settings::{NodeSettings, SetNodeSettings};
use vessel_store::SetAside;

use super::{Settings, SettingsError};

impl Settings {
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
        let writer = self.writers.token();
        if self.writers.is_closed() {
            return Err(SettingsError::Stopping);
        }
        if next == *saved {
            return Ok(self.in_force(*saved));
        }
        if let Some(reason) = &self.kept {
            return Err(SettingsError::Kept(Arc::clone(reason)));
        }
        // Negative control: restore the old caller-owned write/guard lifetime.
        // The real blocking writer and regression fixture remain unchanged.
        let _writer = writer;
        self.write_and_apply(saved, next).await
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
        .map_err(SettingsError::Lost)?
        .map_err(SettingsError::Write)?;
        if let Some(aside) = aside {
            tracing::info!(aside = %aside.display(), "settings.json held a later build's settings; a copy was kept before this change");
            self.later.store(false, Ordering::Relaxed);
        }
        *saved = next;
        let in_force = self.in_force(*saved);
        self.runtime.set_park_after(in_force.park_after.duration());
        self.runtime.set_drivers(in_force.drivers);
        Ok(in_force)
    }

    /// After RPC callers finish, close admission and join every retained writer.
    pub(crate) async fn shutdown(&self) {
        let _first_close = self.writers.close();
        self.writers.wait().await;
    }
}
