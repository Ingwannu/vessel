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

use std::sync::Arc;

use tokio::sync::Mutex;
use vessel_proto::HarnessKind;
use vessel_proto::parking::ParkAfter;
use vessel_proto::settings::{
    AgentDriver, AgentDrivers, NodeSettings, SetAgentDrivers, SetNodeSettings,
};
use vessel_session::SessionRuntime;
use vessel_store::DataDir;

/// Why a change was refused or not kept.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SettingsError {
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
        settings.runtime.set_drivers(in_force.drivers);
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
        for (kind, driver) in change.drivers.iter().flat_map(SetAgentDrivers::named) {
            match driver {
                AgentDriver::Unknown => return Err(SettingsError::UnknownDriver(kind)),
                AgentDriver::Direct if !offered(kind) => {
                    return Err(SettingsError::NoDirect(kind));
                }
                AgentDriver::Direct | AgentDriver::Acp => {}
            }
        }
        let mut saved = self.saved.lock().await;
        let mut next = *saved;
        if let Some(after) = change.park_after {
            next.park_after = Some(after);
        }
        if let Some(drivers) = change.drivers {
            next.drivers = Some(drivers.over(next.drivers.unwrap_or_default()));
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
        self.runtime.set_drivers(in_force.drivers);
        Ok(in_force)
    }
}

/// Whether this build offers `kind` over its own protocol.
fn offered(kind: HarnessKind) -> bool {
    vessel_harness::direct(kind).is_some_and(|direct| direct.selectable)
}
