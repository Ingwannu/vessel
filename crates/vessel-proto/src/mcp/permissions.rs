//! Current permission summaries reuse conversation reads, never replay old log requests.

use super::InvalidMcp;
use crate::{PermissionRequest, SessionId, SessionStatus};
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
