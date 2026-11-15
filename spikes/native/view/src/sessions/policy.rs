//! Closing a managed child removes layout; closing a root archives it.
use vessel_proto::Creator;
/// Distinguish durable archive from local navigation before closing a tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosePolicy {
    /// Preserve the child and its relationship when its tab closes.
    LayoutOnly,
    /// Archive the root through the existing confirmed lifecycle path.
    Archive,
}
/// Paseo's close-tab-policy: only a managed child closes without archiving.
#[must_use]
pub const fn close_policy(creator: &Creator) -> ClosePolicy {
    match creator {
        Creator::Agent { .. } => ClosePolicy::LayoutOnly,
        Creator::Owner | Creator::Device { .. } => ClosePolicy::Archive,
    }
}
