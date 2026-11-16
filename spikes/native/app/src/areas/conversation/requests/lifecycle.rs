//! Owner holds survive transport retirement until exact authoritative resolution.
use super::{ConversationArea, Requests, Stage};
use vessel_app_view::requests::Request;
use vessel_app_view::residents::{
    FollowBinding, FollowRetirement, HoldReason, OperationKind, OwnerReadiness,
};
use vessel_proto::{AgentEvent, EventRecord, SessionEvent};

impl Requests {
    /// Live requests and actual editors remain in `ConversationArea`. No snapshot
    /// or replacement engine is made while they hold the resource.
    #[must_use]
    pub(crate) fn readiness(&self, area: &ConversationArea) -> OwnerReadiness {
        if self.0.iter().any(|effect| effect.stage == Stage::Uncertain) {
            OwnerReadiness::Hold(HoldReason::UncertainEffect)
        } else if !self.0.is_empty()
            || area.permit_sending.is_some()
            || area
                .answering
                .as_ref()
                .is_some_and(super::super::question::Answering::request_sending)
        {
            OwnerReadiness::Hold(HoldReason::PendingEffect)
        } else if area.answering.is_some()
            || area.timeline.waiting_permissions() != 0
            || !area.timeline.waiting_questions().is_empty()
            || !area.timeline.unseen_permissions(&area.live).is_empty()
            || !area.timeline.unseen_questions(&area.live).is_empty()
        {
            OwnerReadiness::Hold(HoldReason::DirtyInput)
        } else {
            OwnerReadiness::Ready
        }
    }

    /// Mandatory synchronous retirement callback, BEFORE replacement launch.
    /// Retained form state is read via stage(request) to hide Sending and show
    /// uncertainty; this never drops an editor or clears its sent fence.
    pub(crate) fn retired(&mut self, notice: &FollowRetirement) {
        for effect in &mut self.0 {
            if effect.binding == notice.binding()
                && notice.operations().iter().any(|operation| {
                    operation.kind() == OperationKind::Request
                        && operation.ticket() == effect.ticket
                })
            {
                effect.stage = Stage::Uncertain;
            }
        }
    }

    /// Call only from an already admitted exact follow ingress. A replacement
    /// binding can reconcile the SAME resident, never another owner. Snapshot
    /// omission, a stale Pending read or an unrelated resolution releases nothing.
    /// Sending tickets remain for the canonical settle path to consume first.
    pub(crate) fn observed(&mut self, ingress: FollowBinding, records: &[EventRecord]) {
        self.0.retain(|effect| {
            effect.stage == Stage::Sending
                || effect.binding.resident != ingress.resident
                || !records
                    .iter()
                    .any(|record| resolves(record, effect.request))
        });
    }
}
fn resolves(record: &EventRecord, request: Request) -> bool {
    match request {
        Request::Permission(expected) => matches!(
            &record.event, SessionEvent::Agent { event: AgentEvent::PermissionResolved { id, .. } }
                if *id == expected
        ),
        Request::Question(expected) => matches!(
            &record.event, SessionEvent::Agent { event: AgentEvent::QuestionResolved { id, .. } }
                if *id == expected
        ),
    }
}
