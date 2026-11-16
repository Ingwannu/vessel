//! Framed SDK calls reuse the selected lease; no second resolver or guard.
use super::*;
use crate::connections::Connected;
use crate::testing::{Home, peer, phase, state, within};
use vessel_app_view::connection::MachineKey;
use vessel_client::DiagnosticError;
use vessel_proto::diagnostics::{DiagnosticReport, DiagnosticReportFields};
use vessel_proto::{Call, ErrorCode, NodeId, Outcome, Reply, WireError};

const NAME: &str = "sdk-diagnostics";
const COMMAND_CAPACITY: usize = 1;
const WALL_SAFETY: std::time::Duration = std::time::Duration::from_secs(20);

struct Fixture {
    state: State,
    selected: SelectedConnection,
    peer: peer::Peer,
    _home: Home,
}
async fn fixture() -> Fixture {
    let home = Home::new();
    let (mut state, _events) = state(&home);
    let mut peer = peer::controlled(NAME).await;
    let client = peer.client.take().unwrap();
    let context = state
        .admissions
        .admit(MachineKey::from_book(NAME), client.node().id)
        .unwrap();
    let binding = vessel_client::book::BookEntry {
        address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
        token: None,
        credential: None,
        id: None,
    };
    let _previous = state.clients.insert(
        NAME.to_owned(),
        Connected::new(client, Vec::new(), binding, context.clone()),
    );
    let selected = state.selected_connection(&context).unwrap();
    Fixture {
        state,
        selected,
        peer,
        _home: home,
    }
}
fn report(id: NodeId) -> DiagnosticReport {
    let mut fields: DiagnosticReportFields = serde_json::from_str(r#"{
        "collected_at":"1970-01-01T00:00:00Z",
        "node":{"id":"00000000-0000-4000-8000-000000000001","version":null,"os":"linux","arch":"x86_64","state":"serving","uptime":0,"home":"redacted"},
        "process":{"state":"unavailable","reason":"unsupported"},
        "store":{"state":"unavailable","reason":"stale"},
        "agents":{"state":"unavailable","reason":"not_checked"},
        "sessions":{"state":"unavailable","reason":"timed_out"},
        "listeners":{"state":"unavailable","reason":"not_checked"},
        "relays":{"state":"unavailable","reason":"not_checked"},
        "recent_failures":{"state":"unavailable","reason":"not_checked"}
    }"#).unwrap();
    fields.node.id = id;
    DiagnosticReport::try_from(fields).unwrap()
}
async fn request(peer: &mut peer::Peer) -> peer::Request {
    let request = phase(
        "diagnostic request frame",
        peer.requests.as_mut().unwrap().recv(),
    )
    .await
    .unwrap();
    assert!(matches!(request.call, Call::Diagnostics(_)));
    request
}

#[tokio::test]
async fn sdk_diagnostics_preserves_typed_report_identity_and_remote_refusal() {
    within(async {
        let mut fixture = fixture().await;
        let expected = fixture.selected.lease.context.node();
        for reported in [expected, NodeId::random()] {
            let (result, ()) = tokio::join!(fixture.selected.diagnostics(), async {
                request(&mut fixture.peer).await.answer.send(Outcome::Ok(Reply::Diagnostics(report(reported)))).unwrap();
            });
            match result.unwrap() {
                Ok(report) => assert_eq!(report.fields().node.id, expected),
                Err(DiagnosticError::Identity { expected: actual, reported: actual_report }) => {
                    assert_eq!(actual, expected);
                    assert_ne!(reported, expected);
                    assert_eq!(actual_report, reported);
                }
                Err(other) => panic!("unexpected SDK failure: {other:?}"),
            }
        }
        let (result, ()) = tokio::join!(fixture.selected.diagnostics(), async {
            request(&mut fixture.peer).await.answer.send(Outcome::Err(WireError::new(ErrorCode::Forbidden, "private-peer-message"))).unwrap();
        });
        assert!(matches!(result, Err(RequestFailure::Call(CallError::Remote(error))) if error.code == ErrorCode::Forbidden));
        assert!(fixture.state.clients[NAME].matches(&fixture.selected.lease.context));
    }).await;
}

