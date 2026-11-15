//! One typed credential path; private values and quoted parser/task text stay sealed.

use super::{Models, config::Provider};
use std::fmt;
use vessel_llm::client::ApiKey;
use vessel_proto::SecretName;

/// Retain the file cause without reopening private TOML keys in diagnostics.
pub(crate) struct ReadFailure(crate::fleet::SecretsError);
impl fmt::Debug for ReadFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let class = match &self.0 {
            crate::fleet::SecretsError::File(_) => "file",
            crate::fleet::SecretsError::Parse { .. } => "parse",
            crate::fleet::SecretsError::Entry { .. } => "entry",
        };
        f.debug_struct("SecretReadFailure")
            .field("class", &class)
            .finish_non_exhaustive()
    }
}
impl fmt::Display for ReadFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the node could not read its secrets")
    }
}
// Even an invalid TOML key may be sensitive. Keep the typed source private.
impl std::error::Error for ReadFailure {}

/// Retain a task failure without exposing a panic payload.
pub(crate) struct TaskFailure(tokio::task::JoinError);
impl fmt::Debug for TaskFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretTaskFailure")
            .field("cancelled", &self.0.is_cancelled())
            .finish_non_exhaustive()
    }
}
impl fmt::Display for TaskFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the node could not read its secrets")
    }
}
// A panic payload can quote the file. A generic chain must not reopen it.
impl std::error::Error for TaskFailure {}

/// Refusals retain typed causes while exposing only safe policy diagnostics.
#[derive(Debug, thiserror::Error)]
pub(crate) enum KeyError {
    #[error("{0}")]
    /// A sealed typed credential-file failure.
    Read(#[from] ReadFailure),
    #[error("{0}")]
    /// A sealed typed blocking-task failure.
    Task(#[from] TaskFailure),
    #[error("the route's provider key `{0}` is not set on this node (`vessel secret set {0}`)")]
    /// The declared name has no value on this node.
    Missing(SecretName),
    #[error(
        "the route's provider key `{0}` is too short or too plain to keep out of model answers: give the provider no key in models.toml (a local server needs none), or a random one of at least {minimum} characters",
        minimum = vessel_llm::client::MIN_KEY_BYTES
    )]
    /// Refuse a value that cannot be reliably withheld from replies.
    Plain(SecretName),
}

impl From<tokio::task::JoinError> for KeyError {
    fn from(cause: tokio::task::JoinError) -> Self {
        Self::Task(TaskFailure(cause))
    }
}

impl Models {
    /// One credential read shared by proxy and metadata; never diagnostics with values.
    ///
    /// # Errors
    /// The read/task failed, the declared secret is absent, or its value is not distinctive.
    pub(crate) async fn key(&self, provider: &Provider) -> Result<Option<ApiKey>, KeyError> {
        let Some(name) = provider.key.clone() else {
            return Ok(None);
        };
        let files = self.files.clone();
        let wanted = name.clone();
        #[cfg(test)]
        let gate = self
            .blocked_reads
            .gate(super::blocking_tests::Stage::Secret);
        let read = tokio::task::spawn_blocking(move || {
            #[cfg(test)]
            if let Some(gate) = gate {
                gate.enter();
            }
            crate::fleet::secret_value(&files, &wanted)
        })
        .await
        .map_err(TaskFailure)?;
        let value = read.map_err(|cause| {
            let failure = ReadFailure(cause);
            tracing::warn!(error = ?failure, "cannot read secrets.toml for a model call");
            failure
        })?;
        let key = value
            .map(|value| ApiKey::new(value.expose().to_owned()))
            .ok_or_else(|| KeyError::Missing(name.clone()))?;
        if !key.distinctive() {
            tracing::warn!(secret = %name, "refused a model call: the route's key is too plain");
            return Err(KeyError::Plain(name));
        }
        Ok(Some(key))
    }
}

#[cfg(test)]
mod tests;
