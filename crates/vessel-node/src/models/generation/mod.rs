//! Generated session titles (ADR 0062): one first-input task, bounded preparation,
//! one dedicated transport and an actor proposal. No branch or Git mutation.

mod attempt;
mod client;
mod driver;
mod preparation;
mod prepare;
mod source;
mod task;
mod text;

use super::Models;
use std::{path::PathBuf, sync::Arc};
use tokio_util::{sync::CancellationToken, task::task_tracker::TaskTrackerToken};
use vessel_proto::{SessionMode, settings::MetadataSettings};
use vessel_session::SessionHandle;
use vessel_workspace::Git;

#[derive(Debug, thiserror::Error)]
enum InitError {
    #[error("metadata HTTP pool could not be built")]
    Http(#[from] vessel_llm::client::ClientError),
    #[error("metadata HTTP policy was refused")]
    Policy(#[from] client::ClientError),
}

/// Keep preparation causes typed without exposing private routing internals to settings.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub(crate) struct ValidationError(#[source] prepare::PrepareError);

/// One pool per `Models` owner, reusing its already initialized TLS trust.
#[derive(Debug)]
pub(super) struct Generator {
    client: Result<client::Client, InitError>,
}
impl Generator {
    pub(super) fn new(proxy: &vessel_llm::client::Client) -> Self {
        let client = proxy
            .with_limits(client::policy())
            .map_err(InitError::from)
            .and_then(|http| client::Client::new(http).map_err(InitError::from));
        if client.is_err() {
            tracing::warn!(class = "initialization", "generated titles are unavailable");
        }
        Self { client }
    }
}

impl Models {
    /// Pure all-candidate preflight; no file, credential, task or socket work.
    pub(crate) fn validate_metadata(
        &self,
        settings: &MetadataSettings,
    ) -> Result<(), ValidationError> {
        prepare::validate(&self.snapshot(), settings).map_err(ValidationError)
    }

    /// Arming never waits for input or a provider, and is disabled until protocol assignment.
    pub(crate) fn arm_title(
        self: &Arc<Self>,
        handle: SessionHandle,
        settings: Arc<crate::settings::Settings>,
        cwd: PathBuf,
        git: Option<Git>,
        stop: CancellationToken,
        work: TaskTrackerToken,
    ) {
        if !vessel_proto::speaks_generated_names() {
            return;
        }
        let info = handle.info();
        let fresh = match info.spec.mode {
            SessionMode::Structured { resume: None, .. } => true,
            SessionMode::Structured {
                resume: Some(_), ..
            }
            | SessionMode::Terminal { .. } => false,
        };
        if !fresh || info.spec.title.is_some() || info.spec.side_question_of.is_some() {
            return;
        }
        let tasks = work.task_tracker().clone();
        let task = task::Task {
            handle,
            models: Arc::clone(self),
            settings,
            cwd,
            git,
            stop,
            work,
        };
        drop(tasks.spawn(async move {
            if let Ok(client) = &task.models.title_generator.client {
                task.run(client).await;
            }
        }));
    }
}

#[cfg(test)]
mod tests;
