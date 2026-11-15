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
use std::sync::atomic::AtomicBool;

use crate::dictation::{ReadError, Reader};
use tokio::sync::Mutex;
use tokio_util::task::TaskTracker;

mod writer;
#[cfg(test)]
mod writer_tests;
use vessel_proto::HarnessKind;
use vessel_proto::dictation::{DictationChange, DictationDecodeError, KeyPresence};
use vessel_proto::parking::ParkAfter;
use vessel_proto::settings::{
    AgentDriver, AgentDrivers, NodeSettings, SetAgentDrivers, SetNodeSettings,
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
    /// A pending feature must not change settings through a legacy path.
    #[error("dictation settings are unavailable for this peer")]
    DictationUnavailable,
    /// Reject the complete merged candidate before any write.
    #[error("the dictation settings were refused: {0}")]
    Dictation(#[source] DictationDecodeError),
    /// A newly selected reference must resolve before it can be saved.
    #[error("the selected dictation key is missing or empty")]
    DictationKeyAbsent,
    /// The private cause never enters the wire or log presentation.
    #[error("the dictation key could not be checked")]
    DictationSecret(#[source] ReadError),
    /// The file could not be written; nothing changed.
    #[error("the settings could not be kept: {0}")]
    Write(#[source] vessel_store::StoreError),
    /// The write's task did not finish.
    #[error("the settings could not be kept: {0}")]
    Lost(#[source] tokio::task::JoinError),
    /// `settings.json` could not be used at start and could not be moved
    /// aside either, so no change replaces it.
    #[error(
        "the settings could not be kept: settings.json could not be read at start and could not be moved aside ({0}); it is left as it is. Move or repair it, then restart the node"
    )]
    Kept(Arc<str>),
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
    /// Writes outlive a cancelled RPC and are joined at node shutdown.
    writers: TaskTracker,
    #[cfg(test)]
    gate: writer_tests::GateSlot,
    /// Why `settings.json` must not be replaced: it could not be used at
    /// start, and moving it aside failed. `None` when changes may write.
    kept: Option<Arc<str>>,
    /// The file holds what only a later build knows: the first change that
    /// writes copies it aside first (`settings.json.later-<UTC time>`).
    later: AtomicBool,
    #[cfg(test)]
    write_gate: Option<Arc<WriteGate>>,
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

    /// Hold the real write boundary after validation for deterministic races.
    #[cfg(test)]
    pub(crate) fn hold_writes(&mut self) -> Arc<WriteGate> {
        let gate = Arc::new(WriteGate::default());
        self.write_gate = Some(Arc::clone(&gate));
        gate
    }

    /// Service preparation reads this only after the prior stream retires.
    pub(crate) async fn dictation_snapshot(&self) -> vessel_proto::dictation::SetDictation {
        self.saved
            .lock()
            .await
            .dictation
            .clone()
            .unwrap_or_default()
    }

    /// The service constructs this once and shares its read gate with starts.
    pub(crate) fn dictation_reader(&self) -> Reader {
        Reader::new(crate::fleet::FleetFiles::new(&self.data_dir))
    }

    /// Supported reads resolve presence from a consistent settings snapshot.
    /// The caller skips this path entirely for hidden/older replies.
    pub(crate) async fn current_dictation(
        &self,
        reader: &Reader,
    ) -> Result<NodeSettings, SettingsError> {
        let saved = self.saved.lock().await;
        // Retain the settings lock through this bounded blocking-worker read,
        // so parking, drivers and the selected reference describe one snapshot.
        let selection = saved.dictation.clone().unwrap_or_default();
        let presence = reader
            .presence(selection.key_name())
            .await
            .map_err(SettingsError::DictationSecret)?;
        Ok(self.projected(&saved, Some(presence)))
    }

    fn projected(&self, saved: &SetNodeSettings, presence: Option<KeyPresence>) -> NodeSettings {
        let mut view = self.in_force(saved);
        view.dictation =
            presence.map(|key| saved.dictation.clone().unwrap_or_default().resolve(key));
        view
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
            saved: Arc::new(Mutex::new(saved.clone())),
            writers: TaskTracker::new(),
            #[cfg(test)]
            gate: writer_tests::GateSlot::default(),
            kept,
            later: AtomicBool::new(later),
            #[cfg(test)]
            write_gate: None,
        };
        let in_force = settings.in_force(&saved);
        if flag_given && saved.park_after.is_some() && in_force.park_after != flag {
            tracing::info!(
                saved = ?in_force.park_after,
                "--park-after is overridden by the value saved with node.set_settings"
            );
        }
        settings
            .runtime
            .set_park_after(in_force.park_after.duration());
        settings.runtime.set_drivers(in_force.drivers);
        settings
    }

    /// What is in force for `saved`: a saved value this node knows, else
    /// the flag's.
    fn in_force(&self, saved: &SetNodeSettings) -> NodeSettings {
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
            dictation: None,
            park_after,
            drivers,
            restart_recovery: vessel_proto::restart::RestartRecovery::default(),
        }
    }

    /// The settings in force.
    pub(crate) async fn current(&self) -> NodeSettings {
        let saved = self.saved.lock().await;
        self.in_force(&saved)
    }

    /// Change what `change` names, keep it, then apply it; answers what is
    /// in force.
    pub(crate) async fn change(
        self: &Arc<Self>,
        change: SetNodeSettings,
    ) -> Result<NodeSettings, SettingsError> {
        self.change_with(change, None).await
    }

    /// Feature-aware writes share the legacy lock, atomic writer and apply order.
    pub(crate) async fn change_dictation(
        self: &Arc<Self>,
        change: SetNodeSettings,
        reader: &Reader,
    ) -> Result<NodeSettings, SettingsError> {
        self.change_with(change, Some(reader)).await
    }

    async fn change_with(
        self: &Arc<Self>,
        change: SetNodeSettings,
        reader: Option<&Reader>,
    ) -> Result<NodeSettings, SettingsError> {
        if change.dictation.is_some() && reader.is_none() {
            return Err(SettingsError::DictationUnavailable);
        }
        check(&change)?;
        let new_key_selected = change
            .dictation
            .as_ref()
            .is_some_and(|patch| matches!(patch.key.as_ref(), Some(DictationChange::Set(_))));
        let saved = Arc::clone(&self.saved).lock_owned().await;
        let next = merged(change, saved.clone())?;
        let presence = match reader {
            Some(reader) => Some(
                reader
                    .presence(
                        next.dictation
                            .as_ref()
                            .and_then(vessel_proto::dictation::SetDictation::key_name),
                    )
                    .await
                    .map_err(SettingsError::DictationSecret)?,
            ),
            None => None,
        };
        // Even an otherwise unchanged explicit selection must still resolve.
        if new_key_selected && presence != Some(KeyPresence::Present) {
            return Err(SettingsError::DictationKeyAbsent);
        }
        let reply = self.projected(&next, presence);
        #[cfg(test)]
        if next != *saved
            && let Some(gate) = &self.write_gate
        {
            gate.entered.notify_one();
            gate.release.notified().await;
        }
        // Even a no-op must pass the shared writer's closed-admission check.
        // The owned guard spans validation, real I/O and runtime publication.
        let _applied = self.persist(saved, next).await?;
        Ok(reply)
    }
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
mod diagnostic_tests;

/// Driver validation remains identical for both legacy and speech patches.
fn check(change: &SetNodeSettings) -> Result<(), SettingsError> {
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

/// One merged candidate is validated before either settings writer can run.
fn merged(
    change: SetNodeSettings,
    mut next: SetNodeSettings,
) -> Result<SetNodeSettings, SettingsError> {
    if let Some(after) = change.park_after {
        next.park_after = Some(after);
    }
    if let Some(drivers) = change.drivers {
        next.drivers = Some(drivers.over(next.drivers.unwrap_or_default()));
    }
    if let Some(patch) = change.dictation {
        next.dictation = Some(
            patch
                .over(next.dictation.take().unwrap_or_default())
                .map_err(SettingsError::Dictation)?,
        );
    }
    Ok(next)
}

/// Observe validation-before-write without substituting a fake writer.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct WriteGate {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}
#[cfg(test)]
impl WriteGate {
    /// The candidate and key were checked, and the settings lock is held.
    pub(crate) async fn entered(&self) {
        self.entered.notified().await;
    }
    /// Let the actual filesystem writer proceed once the test changes its edge.
    pub(crate) fn release(&self) {
        self.release.notify_one();
    }
}

#[cfg(test)]
mod dictation_tests;
#[cfg(test)]
pub(crate) mod test_fixture;
