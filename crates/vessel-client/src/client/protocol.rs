//! Negotiated protocol facts belong beside admission, separate from endpoint calls.

use super::NodeClient;
use vessel_proto::ProtocolVersion;

impl NodeClient {
    /// The protocol version in use.
    #[must_use]
    pub fn protocol(&self) -> ProtocolVersion {
        self.connection.welcome().protocol
    }
}
