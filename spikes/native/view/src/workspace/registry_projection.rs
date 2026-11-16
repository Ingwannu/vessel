//! Pure canonical registry projection; live cache integration requires guarded observations.
//!
//! [Decision Log]
//! - Purpose: sidebar, palette and panes share explicit workspace ownership.
//! - Constraint: a same-node reconnect retires old reads; cwd is not identity.
//! - Alternatives: per-caller registries, path matching, or empty startup as unowned.
//! - Choice: retain the canonical complete snapshot and its shared admission,
//!   with one read ticket and a bounded encoded-content budget.
//! - Impact: unavailable/unsupported/loading stay distinct; metadata never grants
//!   resource liveness, backing generation, process or mutation authority.

mod rows;
mod validate;

pub use rows::{ProjectPlacement, WorkspaceRow};

use crate::{connection::ConnectionContext, layout::LookupProblem, machine_book::Name};
use std::collections::HashMap;
use vessel_proto::{
    registry::RegistrySnapshot,
    workspace::{ProjectList, WorktreeList},
};

/// Finite original-admission reads, including queued complete observations.
pub const MAX_REGISTRY_READS: usize = 4;
/// Finite retained metadata hosts and pending read candidates.
pub const MAX_REGISTRY_HOSTS: usize = 64;
/// Total retained encoded metadata, independent of the transport frame budget.
const MAX_REGISTRY_CACHE_BYTES: usize = 16 * 1024 * 1024;

/// Canonical node values read through one selected connection; no duplicate descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryData {
    /// Placement and memberships originate in the same node read transaction.
    pub registry: RegistrySnapshot,
    /// Used for explicit project metadata/root consistency, never ownership inference.
    pub projects: ProjectList,
    /// Used for explicit backing metadata/root consistency, never ownership inference.
    pub worktrees: WorktreeList,
}

impl RegistryData {
    /// Bound one queued observation by the same total encoded cache budget.
    ///
    /// # Errors
    /// Refuses encoding failure or combined metadata beyond the cache ceiling.
    pub fn check_size(&self) -> Result<(), ProjectionRefusal> {
        validate::bytes(self, MAX_REGISTRY_CACHE_BYTES).map(|_bytes| ())
    }
}

/// A read ticket is correlation only; its context remains the actor's sole admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryReadKey {
    context: ConnectionContext,
    serial: u64,
}
impl RegistryReadKey {
    /// Exact machine snapshot captured before the request.
    #[must_use]
    pub const fn context(&self) -> &ConnectionContext {
        &self.context
    }
}

/// Neither incomplete reads nor unsupported peers are an empty registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryPhase {
    /// No complete current-admission snapshot has arrived.
    Loading,
    /// A complete current-admission snapshot is available.
    Ready,
    /// The original admission is no longer online.
    Offline,
    /// The exact peer cannot provide the registry method.
    Unsupported,
    /// A read failed or the supplied snapshot was incoherent.
    Refused,
}

/// Correlated transport classification contains no peer-controlled diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryReadFailure {
    /// Original connection retired/closed.
    Offline,
    /// Pending or older protocol, never interpreted as no ownership.
    Unsupported,
    /// Scope, read or decoding refusal.
    Refused,
}

/// Admission/capacity failures preserve other hosts and never reuse a ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ProjectionRefusal {
    /// One admission already has an outstanding read.
    #[error("workspace registry read is already pending")]
    Pending,
    /// No retained host/content slot is available.
    #[error("workspace registry cache is full")]
    Full,
    /// A correlation serial cannot wrap into an old request.
    #[error("workspace registry read identities are exhausted")]
    Exhausted,
    /// Node identity, membership or catalog identifiers are inconsistent.
    #[error("workspace registry snapshot is inconsistent")]
    Invalid,
}

#[derive(Debug)]
struct Entry {
    context: ConnectionContext,
    pending: Option<RegistryReadKey>,
    phase: RegistryPhase,
    data: Option<RegistryData>,
    bytes: usize,
}

/// Display metadata only. Every actual operation rechecks its original admission/CAS.
#[derive(Debug, Default)]
pub struct RegistryProjection {
    serial: u64,
    bytes: usize,
    hosts: HashMap<Name, Entry>,
}
impl RegistryProjection {
    /// Reserve one read without evicting another host or resetting same-admission data.
    ///
    /// # Errors
    /// Invalid book name, pending read, full host budget or exhausted correlation.
    pub fn begin(
        &mut self,
        context: ConnectionContext,
    ) -> Result<RegistryReadKey, ProjectionRefusal> {
        let host = Name::parse(context.machine().as_str())
            .map_err(|_invalid| ProjectionRefusal::Invalid)?;
        let previous = self.hosts.get(&host);
        if previous.is_some_and(|entry| entry.context == context && entry.pending.is_some()) {
            return Err(ProjectionRefusal::Pending);
        }
        if previous.is_none() && self.hosts.len() >= MAX_REGISTRY_HOSTS {
            return Err(ProjectionRefusal::Full);
        }
        let serial = self
            .serial
            .checked_add(1)
            .ok_or(ProjectionRefusal::Exhausted)?;
        let key = RegistryReadKey {
            context: context.clone(),
            serial,
        };
        let entry = self.hosts.entry(host).or_insert_with(|| Entry {
            context: context.clone(),
            pending: None,
            phase: RegistryPhase::Loading,
            data: None,
            bytes: 0,
        });
        if entry.context != context {
            self.bytes = self
                .bytes
                .checked_sub(entry.bytes)
                .unwrap_or(MAX_REGISTRY_CACHE_BYTES);
            entry.bytes = 0;
            entry.data = None;
            entry.phase = RegistryPhase::Loading;
            entry.context = context;
        }
        if entry.data.is_none() {
            entry.phase = RegistryPhase::Loading;
        }
        entry.pending = Some(key.clone());
        self.serial = serial;
        Ok(key)
    }

