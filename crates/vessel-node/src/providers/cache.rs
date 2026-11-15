//! Single-owner provider cache reducer. It starts no process and reads no file.
//! Async execution owns admitted work separately and returns it by ticket.

use chrono::{DateTime, Utc};
use std::{collections::BTreeMap, sync::Arc};
use vessel_harness::providers::{ProviderDefinition, ProviderRegistry, merge_models};
use vessel_proto::{
    agents::AgentVersion,
    providers::{
        Catalogue, CatalogueScope, DiscoveryFailure, Enabled, ProviderDiagnostic, ProviderId,
        ProviderSnapshotEntry, ProviderState, UnavailableReason,
    },
};

/// Bound retained provider/scope catalogues, including pending observations.
pub(super) const MAX_CATALOGUES: usize = 256;

/// Warm-up coalesces an existing load; explicit refresh supersedes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Refresh {
    Cached,
    Force,
}

/// Execution capacity affects only new work, never a passive/coalesced answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkBudget {
    Available,
    Full,
}

/// Generation identity is node-minted and cannot silently wrap into old work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Ticket(u64);

/// Only the current ticket for a definition/cache key may publish a result.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    provider: ProviderId,
    scope: CatalogueScope,
}

#[derive(Debug)]
struct Slot {
    state: ProviderState,
    version: Option<AgentVersion>,
    pending: Option<Ticket>,
}

/// Work includes an immutable definition; a newer generation cannot retarget it.
#[derive(Debug, Clone)]
pub(super) struct Work {
    pub(super) ticket: Ticket,
    pub(super) definition: Arc<ProviderDefinition>,
    pub(super) scope: CatalogueScope,
    pub(super) supersedes: Option<Ticket>,
}

/// No-work outcomes are explicit, so a passive cache miss cannot launch a probe.
#[derive(Debug)]
pub(super) enum Admission {
    Ready(ProviderDiagnostic),
    Joined(Ticket),
    Start(Work),
}

/// Producer observation, before configuration model overrides are applied.
#[derive(Debug)]
pub(super) enum Observation {
    Ready {
        catalogue: Catalogue,
        version: Option<AgentVersion>,
    },
    Unavailable(UnavailableReason),
    Failed(DiscoveryFailure),
}

/// Unknown providers and exhausted resources are not silently defaulted.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CacheError {
    /// A list of individually bounded catalogues cannot fit in one reply.
    #[error("provider snapshot exceeds its encoded byte budget")]
    ReplyTooLarge,
    /// The effective registry has no such provider identity.
    #[error("provider is not configured")]
    UnknownProvider,
    /// The bounded provider/scope catalogue cache is full.
    #[error("provider catalogue cache exceeds {MAX_CATALOGUES} entries")]
    Capacity,
    /// Discovery execution has no free bounded admission slot.
    #[error("provider discovery execution capacity is full")]
    Busy,
    /// A fresh generation cannot be issued without wrapping its identity.
    #[error("provider discovery ticket sequence is exhausted")]
    Exhausted,
}

/// Immutable snapshots share catalogue data; the reducer alone mutates slots.
#[derive(Debug)]
pub(super) struct Cache {
    registry: ProviderRegistry,
    slots: BTreeMap<Key, Slot>,
    next: u64,
}

impl Cache {
    pub(super) fn new(registry: ProviderRegistry) -> Self {
        Self {
            registry,
            slots: BTreeMap::new(),
            next: 0,
        }
    }

    /// Passive projection neither inserts a cache slot nor starts discovery.
    pub(super) fn cached(
        &self,
        id: &ProviderId,
        scope: &CatalogueScope,
    ) -> Result<ProviderDiagnostic, CacheError> {
        let definition = self.registry.get(id).ok_or(CacheError::UnknownProvider)?;
        let key = key(definition, scope);
        let slot = self.slots.get(&key);
        let state = slot.map_or_else(|| initial(definition.enabled), |slot| slot.state.clone());
        Ok(ProviderDiagnostic {
            entry: definition.snapshot_entry(state),
            version: slot.and_then(|slot| slot.version.clone()),
        })
    }

    pub(super) fn snapshot(
        &self,
        scope: &CatalogueScope,
    ) -> Result<Vec<ProviderSnapshotEntry>, CacheError> {
        let entries = self
            .registry
            .definitions()
            .iter()
            .map(|definition| self.cached(&definition.id, scope).map(|value| value.entry))
            .collect::<Result<Vec<_>, _>>()?;
        if super::budget::fits(&entries) {
            Ok(entries)
        } else {
            Err(CacheError::ReplyTooLarge)
        }
    }

    /// Admission snapshots the actual definition before any runtime is touched.
    #[cfg(test)]
    pub(super) fn begin(
        &mut self,
        id: &ProviderId,
        scope: &CatalogueScope,
        refresh: Refresh,
    ) -> Result<Admission, CacheError> {
        self.admit(id, scope, refresh, WorkBudget::Available)
    }

