//! Keep worker panic payloads private while retaining the concrete join cause.
//! This diagnostic boundary owns no worker, cancellation or cleanup policy.

use std::{error::Error, fmt};

use tokio::task::JoinError;

/// Only an input-free class may cross a command/helper diagnostic boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkerFailureKind {
    Cancelled,
    Panicked,
}

/// The raw cause remains owned here; no formatter or source accessor exposes it.
pub(crate) struct WorkerFailure {
    cause: JoinError,
}

impl From<JoinError> for WorkerFailure {
    fn from(cause: JoinError) -> Self {
        Self { cause }
    }
}

impl WorkerFailure {
    /// Classification preserves useful failure state without echoing a payload.
    pub(crate) fn kind(&self) -> WorkerFailureKind {
        // The locked Tokio contract has exactly cancellation and panic causes.
        if self.cause.is_cancelled() {
            WorkerFailureKind::Cancelled
        } else {
            WorkerFailureKind::Panicked
        }
    }
}

impl fmt::Display for WorkerFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.kind() {
            WorkerFailureKind::Cancelled => "the worker was cancelled",
            WorkerFailureKind::Panicked => "the worker panicked",
        })
    }
}

impl fmt::Debug for WorkerFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkerFailure")
            .field("kind", &self.kind())
            .finish()
    }
}

// Error::source deliberately ends here: JoinError's Display and Debug can
// contain arbitrary panic input. Retention is private, not a public source link.
impl Error for WorkerFailure {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminals::{port_script::PortScriptError, preview::PreviewError};

    const CANARY: &str = "synthetic-worker-panic-payload-canary";

    // The standard Error chain is the open extension point under test.
    fn assert_safe(mut error: &(dyn Error + 'static)) {
        loop {
            for text in [
                format!("{error}"),
                format!("{error:?}"),
                format!("{error:#?}"),
            ] {
                assert!(!text.contains(CANARY));
            }
            match error.source() {
                Some(source) => error = source,
                None => break,
            }
        }
    }

    fn check_wrappers(cause: JoinError, expected: WorkerFailureKind) {
        let task_id = cause.id();
        let failure = WorkerFailure::from(cause);
        assert_eq!(failure.kind(), expected);
        assert_eq!(failure.cause.id(), task_id);
        if expected == WorkerFailureKind::Panicked {
            // Prove the original payload is retained privately, not flattened.
            assert!(failure.cause.to_string().contains(CANARY));
        }
        assert!(failure.source().is_none());
        assert_safe(&failure);
        let error = PreviewError::Worker(failure);
        assert!(error.source().is_some());
        assert_safe(&error);
        let io = std::io::Error::other(error);
        assert_safe(&io);
        assert_safe(io.get_ref().unwrap());
    }

    #[tokio::test]
    async fn literal_and_owned_panic_payloads_stay_out_of_every_public_diagnostic() {
        let literal: tokio::task::JoinHandle<()> =
            tokio::task::spawn_blocking(|| std::panic::panic_any(CANARY));
        check_wrappers(literal.await.unwrap_err(), WorkerFailureKind::Panicked);
        let owned: tokio::task::JoinHandle<()> =
            tokio::task::spawn_blocking(|| std::panic::panic_any(String::from(CANARY)));
        check_wrappers(owned.await.unwrap_err(), WorkerFailureKind::Panicked);
        // These assertions concern formatting/source chains, not the panic hook.
    }

    #[tokio::test]
    async fn cancellation_keeps_its_class_and_concrete_cause_without_a_raw_source() {
        let task = tokio::spawn(std::future::pending::<()>());
        task.abort();
        check_wrappers(task.await.unwrap_err(), WorkerFailureKind::Cancelled);
    }

    #[tokio::test]
    async fn helper_resolution_reuses_the_same_safe_worker_error_boundary() {
        let task: tokio::task::JoinHandle<()> =
            tokio::task::spawn_blocking(|| std::panic::panic_any(CANARY));
        let error = PortScriptError::Join(WorkerFailure::from(task.await.unwrap_err()));
        assert_safe(&error);
        let io = std::io::Error::other(error);
        assert_safe(&io);
        assert_safe(io.get_ref().unwrap());
    }
}
