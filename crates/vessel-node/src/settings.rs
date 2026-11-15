//! The node's settings a client reads and changes (protocol 12,
//! `node.settings` and `node.set_settings`, ADR 0038): how long an idle
//! agent runs before it is parked, and (protocol 18, ADR 0057) which
//! protocol drives each agent that has one of its own.
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
//! - Drivers (protocol 18) have no flag: ACP unless a person chose an
//!   agent's own protocol. `direct` is refused for an agent whose driver
//!   this build does not offer (`vessel_harness::Direct::selectable`). One
//!   saved by a newer build runs as ACP and stays in the file, so a
//!   downgrade then an upgrade keeps the person's choice; a value this
//!   build cannot read is written back as `unknown` if another driver
//!   changes meanwhile (`serde(other)` keeps no text).
//! - An unusable `settings.json` (2026-11-14): it is moved aside before
//!   anything can write (`settings.json.unreadable-<UTC time>`, see
//!   vessel-store's `settings_file`), and the node starts from its flags.
//!   It used to stay in place and be replaced by the next change, losing
//!   every saved setting. If the move fails too, the file is left alone
//!   and every change that would write is refused with the reason
//!   (`SettingsError::Kept`): that one case fails rather than destroys.
//!   Refusing every change whenever the file was damaged was rejected: a
//!   node could then change nothing from the app until a person repaired
//!   a file they may not know exists.
//! - A file holding what only a later build knows (a field at any depth,
//!   or a value read here as `unknown`), found at start by comparing the
//!   file with what this build parsed: the first change that writes copies
//!   it aside first (`settings.json.later-<UTC time>`), so a downgrade then
//!   a change loses nothing; nothing is merged back on its own. If the
//!   copy fails, the change fails and nothing changes.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Mutex;
use tokio_util::task::TaskTracker;

mod writer;
#[cfg(test)]
mod writer_tests;
use vessel_proto::HarnessKind;
use vessel_proto::parking::ParkAfter;
use vessel_proto::settings::{
    AgentDriver, AgentDrivers, NodeSettingKey, NodeSettings, ReloadedNodeSettings, SetAgentDrivers,
    SetNodeSettings, SettingKeys, UnknownSettingKey,
};
use vessel_session::SessionRuntime;
use vessel_store::{DataDir, SetAside};

