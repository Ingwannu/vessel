//! Provider service composition: passive readers, explicit discovery and one
//! test-staged local contribution handle. Startup performs no discovery.

use super::{
    cache::Refresh,
    configuration::{self, Configuration, ConfigurationFailure},
    owner::{Owner, OwnerError},
    producer::Producer,
};
use crate::{config::Config, profiles::launch::McpSource};
use futures_util::{StreamExt as _, stream};
use std::{path::PathBuf, sync::Arc};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
#[cfg(test)]
use vessel_harness::providers::PluginDefinition;
use vessel_proto::providers::{
    CatalogueScope, DiagnosticRead, Enabled, ProviderDiagnostic, ProviderId, ProviderSnapshotEntry,
};
use vessel_store::DataDir;

/// Trusted node-side plugin supervision publishes inert definitions through
/// this handle after provenance/trust checks. It grants no helper execution or
/// caller RPC authority and must never be given to an untrusted plugin process.
#[cfg(test)]
#[derive(Debug, Clone)]
pub(super) struct ProviderRegistrations {
    configuration: Configuration,
    cache: Owner,
}

#[cfg(test)]
impl ProviderRegistrations {
    /// Scoped cache invalidations for trusted host-side consumers. Slow readers
    /// must reread after Lagged; shutdown emits Stopped/closes late receivers.
    #[must_use]
    pub(super) fn changes(&self) -> tokio::sync::broadcast::Receiver<super::owner::Change> {
        self.cache.changes()
    }
    /// Passive diagnostic projection for host diagnostics and other trusted node
    /// consumers. A miss remains `NotChecked`; this method never starts a process.
    ///
    /// # Errors
    /// Unknown provider, exhausted bounded cache, or owner shutdown.
    pub(super) async fn cached(
        &self,
        provider: ProviderId,
        scope: CatalogueScope,
    ) -> Result<ProviderDiagnostic, OwnerError> {
        self.cache.cached(provider, scope).await
    }

    /// Replace the complete settled contribution set, cancelling changed cache
    /// generations. The plugin runtime owns connection supervision/revocation.
    ///
    /// # Errors
    /// Invalid/colliding definitions retain the previous generation. A stopped
    /// node refuses updates, including handles retained after shutdown.
    pub(super) async fn replace_plugins(
        &self,
        definitions: Vec<PluginDefinition>,
    ) -> Result<(), ConfigurationFailure> {
        self.configuration.plugins(definitions).await
    }
}

/// Read and validate under the node's data-directory lock, before runtime tasks.
pub(crate) struct PreparedProviders(configuration::Prepared);

impl PreparedProviders {
    pub(crate) fn read(directory: DataDir) -> Result<Self, ConfigurationFailure> {
        configuration::Prepared::read(directory, vec![]).map(Self)
    }

    pub(crate) fn start(
        self,
        config: &Config,
        probe_dir: PathBuf,
        secrets: Arc<dyn McpSource>,
        tasks: &TaskTracker,
        stop: CancellationToken,
    ) -> Providers {
        let producer = Producer::new(
            config.harness_lookup.clone(),
            config.harness_env.clone(),
            probe_dir,
            secrets,
        );
        let configuration::Started {
            configuration,
            cache,
            task,
            catalogue_task,
        } = self.0.start(move |work, cancel| {
            let producer = producer.clone();
            async move { producer.observe(work, cancel).await }
        });
        drop(tasks.spawn(async move {
            if let Err(error) = task.await {
                tracing::error!(
                    panicked = error.is_panic(),
                    cancelled = error.is_cancelled(),
                    "provider configuration owner ended unexpectedly"
                );
            }
        }));
        drop(tasks.spawn(async move {
            if let Err(error) = catalogue_task.await {
                tracing::error!(
                    panicked = error.is_panic(),
                    cancelled = error.is_cancelled(),
                    "provider catalogue owner ended unexpectedly"
                );
            }
        }));
        let closing = configuration.clone();
        drop(tasks.spawn(async move {
            stop.cancelled().await;
            if let Err(_stopped) = closing.stop().await {
                tracing::debug!("provider owners were already stopped");
            }
        }));
        Providers {
            cache,
            configuration,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Providers {
    cache: Owner,
    configuration: Configuration,
}

impl Providers {
    #[cfg(test)]
    pub(super) fn registrations(&self) -> ProviderRegistrations {
        ProviderRegistrations {
            configuration: self.configuration.clone(),
            cache: self.cache.clone(),
        }
    }

    pub(crate) async fn diagnostic(
        &self,
        provider: ProviderId,
        scope: CatalogueScope,
        read: DiagnosticRead,
    ) -> Result<ProviderDiagnostic, OwnerError> {
        match read {
            DiagnosticRead::Cached => self.cache.cached(provider, scope).await,
            DiagnosticRead::Discover => self.cache.refresh(provider, scope, Refresh::Cached).await,
            DiagnosticRead::Refresh => self.cache.refresh(provider, scope, Refresh::Force).await,
        }
    }

    pub(crate) async fn snapshot(
        &self,
        scope: CatalogueScope,
        read: DiagnosticRead,
    ) -> Result<Vec<ProviderSnapshotEntry>, OwnerError> {
        if read != DiagnosticRead::Cached {
            let current = self.cache.snapshot(scope.clone()).await?;
            let cache = &self.cache;
            let refresh = if read == DiagnosticRead::Refresh {
                Refresh::Force
            } else {
                Refresh::Cached
            };
            let mut results = stream::iter(current)
                .map(|entry| {
                    let scope = scope.clone();
                    async move { cache.refresh(entry.provider, scope, refresh).await }
                })
                .buffer_unordered(super::owner::PER_PROVIDER);
            tokio::time::timeout(super::owner::REFRESH_TIMEOUT, async {
                while let Some(result) = results.next().await {
                    let _observed = result?;
                }
                Ok::<(), OwnerError>(())
            })
            .await
            .map_err(|_elapsed| OwnerError::Deadline)??;
        }
        self.cache.snapshot(scope).await
    }

    pub(crate) async fn set_enabled(
        &self,
        provider: ProviderId,
        enabled: Enabled,
    ) -> Result<ProviderSnapshotEntry, ConfigurationFailure> {
        self.configuration
            .set_enabled(provider.clone(), enabled)
            .await?;
        Ok(self
            .cache
            .cached(provider, CatalogueScope::Global)
            .await?
            .entry)
    }
}

#[cfg(test)]
mod tests;
