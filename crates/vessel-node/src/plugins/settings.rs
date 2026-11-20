//! A node's plugin settings documents (ADR 0115, "Settings", 2026-11-20):
//! the store the node keeps for its helpers and the three Admin calls.
//!
//! [Decision Log]
//! - Purpose: Paseo's `PluginSettingsStore`
//!   (packages/server/src/server/plugins/settings/index.ts) keeps each
//!   installation's settings documents with a schema version, migration and
//!   an optimistic revision; clients read, save and reset them.
//! - Constraint: a Vessel helper is any program; its file access stays
//!   narrow, so it never writes its own settings files (a stated difference
//!   from Paseo, coordinator ruling 2026-11-20). The node writes them and
//!   asks the helper, which holds the schema, to parse and migrate.
//! - Choice, Paseo's rules over the node's store:
//!   - the file is `plugin-settings/<id>/<settings>.json`, the envelope
//!     `{version, values}`, written owner-only and atomically; the revision
//!     is the sha256 of its bytes, or `missing`;
//!   - read: missing parses `{}`; the declared version parses; an older one
//!     migrates and is saved once; a newer one or unreadable bytes are
//!     invalid and stay on disk;
//!   - write: another revision is a conflict; over a stored document of
//!     another version it is invalid; reset parses `{}` whatever the version;
//!   - each save, reset or migration tells the helper (`changed`, its
//!     `subscribe()`) and moves `plugin.watch` (ADR 0115).
//! - One lock per installation orders reads that migrate, writes and resets,
//!   and the helper's notices with them; `remove` takes it before deleting
//!   the folder, so a write in flight cannot bring the folder back.
//! - Cost: every read asks the helper to parse (Paseo parses on every read
//!   too), one round trip within the actor's budget and deadline.

use std::fmt::Write as _;
use std::sync::{Arc, PoisonError};

use sha2::{Digest as _, Sha256};
use vessel_plugin::{CallError, RuntimeHandle, RuntimeState, SharedLogs};
use vessel_proto::RawJson;
use vessel_proto::plugins::{
    HelperCall, HelperFailure, HelperPayload, PluginAddress, PluginId, PluginSettingsRead,
    PluginSettingsWrite, ResetPluginSettings, SettingsId, SettingsMessage, SettingsNotice,
    SettingsRevision, SettingsStep, SettingsVerdict, SettingsVersion, WritePluginSettings,
};

use super::rows::{blocking, host_line};
use super::{CallFailure, PluginError, Plugins, Run};

/// Paseo's conflict words.
const CONFLICT: &str = "Settings changed on another client. Reload before saving again.";
/// Paseo's words for a stored version newer than the plugin's.
const NEWER: &str = "Settings were saved by a newer plugin version";
/// Paseo's words for a save over a stored document of another version.
const OTHER_VERSION: &str = "Reload or reset settings before saving a different schema version";
/// Stored bytes that are not an envelope.
const UNREADABLE: &str = "The stored settings are not readable. Reset them to start again.";

/// The stored envelope (Paseo's `{version, values}`).
#[derive(serde::Deserialize)]
struct Envelope {
    version: SettingsVersion,
    values: RawJson,
}

/// The envelope as written.
#[derive(serde::Serialize)]
struct EnvelopeOut<'a> {
    version: SettingsVersion,
    values: &'a RawJson,
}

/// A document as it is on disk.
enum Stored {
    Missing,
    Unreadable(SettingsRevision),
    Saved {
        revision: SettingsRevision,
        version: SettingsVersion,
        values: RawJson,
    },
}

impl Stored {
    fn revision(&self) -> SettingsRevision {
        match self {
            Self::Missing => SettingsRevision::missing(),
            Self::Unreadable(revision) | Self::Saved { revision, .. } => revision.clone(),
        }
    }
}

/// The running helper a document belongs to.
struct Target {
    plugin: PluginId,
    settings: SettingsId,
    handle: RuntimeHandle,
    address: PluginAddress,
    version: SettingsVersion,
    logs: SharedLogs,
}

/// Whether a read that migrates tells the helper itself; the delivery
/// after `ready` tells it every document's state once instead.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Migrated {
    Announce,
    Quiet,
}

