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

use peer::admitted;
#[tokio::test]
async fn unassigned_relationship_protocol_refuses_before_any_rpc_is_sent() {
    let peer = peer::connect(vec![]).await;
    let (_home, _state, selected, context, _events) = admitted(&peer);
    let target = Target::new(context, SessionId::random());
    let mut track = Track::new(target);
    let read = track.refresh(&mut Tickets::default()).unwrap();
    assert!(matches!(
        answer(&selected, Ask::Providers(read)).await,
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
    let (_home, _state, selected, context, _events) = admitted(&peer);
    let mut target = Target::new(context, SessionId::random());
    target.address.node = vessel_proto::NodeId::random();
    let ticket = Tickets::default().next_ticket().unwrap();
    assert!(matches!(
        answer(
            &selected,
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
        answer(&selected, Ask::Detach { choice, ticket }).await,
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
    let context = vessel_app_view::connection::ConnectionEpochs::default()
        .admit(
            MachineKey::from_book("fixture"),
            vessel_proto::NodeId::random(),
        )
        .unwrap();
    let child = Target::new(context, SessionId::random());
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
        mutation_failure(crate::connections::RequestFailure::Call(
            vessel_client::CallError::Disconnected
        )),
        TrackProblem::Uncertain
    );
    assert_eq!(
        read_failure(crate::connections::RequestFailure::Call(
            vessel_client::CallError::Disconnected
        )),
        TrackProblem::Offline
    );
}

#[tokio::test]
async fn same_node_reconnect_refuses_an_old_admission_before_any_subagent_rpc() {
    let peer = peer::connect(vec![]).await;
    let (_home, mut state, _selected, old, mut events) = admitted(&peer);
    let previous = state.clients["fixture"].clone();
    let next = state
        .admissions
        .admit(MachineKey::from_book("fixture"), old.node())
        .unwrap();
    assert_eq!(old.node(), next.node());
    assert_ne!(old.epoch(), next.epoch());
    let _previous = state
        .clients
        .insert("fixture".into(), previous.renamed(next));
    let target = Target::new(old, SessionId::random());
    let ticket = Tickets::default().next_ticket().unwrap();
    state.subagents(Ask::Lookup { target, ticket }).await;
    let reply = crate::testing::within(async { events.recv().await.unwrap() }).await;
    let crate::Event::Subagents(answer) = reply else {
        panic!("subagent reply")
    };
    assert!(matches!(
        answer.into_reply(),
        Reply::LookedUp {
            result: Err(TrackProblem::IdentityChanged),
            ..
        }
    ));
    assert!(peer.calls.lock().unwrap().is_empty());
}
