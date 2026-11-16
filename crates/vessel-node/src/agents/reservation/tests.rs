//! The existing agent counter owns admission and exact-entry rollback.
use super::*;
use crate::principal::Principal;
use tokio_util::sync::CancellationToken;
use vessel_proto::creation::{
    Advance, CreationError, Fingerprint, FirstInput, Kind, Receipt, Reservation, Resource,
    Revision, Stage, WorkspaceClaim,
};
use vessel_proto::{AgentAuth, BlobHash, NodeId, WorkspaceId, WorkspaceOperationId};

pub(super) fn marker(session: SessionId) -> Receipt {
    let mut receipt = Receipt::admitted(
        Kind::Agent,
        Reservation {
            operation: WorkspaceOperationId::random(),
            workspace: WorkspaceId::random(),
            workspace_claim: WorkspaceClaim::Existing,
            resource: Some(Resource::Agent { session }),
            first_input: FirstInput::Absent,
        },
        Fingerprint::new(BlobHash::from_bytes([9; 32])),
    )
    .unwrap();
    receipt
        .advance(Revision::INITIAL, Advance::Begin(Stage::Resource))
        .unwrap();
    receipt
}

#[test]
fn a_pending_guard_releases_only_the_entry_it_actually_reserved() {
    let agents = Agents::new(NodeId::random());
    let id = SessionId::random();
    let lease = agents
        .reserve_creation(id, &Principal::Owner, false)
        .unwrap();
    assert!(agents.is_live(id));
    assert!(
        agents
            .reserve_creation(id, &Principal::Owner, false)
            .is_err()
    );
    drop(lease);
    assert!(!agents.is_live(id));
    let replacement = agents
        .reserve_creation(id, &Principal::Owner, false)
        .unwrap();
    drop(replacement);
    assert!(!agents.is_live(id));
}

#[test]
fn old_rollback_cannot_remove_a_later_entry_or_revoke_its_credential() {
    let agents = Arc::new(Agents::new(NodeId::random()));
    let id = SessionId::random();
    let old = agents
        .reserve_creation(id, &Principal::Owner, true)
        .unwrap();
    let old_secret = old.secret.as_ref().unwrap().clone();
    agents.forget(id);
    let new_secret = agents
        .reserve(id, &Principal::Owner, true)
        .unwrap()
        .unwrap();
    let closing = CancellationToken::new();
    let new_auth = AgentAuth {
        session: id,
        secret: new_secret,
    };
    let (_principal, registration) = agents.admit(&new_auth, &closing).unwrap();
    drop(old);
    assert!(agents.is_live(id));
    assert!(!registration.closing().is_cancelled());
    assert!(agents.admit(&new_auth, &closing).is_some());
    assert!(
        agents
            .admit(
                &AgentAuth {
                    session: id,
                    secret: old_secret
                },
                &closing
            )
            .is_none()
    );
    agents.retire(id);
    assert!(registration.closing().is_cancelled());
}

#[test]
fn matching_resource_marker_retains_count_on_cancellation_but_foreign_marker_does_not() {
    let agents = Agents::new(NodeId::random());
    let id = SessionId::random();
    let mut pending = agents
        .reserve_creation(id, &Principal::Owner, false)
        .unwrap();
    assert_eq!(
        pending.begin(&marker(SessionId::random())),
        Err(CreationError::InvalidTransition)
    );
    drop(pending);
    assert!(!agents.is_live(id));
    let mut started = agents
        .reserve_creation(id, &Principal::Owner, false)
        .unwrap();
    started.begin(&marker(id)).unwrap();
    drop(started);
    assert!(
        agents.is_live(id),
        "a lost spawn outcome must retain its count entry"
    );
    assert!(
        agents
            .reserve_creation(id, &Principal::Owner, false)
            .is_err()
    );
    agents.forget(id);
}

#[test]
fn a_stale_guard_cannot_cross_begin_after_its_entry_was_replaced() {
    let agents = Agents::new(NodeId::random());
    let id = SessionId::random();
    let mut old = agents
        .reserve_creation(id, &Principal::Owner, false)
        .unwrap();
    agents.forget(id);
    let _secret = agents.reserve(id, &Principal::Owner, false).unwrap();
    assert_eq!(
        old.begin(&marker(id)),
        Err(CreationError::InvalidTransition)
    );
    drop(old);
    assert!(agents.is_live(id));
    agents.forget(id);
}

