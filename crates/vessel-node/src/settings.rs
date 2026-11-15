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

use tokio::sync::Mutex;

mod admission;
mod opening;
mod recovery_apply;
pub(crate) use opening::Opening;
pub(crate) use recovery_apply::RecoveryApply;
#[cfg(test)]
mod shutdown_tests;
mod writer;
#[cfg(test)]
mod writer_tests;
use vessel_proto::HarnessKind;
use vessel_proto::parking::ParkAfter;
use vessel_proto::settings::{
    AgentDriver, AgentDrivers, NodeSettings, SetAgentDrivers, SetNodeSettings,
};
use vessel_session::SessionRuntime;
use vessel_store::{DataDir, SavedNodeSettings, SetAside, SettingsDisposition};

/// Why a change was refused or not kept.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SettingsError {
    /// Shutdown has closed mutation admission, so no late writer may start.
    #[error("the node is stopping; settings cannot be changed")]
    Stopping,
    /// The bounded waiting lane refused this operation before any work.
    #[error(
        "settings queue is full; at most {} changes may wait",
        admission::SettingsAdmission::MAX_WAITING
    )]
    Full,
    /// The retained consumer ended without answering this operation.
    #[error("the settings operation ended without a reply")]
    Consumer(#[source] tokio::sync::oneshot::error::RecvError),
    /// Actual recovery SQL settlement failed; saved state is not rolled back.
    #[error(transparent)]
    Recovery(#[from] recovery_apply::RecoveryApplyError),
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
}

/// The node's settings: the flags' values and what a person saved.
#[derive(Debug)]
pub(crate) struct Settings {
    state: Arc<State>,
    admission: admission::SettingsAdmission,
}

/// Worker state never owns a sender; dropping the last handle closes the lane.
#[derive(Debug)]
struct State {
    data_dir: DataDir,
    runtime: Arc<SessionRuntime>,
    recovery: RecoveryApply,
    /// `--park-after`, or its default.
    flag: ParkAfter,
    /// What a person changed, as kept; held over write and apply.
    saved: Arc<Mutex<SavedNodeSettings>>,
    #[cfg(test)]
    gate: writer_tests::GateSlot,
    #[cfg(test)]
    cleanup_gate: writer_tests::GateSlot,
    #[cfg(test)]
    fixture_store: std::sync::Mutex<Option<tempfile::TempDir>>,
    /// Why `settings.json` must not be replaced: it could not be used at
    /// start, and moving it aside failed. `None` when changes may write.
    kept: Option<Arc<str>>,
    /// The file holds what only a later build knows: the first change that
    /// writes copies it aside first (`settings.json.later-<UTC time>`).
    later: AtomicBool,
}

impl Settings {
    /// Fixtures retain their real Store directory with worker state, so caller
    /// or last-handle loss cannot remove it before physical SQL settlement.
    #[cfg(test)]
    pub(crate) async fn open(
        data_dir: DataDir,
        runtime: Arc<SessionRuntime>,
        flag: ParkAfter,
        flag_given: bool,
    ) -> Self {
        let opening = Opening::read_parts(data_dir, flag, flag_given);
        Self::fixture_from_opening(opening, runtime).await
    }

    #[cfg(test)]
    pub(super) async fn fixture_from_opening(
        opening: Opening,
        runtime: Arc<SessionRuntime>,
    ) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let policy = opening.recovery_policy();
        let store = vessel_store::Store::open_with_restart_policy(
            &DataDir::new(directory.path()),
            Default::default(),
            policy.enabled,
        )
        .await
        .unwrap()
        .store;
        let recovery = RecoveryApply::new(
            Arc::new(crate::recovery::policy::Policy::new(policy)),
            store,
        );
        let settings = Self::from_opening(opening, runtime, recovery);
        *settings.state.fixture_store.lock().unwrap() = Some(directory);
        settings
    }

    /// Consume the exact pre-Store snapshot; never re-read the file here.
    pub(crate) fn from_opening(
        opening: Opening,
        runtime: Arc<SessionRuntime>,
        recovery: RecoveryApply,
    ) -> Self {
        let Opening {
            data_dir,
            saved,
            flag,
            flag_given,
            kept,
            disposition,
        } = opening;
        let public = saved.public_settings();
        let settings = State {
            data_dir,
            runtime,
            recovery,
            flag,
            saved: Arc::new(Mutex::new(saved)),
            #[cfg(test)]
            gate: writer_tests::GateSlot::default(),
            #[cfg(test)]
            cleanup_gate: writer_tests::GateSlot::default(),
            #[cfg(test)]
            fixture_store: std::sync::Mutex::default(),
            kept,
            later: AtomicBool::new(matches!(
                disposition,
                SettingsDisposition::PreserveBeforeWrite
            )),
        };
        let in_force = settings.in_force(&public);
        if flag_given && public.park_after.is_some() && in_force.park_after != flag {
            tracing::info!(
                saved = ?in_force.park_after,
                "--park-after is overridden by the value saved with node.set_settings"
            );
        }
        settings
            .runtime
            .set_park_after(in_force.park_after.duration());
        settings.runtime.set_drivers(in_force.drivers);
        let state = Arc::new(settings);
        Self {
            admission: admission::SettingsAdmission::start(Arc::clone(&state)),
            state,
        }
    }

    /// One shared policy Arc binds the settings fence to actual recovery enqueue.
    pub(crate) fn policy(&self) -> Arc<crate::recovery::policy::Policy> {
        self.state.recovery.policy()
    }

    #[cfg(test)]
    pub(crate) fn pause_write(
        &self,
    ) -> (
        tokio::sync::oneshot::Receiver<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        self.state.gate.install()
    }

    #[cfg(test)]
    pub(crate) fn pause_cleanup(
        &self,
    ) -> (
        tokio::sync::oneshot::Receiver<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        self.state.cleanup_gate.install()
    }

    /// The settings in force.
    pub(crate) async fn current(&self) -> NodeSettings {
        let public = self.state.saved.lock().await.public_settings();
        self.state.in_force(&public)
    }

    /// Success transfers the typed intent before the caller awaits its reply.
    pub(crate) async fn change(
        &self,
        change: SetNodeSettings,
    ) -> Result<NodeSettings, SettingsError> {
        validate(&change)?;
        self.admission
            .submit(change)?
            .await
            .map_err(SettingsError::Consumer)?
    }

    /// Close generic settings first, then seal effects under their final enqueue
    /// arbiter. Before that final seal linearizes, settings is already closed.
    pub(crate) fn seal(&self) {
        self.admission.seal();
        self.state.recovery.seal_effect_admission();
    }

    /// Seal admission and join queued as well as physical work. Cancelling this
    /// waiter cannot cancel the independent consumer.
    pub(crate) async fn shutdown(&self) {
        self.seal();
        self.admission.shutdown().await;
    }
}

