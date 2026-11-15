//! The agent-tool contract shared by the node and its stdio adapter.
//! Values only: authentication, transport and filesystem work live elsewhere.

mod context;
mod id;
pub use context::VoiceTools;

pub use id::{AddedId, ToolId};

pub(crate) mod bounded;
mod name;
mod permissions;
mod policy;
pub use permissions::{
    MAX_PENDING_PERMISSIONS, MAX_PERMISSION_SESSIONS, PendingPermission, PendingPermissions,
    SessionDetails,
};

pub use name::{MAX_TOOL_NAME_BYTES, StoredTool, ToolName};
pub use policy::{
    Injection, MAX_DISABLED_TOOLS, MAX_PROVIDER_POLICIES, McpSettings, McpSettingsPatch,
    ProviderPolicies, ProviderPolicy, ToolAccess, ToolPolicy,
};

/// Settings and skill calls remain unavailable until integration assigns a version.
/// Assignment requires the node's launch wiring and both settings clients.
pub const TOOL_POLICY_VARIABLE: &str = "VESSEL_TOOL_POLICY";

/// Feature admission stays pending until both settings clients are integrated.
pub const MCP_PARITY: Option<crate::ProtocolVersion> = None;

/// A refused agent-tool configuration; errors never echo untrusted input.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidMcp {
    /// Tool names must be bounded, printable identifiers.
    #[error("tool name must contain 1..={MAX_TOOL_NAME_BYTES} ASCII identifier bytes")]
    ToolName,
    /// A client requested disabling a tool this node cannot enforce.
    #[error("unknown tool in policy change")]
    UnknownTool,
    /// Duplicate names would make the saved policy ambiguous.
    #[error("duplicate tool in denylist")]
    DuplicateTool,
    /// Bound precedes element decoding.
    #[error("denylist exceeds {MAX_DISABLED_TOOLS} tools")]
    DisabledCount,
    /// Only one override may match a provider.
    #[error("duplicate provider policy")]
    DuplicateProvider,
    /// A bounded node configuration is required.
    #[error("policy exceeds {MAX_PROVIDER_POLICIES} providers")]
    ProviderCount,
    /// A complete request inventory must fit the output budget.
    #[error("pending permissions exceed {MAX_PENDING_PERMISSIONS} requests")]
    PermissionCount,
}

#[cfg(test)]
mod tests;
