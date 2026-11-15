//! Screen capture uses the same terminal ownership gate as input and event reads.
//! The method remains pending until the integrator assigns `CLI_PARITY_PROTOCOL`.
use super::{Connection, NodeService};
use vessel_proto::{
    WireError,
    terminals::{CaptureTerminal, TerminalCapture},
};
impl NodeService {
    pub(crate) async fn capture_terminal(
        &self,
        connection: &Connection,
        request: CaptureTerminal,
    ) -> Result<TerminalCapture, WireError> {
        let entry = self
            .terminal_for(connection.principal(), request.terminal)
            .map_err(crate::terminals::TerminalsError::wire)?;
        entry
            .handle
            .capture(request.options)
            .await
            .map_err(|error| WireError::new(error.code(), error.to_string()))
    }
}
