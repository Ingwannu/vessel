//! Current permission summaries reuse conversation reads, never replay old log requests.

use super::InvalidMcp;
use crate::{PermissionRequest, SessionId, SessionInfo, SessionStatus};
use serde::Serialize;

/// Maximum session inventory for one complete aggregate read.
pub const MAX_PERMISSION_SESSIONS: usize = 256;
/// Maximum current permission requests returned without truncation.
pub const MAX_PENDING_PERMISSIONS: usize = 1024;

/// A current request plus the session that owns it; not permission to answer it.
#[derive(Debug, Clone, Serialize)]
pub struct PendingPermission {
    /// Destination-local session identity.
    pub session: SessionId,
    /// Status observed in the inventory before its conversation was read.
    pub status: SessionStatus,
    /// Only the existing sanitized protocol request is exposed.
    pub request: PermissionRequest,
}

/// Complete bounded output; a refusal never returns a partial inventory.
#[derive(Debug, Clone, Serialize)]
pub struct PendingPermissions {
    permissions: Vec<PendingPermission>,
}
impl PendingPermissions {
    /// Bound the full aggregate before it can be serialized as a tool result.
    ///
    /// # Errors
    /// More than [`MAX_PENDING_PERMISSIONS`] requests would be retained.
    pub fn new(permissions: Vec<PendingPermission>) -> Result<Self, InvalidMcp> {
        if permissions.len() > MAX_PENDING_PERMISSIONS {
            return Err(InvalidMcp::PermissionCount);
        }
        Ok(Self { permissions })
    }
}

/// Session metadata plus a sequential read of its current structured permissions.
/// Terminal sessions have no structured permission inventory.
#[derive(Debug, Clone, Serialize)]
pub struct SessionDetails {
    /// Existing session metadata fields retain their original positions.
    #[serde(flatten)]
    pub session: SessionInfo,
    /// Current protocol requests, rather than historic log entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<PermissionRequest>>,
}
