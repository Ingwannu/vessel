//! Starting and stopping installations' runs (ADR 0115): the switch, one
//! start, keeping what started, watching a helper that ends on its own,
//! and stopping. Every lock on the rows is released before an await.

use tokio::sync::watch;
use vessel_plugin::RuntimeState;
use vessel_proto::plugins::{PluginAddress, PluginId, RuntimeGeneration};

use super::rows::{host_line, read_manifest};
use super::{PluginError, Plugins, Run, host};

impl Plugins {
    /// Bring the runs in line with the latest wanted switch.
    pub(super) async fn apply_switch(&self) {
        let Ok(_change) = self.change().await else {
            return;
        };
        let on = *self.inner.wanted_switch.borrow();
        let ids: Vec<PluginId> = {
            let mut state = self.state();
            if state.switch == on {
                return;
            }
            state.switch = on;
            state.rows.keys().cloned().collect()
        };
        for id in &ids {
            if on {
                let enabled = self
                    .state()
                    .rows
                    .get(id)
                    .is_some_and(|row| row.saved.enabled);
                if enabled {
                    // Recorded on the row; nobody waits for this answer.
                    let _recorded = self.start_one(id).await;
                }
            } else {
                self.stop_one(id).await;
            }
        }
        self.bump();
    }

    /// Start one installation (its row exists); the row ends running or
    /// failed.
    pub(super) async fn start_one(&self, plugin: &PluginId) -> Result<(), PluginError> {
        let (saved, logs, attempt) = {
            let mut state = self.state();
            let row = state
                .rows
                .get_mut(plugin)
                .ok_or_else(|| PluginError::NotInstalled(plugin.clone()))?;
            row.attempt = row.attempt.wrapping_add(1);
            row.error = None;
            (row.saved.clone(), row.logs.clone(), row.attempt)
        };
        host_line(&logs, "loading");
        let started = match read_manifest(&saved, &self.inner.version).await {
            Err(reason) => Err(reason),
            Ok((directory, manifest)) => {
                let address = PluginAddress {
                    node: self.inner.node,
                    plugin: plugin.clone(),
                    generation: RuntimeGeneration::random(),
                };
                self.remember_manifest(plugin, attempt, &manifest);
                host::start(
                    &manifest,
                    &directory,
                    address,
                    &logs,
                    &self.inner.env,
                    &self.inner.stop,
                )
                .await
                .map_err(|error| error.to_string())
            }
        };
        let result = match started {
            Ok(started) => {
                host_line(&logs, "ready");
                self.settle_started(plugin, attempt, started).await;
                Ok(())
            }
            Err(reason) => {
                host_line(&logs, &format!("failed: {reason}"));
                let mut state = self.state();
                if let Some(row) = state.rows.get_mut(plugin)
                    && row.attempt == attempt
                {
                    row.error = Some(reason.clone());
                }
                Err(PluginError::Failed {
                    id: plugin.clone(),
                    reason,
                })
            }
        };
        self.bump();
        result
    }

    /// Keep what started on its row, or stop it when the row moved on.
    pub(super) async fn settle_started(
        &self,
        plugin: &PluginId,
        attempt: u64,
        started: host::Started,
    ) {
        let watched = match &started {
            host::Started::Helper(supervisor) => Some(supervisor.handle().subscribe()),
            host::Started::Client => None,
        };
        let late = {
            let mut state = self.state();
            match state.rows.get_mut(plugin) {
                Some(row) if row.attempt == attempt => {
                    row.run = match started {
                        host::Started::Helper(supervisor) => Run::Helper(supervisor),
                        host::Started::Client => Run::Client,
                    };
                    None
                }
                Some(_) | None => Some(started),
            }
        };
        if let Some(host::Started::Helper(supervisor)) = late {
            let _receipt = supervisor.shutdown().await;
            return;
        }
        if let Some(state) = watched {
            let plugins = self.clone();
            let plugin = plugin.clone();
            drop(self.inner.tasks.spawn(async move {
                plugins.watch_helper(plugin, attempt, state).await;
            }));
        }
    }

    /// Mark the row failed when its helper ends on its own.
    pub(super) async fn watch_helper(
        &self,
        plugin: PluginId,
        attempt: u64,
        mut state: watch::Receiver<RuntimeState>,
    ) {
        // A closed sender also means the actor ended; the row is checked
        // either way. The watch reference is not kept across the awaits below.
        let _ended = state
            .wait_for(|state| matches!(state, RuntimeState::Stopped))
            .await
            .is_ok();
        let supervisor = {
            let mut rows = self.state();
            match rows.rows.get_mut(&plugin) {
                Some(row) if row.attempt == attempt => {
                    match std::mem::replace(&mut row.run, Run::Stopped) {
                        Run::Helper(supervisor) => Some((supervisor, row.logs.clone())),
                        other @ (Run::Stopped | Run::Client) => {
                            row.run = other;
                            None
                        }
                    }
                }
                Some(_) | None => None,
            }
        };
        let Some((supervisor, logs)) = supervisor else {
            return;
        };
        let reason = match supervisor.shutdown().await {
            Err(error) => error.to_string(),
            Ok(_receipt) => "plugin helper stopped".to_owned(),
        };
        host_line(&logs, &format!("stopped: {reason}"));
        if let Some(row) = self.state().rows.get_mut(&plugin)
            && row.attempt == attempt
        {
            row.error = Some(reason);
        }
        self.bump();
    }

    /// Stop one installation's run, if any, and wait for its helper.
    pub(super) async fn stop_one(&self, plugin: &PluginId) {
        let taken = {
            let mut state = self.state();
            state.rows.get_mut(plugin).map(|row| {
                row.attempt = row.attempt.wrapping_add(1);
                row.error = None;
                (
                    std::mem::replace(&mut row.run, Run::Stopped),
                    row.logs.clone(),
                )
            })
        };
        let Some((run, logs)) = taken else {
            return;
        };
        match run {
            Run::Helper(supervisor) => {
                host_line(&logs, "stopping");
                if let Err(error) = supervisor.shutdown().await {
                    tracing::warn!(plugin = %plugin, %error, "a plugin helper did not stop cleanly");
                }
                host_line(&logs, "stopped");
            }
            Run::Client => host_line(&logs, "stopped"),
            Run::Stopped => {}
        }
    }

    /// At node stop: every helper, joined before the node's tasks end.
    pub(super) async fn stop_all(&self) {
        let ids: Vec<PluginId> = self.state().rows.keys().cloned().collect();
        for id in &ids {
            self.stop_one(id).await;
        }
    }
}
