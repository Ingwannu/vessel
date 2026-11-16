//! Approval attachment rules depend on the extended request, not display values.
use super::PermissionDeliveryState;
use serde::{Deserialize, Serialize};

impl crate::PermissionRequest {
    /// Older clients require an actual tool and cannot represent synthetic lifetimes
    /// or an already admitted response. Such requests become inert before decoding.
    pub(crate) fn legacy_actionable(&self) -> bool {
        self.tool_call.is_some() && self.plan_proposal.is_none() && self.delivery_state.is_pending()
    }
}

/// Bounded metadata retained when a newer approval degrades to an inbox pointer.
/// Its presence means a legacy client cannot safely offer a response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PermissionReference {
    /// Actual provider proposal, when this is a synthetic plan approval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_proposal: Option<Box<super::PlanProposalKey>>,
    /// Response delivery remains authoritative even without the full card.
    pub delivery_state: PermissionDeliveryState,
}
impl PermissionReference {
    /// Ordinary legacy-compatible pointers retain their original omitted shape.
    #[must_use]
    pub fn from_request(request: &crate::PermissionRequest) -> Option<Self> {
        (!request.legacy_actionable()).then(|| Self {
            plan_proposal: request.plan_proposal.clone(),
            delivery_state: request.delivery_state,
        })
    }
}
