//! `agent.catalogue` (protocol 11, ADR 0032): the node's copy of the ACP
//! registry, fetched only when a person asks, and the entries
//! `agent.add` takes from it.
//!
//! [Decision Log]
//! - Purpose: a person adds an agent from the registry without writing its
//!   command, and sees exactly what would run before it is added.
//! - Constraint: listing runs and installs nothing; the registry is a
//!   network service the node must not depend on to start, and its answer
//!   names programs.
//! - Choice: one copy in memory, empty until the first fetch, replaced
//!   only by a fetch that reads (a failed one keeps the previous copy);
//!   one fetch at a time, and a refresh within a minute of the last fetch
//!   answers the copy. `agent.add` from the registry takes the entry from
//!   this copy, never from the client, so what is written is what the
//!   catalogue showed.
//! - Cost: the copy is at most the reader's bounds (1,000 entries).

mod fetch;
mod registry;
#[cfg(test)]
mod tests;

use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use chrono::{DateTime, Utc};
use vessel_harness::Lookup;
use vessel_proto::HarnessKind;
use vessel_proto::agents::{Catalogue, CatalogueEntry, RegistryId, RegistryVersion};

pub(crate) use fetch::FetchError;
use fetch::Fetcher;
pub(crate) use fetch::REGISTRY_URL;
use registry::{NotTheRegistry, Platform};

/// A refresh this soon after a fetch answers the copy.
const REFETCH_AFTER: Duration = Duration::from_secs(60);

/// Why the registry could not be read; the previous copy stays.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CatalogueError {
    #[error(transparent)]
    Fetch(#[from] FetchError),
    #[error(transparent)]
    Shape(#[from] NotTheRegistry),
}

/// The node's copy of the registry.
#[derive(Debug)]
pub(crate) struct Catalogues {
    fetcher: Result<Fetcher, String>,
    copy: Mutex<Catalogue>,
    fetching: tokio::sync::Mutex<()>,
}

impl Catalogues {
    /// A catalogue fetched from `url` (`Config::registry_url`).
    pub(crate) fn new(url: &str) -> Self {
        Self {
            fetcher: Fetcher::new(url).map_err(|error| error.to_string()),
            copy: Mutex::new(Catalogue::default()),
            fetching: tokio::sync::Mutex::new(()),
        }
    }

    /// The copy as it is.
    pub(crate) fn current(&self) -> Catalogue {
        self.copy
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Fetch and read the registry, unless the copy is younger than a
    /// minute; the copy afterwards.
    ///
    /// # Errors
    /// The registry could not be fetched or read; the copy is unchanged.
    pub(crate) async fn refresh(&self, lookup: &Lookup) -> Result<Catalogue, CatalogueError> {
        let _one = self.fetching.lock().await;
        if let Some(fetched_at) = self.current().fetched_at
            && young(fetched_at)
        {
            return Ok(self.current());
        }
        let fetcher = self
            .fetcher
            .as_ref()
            .map_err(|_error| CatalogueError::Fetch(FetchError::Address))?;
        let bytes = fetcher.fetch().await?;
        let lookup = lookup.clone();
        let read = tokio::task::spawn_blocking(move || {
            let known = known_packages();
            let known: Vec<(&str, HarnessKind)> = known
                .iter()
                .map(|(name, kind)| (name.as_str(), *kind))
                .collect();
            registry::read(&bytes, Platform::here(), &known, |runner| {
                lookup.find(runner).is_some()
            })
        })
        .await
        .map_err(|_error| CatalogueError::Shape(NotTheRegistry("the reader stopped")))??;
        tracing::info!(
            entries = read.entries.len(),
            skipped = read.skipped,
            "the ACP registry was read"
        );
        let catalogue = Catalogue {
            fetched_at: Some(Utc::now()),
            version: Some(read.version),
            entries: read.entries,
            skipped: read.skipped,
        };
        *self.copy.lock().unwrap_or_else(PoisonError::into_inner) = catalogue.clone();
        Ok(catalogue)
    }

    /// The entry `id` at `version` in the copy.
    pub(crate) fn entry(
        &self,
        id: &RegistryId,
        version: &RegistryVersion,
    ) -> Option<CatalogueEntry> {
        self.copy
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entries
            .iter()
            .find(|entry| entry.id == *id && entry.version == *version)
            .cloned()
    }
}

/// Whether a fetch at `fetched_at` is younger than [`REFETCH_AFTER`].
fn young(fetched_at: DateTime<Utc>) -> bool {
    chrono::Duration::from_std(REFETCH_AFTER)
        .ok()
        .and_then(|after| fetched_at.checked_add_signed(after))
        .is_some_and(|until| Utc::now() < until)
}

/// The known agents' packages, as their installs name them: an entry
/// running one of them is that agent. An install a person runs by hand (a
/// script, a download) names no package.
fn known_packages() -> Vec<(String, HarnessKind)> {
    HarnessKind::ALL
        .into_iter()
        .flat_map(|kind| {
            vessel_harness::installs(kind)
                .into_iter()
                .filter(|install| !install.by_hand)
                .filter_map(move |install| {
                    install
                        .args()
                        .last()
                        .map(|package| ((*package).to_owned(), kind))
                })
        })
        .collect()
}
