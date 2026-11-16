//! One live launch-close predicate, shared by async and blocking preparation.
//! This observes existing cancellation sources; it grants no launch authority.

use tokio_util::sync::CancellationToken;

use crate::SessionError;

/// An owned observer of the node's early stop and the runtime's final shutdown.
/// Standalone runtimes observe only their existing runtime shutdown token.
#[derive(Clone, Debug)]
pub struct LaunchAdmission {
    runtime: CancellationToken,
    node: Option<CancellationToken>,
}

impl LaunchAdmission {
    pub(super) fn new(runtime: CancellationToken, node: Option<CancellationToken>) -> Self {
        Self { runtime, node }
    }

    /// Recheck immediately before an effect, including after a held lookup.
    /// Caller policy, permissions and deadlines remain independent obligations.
    ///
    /// # Errors
    /// Either existing close source refuses with `SessionError::ShuttingDown`.
    pub fn check(&self) -> Result<(), SessionError> {
        if self.runtime.is_cancelled()
            || self
                .node
                .as_ref()
                .is_some_and(CancellationToken::is_cancelled)
        {
            Err(SessionError::ShuttingDown)
        } else {
            Ok(())
        }
    }

    /// Wake an observer on either existing close source, including one already
    /// closed. Cancelling this wait changes neither source nor physical work.
    pub async fn cancelled(&self) {
        match &self.node {
            Some(node) => tokio::select! {
                () = self.runtime.cancelled() => {},
                () = node.cancelled() => {},
            },
            None => self.runtime.cancelled().await,
        }
    }
}

#[cfg(test)]
pub(super) mod tests;