#[tokio::test]
async fn sdk_diagnostics_retirement_fences_before_poll_and_after_reply() {
    within(async {
        let mut fixture = fixture().await;
        let calling = fixture.selected.diagnostics();
        tokio::pin!(calling);
        let pending = phase("poll admitted diagnostic", async {
            tokio::select! {
                result = &mut calling => panic!("SDK finished before response: {}", result.is_ok()),
                request = request(&mut fixture.peer) => request,
            }
        })
        .await;
        fixture.state.clients[NAME].retire();
        let _sent = pending.answer.send(Outcome::Ok(Reply::Diagnostics(report(
            fixture.selected.lease.context.node(),
        ))));
        assert!(matches!(
            phase("reject retired diagnostic result", calling).await,
            Err(RequestFailure::Refused(Refusal::Changed))
        ));
        let before = fixture.peer.calls.lock().unwrap().len();
        assert!(matches!(
            fixture.selected.diagnostics().await,
            Err(RequestFailure::Refused(Refusal::Changed))
        ));
        assert_eq!(fixture.peer.calls.lock().unwrap().len(), before);
    })
    .await;
}

#[tokio::test]
async fn sdk_diagnostics_closed_transport_uses_original_actor_retirement() {
    within(async {
        let mut fixture = fixture().await;
        let (commands, mut receive) = tokio::sync::mpsc::channel(COMMAND_CAPACITY);
        fixture.state.commands = commands.downgrade();
        let selected = fixture.state.selected_connection(&fixture.selected.lease.context).unwrap();
        fixture.peer.task.as_ref().unwrap().abort();
        assert!(phase("close framed peer", fixture.peer.task.as_mut().unwrap()).await.is_err());
        let _joined = fixture.peer.task.take();
        phase("observe physical close", selected.lease.client.closed()).await;
        assert!(matches!(phase("closed SDK request", selected.diagnostics()).await, Err(RequestFailure::Refused(Refusal::Closed))));
        assert!(matches!(phase("original actor retirement command", receive.recv()).await, Some(Command::RetireConnection(tag)) if tag == selected.lease.context.tag()));
        assert!(receive.try_recv().is_err());
        assert!(fixture.peer.calls.lock().unwrap().is_empty());
    }).await;
}

#[tokio::test(start_paused = true)]
async fn sdk_diagnostics_keeps_sdk_deadline_typed_without_retiring_healthy_transport() {
    let (finished, watching) = std::sync::mpsc::channel();
    let (abort, aborted) = tokio::sync::oneshot::channel::<()>();
    let safety = std::thread::spawn(move || {
        if watching.recv_timeout(WALL_SAFETY).is_err() {
            let _gone = abort.send(());
        }
    });
    tokio::select! {
        () = deadline_case() => {},
        _expired = aborted => panic!("finite SDK test wall-clock safety expired"),
    }
    let _stopped = finished.send(());
    safety.join().unwrap();
}

async fn deadline_case() {
    let mut fixture = fixture().await;
    let calling = fixture.selected.diagnostics();
    tokio::pin!(calling);
    let pending = tokio::select! {
        result = &mut calling => panic!("SDK completed before held response: {}", result.is_ok()),
        request = request(&mut fixture.peer) => request,
    };
    tokio::time::advance(vessel_client::DIAGNOSTIC_REQUEST_TIMEOUT).await;
    assert!(matches!(
        calling.await,
        Ok(Err(DiagnosticError::Timeout(_)))
    ));
    assert!(fixture.state.clients[NAME].matches(&fixture.selected.lease.context));
    // Keep the peer's response owner alive through the SDK timeout; dropping it
    // first would close the transport and test the wrong boundary.
    drop(pending);
}
