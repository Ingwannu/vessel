//! The rows' bookkeeping (ADR 0115): saved intent, the installations
//! file, list items with Paseo's status, the change lock, the generation,
//! log lines, and blocking file work off the async threads.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::MutexGuard;

use vessel_plugin::SharedLogs;
use vessel_proto::plugins::{LogText, PluginId, PluginListItem, PluginSourceKind, PluginStatus};
use vessel_store::DataDir;

use super::{PluginError, Plugins, Row, Run, SETTLE_LIMIT, State, manifest, store};

impl Plugins {
    pub(super) async fn set_enabled(
        &self,
        plugin: &PluginId,
        enabled: bool,
    ) -> Result<(), PluginError> {
        let previous = {
            let mut state = self.state();
            let row = state
                .rows
                .get_mut(plugin)
                .ok_or_else(|| PluginError::NotInstalled(plugin.clone()))?;
            let previous = row.saved.enabled;
            row.saved.enabled = enabled;
            row.error = None;
            previous
        };
        if let Err(error) = self.persist().await {
            if let Some(row) = self.state().rows.get_mut(plugin) {
                row.saved.enabled = previous;
            }
            return Err(error);
        }
        self.bump();
        Ok(())
    }

    pub(super) fn remember_manifest(
        &self,
        plugin: &PluginId,
        attempt: u64,
        manifest: &manifest::Manifest,
    ) {
        if let Some(row) = self.state().rows.get_mut(plugin)
            && row.attempt == attempt
        {
            row.manifest = Some(manifest.clone());
        }
    }

    /// Write the installations as the rows hold them.
    pub(super) async fn persist(&self) -> Result<(), PluginError> {
        let installations: BTreeMap<PluginId, store::Installation> = self
            .state()
            .rows
            .iter()
            .map(|(id, row)| (id.clone(), row.saved.clone()))
            .collect();
        let data = self.inner.data.clone();
        blocking(move || store::write(&data, &installations))
            .await?
            .map_err(|error| PluginError::Store(error.to_string()))
    }

    pub(super) fn item(
        &self,
        plugin: &PluginId,
        admin: bool,
    ) -> Result<PluginListItem, PluginError> {
        let state = self.state();
        state
            .rows
            .get(plugin)
            .map(|row| list_item(plugin, row, state.switch, admin))
            .ok_or_else(|| PluginError::NotInstalled(plugin.clone()))
    }

    pub(super) async fn change(&self) -> Result<tokio::sync::MutexGuard<'_, ()>, PluginError> {
        if self.inner.stop.is_cancelled() {
            return Err(PluginError::Stopping);
        }
        Ok(self.inner.changes.lock().await)
    }

    /// Wait for the start-up settle, at most [`SETTLE_LIMIT`]: a list or
    /// catalog read at node start shows the configured helpers as started.
    pub(super) async fn settled(&self) {
        let mut settled = self.inner.settled.subscribe();
        let _settled = tokio::time::timeout(SETTLE_LIMIT, settled.wait_for(|settled| *settled))
            .await
            .is_ok_and(|settled| settled.is_ok());
    }

    pub(super) fn bump(&self) {
        self.inner.generation.send_modify(|generation| {
            *generation = generation.next();
        });
    }

    pub(super) fn state(&self) -> MutexGuard<'_, State> {
        // A panic while holding this lock leaves plain data; keep serving it.
        self.inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Paseo's `resolvePluginStatus`.
pub(super) fn status(row: &Row, switch: bool) -> PluginStatus {
    if !row.saved.enabled || !switch {
        return PluginStatus::Disabled;
    }
    match row.run {
        Run::Client { .. } | Run::Helper(_) => PluginStatus::Running,
        Run::Stopped => PluginStatus::Failed,
    }
}

pub(super) fn list_item(id: &PluginId, row: &Row, switch: bool, admin: bool) -> PluginListItem {
    let status = status(row, switch);
    let manifest = row.manifest.as_ref();
    let store::SavedSource::Directory { path } = &row.saved.source;
    PluginListItem {
        id: id.clone(),
        name: manifest.and_then(|manifest| manifest.name.clone()),
        description: manifest.and_then(|manifest| manifest.description.clone()),
        icon: manifest.and_then(|manifest| manifest.icon.clone()),
        path: admin.then(|| path.display().to_string()),
        source: PluginSourceKind::Directory,
        enabled: row.saved.enabled,
        status,
        error: if admin && status == PluginStatus::Failed {
            row.error.clone()
        } else {
            None
        },
    }
}

/// A trusted lifecycle line in the plugin's log, as Paseo's `[paseo]` lines.
pub(super) fn host_line(logs: &SharedLogs, what: &str) {
    let line = format!("[vessel] {what}");
    match line.parse::<LogText>() {
        Ok(text) => {
            if let Err(error) = logs.host(text) {
                tracing::debug!(%error, "a plugin log line was not kept");
            }
        }
        Err(_unsupported) => tracing::debug!("a plugin log line was not valid log text"),
    }
}

/// The installations file, or none: an unreadable one is moved aside so a
/// person's list is never overwritten.
pub(super) async fn load(data: &DataDir) -> BTreeMap<PluginId, store::Installation> {
    let reading = data.clone();
    match blocking(move || store::read(&reading)).await {
        Ok(Ok(installations)) => installations,
        Ok(Err(error)) => {
            let moving = data.clone();
            let aside =
                blocking(move || moving.set_aside_plugin_installations(chrono::Utc::now())).await;
            tracing::warn!(%error, ?aside, "plugins/installations.json could not be used; plugins start empty");
            BTreeMap::new()
        }
        Err(error) => {
            tracing::warn!(%error, "plugins/installations.json could not be read");
            BTreeMap::new()
        }
    }
}

/// The directory and manifest of a kept installation.
pub(super) async fn read_manifest(
    saved: &store::Installation,
    version: &semver::Version,
) -> Result<(PathBuf, manifest::Manifest), String> {
    let directory = saved
        .source
        .directory()
        .ok_or_else(|| "the kept plugin directory is not an absolute path".to_owned())?
        .into_path();
    let version = version.clone();
    let path = directory.clone();
    let manifest = blocking(move || manifest::read(&path, &version))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    Ok((directory, manifest))
}

/// This build's version, for `requirements.vessel`.
pub(super) fn node_version() -> &'static semver::Version {
    static VERSION: std::sync::OnceLock<semver::Version> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| {
        semver::Version::parse(env!("CARGO_PKG_VERSION"))
            .unwrap_or_else(|_unparsed| semver::Version::new(0, 0, 0))
    })
}

/// Run blocking file work off the async threads.
pub(super) async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, PluginError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_join| PluginError::Stopping)
}
