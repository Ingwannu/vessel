//! Connection-owned local admission and exact terminal activity (ADR 0089).
use crate::NodeClient;
use vessel_proto::notifications::{
    ClaimNotification, ClearTerminalAttention, NotificationClaim, NotificationSource,
    ReportTerminalActivity, TerminalAttention,
};
use vessel_rpc::CallError;
impl NodeClient {
    /// Ask whether this connection alone may show a generic local notice.
    /// # Errors
    /// Unassigned/older protocol, person-only authority, or transport/store failure.
    pub async fn claim_notification(
        &self,
        source: NotificationSource,
    ) -> Result<NotificationClaim, CallError> {
        self.call(ClaimNotification { source }).await
    }
    /// Clear only the terminal attention revision actually viewed.
    /// # Errors
    /// Unassigned/older protocol, stale revision, authority or transport failure.
    pub async fn clear_terminal_attention(
        &self,
        request: ClearTerminalAttention,
    ) -> Result<TerminalAttention, CallError> {
        self.call(request).await
    }
    /// Report an explicit semantic producer signal for an owned terminal.
    /// # Errors
    /// Unassigned/older protocol, stale revision, ownership or transport failure.
    pub async fn report_terminal_activity(
        &self,
        request: ReportTerminalActivity,
    ) -> Result<TerminalAttention, CallError> {
        self.call(request).await
    }
}
