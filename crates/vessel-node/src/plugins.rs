//! The node's plugin host (ADR 0115 on ADR 0079): installations, the
//! switch, each installation's run, its log tail, and a generation that
//! moves on every change.
//!
//! [Decision Log]
//! - Purpose: Paseo's `PluginService`
//!   (`packages/server/src/server/plugins/index.ts:86-560`) keeps a host's
//!   installations, starts the enabled ones when plugins are on, and
//!   installs, enables, disables, reloads and removes them one change at a
//!   time; a row is `running`, `disabled` or `failed` by
//!   `resolvePluginStatus` (enabled and switched on: running if it runs,
//!   else failed; otherwise disabled).
//! - Existing pieces reused: `vessel-plugin`'s supervisor (process group,
//!   frames, ready deadline, bounded log tail), `vessel-store`'s owner-only
//!   atomic files, the node's settings file for the switch (ADR 0115).
//! - Choice: one async lock serialises changes (Paseo's `enqueue`); the rows
//!   sit behind a plain lock that is never held across an await, so a list
//!   is answered while a start waits for its helper. A start, stop or
//!   change bumps the generation `plugin.watch` waits on.
//! - A helper that ends on its own leaves its row failed with the
//!   supervisor's reason (Paseo: no longer in the runtime's catalog). Each
//!   run has an attempt number, so the watcher of a run that was stopped
//!   on purpose changes nothing.
//! - Start failures: install, enable and reload answer the failure as an
//!   error while the row stays failed (Paseo's `startExplicit` throws); at
//!   node start and when the switch turns on, a failure is only recorded
//!   (Paseo's `startConfigured`).
//! - The saved switch is in force from node start; a start-up task then
//!   runs the enabled installations not already running (Paseo's
//!   `start`), so an install made first is not started twice. A later
//!   switch is applied by a task that reads the latest wanted value when
//!   it gets the change lock, so quick toggles end in the last one.
//! - Trust: everything started here is trusted, unsandboxed code (ADR 0079).
//!   Callers are checked before they reach this module (Admin for every
//!   change, agents refused); this module keeps paths and errors private
//!   unless asked for an administrator's view.

mod catalog;
mod host;
mod manifest;
mod rows;
mod runs;
mod source;
mod store;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_plugin::{SharedLogs, Supervisor};
use vessel_proto::NodeId;
use vessel_proto::plugins::{
    ClientDocument, PluginAddress, PluginGeneration, PluginId, PluginInspection, PluginList,
    PluginListItem, PluginLogs,
};
use vessel_store::DataDir;

pub(crate) use catalog::CallFailure;
pub(crate) use manifest::ManifestError;
pub(crate) use source::SourceError;

/// How long `plugin.watch` waits before answering the same generation.
pub(crate) const WATCH_LIMIT: Duration = Duration::from_secs(25);
/// How long `plugin.list` waits for the first start at node start: one
/// helper's ready deadline and its stop grace, then it answers as it is.
const SETTLE_LIMIT: Duration = Duration::from_secs(35);

