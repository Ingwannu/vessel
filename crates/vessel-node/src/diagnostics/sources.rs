//! Blocking metadata and bounded source-job ownership; no report text.
use super::collect::{available, unavailable};
use super::{
    State,
    admission::Lease,
    inventory::{Entry, Inventory},
};
use crate::readiness::Key;
use chrono::Utc;
use std::sync::{Arc, Weak};
use vessel_proto::{
    HarnessRef,
    agents::UnknownReason,
    diagnostics::{
        DiagnosticAgent, DiagnosticAgents, DiagnosticAvailability, DiagnosticCount,
        DiagnosticMemory, DiagnosticProcess, DiagnosticProcessId, DiagnosticRelays,
        DiagnosticRuntimeSessions, DiagnosticSection, DiagnosticSignIn, DiagnosticUnavailable,
    },
};

pub(super) struct Metadata {
    pub(super) agents: DiagnosticSection<DiagnosticAgents>,
    pub(super) process: DiagnosticSection<DiagnosticProcess>,
    pub(super) sessions: DiagnosticSection<DiagnosticRuntimeSessions>,
    pub(super) relays: DiagnosticSection<DiagnosticRelays>,
}
impl Metadata {
    pub(super) fn unavailable(reason: DiagnosticUnavailable) -> Self {
        Self {
            agents: unavailable(reason),
            process: unavailable(reason),
            sessions: unavailable(reason),
            relays: unavailable(reason),
        }
    }
}
impl State {
    pub(super) fn metadata(&self, lease: Arc<Lease>) -> tokio::task::JoinHandle<Metadata> {
        let profiles = self.profiles.current();
        let lookup = self.lookup.clone();
        let readiness = Arc::clone(&self.readiness);
        let stop = self.stop.clone();
        let runtime = Arc::clone(&self.runtime);
        let relays = Weak::clone(&self.relays);
        tokio::task::spawn_blocking(move || {
            let _retained = lease;
            if stop.is_cancelled() {
                return Metadata::unavailable(DiagnosticUnavailable::Stopping);
            }
            let sessions = available(runtime.diagnostic_counts());
            let relays = relays.upgrade().map_or_else(
                || unavailable(DiagnosticUnavailable::Stopping),
                |source| available(source.diagnostic_counts()),
            );
            let agents = agents(&profiles, &lookup, &readiness);
            let process = self_process();
            Metadata {
                agents,
                process,
                sessions,
                relays,
            }
        })
    }
}
fn agents(
    profiles: &vessel_harness::Profiles,
    lookup: &vessel_harness::Lookup,
    readiness: &crate::readiness::Readiness,
) -> DiagnosticSection<DiagnosticAgents> {
    let inventory = match Inventory::select(profiles.added()) {
        Ok(value) => value,
        Err(error) => return unavailable(error.unavailable()),
    };
    let mut result = Vec::with_capacity(inventory.entries.len());
    for entry in inventory.entries {
        let id = entry.id();
        let (harness, metadata) = match entry {
            Entry::Builtin(kind) => (
                HarnessRef::Known(kind),
                vessel_harness::builtin_metadata(kind, lookup),
            ),
            Entry::Added { source, .. } => (
                HarnessRef::Added(source.name().clone()),
                vessel_harness::added_metadata(source, lookup),
            ),
        };
        let cached = profiles.default_of(&harness).and_then(|profile| {
            readiness.diagnostic(
                &Key {
                    agent: harness,
                    profile: profile.name().clone(),
                },
                Utc::now(),
            )
        });
        let mut row = DiagnosticAgent {
            id,
            executable: metadata.executable,
            structured: metadata.structured,
            terminal: metadata.terminal,
            sign_in: DiagnosticSignIn::Unknown {
                reason: UnknownReason::NotChecked,
            },
            version: None,
            checked_at: None,
            stale: false,
        };
        if let Some(cached) = cached {
            row.sign_in = cached.sign_in;
            row.version = cached.version;
            row.checked_at = Some(cached.at);
            row.stale = cached.stale;
            if let Some(reason) = cached.refusal
                && matches!(row.structured, DiagnosticAvailability::Available)
            {
                row.structured = DiagnosticAvailability::Unavailable { reason };
            }
        }
        result.push(row);
    }
    match result.try_into() {
        Ok(value) => available(value),
        Err(_invalid) => unavailable(DiagnosticUnavailable::ReadFailed),
    }
}
fn self_process() -> DiagnosticSection<DiagnosticProcess> {
    use vessel_proc::self_process::{self, MemoryObservation, SnapshotError};
    match self_process::observation() {
        Ok(source) => {
            let pid = match DiagnosticProcessId::try_from(source.pid.get()) {
                Ok(value) => value,
                Err(_invalid) => return unavailable(DiagnosticUnavailable::ReadFailed),
            };
            let parent = match source
                .parent
                .map(|id| DiagnosticProcessId::try_from(id.get()))
                .transpose()
            {
                Ok(value) => value,
                Err(_invalid) => return unavailable(DiagnosticUnavailable::ReadFailed),
            };
            let memory = match source.memory {
                MemoryObservation::Available(value) => available(DiagnosticMemory {
                    resident_bytes: DiagnosticCount::new(value.resident.get()),
                    virtual_bytes: DiagnosticCount::new(value.virtual_size.get()),
                }),
                MemoryObservation::NotProvided => unavailable(DiagnosticUnavailable::Unsupported),
            };
            available(DiagnosticProcess {
                pid,
                parent,
                memory,
            })
        }
        Err(SnapshotError::Unsupported) => unavailable(DiagnosticUnavailable::Unsupported),
        Err(_source) => unavailable(DiagnosticUnavailable::ReadFailed),
    }
}
