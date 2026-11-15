//! Retain concrete failures without exposing source text or task panic payloads.
use std::fmt;
use tokio::task::JoinError;

use crate::project_file::ProjectFileError;

enum Cause {
    File(ProjectFileError),
    Task(JoinError),
}

/// Diagnostic boundary for project-file source and task failures.
/// The original error remains owned privately; public formatters and `source()`
/// expose only a fixed class. This does not sanitize the process panic hook.
pub struct ConfigCause {
    cause: Cause,
}
impl ConfigCause {
    pub(super) fn file(cause: ProjectFileError) -> Self {
        Self {
            cause: Cause::File(cause),
        }
    }
    pub(super) fn task(cause: JoinError) -> Self {
        Self {
            cause: Cause::Task(cause),
        }
    }
    fn class(&self) -> &'static str {
        match &self.cause {
            // Retain the typed cause without consulting its source-bearing message.
            Cause::File(_cause) => "project file failure",
            Cause::Task(cause) if cause.is_cancelled() => "cancelled project config task",
            Cause::Task(_) => "panicked project config task",
        }
    }
}
impl fmt::Debug for ConfigCause {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConfigCause")
            .field("class", &self.class())
            .finish_non_exhaustive()
    }
}
impl fmt::Display for ConfigCause {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.class())
    }
}
impl std::error::Error for ConfigCause {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_config::ConfigEditError;

    fn safe_chain(error: &ConfigEditError, canary: &str) {
        let mut current: Option<&(dyn std::error::Error + 'static)> = Some(error);
        while let Some(error) = current {
            assert!(!format!("{error:?} {error}").contains(canary));
            current = error.source();
        }
    }

    #[test]
    fn project_config_parser_cause_remains_private_through_the_complete_chain() {
        let canary = "PRIVATE_CONFIG_PARSER_CANARY";
        let sealed = ConfigCause::file(ProjectFileError::Syntax(canary.to_owned()));
        assert!(
            matches!(&sealed.cause, Cause::File(ProjectFileError::Syntax(text)) if text==canary)
        );
        safe_chain(&ConfigEditError::Invalid(sealed), canary);
        safe_chain(
            &ConfigEditError::Read(ConfigCause::file(ProjectFileError::Syntax(
                canary.to_owned(),
            ))),
            canary,
        );
    }

    #[tokio::test]
    async fn project_config_static_and_owned_panics_remain_private_but_retained() {
        let static_cause = tokio::spawn(async {
            panic!("PRIVATE_CONFIG_STATIC_CANARY");
        })
        .await
        .unwrap_err();
        let owned_cause = tokio::spawn(async {
            std::panic::panic_any(String::from("PRIVATE_CONFIG_OWNED_CANARY"));
        })
        .await
        .unwrap_err();
        for (cause, canary) in [
            (static_cause, "PRIVATE_CONFIG_STATIC_CANARY"),
            (owned_cause, "PRIVATE_CONFIG_OWNED_CANARY"),
        ] {
            let id = cause.id();
            let sealed = ConfigCause::task(cause);
            assert!(matches!(&sealed.cause, Cause::Task(kept) if kept.id()==id && kept.is_panic()));
            safe_chain(&ConfigEditError::Task(sealed), canary);
        }
    }

    #[tokio::test]
    async fn project_config_cancelled_task_has_a_safe_complete_chain() {
        let task = tokio::spawn(std::future::pending::<()>());
        task.abort();
        let sealed = ConfigCause::task(task.await.unwrap_err());
        assert!(matches!(&sealed.cause, Cause::Task(kept) if kept.is_cancelled()));
        let error = ConfigEditError::Task(sealed);
        safe_chain(&error, "PRIVATE_CONFIG_CANCEL_CANARY");
    }
}
