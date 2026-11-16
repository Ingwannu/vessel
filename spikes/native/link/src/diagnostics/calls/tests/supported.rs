//! Canonical protocol-24 replies through the real selected SDK boundary.
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use vessel_proto::diagnostics::{DiagnosticReport, DiagnosticReportFields};

#[derive(Default)]
struct ReadCounts {
    total: AtomicUsize,
    diagnostics: AtomicUsize,
}
fn report(id: NodeId) -> DiagnosticReport {
    // Typed fixture decode exercises the same validated root as an untrusted reply.
    let mut fields: DiagnosticReportFields = serde_json::from_str(r#"{
        "collected_at":"1970-01-01T00:00:00Z",
        "node":{"id":"00000000-0000-4000-8000-000000000001","version":null,"os":"linux","arch":"x86_64","state":"serving","uptime":0,"home":"redacted"},
        "process":{"state":"available","at":"1970-01-01T00:00:00Z","value":{"pid":42,"parent":null,"memory":{"state":"unavailable","reason":"unsupported"}}},
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
async fn reporting_peer(
    admitted: NodeId,
    version: vessel_proto::ProtocolVersion,
    basic: Outcome,
    diagnostic: Outcome,
) -> (NodeClient, Arc<ReadCounts>, tokio::task::JoinHandle<()>) {
    let (local, mut remote) = tokio::io::duplex(FRAME_BYTES);
    let diagnostics = Arc::new(ReadCounts::default());
    let counted = Arc::clone(&diagnostics);
    let task = tokio::spawn(async move {
        assert!(matches!(read(&mut remote).await, ClientFrame::Hello(_)));
        write(
            &mut remote,
            ServerFrame::Welcome(Welcome {
                protocol: version,
                node: node(admitted),
            }),
        )
        .await;
        loop {
            let ClientFrame::Request { id, call } = read(&mut remote).await else {
                panic!("expected explicit read");
            };
            let _previous = counted.total.fetch_add(1, Ordering::SeqCst);
            assert!(matches!(&call, Call::NodeInfo(_) | Call::Diagnostics(_)));
            let outcome = if matches!(call, Call::NodeInfo(_)) {
                basic.clone()
            } else {
                let _previous = counted.diagnostics.fetch_add(1, Ordering::SeqCst);
                diagnostic.clone()
            };
            write(&mut remote, ServerFrame::Response { id, outcome }).await;
        }
    });
    (framed_client(local).await, diagnostics, task)
}
#[tokio::test]
async fn supported_sdk_report_keeps_partial_process_and_independent_basic_failure() {
    crate::testing::within(async {
        let id = NodeId::random();
        let (client, calls, task) = reporting_peer(
            id,
            vessel_proto::PROTOCOL_VERSION,
            Outcome::Err(WireError::new(ErrorCode::Internal, CANARY)),
            Outcome::Ok(Reply::Diagnostics(report(id))),
        )
        .await;
        let result = collect_client(client).await;
        assert_eq!(result.facts, Err(Problem::Failed));
        assert_eq!(result.daemon.node(), Some(id));
        assert!(!result.daemon.complete());
        assert_eq!(calls.diagnostics.load(Ordering::SeqCst), 1);
        assert_eq!(calls.total.load(Ordering::SeqCst), 2);
        assert!(!format!("{result:?}").contains(CANARY));
        task.abort();
    })
    .await;
}
#[tokio::test]
async fn old_peer_retains_basic_facts_and_sends_no_diagnostics_frame() {
    crate::testing::within(async {
        let id = NodeId::random();
        let (client, calls, task) = reporting_peer(
            id,
            vessel_proto::ProtocolVersion::new(23),
            Outcome::Ok(Reply::NodeInfo(node(id))),
            Outcome::Ok(Reply::Diagnostics(report(id))),
        )
        .await;
        let result = collect_client(client).await;
        assert_eq!(
            result.facts.unwrap().protocol,
            vessel_proto::ProtocolVersion::new(23)
        );
        assert_eq!(
            result.daemon,
            DaemonSnapshot::Unavailable(Problem::Unsupported)
        );
        assert_eq!(calls.diagnostics.load(Ordering::SeqCst), 0);
        assert_eq!(calls.total.load(Ordering::SeqCst), 1);
        task.abort();
    })
    .await;
}
#[tokio::test]
async fn sdk_identity_and_typed_refusals_are_closed_without_remote_messages() {
    crate::testing::within(async {
        let id = NodeId::random();
        for (outcome, expected) in [
            (
                Outcome::Ok(Reply::Diagnostics(report(NodeId::random()))),
                Problem::InvalidReply,
            ),
            (
                Outcome::Err(WireError::new(ErrorCode::Forbidden, CANARY)),
                Problem::Forbidden,
            ),
            (
                Outcome::Err(WireError {
                    code: ErrorCode::Forbidden,
                    message: CANARY.into(),
                    detail: Some(vessel_proto::ErrorDetail::Diagnostics {
                        reason: vessel_proto::diagnostics::DiagnosticRefusal::Busy,
                    }),
                }),
                Problem::Forbidden,
            ),
            (
                Outcome::Err(WireError {
                    code: ErrorCode::Unavailable,
                    message: CANARY.into(),
                    detail: Some(vessel_proto::ErrorDetail::Diagnostics {
                        reason: vessel_proto::diagnostics::DiagnosticRefusal::Busy,
                    }),
                }),
                Problem::Busy,
            ),
            (
                Outcome::Err(WireError {
                    code: ErrorCode::Unavailable,
                    message: CANARY.into(),
                    detail: Some(vessel_proto::ErrorDetail::Diagnostics {
                        reason: vessel_proto::diagnostics::DiagnosticRefusal::Stopping,
                    }),
                }),
                Problem::Stopping,
            ),
        ] {
            let (client, _, task) = reporting_peer(
                id,
                vessel_proto::PROTOCOL_VERSION,
                Outcome::Ok(Reply::NodeInfo(node(id))),
                outcome,
            )
            .await;
            let result = collect_client(client).await;
            assert_eq!(result.daemon, DaemonSnapshot::Unavailable(expected));
            assert!(!format!("{result:?}").contains(CANARY));
            task.abort();
        }
    })
    .await;
}
#[tokio::test]
async fn retired_admission_never_polls_either_diagnostic_read() {
    crate::testing::within(async {
        let id = NodeId::random();
        let (client, calls, task) = reporting_peer(
            id,
            vessel_proto::PROTOCOL_VERSION,
            Outcome::Ok(Reply::NodeInfo(node(id))),
            Outcome::Ok(Reply::Diagnostics(report(id))),
        )
        .await;
        let (events, _receiving) = tokio::sync::mpsc::channel(4);
        let (_home, mut state) = fixture_state(events);
        let context = admit(&mut state, client, "fixture");
        let selected = state.selected_connection(&context).unwrap();
        state.retire_connections().await;
        let result = collect(&selected, &context).await;
        assert_eq!(result, Err(Problem::ConnectionChanged));
        assert_eq!(calls.diagnostics.load(Ordering::SeqCst), 0);
        assert_eq!(calls.total.load(Ordering::SeqCst), 0);
        task.abort();
    })
    .await;
}
