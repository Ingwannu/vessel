//! The actor tests exercise real framing and the source connection's guard.
use super::*;
use crate::{Command, testing::actor::Actor};
use vessel_app_view::schedules::load::Tickets;

fn run(actor: &Actor) -> Request {
    let id = "00000000-0000-4000-8000-000000000001".parse().unwrap();
    Request::on(
        actor.context(),
        Tickets::default().issue().unwrap(),
        Ask::Run(id),
    )
}

#[tokio::test]
async fn unsent_write_refuses_its_stale_admission() {
    let mut actor = Actor::new("laptop").await;
    let request = run(&actor);
    actor.retire().await;
    assert!(matches!(actor.next_event().await, Event::DeviceRetired(_)));
    actor.dispatch(Command::Schedules(request.clone())).await;
    let Event::Schedules(answer) = actor.next_event().await else {
        panic!("expected schedule refusal");
    };
    let (echo, result) = answer.settle();
    assert_eq!(echo, request);
    assert!(matches!(result, Err(Failure::ConnectionChanged)));
}

#[tokio::test]
async fn queued_run_receipt_loses_authority_if_its_connection_retires() {
    let mut actor = Actor::new("laptop").await;
    let request = run(&actor);
    actor.dispatch(Command::Schedules(request.clone())).await;
    let sent = actor.request().await;
    assert!(matches!(sent.call, vessel_proto::Call::RunSchedule(_)));
    let _sent = sent
        .answer
        .send(vessel_proto::Outcome::Err(vessel_proto::WireError::new(
            vessel_proto::ErrorCode::Conflict,
            "synthetic schedule refusal",
        )));
    let Event::Schedules(answer) = actor.next_event().await else {
        panic!("expected schedule reply");
    };
    actor.retire().await;
    let (echo, result) = answer.settle();
    assert_eq!(echo, request);
    assert!(matches!(result, Err(Failure::Unconfirmed)));
}
