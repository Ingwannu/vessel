//! Finite current-run plan authority. Log replay never reconstructs these keys.
use std::collections::HashMap;
use vessel_proto::timeline::{
    PermissionAction, PermissionActionIntent, PermissionBehavior, PermissionDeliveryState,
    PermissionKind, PermissionPresentation, PlanDecision, PlanProposal, PlanProposalKey,
    PlanProposalSource, PlanResumeAvailability, ToolCallScope,
};
use vessel_proto::{
    PermissionId, PermissionOption, PermissionOptionId, PermissionOptionKind, PermissionRequest,
    ToolCallId,
};

const MAX_PLAN_KEYS: usize = 128;
const MAX_PLAN_KEY_BYTES: usize = 262_144;
const PLAN_KEY_OVERHEAD: usize = 256;

#[derive(Debug, Default)]
pub(crate) struct PlanApprovals {
    seen: HashMap<PlanProposalKey, PermissionId>,
    bytes: usize,
    pub(super) current: Option<Pending>,
}
#[derive(Debug)]
pub(super) struct Pending {
    pub(super) request: PermissionRequest,
    actions: Vec<(PermissionOptionId, PlanDecision)>,
}
#[derive(Debug)]
pub(super) enum Offer {
    Duplicate,
    Full,
    New {
        request: Box<PermissionRequest>,
        displaced: Option<PermissionId>,
    },
}
impl PlanApprovals {
    pub(super) fn offer(&mut self, proposal: PlanProposal) -> Offer {
        if self.seen.contains_key(&proposal.key) {
            return Offer::Duplicate;
        }
        let key_bytes = key_charge(&proposal.key);
        let Some(bytes) = self.bytes.checked_add(key_bytes) else {
            return Offer::Full;
        };
        if self.seen.len() >= MAX_PLAN_KEYS || bytes > MAX_PLAN_KEY_BYTES {
            return Offer::Full;
        }
        let request = request(&proposal);
        let actions = request
            .options
            .iter()
            .filter_map(|option| {
                let intent = option.action.as_ref()?.intent?;
                Some((
                    option.id.clone(),
                    match intent {
                        PermissionActionIntent::Implement => PlanDecision::Implement,
                        PermissionActionIntent::ImplementResume => PlanDecision::ImplementResume,
                        PermissionActionIntent::Dismiss => PlanDecision::Dismiss,
                    },
                ))
            })
            .collect();
        let _old = self.seen.insert(proposal.key, request.id);
        self.bytes = bytes;
        let displaced = self
            .current
            .replace(Pending {
                request: request.clone(),
                actions,
            })
            .map(|pending| pending.request.id);
        Offer::New {
            request: Box::new(request),
            displaced,
        }
    }
    pub(super) fn current_is(&self, id: PermissionId) -> bool {
        self.current
            .as_ref()
            .is_some_and(|pending| pending.request.id == id)
    }
    pub(super) fn known(&self, id: PermissionId) -> bool {
        self.seen.values().any(|known| *known == id)
    }
    pub(super) fn begin(
        &mut self,
        id: PermissionId,
        outcome: &vessel_proto::PermissionOutcome,
    ) -> Result<(PermissionRequest, PlanDecision), vessel_agent::InputError> {
        let Some(pending) = self
            .current
            .as_mut()
            .filter(|pending| pending.request.id == id)
        else {
            return Err(vessel_agent::InputError::PermissionSettled(id));
        };
        if !pending.request.delivery_state.is_pending() {
            return Err(vessel_agent::InputError::PermissionSettled(id));
        }
        let decision = match outcome {
            vessel_proto::PermissionOutcome::Cancelled => PlanDecision::Dismiss,
            vessel_proto::PermissionOutcome::Selected { option } => pending
                .actions
                .iter()
                .find(|(known, _)| known == option)
                .map(|(_, decision)| *decision)
                .ok_or_else(|| vessel_agent::InputError::UnknownOption {
                    id,
                    option: option.clone(),
                })?,
        };
        pending.request.delivery_state = PermissionDeliveryState::Answering;
        Ok((pending.request.clone(), decision))
    }
    pub(super) fn update(&mut self, state: PermissionDeliveryState) -> Option<PermissionRequest> {
        let pending = self.current.as_mut()?;
        pending.request.delivery_state = state;
        Some(pending.request.clone())
    }
    pub(super) fn take(&mut self) -> Option<PermissionId> {
        self.current.take().map(|pending| pending.request.id)
    }
    pub(super) fn answering(&self) -> bool {
        self.current.as_ref().is_some_and(|pending| {
            pending.request.delivery_state == PermissionDeliveryState::Answering
        })
    }
}
fn key_charge(key: &PlanProposalKey) -> usize {
    // Validated identity lengths are finite; saturation refuses instead of wrapping.
    let source = match &key.source {
        PlanProposalSource::CompletedItem { item } => item.as_str().len(),
        PlanProposalSource::LegacyTurnPlan => 0,
    };
    key.thread
        .as_str()
        .len()
        .saturating_add(key.turn.as_str().len())
        .saturating_add(source)
        .saturating_add(PLAN_KEY_OVERHEAD)
}
fn request(proposal: &PlanProposal) -> PermissionRequest {
    let mut options = vec![
        option(PlanDecision::Implement),
        option(PlanDecision::Dismiss),
    ];
    if proposal.resume == PlanResumeAvailability::Available {
        options.insert(1, option(PlanDecision::ImplementResume));
    }
    let tool_call = match &proposal.key.source {
        PlanProposalSource::CompletedItem { item } => Some(ToolCallId::new(item.as_str())),
        PlanProposalSource::LegacyTurnPlan => None,
    };
    PermissionRequest {
        id: PermissionId::random(),
        tool_call,
        options,
        tool_scope: Some(Box::new(ToolCallScope::ProviderTurn {
            thread: proposal.key.thread.clone(),
            turn: proposal.key.turn.clone(),
        })),
        plan_proposal: Some(Box::new(proposal.key.clone())),
        delivery_state: PermissionDeliveryState::Pending,
        presentation: Some(PermissionPresentation {
            kind: PermissionKind::Plan,
            title: Some("Implement this plan?".to_owned()),
            description: None,
            detail: None,
            plan_text: Some(proposal.text.as_str().to_owned()),
        }),
    }
}
fn option(decision: PlanDecision) -> PermissionOption {
    let (id, name, kind, behavior, intent) = match decision {
        PlanDecision::Implement => (
            "implement",
            "Implement",
            PermissionOptionKind::AllowOnce,
            PermissionBehavior::Allow,
            PermissionActionIntent::Implement,
        ),
        PlanDecision::ImplementResume => (
            "implement_resume",
            "Implement and resume",
            PermissionOptionKind::AllowOnce,
            PermissionBehavior::Allow,
            PermissionActionIntent::ImplementResume,
        ),
        PlanDecision::Dismiss => (
            "dismiss",
            "Dismiss",
            PermissionOptionKind::RejectOnce,
            PermissionBehavior::Deny,
            PermissionActionIntent::Dismiss,
        ),
    };
    PermissionOption {
        id: PermissionOptionId::new(id),
        name: name.to_owned(),
        kind,
        action: Some(PermissionAction {
            behavior,
            variant: None,
            intent: Some(intent),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn proposal(turn: &str) -> PlanProposal {
        PlanProposal {
            key: PlanProposalKey {
                thread: "thread".parse().unwrap(),
                turn: turn.parse().unwrap(),
                source: PlanProposalSource::LegacyTurnPlan,
            },
            text: vessel_proto::timeline::PlanText::new("# Actual plan").unwrap(),
            resume: PlanResumeAvailability::Unavailable,
        }
    }
    #[test]
    fn legacy_plan_has_no_fabricated_tool_and_exact_option_maps_to_typed_decision() {
        let mut owner = PlanApprovals::default();
        let Offer::New { request, .. } = owner.offer(proposal("turn")) else {
            panic!("new");
        };
        assert_eq!(request.tool_call, None);
        assert_eq!(request.options.len(), 2);
        let outcome = vessel_proto::PermissionOutcome::Selected {
            option: request.options[0].id.clone(),
        };
        let (answering, decision) = owner.begin(request.id, &outcome).unwrap();
        assert_eq!(decision, PlanDecision::Implement);
        assert_eq!(answering.delivery_state, PermissionDeliveryState::Answering);
        assert!(owner.begin(request.id, &outcome).is_err());
        owner
            .update(PermissionDeliveryState::OutcomeUnknown)
            .unwrap();
        assert!(owner.begin(request.id, &outcome).is_err());
        assert!(matches!(owner.offer(proposal("turn")), Offer::Duplicate));
    }
    #[test]
    fn no_eviction_at_capacity_and_retired_identity_never_becomes_new_authority() {
        let mut owner = PlanApprovals::default();
        let mut first = None;
        for number in 0..MAX_PLAN_KEYS {
            let Offer::New { request, .. } = owner.offer(proposal(&format!("turn-{number}")))
            else {
                panic!("fits");
            };
            if first.is_none() {
                first = Some(request.id);
            }
            let _retired = owner.take();
        }
        assert!(matches!(owner.offer(proposal("overflow")), Offer::Full));
        assert!(matches!(owner.offer(proposal("turn-0")), Offer::Duplicate));
        let id = first.unwrap();
        assert!(owner.known(id));
        assert!(
            matches!(owner.begin(id, &vessel_proto::PermissionOutcome::Cancelled), Err(vessel_agent::InputError::PermissionSettled(known)) if known == id)
        );
    }
    #[test]
    fn byte_floor_is_independent_of_count_and_resume_requires_explicit_availability() {
        let mut owner = PlanApprovals {
            bytes: MAX_PLAN_KEY_BYTES,
            ..PlanApprovals::default()
        };
        assert!(matches!(owner.offer(proposal("turn")), Offer::Full));
        let mut owner = PlanApprovals::default();
        let mut source = proposal("turn");
        source.resume = PlanResumeAvailability::Available;
        let Offer::New { request, .. } = owner.offer(source) else {
            panic!("new");
        };
        assert_eq!(request.options.len(), 3);
        let (_, decision) = owner
            .begin(
                request.id,
                &vessel_proto::PermissionOutcome::Selected {
                    option: request.options[1].id.clone(),
                },
            )
            .unwrap();
        assert_eq!(decision, PlanDecision::ImplementResume);
    }
}
