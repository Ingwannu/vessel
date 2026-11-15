//! Diagnostic service boundary: the guard admits scope before collection.
use super::{Connection, NodeService};
use vessel_proto::{
    WireError,
    diagnostics::{DiagnosticReport, GetDiagnostics},
};
impl NodeService {
    pub(crate) async fn diagnostics(
        &self,
        _: &Connection,
        _: GetDiagnostics,
    ) -> Result<DiagnosticReport, WireError> {
        self.diagnostics.collect(&self.lifecycle).await
    }
}
