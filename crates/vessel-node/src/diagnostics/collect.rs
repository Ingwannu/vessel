//! Compose allowlisted owner snapshots under one deadline, preserving partial data.
use super::{
    RESPONSE_BUDGET, State, admission,
    inventory::{Entry, Inventory},
    process,
};
use crate::{lifecycle::Lifecycle, readiness::Key};
use chrono::Utc;
use std::sync::Arc;
use tokio::time::{Instant, timeout_at};
use vessel_proto::{ErrorCode, HarnessRef, WireError, agents::UnknownReason, diagnostics::*};
use vessel_store::DiagnosticReadAdmission;

fn available<T>(value: T) -> DiagnosticSection<T> {
    DiagnosticSection::Available {
        at: Utc::now(),
        value,
    }
}
fn unavailable<T>(reason: DiagnosticUnavailable) -> DiagnosticSection<T> {
    DiagnosticSection::Unavailable { reason }
}
impl State {
    pub(crate) async fn collect(
        &self,
        lifecycle: &Lifecycle,
    ) -> Result<DiagnosticReport, WireError> {
        let lease = self.admission.admit().map_err(|error| match error {
            admission::Refused::Busy(_) => WireError::new(ErrorCode::Busy, error.to_string()),
            admission::Refused::Stopping => {
                WireError::new(ErrorCode::Conflict, "the node is stopping")
            }
        })?;
        let deadline = Instant::now() + RESPONSE_BUDGET;
        let profiles = self.profiles.current();
        let lookup = self.lookup.clone();
        let readiness = Arc::clone(&self.readiness);
        let stop = self.stop.clone();
        let work = Arc::clone(&lease);
        let metadata = tokio::task::spawn_blocking(move || {
            let _retained = work;
            if stop.is_cancelled() {
                return (
                    unavailable(DiagnosticUnavailable::Stopping),
                    unavailable(DiagnosticUnavailable::Stopping),
                );
            }
            let agents = agents(&profiles, &lookup, &readiness);
            let process = self_process();
            (agents, process)
        });
        let stop = self.stop.clone();
        let store = self.store.diagnostic_snapshot(Arc::clone(&lease), move || {
            if stop.is_cancelled() {
                DiagnosticReadAdmission::Stopping
            } else {
                DiagnosticReadAdmission::Continue
            }
        });
        let (metadata, store, failures) = tokio::join!(
            timeout_at(deadline, metadata),
            timeout_at(deadline, store),
            timeout_at(deadline, self.journal.snapshot())
        );
        let (agents, process) = match metadata {
            Ok(Ok(value)) => value,
            Ok(Err(_join)) => (
                unavailable(DiagnosticUnavailable::ReadFailed),
                unavailable(DiagnosticUnavailable::ReadFailed),
            ),
            Err(_deadline) => (
                unavailable(DiagnosticUnavailable::TimedOut),
                unavailable(DiagnosticUnavailable::TimedOut),
            ),
        };
        // Owner errors retain typed causes internally; this is the explicit
        // report privacy boundary. Never render or log their source chains.
        let store = match store {
            Ok(Ok((value, _guard))) => available(value),
            Ok(Err(_source)) => unavailable(if self.stop.is_cancelled() {
                DiagnosticUnavailable::Stopping
            } else {
                DiagnosticUnavailable::ReadFailed
            }),
            Err(_deadline) => unavailable(DiagnosticUnavailable::TimedOut),
        };
        let recent_failures = match failures {
            Ok(Ok(value)) => available(value),
            Ok(Err(source)) => unavailable(source.unavailable()),
            Err(_deadline) => unavailable(DiagnosticUnavailable::TimedOut),
        };
        let (at, listeners) = self.listeners.borrow().clone();
        DiagnosticReport::try_from(DiagnosticReportFields {
            collected_at: Utc::now(),
            node: process::observation(lifecycle, self.started),
            process,
            store,
            agents,
            sessions: available(self.runtime.diagnostic_counts()),
            listeners: DiagnosticSection::Available {
                at,
                value: listeners,
            },
            relays: self.relays.upgrade().map_or_else(
                || unavailable(DiagnosticUnavailable::Stopping),
                |source| available(source.diagnostic_counts()),
            ),
            recent_failures,
        })
        .map_err(|_invalid| {
            WireError::new(
                ErrorCode::Internal,
                "diagnostic report violated its bounded contract",
            )
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
            if let Some(reason) = cached.refusal {
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
