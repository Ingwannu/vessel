//! The existing agent counter owns admission and exact-entry rollback.
use super::*;
use crate::principal::Principal;
use tokio_util::sync::CancellationToken;
use vessel_proto::creation::{
    Advance, CreationError, Fingerprint, FirstInput, Kind, Receipt, Reservation, Resource,
    Revision, Stage, WorkspaceClaim,
};
use vessel_proto::{AgentAuth, BlobHash, NodeId, WorkspaceId, WorkspaceOperationId};

fn marker(session: SessionId) -> Receipt {
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
