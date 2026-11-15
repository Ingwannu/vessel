//! Keep the concrete task failure for ownership while sealing panic payload diagnostics.
use std::fmt;
use tokio::task::JoinError;

#[derive(Debug, Clone, Copy)]
enum TaskEnd {
    Cancelled,
    Panicked,
}

/// The source boundary deliberately stops here; panic payloads are not diagnostics.
pub(crate) struct ProjectTaskCause {
    cause: JoinError,
}
impl ProjectTaskCause {
    pub(super) fn new(cause: JoinError) -> Self {
        Self { cause }
    }

    fn end(&self) -> TaskEnd {
        if self.cause.is_cancelled() {
            TaskEnd::Cancelled
        } else {
            TaskEnd::Panicked
        }
    }
}
impl fmt::Debug for ProjectTaskCause {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProjectTaskCause")
            .field("end", &self.end())
            .finish_non_exhaustive()
    }
}
impl fmt::Display for ProjectTaskCause {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.end() {
            TaskEnd::Cancelled => "the project task was cancelled",
            TaskEnd::Panicked => "the project task panicked",
        })
    }
}
impl std::error::Error for ProjectTaskCause {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn project_admission_task_cause_is_retained_privately() {
        let cause = tokio::spawn(async {
            std::panic::panic_any(String::from("PRIVATE_CAUSE_CANARY"));
        })
        .await
        .unwrap_err();
        let id = cause.id();
        let sealed = ProjectTaskCause::new(cause);
        assert_eq!(sealed.cause.id(), id);
        assert!(sealed.cause.is_panic());
        assert!(std::error::Error::source(&sealed).is_none());
        assert!(!format!("{sealed:?} {sealed}").contains("PRIVATE_CAUSE_CANARY"));
    }
}
