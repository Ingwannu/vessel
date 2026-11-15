//! Aggregate only current permission state; answering still uses destination ownership.

use rmcp::model::{JsonObject, Tool};
use serde_json::{Value, json};
use vessel_proto::conversation::GetConversation;
use vessel_proto::mcp::{
    MAX_PENDING_PERMISSIONS, MAX_PERMISSION_SESSIONS, PendingPermission, PendingPermissions,
};
use vessel_proto::{ListSessions, SessionMode};

use super::{ToolError, VesselTools, tool};

impl VesselTools {
    pub(super) async fn pending_permissions(
        &self,
        args: &JsonObject,
    ) -> Result<PendingPermissions, ToolError> {
        let node = self.target(args).await?;
        let sessions = self.on(node, ListSessions::default()).await?;
        if sessions.len() > MAX_PERMISSION_SESSIONS {
            return Err(ToolError::PermissionInventory);
        }
        let mut permissions = Vec::new();
        for session in sessions {
            if !session.status.is_live()
                || matches!(session.spec.mode, SessionMode::Terminal { .. })
            {
                continue;
            }
            let conversation = self
                .on(
                    node,
                    GetConversation {
                        session: session.id,
                    },
                )
                .await?;
            if permissions
                .len()
                .saturating_add(conversation.permissions.len())
                > MAX_PENDING_PERMISSIONS
            {
                return Err(ToolError::PermissionInventory);
            }
            permissions.extend(conversation.permissions.into_iter().map(|request| {
                PendingPermission {
                    session: session.id,
                    status: session.status.clone(),
                    request,
                }
            }));
        }
        PendingPermissions::new(permissions).map_err(|_bound| ToolError::PermissionInventory)
    }
}

pub(super) fn tools(node: &Value) -> Vec<Tool> {
    vec![tool(
        "list_pending_permissions",
        "Current pending permission requests on a machine, read from each live conversation. This is a sequential observation, not an atomic snapshot. Answering still requires ownership. Excessive inventories are refused whole.",
        json!({"node":node}),
        &[],
    )]
}
