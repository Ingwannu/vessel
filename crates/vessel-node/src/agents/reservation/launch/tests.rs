//! The fixed constructor/check preserve the exact existing count owner.
use super::*;
use crate::agents::CreationCountCorrelation;
use crate::principal::Principal;
use vessel_proto::NodeId;

#[test]
fn correlation_requires_begin_and_is_allocated_once_without_minting_authority() {
    let agents = Agents::new(NodeId::random());
    let child = SessionId::random();
    let mut count = agents
        .reserve_creation(child, &Principal::Owner, false)
        .unwrap();
    assert!(matches!(
        count.creation_correlation(),
        Err(CreationError::InvalidTransition)
    ));
    let receipt = super::super::tests::marker(child);
    count.begin(&receipt).unwrap();
    let correlation: CreationCountCorrelation<'_> = count.creation_correlation().unwrap();
    assert_eq!(correlation.child(), child);
    assert_eq!(
        correlation.begin(),
        (
            receipt.snapshot().reserved.operation,
            receipt.snapshot().revision,
        )
    );
    assert!(matches!(
        count.creation_correlation(),
        Err(CreationError::InvalidTransition)
    ));
    let entries = agents.lock();
    assert!(correlation.matches_pending(&agents, entries.get(&child).unwrap()));
}

#[test]
fn a_different_registry_cannot_use_the_same_child_or_original_entry() {
    let agents = Agents::new(NodeId::random());
    let other = Agents::new(NodeId::random());
    let child = SessionId::random();
    let mut count = agents
        .reserve_creation(child, &Principal::Owner, false)
        .unwrap();
    count.begin(&super::super::tests::marker(child)).unwrap();
    let correlation = count.creation_correlation().unwrap();
    let entries = agents.lock();
    assert!(!correlation.matches_pending(&other, entries.get(&child).unwrap()));
    drop(entries);
    let _other = other
        .reserve_creation(child, &Principal::Owner, false)
        .unwrap();
    let entries = other.lock();
    assert!(!correlation.matches_pending(&other, entries.get(&child).unwrap()));
}

#[test]
fn replaced_or_connected_entries_do_not_match_an_old_correlation() {
    let agents = Agents::new(NodeId::random());
    let child = SessionId::random();
    let mut count = agents
        .reserve_creation(child, &Principal::Owner, false)
        .unwrap();
    count.begin(&super::super::tests::marker(child)).unwrap();
    let correlation = count.creation_correlation().unwrap();
    {
        let mut entries = agents.lock();
        let entry = entries.get_mut(&child).unwrap();
        let _previous = entry
            .connections
            .insert(1, tokio_util::sync::CancellationToken::new());
        assert!(!correlation.matches_pending(&agents, entry));
    }
    agents.forget(child);
    let _replacement = agents
        .reserve_creation(child, &Principal::Owner, false)
        .unwrap();
    let entries = agents.lock();
    assert!(!correlation.matches_pending(&agents, entries.get(&child).unwrap()));
}

#[test]
fn a_consumed_launch_or_later_known_refusal_cannot_reuse_an_old_begin_relation() {
    use vessel_proto::creation::{Advance, Certainty};
    let agents = Agents::new(NodeId::random());
    let child = SessionId::random();
    let mut count = agents
        .reserve_creation(child, &Principal::Owner, false)
        .unwrap();
    let mut receipt = super::super::tests::marker(child);
    count.begin(&receipt).unwrap();
    let correlation = count.creation_correlation().unwrap();
    {
        let mut entries = agents.lock();
        let entry = entries.get_mut(&child).unwrap();
        assert!(correlation.admit_pending(&agents, entry));
        assert!(!correlation.admit_pending(&agents, entry));
    }
    receipt
        .advance(
            receipt.snapshot().revision,
            Advance::Fail {
                code: vessel_proto::ErrorCode::Forbidden,
                certainty: Certainty::Known,
            },
        )
        .unwrap();
    assert_eq!(
        count.refused(&receipt),
        Err(CreationError::InvalidTransition)
    );
    drop(count);
    assert!(
        agents.is_live(child),
        "admitted work needs tracked settlement, never rollback"
    );
}

#[test]
fn a_proven_pre_admission_refusal_invalidates_the_old_correlation_before_drop() {
    use vessel_proto::creation::{Advance, Certainty};
    let agents = Agents::new(NodeId::random());
    let child = SessionId::random();
    let mut count = agents
        .reserve_creation(child, &Principal::Owner, false)
        .unwrap();
    let mut receipt = super::super::tests::marker(child);
    count.begin(&receipt).unwrap();
    let correlation = count.creation_correlation().unwrap();
    receipt
        .advance(
            receipt.snapshot().revision,
            Advance::Fail {
                code: vessel_proto::ErrorCode::Forbidden,
                certainty: Certainty::Known,
            },
        )
        .unwrap();
    count.refused(&receipt).unwrap();
    let entries = agents.lock();
    assert!(!correlation.matches_pending(&agents, entries.get(&child).unwrap()));
    drop(entries);
    drop(count);
    assert!(!agents.is_live(child));
}