#[test]
fn only_its_own_known_resource_refusal_can_release_a_begun_count_entry() {
    use vessel_proto::creation::Certainty;
    for certainty in [Certainty::Known, Certainty::Unknown] {
        let agents = Agents::new(NodeId::random());
        let id = SessionId::random();
        let mut lease = agents
            .reserve_creation(id, &Principal::Owner, false)
            .unwrap();
        let mut receipt = marker(id);
        lease.begin(&receipt).unwrap();
        let mut other_operation = marker(id);
        other_operation
            .advance(
                other_operation.snapshot().revision,
                Advance::Fail {
                    code: vessel_proto::ErrorCode::Forbidden,
                    certainty: Certainty::Known,
                },
            )
            .unwrap();
        assert_eq!(
            lease.refused(&other_operation),
            Err(CreationError::InvalidTransition)
        );
        assert_eq!(lease.begin(&receipt), Err(CreationError::InvalidTransition));
        receipt
            .advance(
                receipt.snapshot().revision,
                Advance::Fail {
                    code: vessel_proto::ErrorCode::Forbidden,
                    certainty,
                },
            )
            .unwrap();
        assert_eq!(
            lease.refused(&receipt).is_ok(),
            certainty == Certainty::Known
        );
        drop(lease);
        assert_eq!(agents.is_live(id), certainty == Certainty::Unknown);
        agents.forget(id);
    }
}

#[test]
fn an_already_used_credential_prevents_even_known_refusal_from_releasing_count() {
    let agents = Arc::new(Agents::new(NodeId::random()));
    let id = SessionId::random();
    let mut lease = agents
        .reserve_creation(id, &Principal::Owner, true)
        .unwrap();
    let mut receipt = marker(id);
    lease.begin(&receipt).unwrap();
    let secret = lease.secret.as_ref().unwrap().clone();
    let (_principal, connection) = agents
        .admit(
            &AgentAuth {
                session: id,
                secret,
            },
            &CancellationToken::new(),
        )
        .unwrap();
    receipt
        .advance(
            receipt.snapshot().revision,
            Advance::Fail {
                code: vessel_proto::ErrorCode::Forbidden,
                certainty: vessel_proto::creation::Certainty::Known,
            },
        )
        .unwrap();
    assert_eq!(
        lease.refused(&receipt),
        Err(CreationError::InvalidTransition)
    );
    drop(lease);
    assert!(agents.is_live(id));
    assert!(!connection.closing().is_cancelled());
    agents.retire(id);
    assert!(connection.closing().is_cancelled());
}

#[test]
fn an_earlier_retry_failure_cannot_release_a_newer_begin_for_the_same_operation() {
    use vessel_proto::creation::Certainty;
    let agents = Agents::new(NodeId::random());
    let id = SessionId::random();
    let mut old_failure = marker(id);
    old_failure
        .advance(
            old_failure.snapshot().revision,
            Advance::Fail {
                code: vessel_proto::ErrorCode::Conflict,
                certainty: Certainty::Known,
            },
        )
        .unwrap();
    let mut next = old_failure.clone();
    next.advance(next.snapshot().revision, Advance::Begin(Stage::Resource))
        .unwrap();
    let mut lease = agents
        .reserve_creation(id, &Principal::Owner, false)
        .unwrap();
    lease.begin(&next).unwrap();
    assert_eq!(
        lease.refused(&old_failure),
        Err(CreationError::InvalidTransition)
    );
    assert!(agents.is_live(id));
    next.advance(
        next.snapshot().revision,
        Advance::Fail {
            code: vessel_proto::ErrorCode::Conflict,
            certainty: Certainty::Known,
        },
    )
    .unwrap();
    lease.refused(&next).unwrap();
    drop(lease);
    assert!(!agents.is_live(id));
}

#[test]
fn credential_preparation_does_not_skip_the_authoritative_duplicate_recheck() {
    let agents = Agents::new(NodeId::random());
    let id = SessionId::random();
    let prepared = agents.prepare_reservation(id, true).unwrap();
    let winner = agents
        .reserve_creation(id, &Principal::Owner, true)
        .unwrap();
    let secret = winner.secret.as_ref().unwrap();
    let mut entries = agents.lock();
    let marker = Arc::clone(&entries.get(&id).unwrap().reservation);
    assert!(matches!(
        agents.reserve_prepared(&mut entries, &Principal::Owner, prepared),
        Err(crate::error::ServiceError::AgentReserved(found)) if found == id
    ));
    let entry = entries.get(&id).unwrap();
    assert!(Arc::ptr_eq(&entry.reservation, &marker));
    assert_eq!(entry.credentials.len(), 1);
    assert!(entry.credentials[0].matches_agent(secret));
}