impl Plugins {
    /// `plugin.settings.read`.
    pub(crate) async fn read_settings(
        &self,
        plugin: &PluginId,
        settings: &SettingsId,
    ) -> Result<PluginSettingsRead, PluginError> {
        let lock = self.settings_lock(plugin);
        let _held = lock.lock().await;
        let target = self.settings_target(plugin, settings)?;
        self.settings_read_locked(&target, Migrated::Announce).await
    }

    /// `plugin.settings.write`.
    pub(crate) async fn write_settings(
        &self,
        request: WritePluginSettings,
    ) -> Result<PluginSettingsWrite, PluginError> {
        let WritePluginSettings {
            plugin,
            settings,
            revision,
            values,
        } = request;
        let lock = self.settings_lock(&plugin);
        let _held = lock.lock().await;
        let target = self.settings_target(&plugin, &settings)?;
        let stored = self.settings_stored(&target).await?;
        if stored.revision() != revision {
            return Ok(PluginSettingsWrite::Conflict {
                error: SettingsMessage::bounded(CONFLICT),
            });
        }
        match stored {
            Stored::Unreadable(_) => Ok(PluginSettingsWrite::Invalid {
                error: SettingsMessage::bounded(UNREADABLE),
            }),
            Stored::Saved { version, .. } if version != target.version => {
                Ok(PluginSettingsWrite::Invalid {
                    error: SettingsMessage::bounded(OTHER_VERSION),
                })
            }
            Stored::Missing | Stored::Saved { .. } => self.settings_save(&target, values).await,
        }
    }

    /// `plugin.settings.reset`: the schema's defaults, whatever is stored.
    pub(crate) async fn reset_settings(
        &self,
        request: ResetPluginSettings,
    ) -> Result<PluginSettingsWrite, PluginError> {
        let ResetPluginSettings {
            plugin,
            settings,
            revision,
        } = request;
        let lock = self.settings_lock(&plugin);
        let _held = lock.lock().await;
        let target = self.settings_target(&plugin, &settings)?;
        if self.settings_stored(&target).await?.revision() != revision {
            return Ok(PluginSettingsWrite::Conflict {
                error: SettingsMessage::bounded(CONFLICT),
            });
        }
        self.settings_save(&target, RawJson::empty_object()).await
    }

    /// After a helper's `ready`: tell it each declared document's state
    /// (its `read()`), migrating where needed. Failures go to the log.
    pub(super) async fn deliver_settings(&self, plugin: &PluginId, declared: &[SettingsId]) {
        let lock = self.settings_lock(plugin);
        let _held = lock.lock().await;
        for settings in declared {
            let Ok(target) = self.settings_target(plugin, settings) else {
                return;
            };
            let notice = match self.settings_read_locked(&target, Migrated::Quiet).await {
                Ok(PluginSettingsRead::Ready { revision, values }) => {
                    match HelperPayload::try_from(values) {
                        Ok(values) => SettingsNotice::Ready { revision, values },
                        Err(_large) => continue,
                    }
                }
                Ok(PluginSettingsRead::Invalid { revision, .. }) => {
                    SettingsNotice::Invalid { revision }
                }
                Err(error) => {
                    tracing::debug!(plugin = %plugin, %error, "a plugin's settings were not delivered");
                    continue;
                }
            };
            self.settings_announce(&target, notice).await;
        }
    }

