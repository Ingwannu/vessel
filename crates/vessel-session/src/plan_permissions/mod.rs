//! Synthetic plan permissions belong to one live provider run, never to a log replay.
//! Native follow-up construction and mode transitions stay inside the driver.
mod state;
use crate::moving::Control;
use crate::structured::StructuredActor;
use crate::{SessionError, SessionHandle};
use state::Offer;
pub(crate) use state::PlanApprovals;
use tokio::sync::oneshot;
use vessel_agent::PermissionDispatch;
use vessel_proto::timeline::{
    DecidePlan, PermissionDeliveryState, PermissionResponse, PermissionResponseRefusal,
    PlanDecision, PlanDecisionRefusal, PlanDecisionReply, PlanProposal,
};
use vessel_proto::{AgentEvent, PermissionId, PermissionOutcome, SessionEvent, StopReason};

#[derive(Debug)]
pub(crate) struct PermissionOrder {
    pub(crate) id: PermissionId,
    pub(crate) response: PermissionResponse,
    pub(crate) reply: oneshot::Sender<Result<PermissionDispatch, SessionError>>,
}
impl SessionHandle {
    pub(crate) async fn permission_order(
        &self,
        id: PermissionId,
        response: PermissionResponse,
    ) -> Result<PermissionDispatch, SessionError> {
        let route = self.structured("a permission response")?;
        self.order(route, |reply| {
            Control::Permission(PermissionOrder {
                id,
                response,
                reply,
            })
        })
        .await
    }
}
impl StructuredActor {
    pub(crate) async fn offer_plan(&mut self, proposal: PlanProposal) {
        if !self.ready
            || self.move_state.replaying
            || self.move_state.active
            || self.desk.turn().is_none()
            || self.plans.answering()
        {
            return;
        }
        match self.plans.offer(proposal) {
            Offer::Duplicate => {}
            Offer::Full => {
                self.publish_plan_event(AgentEvent::Error {
                    message: "Plan approval capacity is full for this provider run".to_owned(),
                })
                .await
            }
            Offer::New { request, displaced } => {
                if let Some(id) = displaced {
                    self.plan_resolved(id, PermissionOutcome::Cancelled).await;
                }
                self.publish_plan_event(AgentEvent::PermissionRequested(*request))
                    .await;
            }
        }
    }
    pub(crate) async fn permission_order(&mut self, order: PermissionOrder) {
        let result = if self.plans.known(order.id) {
            self.decide_owned_plan(order.id, order.response).await
        } else {
            let input = self.input.clone();
            self.pump(async move { input.respond_permission(order.id, order.response).await })
                .await
                .map_err(|_stopped| SessionError::Ended)
                .and_then(|reply| reply.map_err(SessionError::from))
        };
        let _closed = order.reply.send(result);
    }
    async fn decide_owned_plan(
        &mut self,
        id: PermissionId,
        response: PermissionResponse,
    ) -> Result<PermissionDispatch, SessionError> {
        if let Some(reason) = vessel_agent::permission_response_refusal(&response) {
            return Ok(PermissionDispatch::Refused(reason));
        }
        let (request, decision) = self.plans.begin(id, &response.outcome)?;
        let Some(proposal) = request.plan_proposal.as_deref().cloned() else {
            return Ok(PermissionDispatch::OutcomeUnknown);
        };
        self.publish_plan_event(AgentEvent::PermissionRequested(request))
            .await;
        let input = self.input.clone();
        let expected = proposal.clone();
        let result = self
            .pump(async move { input.decide_plan(DecidePlan { proposal, decision }).await })
            .await;
        if !self.plans.current_is(id) {
            return Ok(PermissionDispatch::OutcomeUnknown);
        }
        match result {
            Ok(Ok(PlanDecisionReply::Accepted {
                proposal,
                decision: applied,
            })) if proposal == expected && applied == decision => {
                let _retired = self.plans.take();
                if decision != PlanDecision::Dismiss {
                    self.desk.started_one();
                }
                self.plan_resolved(id, response.outcome.clone()).await;
                Ok(PermissionDispatch::Applied(response.outcome))
            }
            Ok(Ok(PlanDecisionReply::Refused { proposal, reason })) if proposal == expected => {
                self.plan_refusal(id, reason).await
            }
            Ok(Ok(
                PlanDecisionReply::Accepted { .. }
                | PlanDecisionReply::Refused { .. }
                | PlanDecisionReply::OutcomeUnknown { .. },
            ))
            | Ok(Err(_))
            | Err(_) => {
                self.plan_state(PermissionDeliveryState::OutcomeUnknown)
                    .await;
                Ok(PermissionDispatch::OutcomeUnknown)
            }
        }
    }
    async fn plan_refusal(
        &mut self,
        id: PermissionId,
        reason: PlanDecisionRefusal,
    ) -> Result<PermissionDispatch, SessionError> {
        match reason {
            PlanDecisionRefusal::StaleProposal | PlanDecisionRefusal::TurnChanged => {
                let _retired = self.plans.take();
                self.plan_resolved(id, PermissionOutcome::Cancelled).await;
                Ok(PermissionDispatch::Applied(PermissionOutcome::Cancelled))
            }
            PlanDecisionRefusal::ResumeUnavailable
            | PlanDecisionRefusal::Unsupported
            | PlanDecisionRefusal::InvalidState => {
                self.plan_state(PermissionDeliveryState::Pending).await;
                Ok(PermissionDispatch::Refused(
                    PermissionResponseRefusal::PlanDecision { reason },
                ))
            }
        }
    }
    pub(crate) async fn plan_before_event(&mut self, event: &AgentEvent) {
        // An in-flight provider transition may itself produce the real follow-up
        // start/end before its ack. Only that provider can distinguish it from a
        // competing native turn; keep the card inert until its exact-key reply.
        if self.plans.answering() {
            return;
        }
        if matches!(
            event,
            AgentEvent::TurnStarted { .. }
                | AgentEvent::TurnFailed { .. }
                | AgentEvent::TurnEnded {
                    reason: StopReason::Cancelled
                        | StopReason::Refusal
                        | StopReason::MaxTokens
                        | StopReason::MaxTurnRequests,
                    ..
                }
        ) {
            self.withdraw_plan().await;
        }
    }
    pub(crate) async fn withdraw_plan(&mut self) {
        if let Some(id) = self.plans.take() {
            self.plan_resolved(id, PermissionOutcome::Cancelled).await;
        }
    }
    async fn plan_state(&mut self, state: PermissionDeliveryState) {
        if let Some(request) = self.plans.update(state) {
            self.publish_plan_event(AgentEvent::PermissionRequested(request))
                .await;
        }
    }
    async fn plan_resolved(&mut self, id: PermissionId, outcome: PermissionOutcome) {
        self.publish_plan_event(AgentEvent::PermissionResolved { id, outcome })
            .await;
    }
    async fn publish_plan_event(&mut self, event: AgentEvent) {
        self.life
            .publish(&mut self.agent, SessionEvent::Agent { event })
            .await;
    }
}
