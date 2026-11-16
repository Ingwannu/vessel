//! Ordered workspace assignment projection; catalog owns color and spelling.
use super::LabelName;
use crate::WorkspaceId;
use serde::{Deserialize, Serialize};

/// Nonempty assignment group joined in the registry's atomic read transaction.
/// Missing/empty group means no assignments; no label data is stored in descriptor JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WorkspaceLabelAssignments {
    /// Stable registry identity on `RegistrySnapshot::node`.
    pub workspace: WorkspaceId,
    /// Committed assignment order, retaining host-canonical display spelling.
    pub labels: Vec<LabelName>,
}
