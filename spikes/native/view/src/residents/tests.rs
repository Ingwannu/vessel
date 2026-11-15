//! Adversarial lifetime tests keep resource input separate from transport replies.
use super::*;
use crate::layout::{
    AgentRole, HostAlias, Layout, LayoutKey, LayoutMode, OpenIntent, PaneId, Placement, Position,
    Target,
};
use std::time::Duration;
use vessel_proto::{NodeId, SessionId};
fn claim(session: SessionId) -> PanelClaim {
    let mut layout = Layout::default();
    let tab = layout
        .open(
            Target::Agent(session),
            OpenIntent::Reveal,
            Placement::Focused,
            AgentRole::Unknown,
            LayoutMode::SplitPanes,
        )
        .unwrap();
    PanelClaim {
        workspace: LayoutKey::Session {
            machine: HostAlias::parse("test").unwrap(),
            session,
        },
        tab,
    }
}
fn fixture() -> (Residents, ResidentToken, PanelClaim) {
    let mut residents = Residents::new();
    let address = SessionAddress {
        node: NodeId::random(),
        session: SessionId::random(),
    };
    let claim = claim(address.session);
    let token = residents
        .claim(address, claim.clone(), PaneId::MAIN)
        .unwrap();
    (residents, token, claim)
}
#[test]
fn claim_key_cannot_substitute_another_node_or_legacy_session_before_allocation() {
    let address = SessionAddress {
        node: NodeId::random(),
        session: SessionId::random(),
    };
    let mut residents = Residents::new();
    let mut wrong_node = claim(address.session);
    wrong_node.workspace = LayoutKey::Workspace {
        node: NodeId::random(),
        workspace: vessel_proto::WorkspaceId::random(),
    };
    assert_eq!(
        residents.claim(address, wrong_node, PaneId::MAIN),
        Err(ResidentProblem::OwnerConflict)
    );
    assert_eq!(
        residents.claim(address, claim(SessionId::random()), PaneId::MAIN),
        Err(ResidentProblem::OwnerConflict)
    );
    let token = residents
        .claim(address, claim(address.session), PaneId::MAIN)
        .unwrap();
    assert_eq!(token.epoch, ResidentEpoch(1));
}
fn ready(residents: &mut Residents, token: ResidentToken) {
    for owner in ResourceOwner::ALL {
        residents
            .readiness(token, owner, OwnerReadiness::Ready)
            .unwrap();
    }
}
#[test]
fn main_timeline_ready_cannot_overwrite_a_provider_child_hold_or_settle_its_ticket() {
    let (mut residents, token, _) = fixture();
    ready(&mut residents, token);
    let binding = residents.begin_follow(token).unwrap();
    let child = residents
        .admit_operation(binding, OperationKind::Subagents)
        .unwrap();
    assert_eq!(
        residents.settle_operation(binding, OperationKind::Timeline, child),
        Err(ResidentProblem::UnadmittedOperation)
    );
    let retired = residents.retire_current(token).unwrap().unwrap();
    assert_eq!(
        retired.operations().first().unwrap().kind(),
        OperationKind::Subagents
    );
    residents
        .readiness(token, ResourceOwner::Timeline, OwnerReadiness::Ready)
        .unwrap();
    assert_eq!(residents.retire_clean(token), Err(ResidentProblem::Held));
    residents
        .readiness(token, ResourceOwner::Subagents, OwnerReadiness::Ready)
        .unwrap();
    assert_eq!(residents.retire_clean(token), Ok(()));
}
#[test]
fn same_node_reconnect_fences_queued_old_metadata_error_and_mutation_reply() {
    let (mut residents, token, _) = fixture();
    let first = residents.begin_follow(token).unwrap();
    let send = residents
        .admit_operation(first, OperationKind::Composer)
        .unwrap();
    residents.retire_follow(first).unwrap();
    let second = residents.begin_follow(token).unwrap();
    assert_eq!(
        second.resident, first.resident,
        "same-node input lifetime survives"
    );
    assert_ne!(second.follow, first.follow);
    // All network event kinds must pass this same ingress check before mutation.
    for _event in ["attach metadata", "stream records", "error"] {
        assert_eq!(
            residents.validate_binding(first),
            Err(ResidentProblem::StaleBinding)
        );
    }
    assert_eq!(
        residents.settle_operation(first, OperationKind::Composer, send),
        Err(ResidentProblem::StaleBinding)
    );
    assert_eq!(residents.validate_binding(second), Ok(()));
}
#[test]
fn aba_and_late_disconnect_cannot_retire_or_rebind_the_successor() {
    let (mut residents, token, _) = fixture();
    let old_a = residents.begin_follow(token).unwrap();
    residents.retire_follow(old_a).unwrap();
    let intermediate = residents.begin_follow(token).unwrap();
    residents.retire_follow(intermediate).unwrap();
    let new_a = residents.begin_follow(token).unwrap();
    assert_eq!(
        residents.retire_follow(old_a),
        Err(ResidentProblem::StaleBinding)
    );
    assert_eq!(
        residents.validate_binding(intermediate),
        Err(ResidentProblem::StaleBinding)
    );
    assert_eq!(residents.validate_binding(new_a), Ok(()));
}
#[test]
fn operation_replies_are_single_use_owner_bound_and_cannot_be_readmitted() {
    let (mut residents, token, _) = fixture();
    let binding = residents.begin_follow(token).unwrap();
    let ticket = residents
        .admit_operation(binding, OperationKind::Lifecycle)
        .unwrap();
    assert_eq!(
        residents.settle_operation(binding, OperationKind::Composer, ticket),
        Err(ResidentProblem::UnadmittedOperation)
    );
    residents
        .settle_operation(binding, OperationKind::Lifecycle, ticket)
        .unwrap();
    assert_eq!(
        residents.settle_operation(binding, OperationKind::Lifecycle, ticket),
        Err(ResidentProblem::UnadmittedOperation)
    );
    assert_ne!(
        residents
            .admit_operation(binding, OperationKind::Lifecycle)
            .unwrap(),
        ticket
    );
}
#[test]
fn hidden_input_and_uncertain_send_hold_resources_after_transport_retirement() {
    let (mut residents, token, _) = fixture();
    ready(&mut residents, token);
    residents
        .readiness(
            token,
            ResourceOwner::Composer,
            OwnerReadiness::Hold(HoldReason::DirtyInput),
        )
        .unwrap();
    assert_eq!(residents.retire_clean(token), Err(ResidentProblem::Held));
    ready(&mut residents, token);
    let binding = residents.begin_follow(token).unwrap();
    let _ticket = residents
        .admit_operation(binding, OperationKind::Composer)
        .unwrap();
    residents.retire_follow(binding).unwrap();
    assert_eq!(residents.retire_clean(token), Err(ResidentProblem::Held));
    assert_eq!(
        residents
            .entry(token)
            .unwrap()
            .owners
            .get(&ResourceOwner::Composer),
        Some(&OwnerReadiness::Hold(HoldReason::UncertainEffect))
    );
}
#[test]
fn move_and_hide_keep_resource_but_fence_old_view_focus_context() {
    let (mut residents, token, claim) = fixture();
    residents
        .present(token, &claim, PaneId::MAIN, Visibility::Visible)
        .unwrap();
    residents.focus(token, &claim).unwrap();
    let original = residents.context(token, &claim).unwrap();
    let mut layout = Layout::default();
    let side = layout
        .split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)
        .unwrap();
    residents
        .present(token, &claim, side, Visibility::Visible)
        .unwrap();
    assert_eq!(
        residents.validate_context(&original),
        Err(ResidentProblem::StaleEpoch)
    );
    assert_eq!(residents.context(token, &claim).unwrap().resident, token);
    residents
        .present(token, &claim, side, Visibility::RetainedHidden)
        .unwrap();
    assert!(residents.focused().is_none());
    assert_eq!(residents.focus(token, &claim), Err(ResidentProblem::Held));
}
#[test]
fn owner_conflict_window_recreation_and_eviction_cannot_reuse_old_tokens() {
    let (mut residents, token, claim) = fixture();
    let other = SessionAddress {
        node: NodeId::random(),
        session: token.address.session,
    };
    assert_eq!(
        residents.claim(other, claim.clone(), PaneId::MAIN),
        Err(ResidentProblem::OwnerConflict)
    );
    assert_eq!(
        Residents::new().context(token, &claim),
        Err(ResidentProblem::StaleEpoch)
    );
    ready(&mut residents, token);
    residents.retire_clean(token).unwrap();
    let replacement = residents
        .claim(token.address, claim.clone(), PaneId::MAIN)
        .unwrap();
    assert_ne!(replacement.epoch, token.epoch);
    assert_eq!(
        residents.context(token, &claim),
        Err(ResidentProblem::StaleEpoch)
    );
}
#[test]
fn capacity_and_operation_bounds_preserve_existing_admissions() {
    let (mut residents, token, original) = fixture();
    for _ in 1..MAX_RESIDENT_SESSIONS {
        let address = SessionAddress {
            node: token.address.node,
            session: SessionId::random(),
        };
        let _token = residents
            .claim(address, claim(address.session), PaneId::MAIN)
            .unwrap();
    }
    let extra = SessionAddress {
        node: token.address.node,
        session: SessionId::random(),
    };
    assert_eq!(
        residents.claim(extra, claim(extra.session), PaneId::MAIN),
        Err(ResidentProblem::Capacity)
    );
    assert_eq!(
        residents.claim(token.address, original, PaneId::MAIN),
        Ok(token)
    );
    let binding = residents.begin_follow(token).unwrap();
    for _ in 0..MAX_RESIDENT_OPERATIONS {
        let _ticket = residents
            .admit_operation(binding, OperationKind::Timeline)
            .unwrap();
    }
    assert_eq!(
        residents.admit_operation(binding, OperationKind::Composer),
        Err(ResidentProblem::Capacity)
    );
    assert_eq!(residents.validate_binding(binding), Ok(()));
}
#[test]
fn unclaimed_expiry_does_not_release_holds_or_authorize_backend_effects() {
    let (mut residents, token, claim) = fixture();
    residents
        .release_claim(token, &claim, Duration::ZERO)
        .unwrap();
    assert!(!residents.expired(
        token,
        UNCLAIMED_RESIDENT_TTL.saturating_sub(Duration::from_secs(1))
    ));
    assert!(residents.expired(token, UNCLAIMED_RESIDENT_TTL));
    assert_eq!(residents.retire_clean(token), Err(ResidentProblem::Held));
    ready(&mut residents, token);
    residents.retire_clean(token).unwrap();
    assert_eq!(
        residents.context(token, &claim),
        Err(ResidentProblem::Missing)
    );
}
#[test]
fn exhausted_follow_retires_old_attempt_without_wrapping_or_losing_input_lifetime() {
    let (mut residents, token, claim) = fixture();
    let binding = residents.begin_follow(token).unwrap();
    residents.entry_mut(token).unwrap().follow = FollowGeneration(u64::MAX);
    assert_eq!(
        residents.begin_follow(token),
        Err(ResidentProblem::Exhausted)
    );
    assert_eq!(
        residents.validate_binding(binding),
        Err(ResidentProblem::StaleBinding)
    );
    assert_eq!(residents.context(token, &claim).unwrap().resident, token);
}

