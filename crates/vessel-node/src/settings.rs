//! The node's settings a client reads and changes (protocol 12,
//! `node.settings` and `node.set_settings`, ADR 0038): for now how long an
//! idle agent runs before it is parked.
//!
//! Owns: what is in force (the flags' values with what a person saved laid
//! over them), keeping a change (`settings.json`) and applying it to the
//! session runtime. Must not: decide who may change it (the guard: admin).
//!
//! [Decision Log]
//! - Purpose: parking had only `vesseld --park-after`; Paseo's host
//!   settings card turns idle cleanup on and off, kept by the daemon and
//!   applied at once.
//! - Constraint: a service started with the flag would undo a change made
//!   from the app at every restart if the flag won.
//! - Choice: what a person saved wins over the flag, field by field; a
//!   setting nobody changed keeps following its flag. A change is written
//!   first and applied after, under one lock, so two changes at once
//!   leave the file and the sessions agreeing, and a failed write changes
//!   nothing. A change that changes nothing writes nothing.
//! - Cost: a saved value silently outranks a flag an operator gives later;
//!   the node logs it at start.

use std::sync::Arc;

use tokio::sync::Mutex;
use vessel_proto::parking::ParkAfter;
use vessel_proto::settings::{NodeSettings, SetNodeSettings};
use vessel_session::SessionRuntime;
use vessel_store::DataDir;

/// Why a change was refused or not kept.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SettingsError {
    /// A choice this node does not know.
    #[error("this node does not know that park_after; send never or idle with minutes")]
    Unknown,
    /// The file could not be written; nothing changed.
    #[error("the settings could not be kept: {0}")]
    Write(#[source] vessel_store::StoreError),
    /// The write's task did not finish.
    #[error("the settings could not be kept: {0}")]
    Lost(#[source] tokio::task::JoinError),
}

/// The node's settings: the flags' values and what a person saved.
#[derive(Debug)]
pub(crate) struct Settings {
    data_dir: DataDir,
    runtime: Arc<SessionRuntime>,
    /// `--park-after`, or its default.
    flag: ParkAfter,
    /// What a person changed, as kept; held over write and apply.
    saved: Mutex<SetNodeSettings>,
}

impl Settings {
    /// Read `settings.json` and apply what it holds to `runtime`. A file
    /// that cannot be used is logged, and the flags' values stand.
    pub(crate) fn open(
        data_dir: DataDir,
        runtime: Arc<SessionRuntime>,
        flag: ParkAfter,
        flag_given: bool,
    ) -> Self {
        let saved = match data_dir.read_settings() {
            Ok(saved) => saved.unwrap_or_default(),
            Err(error) => {
                tracing::warn!(%error, "settings.json was not used; the node starts from its flags");
                SetNodeSettings::default()
            }
        };
        let settings = Self {
            data_dir,
            runtime,
            flag,
            saved: Mutex::new(saved),
        };
        let in_force = settings.in_force(saved);
        if flag_given && saved.park_after.is_some() && in_force.park_after != flag {
            tracing::info!(
                saved = ?in_force.park_after,
                "--park-after is overridden by the value saved with node.set_settings"
            );
        }
        settings
            .runtime
            .set_park_after(in_force.park_after.duration());
        settings
    }

    /// What is in force for `saved`: a saved value this node knows, else
    /// the flag's.
    fn in_force(&self, saved: SetNodeSettings) -> NodeSettings {
        let park_after = match saved.park_after {
            Some(after @ (ParkAfter::Never | ParkAfter::Idle { .. })) => after,
            // A newer build's choice, read after a downgrade: the flag's.
            Some(ParkAfter::Unknown) | None => self.flag,
        };
        NodeSettings { park_after }
    }

    /// The settings in force.
    pub(crate) async fn current(&self) -> NodeSettings {
        let saved = *self.saved.lock().await;
        self.in_force(saved)
    }

    /// Change what `change` names, keep it, then apply it; answers what is
    /// in force.
    pub(crate) async fn change(
        &self,
        change: SetNodeSettings,
    ) -> Result<NodeSettings, SettingsError> {
        if change.park_after == Some(ParkAfter::Unknown) {
            return Err(SettingsError::Unknown);
        }
        let mut saved = self.saved.lock().await;
        let mut next = *saved;
        if let Some(after) = change.park_after {
            next.park_after = Some(after);
        }
        if next == *saved {
            return Ok(self.in_force(*saved));
        }
        let data_dir = self.data_dir.clone();
        tokio::task::spawn_blocking(move || data_dir.write_settings(&next))
            .await
            .map_err(SettingsError::Lost)?
            .map_err(SettingsError::Write)?;
        *saved = next;
        let in_force = self.in_force(*saved);
        self.runtime.set_park_after(in_force.park_after.duration());
        Ok(in_force)
    }
}
