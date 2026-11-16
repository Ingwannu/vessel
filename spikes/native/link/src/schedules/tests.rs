//! Delivery classification is contextual: a lost write is never a safe retry.
use super::RowRequest as Request;
use super::*;
use vessel_app_view::schedules::{
    load::{Failure, MachineKey, Tickets},
    page::Ask,
};
use vessel_proto::{ErrorCode, ScheduleId, WireError};
const ANSWER_WAIT: std::time::Duration = std::time::Duration::from_secs(5);
fn settled(event: crate::Event) -> crate::Event {
    if let crate::Event::Schedules(Message::Delivered(delivered)) = event {
        crate::Event::Schedules(delivered.settle())
    } else {
        event
    }
}
#[test]
fn call_sources_remain_available_and_lost_writes_are_uncertain() {
    let write = Ask::Run(
        "00000000-0000-4000-8000-000000000001"
            .parse::<ScheduleId>()
            .unwrap(),
    );
    let lost = ScheduleError::from(vessel_client::CallError::Disconnected);
    assert_eq!(lost.failure(Ask::List), Failure::Offline);
    assert_eq!(lost.failure(write), Failure::Unconfirmed);
    assert!(std::error::Error::source(&lost).is_some());
    let refused = ScheduleError::from(vessel_client::CallError::Remote(WireError::new(
        ErrorCode::Forbidden,
        "diagnostic-only",
    )));
    assert_eq!(
        refused.failure(write),
        Failure::Node(WireError::new(ErrorCode::Forbidden, "diagnostic-only"))
    );
    let mismatch = ScheduleError::from(vessel_client::CallError::UnexpectedReply(
        vessel_proto::UnexpectedReply {
            expected: vessel_proto::Method::ListSchedules,
            got: vessel_proto::Method::RunSchedule,
        },
    ));
    assert_eq!(mismatch.failure(Ask::List), Failure::Unexpected);
    assert_eq!(mismatch.failure(write), Failure::Unconfirmed);
    assert_eq!(
        ScheduleError::from(vessel_client::CallError::RequestIdsExhausted).failure(write),
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
    let context = vessel_app_view::connection::ConnectionEpochs::default()
        .admit(
            vessel_app_view::connection::MachineKey::from_book(
                MachineKey::of("laptop").unwrap().as_str(),
            ),
            vessel_proto::NodeId::random(),
        )
        .unwrap();
    let expected = Request::on(context, Tickets::default().issue().unwrap(), Ask::List);
    hub.ask(Err(ScheduleError::Offline), expected.clone(), events)
        .await;
    let event = tokio::time::timeout(ANSWER_WAIT, received.recv())
        .await
        .unwrap()
        .unwrap();
    let crate::Event::Schedules(Message::Replied(answer)) = settled(event) else {
        panic!("wrong event")
    };
    assert_eq!(answer.request, expected);
    assert!(matches!(answer.result, Err(ScheduleError::Offline)));
}

#[test]
fn an_unassigned_feature_is_unsent_and_distinct_from_an_older_peer() {
    let error = ScheduleError::from(vessel_client::CallError::UnassignedProtocol {
        method: vessel_proto::Method::RunSchedule,
    });
    let id = "00000000-0000-4000-8000-000000000001".parse().unwrap();
    for ask in [Ask::List, Ask::Run(id)] {
        assert_eq!(error.failure(ask), Failure::NotEnabled);
    }
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn unsent_refusal_and_unconfirmed_retirement_have_distinct_write_semantics() {
    let id = "00000000-0000-4000-8000-000000000001".parse().unwrap();
    for why in [
        vessel_app_view::connection::Refusal::Changed,
        vessel_app_view::connection::Refusal::Closed,
    ] {
        let expected = match why {
            vessel_app_view::connection::Refusal::Changed => Failure::ConnectionChanged,
            vessel_app_view::connection::Refusal::Closed => Failure::Offline,
        };
        assert_eq!(
            ScheduleError::NotSelected(why).failure(Ask::Run(id)),
            expected
        );
        assert_eq!(
            ScheduleError::Retired(why).failure(Ask::Run(id)),
            Failure::Unconfirmed
        );
        assert_eq!(ScheduleError::Retired(why).failure(Ask::List), expected);
    }
    assert_eq!(
        ScheduleError::Deadline.failure(Ask::Run(id)),
        Failure::Unconfirmed
    );
    assert_eq!(
        ScheduleError::Deadline.failure(Ask::List),
        Failure::TimedOut
    );
}
#[test]
fn retained_peer_errors_do_not_disclose_their_message_in_diagnostic_chains() {
    let canary = "synthetic-schedule-credential";
    let error = ScheduleError::from(vessel_client::CallError::Remote(WireError::new(
        ErrorCode::Conflict,
        canary,
    )));
    assert!(!format!("{error:?}").contains(canary));
    let mut current: Option<&dyn std::error::Error> = Some(&error);
    while let Some(error) = current {
        assert!(!error.to_string().contains(canary));
        current = error.source();
    }
    assert_eq!(
        error.failure(Ask::List),
        Failure::Node(WireError::new(ErrorCode::Conflict, canary))
    );
}
mod admission;

mod form;

mod history;

mod preferences;

mod delivery;
