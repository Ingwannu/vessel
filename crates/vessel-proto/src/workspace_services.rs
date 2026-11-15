//! Workspace-scoped service observations and controls, never runtime authority.
//! Values remain separate from legacy root-based script calls (ADR 0066).

mod origin;

pub use crate::id::{ServiceSnapshotId, ServicesSettingsRevision};
pub use origin::{
    InvalidServiceProxyUrl, MAX_PROXY_HOST_BYTES, MAX_PROXY_LABEL_BYTES, MAX_PROXY_URL_BYTES,
    ServiceProxyUrl,
};

use serde::{Deserialize, Serialize};

use crate::{ProtocolVersion, WorkspaceId, scripts::ScriptName};

/// Assign only after complete native status/env/public controls, every client's
/// approval display and qualified node/runtime/registry/connection admission.
/// Staged values cannot enable a helper, public route or private Preview.
pub const WORKSPACE_SERVICES: Option<ProtocolVersion> = None;

/// A canonical workspace/name query target, not a process or socket capability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WorkspaceScriptRef {
    /// Identity comes from the registry, never a directory match.
    pub workspace: WorkspaceId,
    /// The approved definition's validated name.
    pub script: ScriptName,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_targets_and_etags_decode_without_inventing_ownership() {
        let target = WorkspaceScriptRef {
            workspace: WorkspaceId::random(),
            script: "web-app".parse().unwrap(),
        };
        let text = serde_json::to_string(&target).unwrap();
        assert_eq!(
            serde_json::from_str::<WorkspaceScriptRef>(&text).unwrap(),
            target
        );
        let another = WorkspaceScriptRef {
            workspace: WorkspaceId::random(),
            script: target.script.clone(),
        };
        assert_ne!(target, another);
        assert!(serde_json::from_str::<WorkspaceScriptRef>(r#"{"script":"web-app"}"#).is_err());
        for text in ["not-an-id", ""] {
            let encoded = serde_json::to_string(text).unwrap();
            assert!(serde_json::from_str::<ServiceSnapshotId>(&encoded).is_err());
            assert!(serde_json::from_str::<ServicesSettingsRevision>(&encoded).is_err());
        }
        let etag = ServiceSnapshotId::random();
        assert_eq!(etag.to_string().parse::<ServiceSnapshotId>().unwrap(), etag);
        let revision = ServicesSettingsRevision::random();
        assert_eq!(
            revision
                .to_string()
                .parse::<ServicesSettingsRevision>()
                .unwrap(),
            revision
        );
        assert!(WORKSPACE_SERVICES.is_none());
    }
}
