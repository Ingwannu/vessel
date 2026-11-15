//! Negotiated protocol facts belong beside admission, separate from endpoint calls.

use super::NodeClient;
use crate::CallError;
use vessel_proto::{Call, ProtocolVersion};

impl NodeClient {
    /// The protocol version in use.
    #[must_use]
    pub fn protocol(&self) -> ProtocolVersion {
        self.connection.welcome().protocol
    }
    /// Validate negotiated support before preparatory work, using the
    /// connection's own send-time rule without reserving an id or sending.
    ///
    /// # Errors
    /// The call needs an unassigned or newer protocol version.
    pub fn check_call(&self, call: &Call) -> Result<(), CallError> {
        self.connection.check_call(call)
    }
}