/// Why a plugin call failed, in the node's words.
#[derive(Debug, thiserror::Error)]
pub(crate) enum PluginError {
    /// No installation with that id (Paseo's "Plugin is not configured").
    #[error("plugin is not installed: {0}")]
    NotInstalled(PluginId),
    /// The id is taken.
    #[error("plugin id \"{0}\" is already installed; choose another id with --id")]
    Exists(PluginId),
    /// The source string.
    #[error(transparent)]
    Source(#[from] SourceError),
    /// The manifest.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// Reload of a disabled installation (Paseo's "Plugin is disabled").
    #[error("plugin is disabled: {0}")]
    Disabled(PluginId),
    /// Reload while plugins are off (Paseo's "Plugins are globally disabled").
    #[error("plugins are turned off on this node")]
    Off,
    /// It did not start; the row is failed with the same reason.
    #[error("plugin {id} failed to start: {reason}")]
    Failed {
        /// Which.
        id: PluginId,
        /// Why, in the node's words.
        reason: String,
    },
    /// The installations file could not be written.
    #[error("the plugin list could not be kept: {0}")]
    Store(String),
    /// The node is stopping.
    #[error("the node is stopping")]
    Stopping,
    /// Invoke on an installation that is not running (stopped, failed, or
    /// plugins are off).
    #[error("plugin is not running: {0}")]
    NotRunning(PluginId),
    /// Invoke on a client-only installation: no helper, no methods.
    #[error("plugin {0} has no helper program, so it has no methods")]
    NoMethods(PluginId),
    /// A method call failed, in a class safe for any administrator.
    #[error("plugin {id} could not run that: {failure}")]
    Call {
        /// Which.
        id: PluginId,
        /// Why.
        failure: catalog::CallFailure,
    },
    /// The running declarations exceed the node's catalog budget.
    #[error("the plugin catalog could not be built: {0}")]
    Catalog(String),
}

/// The node's plugins.
#[derive(Debug, Clone)]
pub(crate) struct Plugins {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    node: NodeId,
    data: DataDir,
    home: PathBuf,
    env: host::LaunchEnv,
    version: semver::Version,
    /// One change at a time (Paseo's `enqueue`).
    changes: tokio::sync::Mutex<()>,
    state: Mutex<State>,
    generation: watch::Sender<PluginGeneration>,
    settled: watch::Sender<bool>,
    wanted_switch: watch::Sender<bool>,
    tasks: TaskTracker,
    stop: CancellationToken,
}

#[derive(Debug, Default)]
struct State {
    switch: bool,
    rows: BTreeMap<PluginId, Row>,
}

#[derive(Debug)]
struct Row {
    saved: store::Installation,
    manifest: Option<manifest::Manifest>,
    logs: SharedLogs,
    run: Run,
    error: Option<String>,
    attempt: u64,
}

#[derive(Debug)]
enum Run {
    Stopped,
    /// A client-only installation: its document is its catalog entry.
    Client {
        address: PluginAddress,
        document: Arc<ClientDocument>,
    },
    Helper(Supervisor),
}

/// What the node gives the plugin host at start.
pub(crate) struct Opening {
    pub(crate) node: NodeId,
    pub(crate) data: DataDir,
    /// The node user's home (`~`).
    pub(crate) home: PathBuf,
    /// The switch as saved.
    pub(crate) switch: bool,
    pub(crate) tasks: TaskTracker,
    pub(crate) stop: CancellationToken,
}

impl Plugins {
    /// Read the installations and start the enabled ones in the background
    /// when plugins are on; stop every helper when `stop` is cancelled.
    pub(crate) async fn start(opening: Opening) -> Self {
        let Opening {
            node,
            data,
            home,
            switch,
            tasks,
            stop,
        } = opening;
        let installations = rows::load(&data).await;
        let mut rows = BTreeMap::new();
        for (id, saved) in installations {
            let manifest = rows::read_manifest(&saved, rows::node_version())
                .await
                .ok()
                .map(|(_directory, manifest)| manifest);
            let _none = rows.insert(
                id,
                Row {
                    saved,
                    manifest,
                    logs: SharedLogs::default(),
                    run: Run::Stopped,
                    error: None,
                    attempt: 0,
                },
            );
        }
        let plugins = Self {
            inner: Arc::new(Inner {
                node,
                env: host::LaunchEnv::from_node(&home),
                data,
                home,
                version: rows::node_version().clone(),
                changes: tokio::sync::Mutex::new(()),
                state: Mutex::new(State {
                    // In force from the first moment, so a change made
                    // before the start-up task runs sees plugins as on.
                    switch,
                    rows,
                }),
                generation: watch::Sender::new(PluginGeneration::default()),
                settled: watch::Sender::new(false),
                wanted_switch: watch::Sender::new(switch),
                tasks: tasks.clone(),
                stop: stop.clone(),
            }),
        };
        let starting = plugins.clone();
        drop(tasks.spawn(async move {
            starting.start_enabled().await;
            let _was = starting.inner.settled.send_replace(true);
        }));
        let stopping = plugins.clone();
        drop(tasks.spawn(async move {
            stop.cancelled().await;
            stopping.stop_all().await;
        }));
        plugins
    }

    /// Every installation, in id order; `admin` adds paths and errors.
    pub(crate) async fn list(&self, admin: bool) -> PluginList {
        self.settled().await;
        let state = self.state();
        PluginList {
            plugins: state
                .rows
                .iter()
                .map(|(id, row)| rows::list_item(id, row, state.switch, admin))
                .collect(),
        }
    }

    /// One installation's retained log tail.
    pub(crate) fn logs(&self, plugin: &PluginId) -> Result<PluginLogs, PluginError> {
        let logs = self
            .state()
            .rows
            .get(plugin)
            .map(|row| row.logs.clone())
            .ok_or_else(|| PluginError::NotInstalled(plugin.clone()))?;
        Ok(PluginLogs {
            entries: logs.snapshot().unwrap_or_default(),
        })
    }

    /// A directory's manifest identity, without installing it.
    pub(crate) async fn inspect(&self, path: &str) -> Result<PluginInspection, PluginError> {
        let directory = source::path(path, &self.inner.home)?;
        let version = self.inner.version.clone();
        let manifest = rows::blocking(move || manifest::read(&directory, &version)).await??;
        Ok(PluginInspection {
            id: manifest.id,
            name: manifest.name,
            description: manifest.description,
        })
    }