    /// The per-installation lock.
    pub(super) fn settings_lock(&self, plugin: &PluginId) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self
            .inner
            .settings_locks
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        Arc::clone(locks.entry(plugin.clone()).or_default())
    }

    /// The running helper that declares `settings`.
    fn settings_target(
        &self,
        plugin: &PluginId,
        settings: &SettingsId,
    ) -> Result<Target, PluginError> {
        let wanted = *self.inner.wanted_switch.borrow();
        let state = self.state();
        let row = state
            .rows
            .get(plugin)
            .ok_or_else(|| PluginError::NotInstalled(plugin.clone()))?;
        let none = || PluginError::NoSettings {
            id: plugin.clone(),
            settings: settings.clone(),
        };
        if !(wanted && state.switch && row.saved.enabled) {
            return Err(PluginError::NotRunning(plugin.clone()));
        }
        let supervisor = match &row.run {
            Run::Helper(supervisor) => supervisor,
            Run::Client { .. } => return Err(none()),
            Run::Stopped => return Err(PluginError::NotRunning(plugin.clone())),
        };
        let handle = supervisor.handle();
        let (address, version) = match &*handle.subscribe().borrow() {
            RuntimeState::Running {
                address,
                settings: declared,
                ..
            } => (
                address.clone(),
                declared.version_of(settings).ok_or_else(none)?,
            ),
            RuntimeState::Starting | RuntimeState::Stopped => {
                return Err(PluginError::NotRunning(plugin.clone()));
            }
        };
        Ok(Target {
            plugin: plugin.clone(),
            settings: settings.clone(),
            handle,
            address,
            version,
            logs: row.logs.clone(),
        })
    }

    /// Read under the lock; a migration is saved once.
    async fn settings_read_locked(
        &self,
        target: &Target,
        migrated: Migrated,
    ) -> Result<PluginSettingsRead, PluginError> {
        let (revision, step) = match self.settings_stored(target).await? {
            Stored::Missing => (SettingsRevision::missing(), parse(RawJson::empty_object())?),
            Stored::Unreadable(revision) => {
                return Ok(PluginSettingsRead::Invalid {
                    revision,
                    error: SettingsMessage::bounded(UNREADABLE),
                });
            }
            Stored::Saved {
                revision, version, ..
            } if version > target.version => {
                return Ok(PluginSettingsRead::Invalid {
                    revision,
                    error: SettingsMessage::bounded(NEWER),
                });
            }
            Stored::Saved {
                revision,
                version,
                values,
            } => {
                let values = payload(target, values)?;
                let step = if version == target.version {
                    SettingsStep::Parse { values }
                } else {
                    SettingsStep::Migrate {
                        from: version,
                        values,
                    }
                };
                (revision, step)
            }
        };
        let migrating = matches!(step, SettingsStep::Migrate { .. });
        match self.settings_judge(target, step).await? {
            Err(error) => Ok(PluginSettingsRead::Invalid { revision, error }),
            Ok(values) if migrating => {
                let revision = self.settings_persist(target, &values).await?;
                if migrated == Migrated::Announce {
                    self.settings_announce_ready(target, &revision, &values)
                        .await;
                }
                self.bump();
                Ok(PluginSettingsRead::Ready { revision, values })
            }
            Ok(values) => Ok(PluginSettingsRead::Ready { revision, values }),
        }
    }

    /// Parse `values`, then save, tell the helper and move the watch.
    async fn settings_save(
        &self,
        target: &Target,
        values: RawJson,
    ) -> Result<PluginSettingsWrite, PluginError> {
        let step = SettingsStep::Parse {
            values: payload(target, values)?,
        };
        match self.settings_judge(target, step).await? {
            Err(error) => Ok(PluginSettingsWrite::Invalid { error }),
            Ok(values) => {
                let revision = self.settings_persist(target, &values).await?;
                self.settings_announce_ready(target, &revision, &values)
                    .await;
                self.bump();
                Ok(PluginSettingsWrite::Saved { revision, values })
            }
        }
    }

    /// The document on disk.
    async fn settings_stored(&self, target: &Target) -> Result<Stored, PluginError> {
        let data = self.inner.data.clone();
        let (plugin, settings) = (target.plugin.clone(), target.settings.clone());
        let bytes = blocking(move || data.read_plugin_settings(&plugin, &settings))
            .await?
            .map_err(|error| PluginError::SettingsStore(error.to_string()))?;
        let Some(bytes) = bytes else {
            return Ok(Stored::Missing);
        };
        let revision = revision_of(&bytes);
        Ok(match serde_json::from_slice::<Envelope>(&bytes) {
            Ok(Envelope { version, values }) => Stored::Saved {
                revision,
                version,
                values,
            },
            Err(_unreadable) => Stored::Unreadable(revision),
        })
    }

    /// Write `values` at the declared version; its new revision.
    async fn settings_persist(
        &self,
        target: &Target,
        values: &RawJson,
    ) -> Result<SettingsRevision, PluginError> {
        let raw = serde_json::to_vec(&EnvelopeOut {
            version: target.version,
            values,
        })
        .map_err(|error| PluginError::SettingsStore(error.to_string()))?;
        let revision = revision_of(&raw);
        let data = self.inner.data.clone();
        let (plugin, settings) = (target.plugin.clone(), target.settings.clone());
        blocking(move || data.write_plugin_settings(&plugin, &settings, &raw))
            .await?
            .map_err(|error| PluginError::SettingsStore(error.to_string()))?;
        Ok(revision)
    }

    /// Ask the helper to parse or migrate: the values, or its refusal in
    /// bounded words (its full words go to its log).
    async fn settings_judge(
        &self,
        target: &Target,
        step: SettingsStep,
    ) -> Result<Result<RawJson, SettingsMessage>, PluginError> {
        let output = self.settings_ask(target, step).await?;
        let answer = RawJson::from(output);
        match serde_json::from_str::<SettingsVerdict>(answer.as_text()) {
            Ok(SettingsVerdict::Valid { values }) => Ok(Ok(values)),
            Ok(SettingsVerdict::Invalid { message }) => {
                host_line(
                    &target.logs,
                    &format!("settings {} refused: {message}", target.settings),
                );
                Ok(Err(SettingsMessage::bounded(&message)))
            }
            Err(_malformed) => {
                host_line(
                    &target.logs,
                    &format!("settings {}: the answer is not a verdict", target.settings),
                );
                Err(PluginError::Call {
                    id: target.plugin.clone(),
                    failure: CallFailure::Failed,
                })
            }
        }
    }

    /// Tell the helper a document's state; its answer is not read.
    async fn settings_announce(&self, target: &Target, notice: SettingsNotice) {
        if let Err(error) = self
            .settings_ask(target, SettingsStep::Changed { state: notice })
            .await
        {
            tracing::debug!(plugin = %target.plugin, %error, "a settings notice was not delivered");
        }
    }

    async fn settings_announce_ready(
        &self,
        target: &Target,
        revision: &SettingsRevision,
        values: &RawJson,
    ) {
        if let Ok(values) = HelperPayload::try_from(values.clone()) {
            let notice = SettingsNotice::Ready {
                revision: revision.clone(),
                values,
            };
            self.settings_announce(target, notice).await;
        }
    }

    async fn settings_ask(
        &self,
        target: &Target,
        step: SettingsStep,
    ) -> Result<HelperPayload, PluginError> {
        // The node's stop cancels a call in flight, as `invoke`'s.
        let cancel = self.inner.stop.child_token();
        let call = HelperCall::Settings {
            settings: target.settings.clone(),
            step,
        };
        target
            .handle
            .call(target.address.clone(), call, &cancel)
            .await
            .map_err(|error| PluginError::Call {
                id: target.plugin.clone(),
                failure: failure_of(error),
            })
    }
}

