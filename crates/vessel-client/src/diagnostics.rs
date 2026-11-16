//! One bounded diagnostic request over the existing authenticated connection.
use crate::{CallError, NodeClient};
use std::time::Duration;
use vessel_proto::diagnostics::{DiagnosticReport, GetDiagnostics};

/// End-to-end budget, including transport, longer than the node source budget.
pub const DIAGNOSTIC_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Typed diagnostic call failure. Its causes are private diagnostic evidence,
/// not shareable report text; render closed categories at an output boundary.
#[derive(Debug, thiserror::Error)]
pub enum DiagnosticError {
    /// Existing version, authority or connection admission failed.
    #[error("diagnostic request failed")]
    Call(#[from] CallError),
    /// The response identity differs from the admitted peer.
    #[error("diagnostic response belongs to another node")]
    Identity {
        /// Identity admitted by the connection.
        expected: vessel_proto::NodeId,
        /// Identity in the untrusted response.
        reported: vessel_proto::NodeId,
    },
    /// The response budget elapsed; this does not cancel running node I/O.
    #[error("diagnostic request exceeded {DIAGNOSTIC_REQUEST_TIMEOUT:?}")]
    Timeout(#[source] tokio::time::error::Elapsed),
}
impl NodeClient {
    /// Collect metadata only; never refreshes a provider or starts an agent.
    ///
    /// # Errors
    /// Unassigned/older protocol refuses before sending a frame. Read scope,
    /// transport, node resource admission and the named timeout may refuse.
    pub async fn diagnostics(&self) -> Result<DiagnosticReport, DiagnosticError> {
        let report = tokio::time::timeout(DIAGNOSTIC_REQUEST_TIMEOUT, self.call(GetDiagnostics {}))
            .await
            .map_err(DiagnosticError::Timeout)?
            .map_err(DiagnosticError::Call)?;
        let expected = self.node().id;
        let reported = report.fields().node.id;
        if expected != reported {
            return Err(DiagnosticError::Identity { expected, reported });
        }
        Ok(report)
    }
}
