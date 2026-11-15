//! The existing settings mutation lock survives caller cancellation.
//!
//! [Decision Log]
//! - Purpose: a detached blocking writer must not lose serialization on RPC drop.
//! - Constraint: one admitted mutation owns capacity through persistence/apply.
//! - Choice: acquire the same owned guard before a tracked task; only its waiter
//!   has a deadline. Shutdown tracks that task through the existing node owner.
//! - Why: spawning before admission would accumulate work; rollback after an
//!   uncertain registration stop would lie about already committed consent.
//! - Cost: a stalled filesystem holds the sole permit until its worker ends.
use super::{Settings, SettingsError, offered};
use std::sync::{Arc, atomic::Ordering};
use std::time::Duration;
use tokio::sync::{OwnedMutexGuard, oneshot};
use tokio::time::Instant;
use vessel_proto::parking::ParkAfter;
use vessel_proto::relay_settings::RelaySettingsError;
use vessel_proto::settings::{AgentDriver, NodeSettings, SetAgentDrivers, SetNodeSettings};
use vessel_store::SetAside;

/// Measured after admission, never after releasing unfinished work.
pub(crate) const RELAY_CHANGE_DEADLINE: Duration = Duration::from_secs(10);

impl Settings {
    /// Validation and admission precede spawning; cancellation owns no new slot.
    pub(crate) async fn change(
        self: &Arc<Self>,
        change: SetNodeSettings,
    ) -> Result<NodeSettings, SettingsError> {
        validate(change)?;
        let saved = if change.relay.is_some() {
            Arc::clone(&self.saved)
                .try_lock_owned()
                .map_err(|_busy| RelaySettingsError::Busy)?
        } else {
            Arc::clone(&self.saved).lock_owned().await
        };
        let deadline = Instant::now()
            .checked_add(RELAY_CHANGE_DEADLINE)
            .ok_or(RelaySettingsError::Uncertain)?;
        let (send, receive) = oneshot::channel();
        let settings = Arc::clone(self);
        drop(self.mutations.spawn(async move {
            let result = settings.commit(change, saved, deadline).await;
            let _caller_present = send.send(result);
        }));
        if change.relay.is_some() {
            tokio::time::timeout_at(deadline, receive)
                .await
                .map_err(|_deadline| RelaySettingsError::Uncertain)?
                .map_err(|_lost| RelaySettingsError::Uncertain)?
        } else {
            receive
                .await
                .map_err(|_lost| RelaySettingsError::Uncertain)?
        }
    }

    async fn commit(
        &self,
        change: SetNodeSettings,
        mut saved: OwnedMutexGuard<SetNodeSettings>,
        deadline: Instant,
    ) -> Result<NodeSettings, SettingsError> {
        let relays = self.relays.upgrade().ok_or(RelaySettingsError::Stopping)?;
        if let Some(choice) = change.relay {
            relays.validate_change(choice)?;
        }
        let mut next = *saved;
        if let Some(after) = change.park_after {
            next.park_after = Some(after);
        }
        if let Some(drivers) = change.drivers {
            next.drivers = Some(drivers.over(next.drivers.unwrap_or_default()));
        }
        if let Some(choice) = change.relay {
            next.relay = Some(choice);
        }
        if next != *saved {
            self.persist(next).await?;
            *saved = next;
            let in_force = self.in_force(next);
            self.runtime.set_park_after(in_force.park_after.duration());
            self.runtime.set_drivers(in_force.drivers);
        }
        // No-op disable may still be waiting on the same retained generation.
        if let Some(choice) = change.relay {
            relays.apply_committed(choice, deadline).await?;
        }
        Ok(self.in_force(*saved))
    }

    async fn persist(&self, next: SetNodeSettings) -> Result<(), SettingsError> {
        if let Some(reason) = &self.kept {
            return Err(SettingsError::Kept(Arc::clone(reason)));
        }
        let data_dir = self.data_dir.clone();
        let later = self.later.load(Ordering::Relaxed);
        let aside = tokio::task::spawn_blocking(move || {
            let aside = later
                .then(|| data_dir.set_aside_settings(chrono::Utc::now(), SetAside::Later))
                .transpose()?;
            data_dir.write_settings(&next).map(|()| aside)
        })
        .await
        .map_err(SettingsError::Lost)?
        .map_err(SettingsError::Write)?;
        if aside.is_some() {
            // The new boundary never logs a path possibly carrying private data.
            tracing::info!("a copy of later-build settings was kept before this change");
            self.later.store(false, Ordering::Relaxed);
        }
        Ok(())
    }
}

fn validate(change: SetNodeSettings) -> Result<(), SettingsError> {
    if change.park_after == Some(ParkAfter::Unknown) {
        return Err(SettingsError::Unknown);
    }
    for (kind, driver) in change.drivers.iter().flat_map(SetAgentDrivers::named) {
        match driver {
            AgentDriver::Unknown => return Err(SettingsError::UnknownDriver(kind)),
            AgentDriver::Direct if !offered(kind) => return Err(SettingsError::NoDirect(kind)),
            AgentDriver::Direct | AgentDriver::Acp => {}
        }
    }
    if change.relay == Some(vessel_proto::relay_settings::RelayChange::Unknown) {
        return Err(RelaySettingsError::Unknown.into());
    }
    Ok(())
}
