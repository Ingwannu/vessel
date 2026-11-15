//! Serialized provider generations: validate, preserve/write, then publish.
//! An admitted edit belongs to this task, so dropping its RPC waiter cannot
//! cancel between the durable write and the cache-generation replacement.

use super::{
    cache::Observation,
    owner::{Owner, OwnerError},
};
use std::future::Future;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use vessel_harness::providers::{PluginDefinition, ProviderRegistry, RegistryError};
use vessel_proto::providers::{Enabled, ProviderId, ProviderOverrides};
use vessel_store::{DataDir, ProvidersFileError};

const EDIT_CAPACITY: usize = 16;
type Reply = oneshot::Sender<Result<(), ConfigurationFailure>>;

/// A provider generation could not be validated, durably stored or published.
/// Native command/environment content is absent from formatted diagnostics.
#[derive(Debug, thiserror::Error)]
pub enum ConfigurationFailure {
    /// Complete registry validation failed before any durable mutation.
    #[error("provider configuration generation is invalid")]
    Invalid(#[source] RegistryError),
    /// Preservation or owner-only file replacement failed; old state stays.
    #[error("provider configuration cannot be kept")]
    Store(#[from] ProvidersFileError),
    /// A filesystem worker did not finish; its cause is retained internally.
    #[error("provider configuration task did not finish")]
    Task(#[from] tokio::task::JoinError),
    /// The node or its configuration actor already stopped.
    #[error("provider configuration owner is stopped")]
    Stopped,
    /// No configured provider can receive this targeted change.
    #[error("provider is not configured")]
    Unknown,
    /// The catalogue owner ended; no further generation is admitted.
    #[error("provider catalogue owner is unavailable")]
    Owner(#[from] OwnerError),
}

#[derive(Debug)]
enum Edit {
    #[cfg(test)]
    Replace {
        overrides: ProviderOverrides,
        reply: Reply,
    },
    Enabled {
        provider: ProviderId,
        enabled: Enabled,
        reply: Reply,
    },
    #[cfg(test)]
    Plugins {
        definitions: Vec<PluginDefinition>,
        reply: Reply,
    },
    Stop(oneshot::Sender<()>),
}

#[derive(Debug, Clone)]
pub(super) struct Configuration {
    edits: mpsc::Sender<Edit>,
}

/// Both owners come from one validated generation. The node must track their
/// tasks and stop configuration first, allowing admitted writes to finish.
pub(super) struct Started {
    pub(super) configuration: Configuration,
    pub(super) cache: Owner,
    pub(super) task: JoinHandle<()>,
    pub(super) catalogue_task: JoinHandle<()>,
}

/// A complete generation prepared before node runtime tasks are started.
#[derive(Debug)]
pub(super) struct Prepared {
    directory: DataDir,
    overrides: ProviderOverrides,
    plugins: Vec<PluginDefinition>,
    registry: ProviderRegistry,
}

impl Prepared {
    pub(super) fn read(
        directory: DataDir,
        plugins: Vec<PluginDefinition>,
    ) -> Result<Self, ConfigurationFailure> {
        let loaded = directory.load_provider_overrides(chrono::Utc::now())?;
        if loaded.state == vessel_store::ProviderFileState::Recovered {
            tracing::warn!(
                "unreadable provider configuration preserved; manifest defaults selected"
            );
        }
        let overrides = loaded.overrides;
        let registry =
            ProviderRegistry::build(&overrides, &plugins).map_err(ConfigurationFailure::Invalid)?;
        Ok(Self {
            directory,
            overrides,
            plugins,
            registry,
        })
    }

    pub(super) fn start<P, F>(self, producer: P) -> Started
    where
        P: Fn(super::cache::Work, CancellationToken) -> F + Send + Sync + 'static,
        F: Future<Output = Observation> + Send + 'static,
    {
        let Self {
            directory,
            overrides,
            plugins,
            registry,
        } = self;
        let (cache, catalogue_task) = Owner::start(registry.clone(), producer);
        let (edits, receiver) = mpsc::channel(EDIT_CAPACITY);
        let configuration = Configuration { edits };
        let task = tokio::spawn(run(
            directory,
            overrides,
            plugins,
            registry,
            cache.clone(),
            receiver,
        ));
        Started {
            configuration,
            cache,
            task,
            catalogue_task,
        }
    }
}

impl Configuration {
    #[cfg(test)]
    pub(super) async fn open<P, F>(
        directory: DataDir,
        plugins: Vec<PluginDefinition>,
        producer: P,
    ) -> Result<Started, ConfigurationFailure>
    where
        P: Fn(super::cache::Work, CancellationToken) -> F + Send + Sync + 'static,
        F: Future<Output = Observation> + Send + 'static,
    {
        let prepared =
            tokio::task::spawn_blocking(move || Prepared::read(directory, plugins)).await??;
        Ok(prepared.start(producer))
    }

    #[cfg(test)]
    pub(super) async fn replace(
        &self,
        overrides: ProviderOverrides,
    ) -> Result<(), ConfigurationFailure> {
        let (reply, response) = oneshot::channel();
        self.edits
            .send(Edit::Replace { overrides, reply })
            .await
            .map_err(|_closed| ConfigurationFailure::Stopped)?;
        response
            .await
            .map_err(|_closed| ConfigurationFailure::Stopped)?
    }

    pub(super) async fn set_enabled(
        &self,
        provider: ProviderId,
        enabled: Enabled,
    ) -> Result<(), ConfigurationFailure> {
        let (reply, response) = oneshot::channel();
        self.edits
            .send(Edit::Enabled {
                provider,
                enabled,
                reply,
            })
            .await
            .map_err(|_closed| ConfigurationFailure::Stopped)?;
        response
            .await
            .map_err(|_closed| ConfigurationFailure::Stopped)?
    }

    /// The plugin runtime owns trust/execution; this accepts inert definitions.
    #[cfg(test)]
    pub(super) async fn plugins(
        &self,
        definitions: Vec<PluginDefinition>,
    ) -> Result<(), ConfigurationFailure> {
        let (reply, response) = oneshot::channel();
        self.edits
            .send(Edit::Plugins { definitions, reply })
            .await
            .map_err(|_closed| ConfigurationFailure::Stopped)?;
        response
            .await
            .map_err(|_closed| ConfigurationFailure::Stopped)?
    }

    pub(super) async fn stop(&self) -> Result<(), ConfigurationFailure> {
        let (reply, response) = oneshot::channel();
        self.edits
            .send(Edit::Stop(reply))
            .await
            .map_err(|_closed| ConfigurationFailure::Stopped)?;
        response
            .await
            .map_err(|_closed| ConfigurationFailure::Stopped)
    }
}

async fn run(
    directory: DataDir,
    mut overrides: ProviderOverrides,
    mut plugins: Vec<PluginDefinition>,
    mut registry: ProviderRegistry,
    cache: Owner,
    mut receiver: mpsc::Receiver<Edit>,
) {
    while let Some(edit) = receiver.recv().await {
        let (next, next_plugins, reply) = match edit {
            Edit::Stop(reply) => {
                let _stopped = cache.stop().await;
                let _gone = reply.send(());
                return;
            }
            #[cfg(test)]
            Edit::Replace { overrides, reply } => (overrides, plugins.clone(), reply),
            #[cfg(test)]
            Edit::Plugins { definitions, reply } => (overrides.clone(), definitions, reply),
            Edit::Enabled {
                provider,
                enabled,
                reply,
            } => {
                if registry.get(&provider).is_none() {
                    let _gone = reply.send(Err(ConfigurationFailure::Unknown));
                    continue;
                }
                let mut entries = overrides.entries().clone();
                entries.entry(provider).or_default().enabled = Some(enabled);
                let next = match ProviderOverrides::new(entries) {
                    Ok(next) => next,
                    Err(error) => {
                        let _gone = reply.send(Err(ConfigurationFailure::Invalid(error.into())));
                        continue;
                    }
                };
                (next, plugins.clone(), reply)
            }
        };
        let result = commit(
            &directory,
            &overrides,
            &registry,
            &next,
            &next_plugins,
            &cache,
        )
        .await;
        let fatal = matches!(&result, Err(ConfigurationFailure::Owner(_)));
        if let Ok(next_registry) = &result {
            registry = next_registry.clone();
            overrides = next;
            plugins = next_plugins;
        }
        let _gone = reply.send(result.map(|_registry| ()));
        if fatal {
            break;
        }
    }
    let _stopped = cache.stop().await;
}

async fn commit(
    directory: &DataDir,
    current: &ProviderOverrides,
    current_registry: &ProviderRegistry,
    next: &ProviderOverrides,
    plugins: &[PluginDefinition],
    cache: &Owner,
) -> Result<ProviderRegistry, ConfigurationFailure> {
    let registry = ProviderRegistry::build(next, plugins).map_err(ConfigurationFailure::Invalid)?;
    if next != current {
        let (directory, next) = (directory.clone(), next.clone());
        tokio::task::spawn_blocking(move || {
            directory.save_provider_overrides(&next, chrono::Utc::now())
        })
        .await??;
    }
    if &registry != current_registry {
        cache.replace(registry.clone()).await?;
    }
    Ok(registry)
}

#[cfg(test)]
mod tests;
