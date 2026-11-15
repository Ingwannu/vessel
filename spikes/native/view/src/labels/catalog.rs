//! Authoritative host catalogs and cross-host display projection.

use std::collections::BTreeMap;

use super::values::Key;
use super::{Failure, HostKey, Label, Name};

/// Cached authoritative definitions for one host, indexed by normalized name.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Catalog(BTreeMap<Key, Label>);

impl Catalog {
    /// A complete snapshot replaces removed definitions as well as new ones.
    #[must_use]
    pub fn snapshot(labels: impl IntoIterator<Item = Label>) -> Self {
        Self(
            labels
                .into_iter()
                .map(|label| (label.name.key(), label))
                .collect(),
        )
    }

    /// The host definition owns spelling and color even for differently cased input.
    #[must_use]
    pub fn get(&self, name: &Name) -> Option<&Label> {
        self.0.get(&name.key())
    }

    /// Deterministic name order, suitable for search and lists.
    pub fn labels(&self) -> impl Iterator<Item = &Label> {
        self.0.values()
    }

    /// Apply a committed catalog change. Call only after the node acknowledges.
    pub fn upsert(&mut self, previous: Option<&Name>, label: Label) {
        if let Some(previous) = previous {
            self.0.remove(&previous.key());
        }
        self.0.insert(label.name.key(), label);
    }

    /// Apply a committed deletion without creating an unknown definition.
    pub fn remove(&mut self, name: &Name) {
        self.0.remove(&name.key());
    }
}

/// Whether a host's catalog can authorize mutations or merged projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostState {
    /// No connection; cached row colors remain available locally.
    Offline,
    /// Waiting for the first authoritative snapshot.
    Loading,
    /// Contract absent or peer too old.
    Unsupported,
    /// An authoritative load failed; this does not prove deletion.
    Failed(Failure),
    /// Snapshot loaded and connection active.
    Online,
}

impl HostState {
    /// Refuse operations before the caller enters a pending state.
    /// # Errors
    /// Distinguishes unavailable contracts, failed loads, and disconnected hosts.
    pub fn writable(self) -> Result<(), Failure> {
        match self {
            Self::Online => Ok(()),
            Self::Unsupported => Err(Failure::Unsupported),
            Self::Offline => Err(Failure::Offline),
            Self::Loading => Err(Failure::Load),
            Self::Failed(failure) => Err(failure),
        }
    }
}

/// A host's last known catalog is retained independently of reachability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    /// Existing node-book identity; not a new server-id protocol.
    pub key: HostKey,
    /// Reachability and authority.
    pub state: HostState,
    /// Last successfully loaded definitions.
    pub catalog: Catalog,
}

/// Merge only online authoritative catalogs; target host wins, then host order.
/// Callers rendering a particular workspace use that host's catalog directly.
#[must_use]
pub fn project(hosts: &[Host], target: Option<&HostKey>) -> Vec<Label> {
    let mut ordered: Vec<_> = hosts
        .iter()
        .filter(|host| host.state == HostState::Online)
        .collect();
    ordered.sort_by(|left, right| {
        let left_target = Some(&left.key) == target;
        let right_target = Some(&right.key) == target;
        right_target
            .cmp(&left_target)
            .then_with(|| left.key.cmp(&right.key))
    });
    let mut labels = BTreeMap::new();
    for host in ordered {
        for label in host.catalog.labels() {
            labels
                .entry(label.name.key())
                .or_insert_with(|| label.clone());
        }
    }
    labels.into_values().collect()
}
