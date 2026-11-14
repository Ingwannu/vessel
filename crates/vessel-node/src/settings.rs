//! The node's settings a client reads and changes (protocol 12,
//! `node.settings` and `node.set_settings`, ADR 0038): how long an idle
//! agent runs before it is parked, and (protocol 18, ADR 0057) which
//! protocol drives each agent that has one of its own. ADR 0059 adds
//! startup continuation, off unless a person enables it.
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
//!   first and applied after in one bounded node-owned FIFO lane, so a
//!   dropped RPC cannot abandon application after a successful save.
//!   A failed write changes nothing. An equal-value change writes nothing,
//!   but still completes any previously saved, unapplied recovery phase.
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

use crate::recovery::policy::Policy;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_proto::HarnessKind;
use vessel_proto::parking::ParkAfter;
use vessel_proto::settings::{
    AgentDriver, AgentDrivers, NodeSettings, SetAgentDrivers, SetNodeSettings,
};
use vessel_session::SessionRuntime;
use vessel_store::{DataDir, Store};

/// Why a change was refused or not kept.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SettingsError {
    /// No operation was admitted; a caller may make a new explicit attempt.
    #[error("node settings already has eight queued changes")]
    Full,
    /// Node shutdown has closed settings admission.
    #[error("the node is stopping; no settings change was admitted")]
    Closed,
    /// The node-owned operation ended before reporting its actual result.
    #[error("the node settings operation ended before replying: {0}")]
    Reply(#[source] oneshot::error::RecvError),
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
    /// Off is saved and effective, but durable intent cleanup must be retried.
    #[error(
        "startup recovery is off, but its pending intents could not be cleared: {0}; retry the settings change"
    )]
    Cleanup(#[source] vessel_store::StoreError),
    /// On was saved, but automatic admission stays paused until this phase
    /// succeeds; retrying the same setting must retry the actual Store ACK.
    #[error(
        "startup recovery was saved as on, but its policy could not be applied; admissions remain paused; retry this settings change: {0}"
    )]
    Apply(#[source] vessel_store::StoreError),
    /// The write's task did not finish.
    #[error("the settings could not be kept: {0}")]
    Lost(#[source] tokio::task::JoinError),
}

/// The node's settings: the flags' values and what a person saved.
#[derive(Debug)]
struct Engine {
    data_dir: DataDir,
    store: Store,
    policy: Arc<Policy>,
    runtime: Arc<SessionRuntime>,
    /// `--park-after`, or its default.
    flag: ParkAfter,
    /// What a person changed, as kept; held over write and apply.
    saved: Mutex<SetNodeSettings>,
    #[cfg(test)]
    gates: tests::Gates,
}

impl Engine {
    /// Read `settings.json` and apply what it holds to `runtime`. A file
    /// that cannot be used is logged, and the flags' values stand.
    fn open(
        opening: Opening,
        runtime: Arc<SessionRuntime>,
        flag: ParkAfter,
        flag_given: bool,
    ) -> Self {
        let Opening {
            data_dir,
            store,
            saved,
            policy,
        } = opening;
        let settings = Self {
            data_dir,
            store,
            policy,
            runtime,
            flag,
            saved: Mutex::new(saved.clone()),
            #[cfg(test)]
            gates: tests::Gates::default(),
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
        let restart_recovery = saved
            .restart_recovery
            .as_ref()
            .map(vessel_proto::restart::SetRestartRecovery::effective)
            .unwrap_or_default();
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

    /// The settings in force.
    async fn current(&self) -> NodeSettings {
        let saved = self.saved.lock().await;
        self.in_force(&saved)
    }

    /// Change what `change` names, keep it, then apply it; answers what is
    /// in force.
    async fn change(&self, change: SetNodeSettings) -> Result<NodeSettings, SettingsError> {
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
        let mut next = saved.clone();
        if let Some(after) = change.park_after {
            next.park_after = Some(after);
        }
        if let Some(drivers) = change.drivers {
            next.drivers = Some(drivers.over(next.drivers.unwrap_or_default()));
        }
        if let Some(recovery) = change.restart_recovery {
            next.restart_recovery = Some(match next.restart_recovery.as_ref() {
                Some(previous) => recovery.over(previous),
                None => recovery,
            });
        }
        // An earlier successful Off may have lost only its cleanup. Keep
        // that obligation through reply loss and force it ahead of later On.
        if self.policy.needs_cleanup() {
            self.store
                .set_restart_policy(false)
                .await
                .map_err(SettingsError::Cleanup)?;
            self.policy.cleaned();
        }
        if next == *saved {
            let current = self.in_force(&saved);
            if self.policy.needs_sync() {
                self.synchronize(&current).await?;
            }
            return Ok(current);
        }
        let effective = self.in_force(&next);
        self.policy.begin_write(&effective.restart_recovery);
        let data_dir = self.data_dir.clone();
        let writing = next.clone();
        #[cfg(test)]
        let gate = self.gates.write();
        let written = tokio::task::spawn_blocking(move || {
            #[cfg(test)]
            if let Some(gate) = gate {
                gate.wait();
            }
            data_dir.write_settings(&writing)
        })
        .await
        .map_err(SettingsError::Lost)
        .and_then(|result| result.map_err(SettingsError::Write));
        if let Err(error) = written {
            self.policy.write_failed();
            return Err(error);
        }
        #[cfg(test)]
        self.gates.before_apply().await;
        *saved = next;
        let in_force = self.in_force(&saved);
        self.policy.apply(in_force.restart_recovery.clone());
        self.runtime.set_park_after(in_force.park_after.duration());
        self.runtime.set_drivers(in_force.drivers);
        #[cfg(test)]
        self.gates.before_cleanup().await;
        self.synchronize(&in_force).await?;
        Ok(in_force)
    }
    async fn synchronize(&self, current: &NodeSettings) -> Result<(), SettingsError> {
        self.store
            .set_restart_policy(current.restart_recovery.enabled)
            .await
            .map_err(|error| {
                if current.restart_recovery.enabled {
                    SettingsError::Apply(error)
                } else {
                    SettingsError::Cleanup(error)
                }
            })?;
        self.policy.cleaned();
        Ok(())
    }
}

/// One admitted change. Its reply does not own the write/apply operation.
#[derive(Debug)]
struct Change {
    request: SetNodeSettings,
    reply: oneshot::Sender<Result<NodeSettings, SettingsError>>,
}

/// Settings admission, independent of a connection's cancellable handler.
/// Eight queued changes plus one active write bound memory and work. The
/// same engine owns the saved lock, file write and runtime apply throughout.
#[derive(Debug)]
pub(crate) struct Settings {
    engine: Arc<Engine>,
    changes: mpsc::Sender<Change>,
    stopping: CancellationToken,
    finished: TaskTracker,
}

impl Settings {
    pub(crate) fn open(
        opening: Opening,
        runtime: Arc<SessionRuntime>,
        flag: ParkAfter,
        flag_given: bool,
        tasks: &TaskTracker,
        stopping: &CancellationToken,
    ) -> Self {
        let engine = Arc::new(Engine::open(opening, runtime, flag, flag_given));
        let (changes, receiver) = mpsc::channel(8);
        let stopping = stopping.child_token();
        // Drain this one worker before sessions stop. All-node tasks also
        // contain persistence, which cannot finish until sessions end.
        let finished = TaskTracker::new();
        drop(tasks.spawn(finished.track_future(owned::run(
            Arc::clone(&engine),
            receiver,
            stopping.clone(),
        ))));
        let _closed = finished.close();
        Self {
            engine,
            changes,
            stopping,
            finished,
        }
    }

    /// Close admission and settle saved policy before actors capture shutdown.
    pub(crate) async fn drain(&self) {
        self.stopping.cancel();
        self.finished.wait().await;
    }

    pub(crate) fn policy(&self) -> Arc<Policy> {
        Arc::clone(&self.engine.policy)
    }

    pub(crate) async fn current(&self) -> NodeSettings {
        self.engine.current().await
    }

    pub(crate) async fn change(
        &self,
        request: SetNodeSettings,
    ) -> Result<NodeSettings, SettingsError> {
        if self.stopping.is_cancelled() {
            return Err(SettingsError::Closed);
        }
        let (reply, receive) = oneshot::channel();
        self.changes
            .try_send(Change { request, reply })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => SettingsError::Full,
                mpsc::error::TrySendError::Closed(_) => SettingsError::Closed,
            })?;
        receive.await.map_err(SettingsError::Reply)?
    }
}

mod owned;
#[cfg(test)]
mod tests;

/// Whether this build offers `kind` over its own protocol.
fn offered(kind: HarnessKind) -> bool {
    vessel_harness::direct(kind).is_some_and(|direct| direct.selectable)
}

/// Validated once before opening the store, then reused by the settings engine.
#[derive(Debug)]
pub(crate) struct Opening {
    pub(crate) data_dir: DataDir,
    pub(crate) store: Store,
    pub(crate) saved: SetNodeSettings,
    pub(crate) policy: Arc<Policy>,
}
/// Blocking startup input; malformed settings retain the established defaults.
pub(crate) fn read_saved(data_dir: &DataDir) -> SetNodeSettings {
    match data_dir.read_settings() {
        Ok(saved) => saved.unwrap_or_default(),
        Err(error) => {
            tracing::warn!(%error, "settings.json was not used; the node starts from its flags");
            SetNodeSettings::default()
        }
    }
}
pub(crate) fn recovery(saved: &SetNodeSettings) -> vessel_proto::restart::RestartRecovery {
    saved
        .restart_recovery
        .as_ref()
        .map(vessel_proto::restart::SetRestartRecovery::effective)
        .unwrap_or_default()
}

#[cfg(test)]
impl Settings {
    pub(crate) fn pause_write(&self) -> (oneshot::Receiver<()>, std::sync::mpsc::SyncSender<()>) {
        self.engine.gates.arm_write()
    }
    pub(crate) fn pause_cleanup(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        self.engine.gates.arm_cleanup()
    }
}