    pub(super) fn admit(
        &mut self,
        id: &ProviderId,
        scope: &CatalogueScope,
        refresh: Refresh,
        budget: WorkBudget,
    ) -> Result<Admission, CacheError> {
        let definition = self.registry.get(id).ok_or(CacheError::UnknownProvider)?;
        if definition.enabled == Enabled::Off {
            return self.cached(id, scope).map(Admission::Ready);
        }
        let key = key(definition, scope);
        if let Some(slot) = self.slots.get(&key)
            && refresh == Refresh::Cached
        {
            if let Some(ticket) = slot.pending {
                return Ok(Admission::Joined(ticket));
            }
            return self.cached(id, scope).map(Admission::Ready);
        }
        if budget == WorkBudget::Full {
            return Err(CacheError::Busy);
        }
        if !self.slots.contains_key(&key) && self.slots.len() == MAX_CATALOGUES {
            return Err(CacheError::Capacity);
        }
        self.next = self.next.checked_add(1).ok_or(CacheError::Exhausted)?;
        let ticket = Ticket(self.next);
        let work = Work {
            ticket,
            definition: Arc::new(definition.clone()),
            scope: key.scope.clone(),
            supersedes: self.slots.get(&key).and_then(|slot| slot.pending),
        };
        let prior_state = self
            .slots
            .get(&key)
            .map_or(ProviderState::Loading, |slot| slot.state.clone());
        let prior_version = self.slots.get(&key).and_then(|slot| slot.version.clone());
        let _previous = self.slots.insert(
            key,
            Slot {
                state: prior_state,
                version: prior_version,
                pending: Some(ticket),
            },
        );
        Ok(Admission::Start(work))
    }

    /// Superseded work cannot publish or replace a newer definition's result.
    pub(super) fn complete(
        &mut self,
        work: &Work,
        observed: Observation,
        at: DateTime<Utc>,
    ) -> Option<ProviderDiagnostic> {
        let definition = self.registry.get(&work.definition.id)?;
        if definition != work.definition.as_ref() {
            return None;
        }
        let slot = self.slots.get_mut(&key(definition, &work.scope))?;
        if slot.pending != Some(work.ticket) {
            return None;
        }
        let (state, version) = observed_state(definition, observed, at);
        let mut diagnostic = ProviderDiagnostic {
            entry: definition.snapshot_entry(state),
            version,
        };
        if !super::budget::fits(&diagnostic) {
            // Reject the whole catalogue, never publish a partial Ready value.
            diagnostic.entry.state = ProviderState::Error {
                failure: DiscoveryFailure::InvalidCatalogue,
            };
        }
        slot.state = diagnostic.entry.state.clone();
        slot.version.clone_from(&diagnostic.version);
        slot.pending = None;
        Some(diagnostic)
    }

    /// Preserve byte-equivalent definitions, invalidating only affected clients.
    /// The runtime owner cancels returned tickets and settles their waiters.
    pub(super) fn replace(&mut self, registry: ProviderRegistry) -> Vec<Ticket> {
        let mut cancelled = Vec::new();
        self.slots.retain(|key, slot| {
            let keep = self.registry.get(&key.provider) == registry.get(&key.provider);
            if !keep && let Some(ticket) = slot.pending {
                cancelled.push(ticket);
            }
            keep
        });
        self.registry = registry;
        cancelled
    }
}

fn key(definition: &ProviderDefinition, scope: &CatalogueScope) -> Key {
    Key {
        provider: definition.id.clone(),
        scope: definition.catalogue_key(scope),
    }
}

fn initial(enabled: Enabled) -> ProviderState {
    let reason = match enabled {
        Enabled::On => UnavailableReason::NotChecked,
        Enabled::Off => UnavailableReason::Disabled,
    };
    ProviderState::Unavailable { reason }
}

fn observed_state(
    definition: &ProviderDefinition,
    observation: Observation,
    at: DateTime<Utc>,
) -> (ProviderState, Option<AgentVersion>) {
    match observation {
        Observation::Unavailable(reason) => (ProviderState::Unavailable { reason }, None),
        Observation::Failed(failure) => (ProviderState::Error { failure }, None),
        Observation::Ready {
            mut catalogue,
            version,
        } => {
            match merge_models(
                &catalogue.models,
                definition.models.as_ref(),
                &definition.additional_models,
            ) {
                Ok(merged) => {
                    for id in merged.duplicates {
                        tracing::warn!(provider = %definition.id, model = %id, "provider catalogue repeated a model id; retaining first definition");
                    }
                    catalogue.models = merged.models;
                    (
                        ProviderState::Ready {
                            catalogue: Arc::new(catalogue),
                            fetched_at: at,
                        },
                        version,
                    )
                }
                Err(_invalid) => (
                    ProviderState::Error {
                        failure: DiscoveryFailure::InvalidCatalogue,
                    },
                    version,
                ),
            }
        }
    }
}