/// `values` within the helper payload budget.
fn payload(target: &Target, values: RawJson) -> Result<HelperPayload, PluginError> {
    HelperPayload::try_from(values).map_err(|_large| PluginError::Call {
        id: target.plugin.clone(),
        failure: CallFailure::TooLarge,
    })
}

/// The step that parses `values` (a missing document's `{}`).
fn parse(values: RawJson) -> Result<SettingsStep, PluginError> {
    HelperPayload::try_from(values)
        .map(|values| SettingsStep::Parse { values })
        .map_err(|_large| PluginError::SettingsStore("an empty document is over budget".to_owned()))
}

/// The safe class of a runtime failure. A declaration that vanished
/// between the check and the call (a new run) reads as a change.
const fn failure_of(error: CallError) -> CallFailure {
    match error {
        CallError::Unregistered | CallError::Stale => CallFailure::Changed,
        CallError::Failed(HelperFailure::Refused) => CallFailure::Refused,
        CallError::Failed(HelperFailure::Callback) => CallFailure::Failed,
        CallError::NotReady
        | CallError::Busy
        | CallError::Cancelled
        | CallError::Timeout
        | CallError::Closed
        | CallError::Failed(HelperFailure::Cancelled) => CallFailure::Unavailable,
    }
}

/// The sha256 of `bytes` in lowercase hex.
fn revision_of(bytes: &[u8]) -> SettingsRevision {
    let mut hex = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _infallible = write!(hex, "{byte:02x}");
    }
    SettingsRevision::from_sha256_hex(&hex).unwrap_or_else(SettingsRevision::missing)
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
