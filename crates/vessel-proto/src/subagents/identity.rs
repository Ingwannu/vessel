//! A provider-owned task is a read-only source, never a managed lifecycle target.
use super::ProviderChildId;
use serde::{Deserialize, Serialize};
/// Fleet-wide identity, independent of a mutable machine-book alias.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProviderChildRef {
    /// Managed parent that owns the durable provider observations.
    pub parent: crate::SessionRef,
    /// Actual provider task/thread identity within that parent.
    pub child: ProviderChildId,
}
