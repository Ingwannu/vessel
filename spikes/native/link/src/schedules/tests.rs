//! Delivery classification is contextual: a lost write is never a safe retry.
use super::*;
use vessel_app_view::schedules::load::{MachineKey, Tickets};
use vessel_proto::{ErrorCode, ScheduleId, WireError};
const ANSWER_WAIT: std::time::Duration = std::time::Duration::from_secs(5);
#[test]
fn call_sources_remain_available_and_lost_writes_are_uncertain() {
    let write = Ask::Run(
        "00000000-0000-4000-8000-000000000001"
            .parse::<ScheduleId>()
            .unwrap(),
    );
    let lost = ScheduleError::Call(vessel_client::CallError::Disconnected);
    assert_eq!(lost.failure(Ask::List), Failure::Offline);
    assert_eq!(lost.failure(write), Failure::Unconfirmed);
    assert!(std::error::Error::source(&lost).is_some());
    let refused = ScheduleError::Call(vessel_client::CallError::Remote(WireError::new(
        ErrorCode::Forbidden,
        "diagnostic-only",
    )));
    assert_eq!(
        refused.failure(write),
        Failure::Node(WireError::new(ErrorCode::Forbidden, "diagnostic-only"))
    );
    let mismatch = ScheduleError::Call(vessel_client::CallError::UnexpectedReply(
        vessel_proto::UnexpectedReply {
            expected: vessel_proto::Method::ListSchedules,
            got: vessel_proto::Method::RunSchedule,
        },
    ));
    assert_eq!(mismatch.failure(Ask::List), Failure::Unexpected);
    assert_eq!(mismatch.failure(write), Failure::Unconfirmed);
    assert_eq!(
        ScheduleError::Call(vessel_client::CallError::RequestIdsExhausted).failure(write),
        Failure::Exhausted
    );
    assert_eq!(ScheduleError::Offline.failure(write), Failure::Offline);
    assert_eq!(ScheduleError::Busy.failure(write), Failure::Busy);
    assert_eq!(
        ScheduleError::NoTransport.failure(Ask::List),
        Failure::NoTransport
    );
}
#[tokio::test]
async fn a_missing_client_still_answers_the_exact_ticket_without_a_network_call() {
    let hub = Hub::default();
    let (events, mut received) = tokio::sync::mpsc::channel(1);
    let expected = Request {
        machine: MachineKey::of("laptop").unwrap(),
        ticket: Tickets::default().issue().unwrap(),
        ask: Ask::List,
    };
    hub.ask(None, expected.clone(), events).await;
    let event = tokio::time::timeout(ANSWER_WAIT, received.recv())
        .await
        .unwrap()
        .unwrap();
    let crate::Event::Schedules(answer) = event else {
        panic!("wrong event")
    };
    assert_eq!(answer.request, expected);
    assert!(matches!(answer.result, Err(ScheduleError::Offline)));
}
