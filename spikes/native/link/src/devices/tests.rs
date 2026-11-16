//! Typed error projection and bounded dispatch.

use super::*;
#[test]
fn a_peer_cannot_echo_a_credential_through_an_error_or_debug() {
    let raw = "a-credential-that-a-faulty-peer-echoed";
    let error = vessel_client::CallError::Remote(vessel_proto::WireError::new(
        vessel_proto::ErrorCode::Forbidden,
        raw,
    ));
    let safe = problem(error);
    assert!(!format!("{safe:?}").contains(raw));
    assert!(!format!("{safe}").contains(raw));
    assert_eq!(safe, Problem::Refused(vessel_proto::ErrorCode::Forbidden));
    assert_eq!(
        problem(vessel_client::CallError::Disconnected),
        Problem::Offline
    );
    assert_eq!(
        problem(vessel_client::CallError::RequestIdsExhausted),
        Problem::Exhausted
    );
}
#[test]
fn a_full_or_closed_command_queue_is_an_answerable_failure() {
    let (commands, inbox) = tokio::sync::mpsc::channel(1);
    let mut tickets = vessel_app_view::devices::Tickets::default();
    let mut epochs = vessel_app_view::devices::ConnectionEpochs::default();
    let context = epochs
        .admit(
            vessel_app_view::devices::MachineKey::from_book("laptop"),
            vessel_proto::NodeId::random(),
        )
        .unwrap();
    enqueue(
        &commands,
        context.clone(),
        tickets.next_ticket().unwrap(),
        Ask::List,
    )
    .unwrap();
    assert_eq!(
        enqueue(
            &commands,
            context.clone(),
            tickets.next_ticket().unwrap(),
            Ask::List
        ),
        Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable))
    );
    drop(inbox);
    assert_eq!(
        enqueue(
            &commands,
            context,
            tickets.next_ticket().unwrap(),
            Ask::List
        ),
        Err(Problem::Offline)
    );
}

#[test]
fn device_calls_have_a_hard_concurrency_bound() {
    let hub = Hub::default();
    let permits: Vec<_> = (0..MAX_CALLS)
        .map(|_| std::sync::Arc::clone(&hub.0).try_acquire_owned().unwrap())
        .collect();
    assert!(std::sync::Arc::clone(&hub.0).try_acquire_owned().is_err());
    drop(permits);
    assert!(std::sync::Arc::clone(&hub.0).try_acquire_owned().is_ok());
}

mod admission;

#[test]
fn an_unassigned_call_is_unsupported() {
    assert_eq!(
        problem(vessel_client::CallError::UnassignedProtocol {
            method: vessel_proto::Method::NodeInfo
        }),
        Problem::Refused(vessel_proto::ErrorCode::Unsupported)
    );
}

#[test]
fn invitation_route_refusal_uses_only_the_matching_code_and_typed_detail() {
    use vessel_proto::{DeviceInviteError, ErrorCode, ErrorDetail, WireError};
    for (code, detail, expected) in [
        (
            ErrorCode::Conflict,
            Some(ErrorDetail::DeviceInvite {
                reason: DeviceInviteError::NoRoutes,
            }),
            Problem::NoRoute,
        ),
        (
            ErrorCode::Forbidden,
            Some(ErrorDetail::DeviceInvite {
                reason: DeviceInviteError::NoRoutes,
            }),
            Problem::Refused(ErrorCode::Forbidden),
        ),
        (
            ErrorCode::Conflict,
            Some(ErrorDetail::DeviceInvite {
                reason: DeviceInviteError::Unknown,
            }),
            Problem::Refused(ErrorCode::Conflict),
        ),
        (
            ErrorCode::Conflict,
            Some(ErrorDetail::Unknown),
            Problem::Refused(ErrorCode::Conflict),
        ),
        (
            ErrorCode::Conflict,
            None,
            Problem::Refused(ErrorCode::Conflict),
        ),
    ] {
        let safe = problem(vessel_client::CallError::Remote(WireError {
            code,
            detail,
            message: "no_routes; synthetic credential a peer must never echo".into(),
        }));
        assert_eq!(safe, expected);
        assert!(!format!("{safe}").contains("synthetic credential"));
        assert!(!format!("{safe:?}").contains("synthetic credential"));
    }
}

#[test]
fn a_local_deadline_is_uncertain_for_writes_and_retryable_only_for_reads() {
    let ticket = vessel_app_view::devices::Tickets::default()
        .next_ticket()
        .unwrap();
    for kind in [Kind::Invite, Kind::Revoke] {
        assert!(matches!(kind.timed_out(ticket), Outcome::Unconfirmed(actual) if actual == ticket));
    }
    assert!(
        matches!(Kind::Authority.timed_out(ticket), Outcome::Authority(actual, Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable))) if actual == ticket)
    );
    assert!(
        matches!(Kind::List.timed_out(ticket), Outcome::Listed(actual, Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable))) if actual == ticket)
    );
}
