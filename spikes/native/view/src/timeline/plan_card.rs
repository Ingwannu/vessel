//! Plan proposals share one outcome model between their dock and stream.
//! A tool finishing does not prove approval: only its permission record does.

use serde::Deserialize;
use vessel_proto::{PermissionOutcome, ToolCallId};

use super::{Entry, ToolView, allows};

/// The plan card's title and initial disclosure state follow its decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanOutcome {
    /// No permission decision has been observed.
    Proposed,
    /// Its permission is still actionable in the dock.
    Pending,
    /// The person selected an offered allow action.
    Approved,
    /// The person selected an offered reject action.
    Rejected,
    /// The request was withdrawn by the provider.
    Canceled,
}

impl PlanOutcome {
    /// Rejected and canceled plans start folded, as Paseo's PlanCard does.
    #[must_use]
    pub const fn expanded(self) -> bool {
        match self {
            Self::Proposed | Self::Pending | Self::Approved => true,
            Self::Rejected | Self::Canceled => false,
        }
    }
}

/// The legacy wire carries a plan in opaque tool input. Decode only this
/// field here until the canonical detail contract supplies it explicitly.
#[derive(Deserialize)]
struct PlanInput {
    plan: String,
}

/// Plan Markdown already carried by the tool, without rendering its JSON wrapper.
#[must_use]
pub fn plan_text(tool: &ToolView) -> Option<String> {
    serde_json::from_str::<PlanInput>(tool.input.as_ref()?.as_text())
        .ok()
        .map(|input| input.plan)
        .filter(|plan| !plan.trim().is_empty())
}

/// The latest permission for this tool determines its displayed decision;
/// another tool's decision and unknown option ids never imply approval.
#[must_use]
pub fn plan_outcome(entries: &[Entry], tool: &ToolCallId) -> PlanOutcome {
    let permission = entries
        .iter()
        .rev()
        .filter_map(|entry| entry.kind.as_permission())
        .find(|permission| permission.request.tool_call == *tool);
    let Some(permission) = permission else {
        return PlanOutcome::Proposed;
    };
    match &permission.outcome {
        None => PlanOutcome::Pending,
        Some(PermissionOutcome::Cancelled) => PlanOutcome::Canceled,
        Some(PermissionOutcome::Selected { option, .. }) => permission
            .request
            .options
            .iter()
            .find(|offered| offered.id == *option)
            .map_or(PlanOutcome::Proposed, |offered| {
                if allows(offered.kind) {
                    PlanOutcome::Approved
                } else {
                    PlanOutcome::Rejected
                }
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_negative_decisions_start_folded() {
        for state in [
            PlanOutcome::Proposed,
            PlanOutcome::Pending,
            PlanOutcome::Approved,
        ] {
            assert!(state.expanded());
        }
        for state in [PlanOutcome::Rejected, PlanOutcome::Canceled] {
            assert!(!state.expanded());
        }
        assert_eq!(
            plan_outcome(&[], &ToolCallId::new("plan")),
            PlanOutcome::Proposed
        );
    }
}