    /// Install from a source string; starts it when plugins are on.
    pub(crate) async fn install(
        &self,
        source: &str,
        id: Option<PluginId>,
        reference: Option<&str>,
    ) -> Result<PluginListItem, PluginError> {
        let _change = self.change().await?;
        let directory = source::directory(source, reference, &self.inner.home)?;
        let manifest = {
            let path = directory.path().to_path_buf();
            let version = self.inner.version.clone();
            rows::blocking(move || manifest::read(&path, &version)).await??
        };
        let id = id.unwrap_or_else(|| manifest.id.clone());
        {
            let mut state = self.state();
            if state.rows.contains_key(&id) {
                return Err(PluginError::Exists(id));
            }
            let _none = state.rows.insert(
                id.clone(),
                Row {
                    saved: store::Installation {
                        source: store::SavedSource::Directory {
                            path: directory.into_path(),
                        },
                        enabled: true,
                    },
                    manifest: Some(manifest),
                    logs: SharedLogs::default(),
                    run: Run::Stopped,
                    error: None,
                    attempt: 0,
                },
            );
        }
        if let Err(error) = self.persist().await {
            let _row = self.state().rows.remove(&id);
            return Err(error);
        }
        self.bump();
        if self.state().switch {
            self.start_one(&id).await?;
        }
        self.item(&id, true)
    }

    /// Save as enabled; start it when plugins are on.
    pub(crate) async fn enable(&self, plugin: &PluginId) -> Result<PluginListItem, PluginError> {
        let _change = self.change().await?;
        self.set_enabled(plugin, true).await?;
        let (switch, running) = {
            let state = self.state();
            let row = state.rows.get(plugin);
            (
                state.switch,
                row.is_some_and(|row| !matches!(row.run, Run::Stopped)),
            )
        };
        if switch && !running {
            self.start_one(plugin).await?;
        }
        self.item(plugin, true)
    }

    /// Save as disabled and stop it.
    pub(crate) async fn disable(&self, plugin: &PluginId) -> Result<PluginListItem, PluginError> {
        let _change = self.change().await?;
        self.set_enabled(plugin, false).await?;
        self.stop_one(plugin).await;
        self.bump();
        self.item(plugin, true)
    }

    /// Stop it and start it again from disk.
    pub(crate) async fn reload(&self, plugin: &PluginId) -> Result<PluginListItem, PluginError> {
        let _change = self.change().await?;
        {
            let state = self.state();
            let row = state
                .rows
                .get(plugin)
                .ok_or_else(|| PluginError::NotInstalled(plugin.clone()))?;
            if !row.saved.enabled {
                return Err(PluginError::Disabled(plugin.clone()));
            }
            if !state.switch {
                return Err(PluginError::Off);
            }
        }
        self.stop_one(plugin).await;
        self.start_one(plugin).await?;
        self.item(plugin, true)
    }

    /// Stop and forget it, with its settings and logs; a directory source
    /// stays on disk.
    pub(crate) async fn remove(&self, plugin: &PluginId) -> Result<(), PluginError> {
        let _change = self.change().await?;
        if !self.state().rows.contains_key(plugin) {
            return Err(PluginError::NotInstalled(plugin.clone()));
        }
        self.stop_one(plugin).await;
        let removed = self.state().rows.remove(plugin);
        if let Err(error) = self.persist().await {
            if let Some(row) = removed {
                let _none = self.state().rows.insert(plugin.clone(), row);
            }
            return Err(error);
        }
        let data = self.inner.data.clone();
        let id = plugin.clone();
        if let Ok(Err(error)) = rows::blocking(move || data.remove_plugin_settings(&id)).await {
            tracing::warn!(plugin = %plugin, %error, "a removed plugin's settings could not be deleted");
        }
        self.bump();
        Ok(())
    }

    /// Wait for a generation other than `after`, at most [`WATCH_LIMIT`].
    pub(crate) async fn watch(&self, after: PluginGeneration) -> PluginGeneration {
        let mut generation = self.inner.generation.subscribe();
        let _changed = tokio::select! {
            changed = tokio::time::timeout(
                WATCH_LIMIT,
                generation.wait_for(|current| *current != after),
            ) => changed.is_ok(),
            () = self.inner.stop.cancelled() => false,
        };
        *generation.borrow()
    }

    /// The switch was saved as `on`: apply it in the background.
    pub(crate) fn set_switch(&self, on: bool) {
        let _was = self.inner.wanted_switch.send_replace(on);
        let plugins = self.clone();
        drop(self.inner.tasks.spawn(async move {
            plugins.apply_switch().await;
        }));
    }
}