/// Why a change was refused or not kept.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SettingsError {
    /// Shutdown has closed mutation admission, so no late writer may start.
    #[error("the node is stopping; settings cannot be changed")]
    Stopping,
    /// A choice this node does not know.
    #[error("this node does not know that park_after; send never or idle with minutes")]
    Unknown,
    /// A driver this node does not know.
    #[error("this node does not know that driver for {0}; send acp or direct")]
    UnknownDriver(HarnessKind),
    /// An agent's own protocol this build does not offer yet.
    #[error("this build of Vessel does not offer {0} over its own protocol yet; it runs over ACP")]
    NoDirect(HarnessKind),
    /// The file could not be written; nothing changed.
    #[error("the settings could not be kept: {0}")]
    Write(#[source] vessel_store::StoreError),
    /// The write's task did not finish.
    #[error("the settings could not be kept: {0}")]
    Lost(#[source] writer::TaskFailure),
    /// `settings.json` could not be used at start and could not be moved
    /// aside either, so no change replaces it.
    #[error(
        "the settings could not be kept: settings.json could not be read at start and could not be moved aside ({0}); it is left as it is. Move or repair it, then restart the node"
    )]
    Kept(Arc<str>),
    /// Explicit reload never falls back after a failed read or validation.
    #[error("the saved settings could not be reloaded: {0}")]
    Reload(#[from] vessel_store::ReloadSettingsError),
    /// Unknown reset keys cannot remove a future build's settings.
    #[error(transparent)]
    Key(#[from] UnknownSettingKey),
}

/// The node's settings: the flags' values and what a person saved.
#[derive(Debug)]
pub(crate) struct Settings {
    data_dir: DataDir,
    runtime: Arc<SessionRuntime>,
    /// `--park-after`, or its default.
    flag: ParkAfter,
    /// What a person changed, as kept; held over write and apply.
    saved: Arc<Mutex<SetNodeSettings>>,
    /// Mutations survive caller cancellation and are joined at shutdown.
    writers: TaskTracker,
    #[cfg(test)]
    gate: writer_tests::GateSlot,
    #[cfg(test)]
    reload_gate: writer_tests::GateSlot,
    /// Why `settings.json` must not be replaced: it could not be used at
    /// start, and moving it aside failed. `None` when changes may write.
    kept: Option<Arc<str>>,
    /// The file holds what only a later build knows: the first change that
    /// writes copies it aside first (`settings.json.later-<UTC time>`).
    later: AtomicBool,
}

impl Settings {
    /// Keep the node startup adapter beside saved-value precedence, not in its
    /// already large composition function. This reads only the passed config.
    pub(crate) fn for_config(config: &crate::Config, runtime: Arc<SessionRuntime>) -> Self {
        Self::open(
            config.data_dir.clone(),
            runtime,
            ParkAfter::from_duration(config.park_after),
            config.park_after_given,
        )
    }

    /// Read `settings.json` and apply what it holds to `runtime`. A file
    /// that cannot be used is moved aside and logged, and the flags'
    /// values stand.
    pub(crate) fn open(
        data_dir: DataDir,
        runtime: Arc<SessionRuntime>,
        flag: ParkAfter,
        flag_given: bool,
    ) -> Self {
        let (saved, kept, later) = match data_dir.read_settings() {
            Ok(saved) => {
                let saved = saved.unwrap_or_default();
                // Unsure counts as more: a copy too many costs nothing.
                let later = data_dir.settings_hold_more_than(&saved).unwrap_or(true);
                (saved, None, later)
            }
            Err(error) => {
                let kept = set_aside(&data_dir, &error);
                (SetNodeSettings::default(), kept, false)
            }
        };
        let settings = Self {
            data_dir,
            runtime,
            flag,
            saved: Arc::new(Mutex::new(saved)),
            writers: TaskTracker::new(),
            #[cfg(test)]
            gate: writer_tests::GateSlot::default(),
            #[cfg(test)]
            reload_gate: writer_tests::GateSlot::default(),
            kept,
            later: AtomicBool::new(later),
        };
        let in_force = settings.in_force(saved);
        if flag_given && saved.park_after.is_some() && in_force.park_after != flag {
            tracing::info!(
                saved = ?in_force.park_after,
                "--park-after is overridden by the value saved with node.set_settings"
            );
        }
        settings.apply(&in_force);
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
        let saved = saved.drivers.unwrap_or_default();
        let driver = |kind, chosen: Option<AgentDriver>| match chosen {
            Some(AgentDriver::Direct) if offered(kind) => AgentDriver::Direct,
            // Not offered by this build, a newer build's choice, or never
            // chosen: ACP.
            Some(AgentDriver::Direct | AgentDriver::Acp | AgentDriver::Unknown) | None => {
                AgentDriver::Acp
            }
        };
        let drivers = AgentDrivers {
            claude_code: driver(HarnessKind::ClaudeCode, saved.claude_code),
            codex: driver(HarnessKind::Codex, saved.codex),
            open_code: driver(HarnessKind::OpenCode, saved.open_code),
        };
        NodeSettings {
            park_after,
            drivers,
            restart_recovery: vessel_proto::restart::RestartRecovery::default(),
        }
    }

    // Keep startup and successful replacements in the same application order.
    fn apply(&self, in_force: &NodeSettings) {
        self.runtime.set_park_after(in_force.park_after.duration());
        self.runtime.set_drivers(in_force.drivers);
    }

    /// The settings in force.
    pub(crate) async fn current(&self) -> NodeSettings {
        let saved = *self.saved.lock().await;
        self.in_force(saved)
    }

    /// Change what `change` names, keep it, then apply it; answers what is
    /// in force.
    pub(crate) async fn change(
        self: &Arc<Self>,
        change: SetNodeSettings,
    ) -> Result<NodeSettings, SettingsError> {
        validate(change)?;
        let saved = Arc::clone(&self.saved).lock_owned().await;
        let mut next = *saved;
        if let Some(after) = change.park_after {
            next.park_after = Some(after);
        }
        if let Some(drivers) = change.drivers {
            next.drivers = Some(drivers.over(next.drivers.unwrap_or_default()));
        }
        self.persist(saved, next).await
    }

    /// Remove saved overrides through the existing locked write-before-apply rule.
    pub(crate) async fn unset(
        self: &Arc<Self>,
        keys: &SettingKeys,
    ) -> Result<NodeSettings, SettingsError> {
        let saved = Arc::clone(&self.saved).lock_owned().await;
        let mut next = *saved;
        for key in keys.iter() {
            key.clear(&mut next)?;
        }
        self.persist(saved, next).await
    }

    /// Keep an admitted reload through reading, validation and publication.
    /// A cancelled RPC drops only its waiter; shutdown joins the mutation.
    pub(crate) async fn reload(self: &Arc<Self>) -> Result<ReloadedNodeSettings, SettingsError> {
        let saved = Arc::clone(&self.saved).lock_owned().await;
        let mutation = self.admit()?;
        let settings = Arc::clone(self);
        mutation
            .run(saved, move |saved| async move {
                settings.read_and_apply(saved).await
            })
            .await
    }

    async fn read_and_apply(
        &self,
        mut saved: tokio::sync::OwnedMutexGuard<SetNodeSettings>,
    ) -> Result<ReloadedNodeSettings, SettingsError> {
        let data_dir = self.data_dir.clone();
        #[cfg(test)]
        let gate = self.reload_gate.take();
        let next = tokio::task::spawn_blocking(move || {
            #[cfg(test)]
            if let Some(gate) = gate {
                gate.pause();
            }
            data_dir.read_settings_for_reload()
        })
        .await
        .map_err(|error| SettingsError::Lost(error.into()))??
        .unwrap_or_default();
        validate(next)?;
        let before = self.in_force(*saved);
        let after = self.in_force(next);
        let mut report = ReloadedNodeSettings::default();
        for key in NodeSettingKey::ALL {
            key.report_change(&before, &after, &mut report)?;
        }
        // A strict snapshot contains nothing this build would discard later.
        // No runtime state is published before every field and report validates.
        *saved = next;
        self.later.store(false, Ordering::Relaxed);
        if before != after {
            self.apply(&after);
        }
        Ok(report)
    }
}

fn validate(change: SetNodeSettings) -> Result<(), SettingsError> {
    if change.park_after == Some(ParkAfter::Unknown) {
        return Err(SettingsError::Unknown);
    }
    for (kind, driver) in change.drivers.iter().flat_map(SetAgentDrivers::named) {
        match driver {
            AgentDriver::Unknown => return Err(SettingsError::UnknownDriver(kind)),
            AgentDriver::Direct if !offered(kind) => {
                return Err(SettingsError::NoDirect(kind));
            }
            AgentDriver::Direct | AgentDriver::Acp => {}
        }
    }
    Ok(())
}

/// Move an unusable `settings.json` aside, so no change replaces it;
/// answers why changes must not write when even that failed.
fn set_aside(data_dir: &DataDir, unusable: &vessel_store::SettingsFileError) -> Option<Arc<str>> {
    match data_dir.set_aside_settings(chrono::Utc::now(), SetAside::Unreadable) {
        Ok(aside) => {
            tracing::warn!(
                error = %unusable,
                aside = %aside.display(),
                "settings.json could not be used and was moved aside; the node starts from its flags"
            );
            None
        }
        Err(moved) => {
            tracing::error!(
                error = %unusable,
                %moved,
                "settings.json could not be used or moved aside; the node starts from its flags and will not replace it"
            );
            Some(Arc::from(moved.to_string()))
        }
    }
}

/// Whether this build offers `kind` over its own protocol.
fn offered(kind: HarnessKind) -> bool {
    vessel_harness::direct(kind).is_some_and(|direct| direct.selectable)
}

#[cfg(test)]
mod tests;
