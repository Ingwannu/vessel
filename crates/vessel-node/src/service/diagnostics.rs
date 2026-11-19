//! Diagnostic service boundary: the guard admits scope before collection.
use super::{Connection, NodeService};
use vessel_proto::{
    WireError,
    diagnostics::{DIAGNOSTICS_WIRE_PROTOCOL, DiagnosticReport, GetDiagnostics},
};
impl NodeService {
    pub(crate) async fn diagnostics(
        &self,
        connection: &Connection,
        _: GetDiagnostics,
    ) -> Result<DiagnosticReport, WireError> {
        let protocol = connection.protocol();
        let more = DIAGNOSTICS_WIRE_PROTOCOL.is_some_and(|since| protocol >= since);
        Ok(self
            .diagnostics
            .collect(&self.lifecycle, more)
            .await?
            .for_protocol(protocol))
    }
}
