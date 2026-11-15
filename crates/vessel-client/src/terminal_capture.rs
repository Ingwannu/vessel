//! Screen capture is a typed unary read, separate from terminal byte-stream following.
use crate::{CallError, NodeClient};
use vessel_proto::{
    TerminalId,
    terminals::{CaptureOptions, CaptureTerminal, TerminalCapture},
};
impl NodeClient {
    /// Read an actor-owned screen snapshot, including retained history or final output.
    /// # Errors
    /// Unassigned/old peers refuse before sending; ownership, bounds and actor failures stay typed.
    pub async fn capture_terminal(
        &self,
        terminal: TerminalId,
        options: CaptureOptions,
    ) -> Result<TerminalCapture, CallError> {
        self.call(CaptureTerminal { terminal, options }).await
    }
}
