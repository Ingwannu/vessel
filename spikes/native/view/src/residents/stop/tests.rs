//! Model ordering evidence only: actual framed transport/owner/Window proof is separate.
use super::*;
use crate::residents::tests::fixture;

#[derive(Clone, Copy)]
enum Order {
    EndFirst,
    AckFirst,
}
fn acknowledged(
    residents: &mut Residents,
    binding: FollowBinding,
    ticket: OperationTicket,
) -> StopWindowReceipt {
    let StopSettlement::WindowHeld(receipt) = residents
        .settle_stop(binding, ticket, StopCompletion::Acknowledged)
        .unwrap()
    else {
        panic!("typed ACK must retain local Window work")
    };
    receipt
}
fn sealed(residents: &mut Residents, binding: FollowBinding) -> FollowSeal {
    let FollowEnd::Sealed(notice) = residents.end_follow(binding).unwrap() else {
        panic!("prearmed stop must seal, not fully retire")
    };
    notice
}
fn run_order(order: Order, action: StopAction) {
    let (mut residents, token, _) = fixture();
    let binding = residents.begin_follow(token).unwrap();
    let other = residents
        .admit_operation(binding, OperationKind::Composer)
        .unwrap();
    let stop = residents.admit_stop(binding, action).unwrap();
    let (receipt, notice) = match order {
        Order::EndFirst => {
            let notice = sealed(&mut residents, binding);
            (acknowledged(&mut residents, binding, stop), notice)
        }
        Order::AckFirst => {
            let receipt = acknowledged(&mut residents, binding, stop);
            (receipt, sealed(&mut residents, binding))
        }
    };
    assert_eq!(notice.binding(), binding);
    assert_eq!(notice.operations().len(), 1);
    assert_eq!(notice.operations().first().unwrap().ticket(), other);
    sealed_ingress_refuses(&mut residents, binding);
    assert_eq!(
        residents.settle_stop(binding, stop, StopCompletion::Acknowledged),
        Err(ResidentProblem::UnadmittedOperation)
    );
    assert_eq!(residents.validate_stop_window(binding, receipt), Ok(action));
    assert_eq!(residents.finish_stop_window(binding, receipt), Ok(action));
    assert_eq!(
        residents.finish_stop_window(binding, receipt),
        Err(ResidentProblem::UnadmittedOperation)
    );
    assert_eq!(
        vote(&residents, token, ResourceOwner::Composer),
        OwnerReadiness::Hold(HoldReason::UncertainEffect)
    );
    assert_eq!(
        vote(&residents, token, ResourceOwner::Lifecycle),
        OwnerReadiness::Hold(HoldReason::PendingEffect)
    );
}
fn sealed_ingress_refuses(residents: &mut Residents, binding: FollowBinding) {
    assert_eq!(
        residents.validate_binding(binding),
        Err(ResidentProblem::StaleBinding)
    );
    assert_eq!(
        residents.admit_operation(binding, OperationKind::Timeline),
        Err(ResidentProblem::StaleBinding)
    );
    assert_eq!(
        residents.end_follow(binding),
        Err(ResidentProblem::StaleBinding)
    );
}
fn vote(
    residents: &Residents,
    token: super::super::ResidentToken,
    owner: ResourceOwner,
) -> OwnerReadiness {
    residents.entry(token).unwrap().owners[&owner]
}
#[test]
fn both_end_ack_orderings_preserve_only_original_window_work_and_never_make_ready() {
    for action in [StopAction::Archive, StopAction::Exit, StopAction::Fresh] {
        for order in [Order::EndFirst, Order::AckFirst] {
            run_order(order, action);
        }
    }
}
#[test]
fn generic_lifecycle_ticket_or_end_cannot_create_an_expectation() {
    let (mut residents, token, _) = fixture();
    let binding = residents.begin_follow(token).unwrap();
    let generic = residents
        .admit_operation(binding, OperationKind::Lifecycle)
        .unwrap();
    assert_eq!(
        residents.settle_stop(binding, generic, StopCompletion::Acknowledged),
        Err(ResidentProblem::UnadmittedOperation)
    );
    let FollowEnd::Retired(notice) = residents.end_follow(binding).unwrap() else {
        panic!("ordinary end must fully retire")
    };
    assert_eq!(notice.operations().first().unwrap().ticket(), generic);
    assert_eq!(
        residents.validate_binding(binding),
        Err(ResidentProblem::StaleBinding)
    );
}
#[test]
fn armed_ticket_has_one_dedicated_settlement_and_second_expectation_mints_nothing() {
    let (mut residents, token, _) = fixture();
    let binding = residents.begin_follow(token).unwrap();
    let wrong = residents
        .admit_operation(binding, OperationKind::Timeline)
        .unwrap();
    let stop = residents.admit_stop(binding, StopAction::Exit).unwrap();
    let next = residents.entry(token).unwrap().next_operation;
    assert_eq!(
        residents.admit_stop(binding, StopAction::Fresh),
        Err(ResidentProblem::Held)
    );
    assert_eq!(residents.entry(token).unwrap().next_operation, next);
    assert_eq!(
        residents.settle_operation(binding, OperationKind::Lifecycle, stop),
        Err(ResidentProblem::UnadmittedOperation)
    );
    assert_eq!(
        residents.settle_stop(binding, wrong, StopCompletion::Acknowledged),
        Err(ResidentProblem::UnadmittedOperation)
    );
    let receipt = acknowledged(&mut residents, binding, stop);
    assert_eq!(
        residents.validate_stop_window(binding, receipt),
        Ok(StopAction::Exit)
    );
    assert_eq!(
        residents.settle_operation(binding, OperationKind::Timeline, wrong),
        Ok(())
    );
}
#[test]
fn full_retirement_invalidates_acknowledged_work_even_with_no_pending_operations() {
    for order in [Order::EndFirst, Order::AckFirst] {
        let (mut residents, token, _) = fixture();
        let binding = residents.begin_follow(token).unwrap();
        let stop = residents.admit_stop(binding, StopAction::Archive).unwrap();
        if matches!(order, Order::EndFirst) {
            let _seal = sealed(&mut residents, binding);
        }
        let receipt = acknowledged(&mut residents, binding, stop);
        let notice = residents.retire_binding(binding).unwrap();
        assert_eq!(notice.binding(), binding);
        assert!(notice.operations().is_empty());
        assert_eq!(
            residents.validate_stop_window(binding, receipt),
            Err(ResidentProblem::StaleBinding)
        );
        assert_eq!(
            residents.finish_stop_window(binding, receipt),
            Err(ResidentProblem::StaleBinding)
        );
        assert_eq!(residents.retire_current(token), Ok(None));
    }
}
#[test]
fn pending_wire_retirement_wins_before_ack_and_retained_owner_hold_remains() {
    let (mut residents, token, _) = fixture();
    let binding = residents.begin_follow(token).unwrap();
    let stop = residents.admit_stop(binding, StopAction::Fresh).unwrap();
    let _seal = sealed(&mut residents, binding);
    let notice = residents.retire_binding(binding).unwrap();
    assert_eq!(notice.operations().first().unwrap().ticket(), stop);
    assert_eq!(
        residents.settle_stop(binding, stop, StopCompletion::Acknowledged),
        Err(ResidentProblem::StaleBinding)
    );
    assert_eq!(
        residents
            .entry(token)
            .unwrap()
            .owners
            .get(&ResourceOwner::Lifecycle),
        Some(&OwnerReadiness::Hold(HoldReason::UncertainEffect))
    );
}
#[test]
fn old_ack_receipt_end_or_retirement_cannot_change_same_node_successor() {
    let (mut residents, token, _) = fixture();
    let first = residents.begin_follow(token).unwrap();
    let stop = residents.admit_stop(first, StopAction::Exit).unwrap();
    let receipt = acknowledged(&mut residents, first, stop);
    let _notice = residents.retire_binding(first).unwrap();
    let second = residents.begin_follow(token).unwrap();
    assert_eq!(
        residents.end_follow(first),
        Err(ResidentProblem::StaleBinding)
    );
    assert_eq!(
        residents.retire_binding(first),
        Err(ResidentProblem::StaleBinding)
    );
    assert_eq!(
        residents.validate_stop_window(second, receipt),
        Err(ResidentProblem::UnadmittedOperation)
    );
    assert_eq!(
        residents.settle_stop(second, stop, StopCompletion::Acknowledged),
        Err(ResidentProblem::UnadmittedOperation)
    );
    assert_eq!(residents.validate_binding(second), Ok(()));
}
#[test]
fn deadline_or_refusal_consumes_authority_without_reopening_sealed_ingress() {
    let (mut residents, token, _) = fixture();
    let binding = residents.begin_follow(token).unwrap();
    let stop = residents.admit_stop(binding, StopAction::Archive).unwrap();
    let _seal = sealed(&mut residents, binding);
    let receipt = acknowledged(&mut residents, binding, stop);
    assert!(
        residents
            .revoke_stop_window(binding, receipt)
            .unwrap()
            .is_some()
    );
    assert_eq!(
        residents.validate_stop_window(binding, receipt),
        Err(ResidentProblem::StaleBinding)
    );
    let successor = residents.begin_follow(token).unwrap();
    let stop = residents.admit_stop(successor, StopAction::Exit).unwrap();
    let _seal = sealed(&mut residents, successor);
    let StopSettlement::Refused(notice) = residents
        .settle_stop(successor, stop, StopCompletion::Refused)
        .unwrap()
    else {
        panic!("refusal cannot become Window work")
    };
    assert!(notice.is_some());
    assert_eq!(
        residents.validate_binding(successor),
        Err(ResidentProblem::StaleBinding)
    );
}
#[test]
fn acknowledgements_cannot_dehydrate_resources_before_local_disposition() {
    let (mut residents, token, _) = fixture();
    let binding = residents.begin_follow(token).unwrap();
    let stop = residents.admit_stop(binding, StopAction::Archive).unwrap();
    let receipt = acknowledged(&mut residents, binding, stop);
    for owner in ResourceOwner::ALL {
        residents
            .readiness(token, owner, OwnerReadiness::Ready)
            .unwrap();
    }
    assert_eq!(residents.retire_clean(token), Err(ResidentProblem::Held));
    assert_eq!(
        residents.finish_stop_window(binding, receipt),
        Ok(StopAction::Archive)
    );
    assert_eq!(residents.retire_clean(token), Ok(()));
}

#[test]
fn stop_admission_cannot_weaken_existing_concrete_owner_holds() {
    for reason in [
        HoldReason::DirtyInput,
        HoldReason::Ime,
        HoldReason::Overlay,
        HoldReason::UncertainEffect,
    ] {
        let (mut residents, token, _) = fixture();
        let binding = residents.begin_follow(token).unwrap();
        residents
            .readiness(
                token,
                ResourceOwner::Lifecycle,
                OwnerReadiness::Hold(reason),
            )
            .unwrap();
        let _stop = residents.admit_stop(binding, StopAction::Archive).unwrap();
        assert_eq!(
            vote(&residents, token, ResourceOwner::Lifecycle),
            OwnerReadiness::Hold(reason)
        );
    }
}