    /// Apply only the outstanding exact-admission reply; stale replies are harmless.
    ///
    /// # Errors
    /// An inconsistent or over-budget current snapshot is refused without eviction.
    pub fn finish(
        &mut self,
        key: &RegistryReadKey,
        result: Result<RegistryData, RegistryReadFailure>,
    ) -> Result<bool, ProjectionRefusal> {
        let host = Name::parse(key.context.machine().as_str())
            .map_err(|_invalid| ProjectionRefusal::Invalid)?;
        let Some(entry) = self.hosts.get(&host) else {
            return Ok(false);
        };
        if entry.context != key.context || entry.pending.as_ref() != Some(key) {
            return Ok(false);
        }
        let retained_bytes = self
            .bytes
            .checked_sub(entry.bytes)
            .ok_or(ProjectionRefusal::Invalid)?;
        let remaining = MAX_REGISTRY_CACHE_BYTES
            .checked_sub(retained_bytes)
            .ok_or(ProjectionRefusal::Full)?;
        let accepted = match result {
            Ok(data) => {
                if entry.data.as_ref().is_some_and(|old| {
                    old.registry.revision == data.registry.revision && old.registry != data.registry
                }) {
                    self.fail(&host, RegistryReadFailure::Refused);
                    return Err(ProjectionRefusal::Invalid);
                }
                validate::bytes(&data, remaining)
                    .and_then(|bytes| validate::data(&key.context, &data).map(|()| (data, bytes)))
            }
            Err(failure) => {
                self.fail(&host, failure);
                return Ok(true);
            }
        };
        let (data, bytes) = match accepted {
            Ok(value) => value,
            Err(why) => {
                self.fail(&host, RegistryReadFailure::Refused);
                return Err(why);
            }
        };
        let Some(entry) = self.hosts.get_mut(&host) else {
            return Ok(false);
        };
        entry.pending = None;
        // A lower complete revision cannot replace newer facts even on one admission.
        if entry
            .data
            .as_ref()
            .is_some_and(|old| old.registry.revision > data.registry.revision)
        {
            return Ok(false);
        }
        self.bytes = retained_bytes
            .checked_add(bytes)
            .ok_or(ProjectionRefusal::Full)?;
        entry.bytes = bytes;
        entry.data = Some(data);
        entry.phase = RegistryPhase::Ready;
        Ok(true)
    }

    fn fail(&mut self, host: &Name, why: RegistryReadFailure) {
        if let Some(entry) = self.hosts.get_mut(host) {
            self.bytes = self
                .bytes
                .checked_sub(entry.bytes)
                .unwrap_or(MAX_REGISTRY_CACHE_BYTES);
            entry.bytes = 0;
            entry.pending = None;
            entry.data = None;
            entry.phase = match why {
                RegistryReadFailure::Offline => RegistryPhase::Offline,
                RegistryReadFailure::Unsupported => RegistryPhase::Unsupported,
                RegistryReadFailure::Refused => RegistryPhase::Refused,
            };
        }
    }

    /// Fleet reconciliation revokes even coalesced same-node reconnects before lookup.
    pub fn reconcile(&mut self, current: &[ConnectionContext]) {
        let retired: Vec<_> = self
            .hosts
            .iter()
            .filter(|(_, entry)| !current.contains(&entry.context))
            .map(|(host, _)| host.clone())
            .collect();
        for host in retired {
            self.fail(&host, RegistryReadFailure::Offline);
        }
    }

    /// A forgotten book entry releases its host slot; serials remain lifetime-monotonic.
    pub fn forget(&mut self, host: &Name) {
        if let Some(entry) = self.hosts.remove(host) {
            self.bytes = self
                .bytes
                .checked_sub(entry.bytes)
                .unwrap_or(MAX_REGISTRY_CACHE_BYTES);
        }
    }

    /// The last complete revision is a long-poll cursor, never mutation authority.
    #[must_use]
    pub fn changed_from(
        &self,
        context: &ConnectionContext,
    ) -> Option<vessel_proto::registry::RegistryRevision> {
        self.admitted(context)
            .ok()?
            .data
            .as_ref()
            .map(|data| data.registry.revision)
    }

    /// Read phase without treating startup/unsupported as an empty host.
    #[must_use]
    pub fn phase(&self, context: &ConnectionContext) -> Option<RegistryPhase> {
        self.hosts
            .values()
            .find(|entry| entry.context == *context)
            .map(|entry| entry.phase)
    }

    fn admitted(&self, context: &ConnectionContext) -> Result<&Entry, LookupProblem> {
        let host = Name::parse(context.machine().as_str())
            .map_err(|_invalid| LookupProblem::Unavailable)?;
        let entry = self.hosts.get(&host).ok_or(LookupProblem::NotHydrated)?;
        if entry.context != *context {
            return Err(LookupProblem::Unavailable);
        }
        match entry.phase {
            RegistryPhase::Ready => Ok(entry),
            RegistryPhase::Loading => Err(LookupProblem::NotHydrated),
            RegistryPhase::Offline | RegistryPhase::Unsupported | RegistryPhase::Refused => {
                Err(LookupProblem::Unavailable)
            }
        }
    }
}

#[cfg(test)]
mod tests;
