//! Screen capture uses the same terminal ownership gate as input and event reads.
//! Registration is intentionally pending until Quality B assigns `CLI_PARITY_PROTOCOL`.
use super::{Connection, NodeService};
use vessel_proto::{
    ErrorCode, WireError,
    terminals::{CaptureTerminal, TerminalCapture},
};
use vessel_session::terminals::CaptureError;
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
            .map_err(|error| {
                let code = match error {
                    CaptureError::Geometry | CaptureError::TooLarge | CaptureError::WrongTarget => {
                        ErrorCode::InvalidRequest
                    }
                    CaptureError::Unavailable | CaptureError::Timeout => ErrorCode::Unavailable,
                };
                WireError::new(code, error.to_string())
            })
    }
}
