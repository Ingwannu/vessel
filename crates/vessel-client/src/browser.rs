//! ADR 0078's pending direct-node browser calls, without engine/tool admission.
//!
//! These methods preserve the node's typed refusal. They never turn a pending
//! protocol, missing engine or disabled session into a successful fixture result.

#[cfg(test)]
mod watch;

use vessel_proto::browser::{BrowserCall, BrowserControl, BrowserResult, ClearBrowserProfile};

use crate::{CallError, NodeClient};

impl NodeClient {
    /// Drive only the authenticated local agent session's admitted browser context.
    ///
    /// # Errors
    /// Pending protocol is refused before sending; the node also rechecks session admission.
    pub async fn browser_call(&self, request: BrowserCall) -> Result<BrowserResult, CallError> {
        self.call(request).await
    }

    /// Direct human panel control; this is not an alternate agent tool path.
    ///
    /// # Errors
    /// Pending protocol, non-human/delegated caller, scope and engine failures remain explicit.
    pub async fn browser_control(
        &self,
        request: BrowserControl,
    ) -> Result<BrowserResult, CallError> {
        self.call(request).await
    }

    /// Clear only node-owned browser profile locations, with direct human Admin authority.
    ///
    /// # Errors
    /// No arbitrary path, pending protocol or missing engine can be accepted silently.
    pub async fn clear_browser_profile(&self) -> Result<(), CallError> {
        self.call(ClearBrowserProfile {}).await.map(|_ack| ())
    }
}