#[test]
fn transport_retirement_reports_exact_tickets_despite_protected_resources_and_exhaustion() {
    let (mut residents, token, claim) = fixture();
    let _initial = residents.begin_follow(token).unwrap();
    residents.entry_mut(token).unwrap().follow = FollowGeneration(u64::MAX);
    let binding = FollowBinding {
        resident: token,
        follow: FollowGeneration(u64::MAX),
    };
    let ticket = residents
        .admit_operation(binding, OperationKind::Request)
        .unwrap();
    let retired = residents.retire_current(token).unwrap().unwrap();
    assert_eq!(retired.binding(), binding);
    assert_eq!(retired.operations().len(), 1);
    let operation = retired.operations().first().unwrap();
    assert_eq!(operation.kind(), OperationKind::Request);
    assert_eq!(operation.ticket(), ticket);
    assert_eq!(residents.retire_clean(token), Err(ResidentProblem::Held));
    assert_eq!(
        residents.begin_follow(token),
        Err(ResidentProblem::Exhausted)
    );
    assert_eq!(
        residents.validate_binding(binding),
        Err(ResidentProblem::StaleBinding)
    );
    assert_eq!(
        residents.settle_operation(binding, OperationKind::Request, ticket),
        Err(ResidentProblem::StaleBinding)
    );
    assert_eq!(residents.context(token, &claim).unwrap().resident, token);
}

