//! One tool renderer accepts legacy projection or canonical in-memory fixtures.
//! The node owner attaches the canonical fields; this adapter never decodes them.

use super::ToolView;
use vessel_proto::timeline::{ToolCallDetail, ToolFailure};

/// Typed presentation seam while canonical fields await the node producer.
/// No method authorizes an operation or manufactures a provider detail.
pub trait ToolDetailSource {
    /// Existing projection supplies title, status, identity and legacy fallback.
    fn tool(&self) -> &ToolView;

    /// Canonical meaning, when the producer actually supplied it.
    fn detail(&self) -> Option<&ToolCallDetail>;

    /// An explicit provider failure remains separate from ordinary tool output.
    fn failure(&self) -> Option<&ToolFailure>;
}

impl ToolDetailSource for ToolView {
    fn tool(&self) -> &ToolView {
        self
    }

    fn detail(&self) -> Option<&ToolCallDetail> {
        // Standalone DTOs do not attach fields to ToolCall yet. W23's checked
        // field/scope handoff replaces this compatibility adapter directly.
        None
    }

    fn failure(&self) -> Option<&ToolFailure> {
        None
    }
}
