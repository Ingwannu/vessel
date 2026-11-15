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

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Weak};

use tokio::sync::Mutex;
use tokio_util::task::TaskTracker;
use vessel_proto::relay_settings::RelaySettingsError;

use crate::relays::Relays;

mod mutation;
use vessel_proto::HarnessKind;
use vessel_proto::parking::ParkAfter;
use vessel_proto::settings::{AgentDriver, AgentDrivers, NodeSettings, SetNodeSettings};
use vessel_session::SessionRuntime;
use vessel_store::{DataDir, SetAside};

/// Why a change was refused or not kept.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SettingsError {
    /// A choice this node does not know.
    #[error("this node does not know that park_after; send never or idle with minutes")]
    Unknown,
    /// A closed, credential-free relay refusal.
    #[error(transparent)]
    Relay(#[from] RelaySettingsError),
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
    /// Weak to avoid a service/gateway/relay ownership cycle.
    relays: Weak<Relays>,
    /// Reuse node shutdown ownership for mutations after an RPC disappears.
    mutations: TaskTracker,
    /// Why `settings.json` must not be replaced: it could not be used at
    /// start, and moving it aside failed. `None` when changes may write.
    kept: Option<Arc<str>>,
    /// The file holds what only a later build knows: the first change that
    /// writes copies it aside first (`settings.json.later-<UTC time>`).
    later: AtomicBool,
}

/// Existing owners, not a second settings task or relay registry.
#[derive(Debug)]
pub(crate) struct Owners {
    pub(crate) relays: Weak<Relays>,
    pub(crate) mutations: TaskTracker,
}

impl Settings {
    /// Read `settings.json` and apply what it holds to `runtime`. A file
    /// that cannot be used is moved aside and logged, and the flags'
    /// values stand.
    pub(crate) fn open(
        data_dir: DataDir,
        runtime: Arc<SessionRuntime>,
        flag: ParkAfter,
        flag_given: bool,
        owners: Owners,
    ) -> Result<Self, RelaySettingsError> {
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
        let relays = owners
            .relays
            .upgrade()
            .ok_or(RelaySettingsError::Stopping)?;
        relays.initial_consent(saved.relay)?;
        let settings = Self {
            data_dir,
            runtime,
            flag,
            saved: Arc::new(Mutex::new(saved)),
            relays: owners.relays,
            mutations: owners.mutations,
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
        settings
            .runtime
            .set_park_after(in_force.park_after.duration());
        settings.runtime.set_drivers(in_force.drivers);
        Ok(settings)
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
            relay: self.relays.upgrade().map(|relays| relays.settings()),
            park_after,
            drivers,
            restart_recovery: vessel_proto::restart::RestartRecovery::default(),
        }
    }

    /// The settings in force.
    pub(crate) async fn current(&self) -> NodeSettings {
        let saved = *self.saved.lock().await;
        self.in_force(saved)
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
