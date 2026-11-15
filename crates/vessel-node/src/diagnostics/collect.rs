//! Compose allowlisted owner snapshots under one deadline, preserving partial data.
use super::{RESPONSE_BUDGET, State, admission, process, sources::Metadata};
use crate::lifecycle::Lifecycle;
use chrono::Utc;
use std::sync::Arc;
use tokio::time::{Instant, timeout_at};
use vessel_proto::{
    ErrorCode, ErrorDetail, WireError,
    diagnostics::{
        DiagnosticFailureWindow, DiagnosticRefusal, DiagnosticReport, DiagnosticReportFields,
        DiagnosticSection, DiagnosticStore, DiagnosticUnavailable,
    },
};
use vessel_store::DiagnosticReadAdmission;

pub(super) fn available<T>(value: T) -> DiagnosticSection<T> {
    DiagnosticSection::Available {
        at: Utc::now(),
        value,
    }
}
pub(super) fn unavailable<T>(reason: DiagnosticUnavailable) -> DiagnosticSection<T> {
    DiagnosticSection::Unavailable { reason }
}
impl State {
    pub(crate) async fn collect(
        &self,
        lifecycle: &Lifecycle,
    ) -> Result<DiagnosticReport, WireError> {
        let lease = self.admission.admit().map_err(|error| refusal(&error))?;
        let deadline = Instant::now().checked_add(RESPONSE_BUDGET).ok_or_else(|| {
            WireError::new(
                ErrorCode::Internal,
                format!("diagnostic response budget {RESPONSE_BUDGET:?} cannot be represented"),
            )
        })?;
        let metadata = self.metadata(Arc::clone(&lease));
        let (metadata, store, recent_failures) = tokio::join!(
            timeout_at(deadline, metadata),
            self.store_section(Arc::clone(&lease), deadline),
            self.failure_section(deadline),
        );
        let metadata = match metadata {
            Ok(Ok(value)) => value,
            Ok(Err(_join)) => Metadata::unavailable(DiagnosticUnavailable::ReadFailed),
            Err(_deadline) => Metadata::unavailable(DiagnosticUnavailable::TimedOut),
        };
        let (at, listeners) = self.listeners.borrow().clone();
        DiagnosticReport::try_from(DiagnosticReportFields {
            collected_at: Utc::now(),
            node: process::observation(lifecycle, self.started),
            process: metadata.process,
            agents: metadata.agents,
            sessions: metadata.sessions,
            relays: metadata.relays,
            store,
            recent_failures,
            listeners: DiagnosticSection::Available {
                at,
                value: listeners,
            },
        })
        .map_err(|_invalid| {
            WireError::new(
                ErrorCode::Internal,
                "diagnostic report violated its bounded contract",
            )
        })
    }
    async fn store_section(
        &self,
        lease: Arc<admission::Lease>,
        deadline: Instant,
    ) -> DiagnosticSection<DiagnosticStore> {
        let stop = self.stop.clone();
        let source = self.store.diagnostic_snapshot(lease, move || {
            if stop.is_cancelled() {
                DiagnosticReadAdmission::Stopping
            } else {
                DiagnosticReadAdmission::Continue
            }
        });
        // Typed source causes remain internal. This report privacy boundary
        // returns only the closed reason and the actual completion timestamp.
        match timeout_at(deadline, source).await {
            Ok(Ok((value, _guard))) => available(value),
            Ok(Err(_source)) => unavailable(if self.stop.is_cancelled() {
                DiagnosticUnavailable::Stopping
            } else {
                DiagnosticUnavailable::ReadFailed
            }),
            Err(_deadline) => unavailable(DiagnosticUnavailable::TimedOut),
        }
    }
    async fn failure_section(
        &self,
        deadline: Instant,
    ) -> DiagnosticSection<DiagnosticFailureWindow> {
        match timeout_at(deadline, self.journal.snapshot()).await {
            Ok(Ok(value)) => available(value),
            Ok(Err(source)) => unavailable(source.unavailable()),
            Err(_deadline) => unavailable(DiagnosticUnavailable::TimedOut),
        }
    }
}
fn refusal(error: &admission::Refused) -> WireError {
    let reason = match error {
        admission::Refused::Busy(_) => DiagnosticRefusal::Busy,
        admission::Refused::Stopping => DiagnosticRefusal::Stopping,
    };
    let mut wire = WireError::new(ErrorCode::Unavailable, error.to_string());
    wire.detail = Some(ErrorDetail::Diagnostics { reason });
    wire
}