impl State {
    /// What is in force for `saved`: a saved value this node knows, else
    /// the flag's.
    fn in_force(&self, saved: &SetNodeSettings) -> NodeSettings {
        let park_after = match saved.park_after {
            Some(after @ (ParkAfter::Never | ParkAfter::Idle { .. })) => after,
            // A newer build's choice, read after a downgrade: the flag's.
            Some(ParkAfter::Unknown) | None => self.flag,
        };
        let restart_recovery = saved.restart_recovery.as_ref().map_or_else(
            vessel_proto::restart::RestartRecovery::default,
            vessel_proto::restart::SetRestartRecovery::effective,
        );
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
            restart_recovery,
        }
    }

    /// Only the consumer prepares a candidate, so queued patches see all earlier
    /// publications even when the original callers have gone away.
    async fn change(&self, change: SetNodeSettings) -> Result<NodeSettings, SettingsError> {
        let saved = Arc::clone(&self.saved).lock_owned().await;
        self.recovery.settle_before().await?;
        let mut next = saved.public_settings();
        if let Some(after) = change.park_after {
            next.park_after = Some(after);
        }
        if let Some(drivers) = change.drivers {
            next.drivers = Some(drivers.over(next.drivers.unwrap_or_default()));
        }
        if let Some(recovery) = &change.restart_recovery {
            next.restart_recovery = Some(
                recovery.over(
                    next.restart_recovery
                        .as_ref()
                        .unwrap_or(&Default::default()),
                ),
            );
        }
        // Keep the old document intact until the candidate is durable.
        let next = saved.clone().with_public_settings(next);
        self.persist(saved, next).await
    }
}

/// Pure validation precedes capacity admission, without acquiring a saved guard.
fn validate(change: &SetNodeSettings) -> Result<(), SettingsError> {
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
mod diagnostic_tests;