#[test]
fn tickets_cannot_settle_another_resident_with_the_same_local_operation_ordinal() {
    let (mut residents, first, first_claim) = fixture();
    let address = SessionAddress {
        node: first.address.node,
        session: SessionId::random(),
    };
    assert_eq!(
        residents.claim(address, first_claim, PaneId::MAIN),
        Err(ResidentProblem::OwnerConflict)
    );
    let second = residents
        .claim(address, claim(address.session), PaneId::MAIN)
        .unwrap();
    let first_binding = residents.begin_follow(first).unwrap();
    let second_binding = residents.begin_follow(second).unwrap();
    let first_ticket = residents
        .admit_operation(first_binding, OperationKind::Lifecycle)
        .unwrap();
    let second_ticket = residents
        .admit_operation(second_binding, OperationKind::Lifecycle)
        .unwrap();
    assert_eq!(
        residents.settle_operation(second_binding, OperationKind::Lifecycle, first_ticket),
        Err(ResidentProblem::UnadmittedOperation)
    );
    residents
        .settle_operation(second_binding, OperationKind::Lifecycle, second_ticket)
        .unwrap();
    residents
        .settle_operation(first_binding, OperationKind::Lifecycle, first_ticket)
        .unwrap();
}

#[test]
fn provider_child_tickets_and_holds_are_not_owned_by_the_main_timeline() {
    let (mut residents, token, claim) = fixture();
    ready(&mut residents, token);
    let binding = residents.begin_follow(token).unwrap();
    let ticket = residents
        .admit_operation(binding, OperationKind::Subagents)
        .unwrap();
    residents
        .readiness(
            token,
            ResourceOwner::Subagents,
            OwnerReadiness::Hold(HoldReason::PendingEffect),
        )
        .unwrap();
    assert_eq!(
        residents.settle_operation(binding, OperationKind::Timeline, ticket),
        Err(ResidentProblem::UnadmittedOperation)
    );
    residents
        .settle_operation(binding, OperationKind::Subagents, ticket)
        .unwrap();
    residents
        .readiness(token, ResourceOwner::Timeline, OwnerReadiness::Ready)
        .unwrap();
    residents
        .release_claim(token, &claim, Duration::ZERO)
        .unwrap();
    assert_eq!(residents.retire_clean(token), Err(ResidentProblem::Held));
    residents
        .readiness(token, ResourceOwner::Subagents, OwnerReadiness::Ready)
        .unwrap();
    residents.retire_clean(token).unwrap();
}
