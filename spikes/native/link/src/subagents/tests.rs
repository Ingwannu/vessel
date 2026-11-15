use super::*;
use crate::history::tests::peer;
use vessel_app_view::{
    devices::{MachineKey, Tickets},
    subagents::{DetachChoice, Target, Track},
};
use vessel_proto::subagents::{DetachDisposition, DetachReceipt, DetachedSession};
use vessel_proto::{
    Creator, HarnessKind, SessionId, SessionInfo, SessionMode, SessionRef, SessionSpec,
};

#[tokio::test]
async fn unassigned_relationship_protocol_refuses_before_any_rpc_is_sent() {
    let peer = peer::connect(vec![]).await;
    let target = Target {
        machine: MachineKey::from_book("fixture"),
        address: SessionRef {
            node: peer.client.node().id,
            session: SessionId::random(),
        },
    };
    let mut track = Track::new(target);
    let read = track.refresh(&mut Tickets::default()).unwrap();
    assert!(matches!(
        answer(&peer.client, Ask::Providers(read)).await,
        Reply::Providers {
            result: Err(TrackProblem::Unsupported),
            ..
        }
    ));
    assert!(peer.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn a_reused_book_alias_is_refused_before_read_or_mutation() {
    let peer = peer::connect(vec![]).await;
    let target = Target {
        machine: MachineKey::from_book("fixture"),
        address: SessionRef {
            node: vessel_proto::NodeId::random(),
            session: SessionId::random(),
        },
    };
    let ticket = Tickets::default().next_ticket().unwrap();
    assert!(matches!(
        answer(
            &peer.client,
            Ask::Lookup {
                target: target.clone(),
                ticket
            }
        )
        .await,
        Reply::LookedUp {
            result: Err(TrackProblem::IdentityChanged),
            ..
        }
    ));
    let choice = DetachChoice {
        parent: target.address,
        child: target,
    };
    assert!(matches!(
        answer(&peer.client, Ask::Detach { choice, ticket }).await,
        Reply::Detached {
            result: Err(TrackProblem::IdentityChanged),
            ..
        }
    ));
    assert!(peer.calls.lock().unwrap().is_empty());
}
fn receipt() -> (DetachChoice, DetachedSession) {
    let parent = SessionRef {
        node: vessel_proto::NodeId::random(),
        session: SessionId::random(),
    };
    let child = Target {
        machine: MachineKey::from_book("fixture"),
        address: SessionRef {
            node: vessel_proto::NodeId::random(),
            session: SessionId::random(),
        },
    };
    let mut info = SessionInfo::new(
        child.address.session,
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![],
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    )
    .with_creator(Creator::Agent {
        node: parent.node,
        session: parent.session,
    });
    let receipt = DetachReceipt::new(parent, chrono::DateTime::UNIX_EPOCH);
    info.detach_receipt = Some(receipt);
    (
        DetachChoice { parent, child },
        DetachedSession {
            session: Box::new(info),
            receipt,
            disposition: DetachDisposition::AlreadyDetached,
        },
    )
}
#[test]
fn durable_detach_receipts_are_bound_to_the_confirmed_complete_identities() {
    let (choice, result) = receipt();
    assert!(validate_detach(&choice, result.clone()).is_ok());
    let mut wrong = result.clone();
    wrong.session.id = SessionId::random();
    assert_eq!(
        validate_detach(&choice, wrong),
        Err(TrackProblem::Uncertain)
    );
    let mut wrong = result.clone();
    wrong.session.detach_receipt = None;
    assert_eq!(
        validate_detach(&choice, wrong),
        Err(TrackProblem::Uncertain)
    );
    let mut wrong_choice = choice.clone();
    wrong_choice.parent.node = vessel_proto::NodeId::random();
    assert_eq!(
        validate_detach(&wrong_choice, result),
        Err(TrackProblem::Uncertain)
    );
}
#[test]
fn a_lost_mutation_response_is_uncertain_and_never_automatic_retry() {
    assert_eq!(
        mutation_failure(vessel_client::CallError::Disconnected),
        TrackProblem::Uncertain
    );
    assert_eq!(
        read_failure(vessel_client::CallError::Disconnected),
        TrackProblem::Offline
    );
}
