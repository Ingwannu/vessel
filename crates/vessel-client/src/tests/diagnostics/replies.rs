//! Typed responder fixtures isolate identity, correlation and the client clock.
//! They are not evidence of authentication or real node collection.
use super::{PHASE, SAFETY};
use crate::{CallError, DIAGNOSTIC_REQUEST_TIMEOUT, DiagnosticError, NodeClient};
use futures_util::{SinkExt as _, StreamExt as _};
use vessel_proto::diagnostics::{
    DIAGNOSTICS_PROTOCOL, DiagnosticArch, DiagnosticHome, DiagnosticNode, DiagnosticOs,
    DiagnosticReport, DiagnosticReportFields, DiagnosticSection, DiagnosticUnavailable,
};
use vessel_proto::{
    ClientFrame, DurationMs, Method, NodeId, NodeState, Outcome, ProtocolVersion, Reply, RequestId,
    ServerFrame, Welcome,
};
use vessel_rpc::{ClientConfig, memory::MemoryTransport};

const RESPONDER_CAPACITY: usize = 4;

fn unavailable<T>() -> DiagnosticSection<T> {
    DiagnosticSection::Unavailable {
        reason: DiagnosticUnavailable::NotChecked,
    }
}
fn report(id: NodeId) -> DiagnosticReport {
    DiagnosticReportFields {
        collected_at: chrono::DateTime::UNIX_EPOCH,
        node: DiagnosticNode {
            id,
            version: None,
            os: DiagnosticOs::Linux,
            arch: DiagnosticArch::X86_64,
            state: NodeState::Serving,
            uptime: DurationMs::from_duration(std::time::Duration::ZERO),
            home: DiagnosticHome::Redacted,
        },
        process: unavailable(),
        store: unavailable(),
        agents: unavailable(),
        sessions: unavailable(),
        listeners: unavailable(),
        relays: unavailable(),
        recent_failures: unavailable(),
    }
    .try_into()
    .unwrap()
}
async fn phase<F: std::future::Future>(name: &str, work: F) -> F::Output {
    tokio::time::timeout(PHASE, Box::pin(work))
        .await
        .unwrap_or_else(|error| panic!("diagnostic responder {name}: {error}"))
}
async fn open(version: ProtocolVersion) -> (NodeClient, MemoryTransport) {
    let (client_end, mut peer) =
        vessel_rpc::memory::pair(std::num::NonZeroUsize::new(RESPONDER_CAPACITY).unwrap());
    let serving = tokio::spawn(async move {
        let hello = phase("hello", peer.next()).await.unwrap().unwrap();
        assert!(matches!(
            serde_json::from_str::<ClientFrame>(&hello).unwrap(),
            ClientFrame::Hello(_)
        ));
        phase(
            "welcome",
            peer.send(
                serde_json::to_string(&ServerFrame::Welcome(Welcome {
                    protocol: version,
                    node: super::super::node(),
                }))
                .unwrap(),
            ),
        )
        .await
        .unwrap();
        peer
    });
    let client = phase(
        "client admitted",
        NodeClient::over(
            client_end,
            super::super::client_info(),
            &ClientConfig::default(),
        ),
    )
    .await
    .unwrap();
    (
        client,
        phase("handshake responder joined", serving).await.unwrap(),
    )
}
async fn request(peer: &mut MemoryTransport) -> RequestId {
    let text = phase("one diagnostic request", peer.next())
        .await
        .unwrap()
        .unwrap();
    let ClientFrame::Request { id, call } = serde_json::from_str(&text).unwrap() else {
        panic!("expected diagnostic request")
    };
    assert_eq!(call.method(), Method::Diagnostics);
    id
}
async fn answer(peer: &mut MemoryTransport, id: RequestId, value: DiagnosticReport) {
    phase(
        "correlated report",
        peer.send(
            serde_json::to_string(&ServerFrame::Response {
                id,
                outcome: Outcome::Ok(Reply::Diagnostics(value)),
            })
            .unwrap(),
        ),
    )
    .await
    .unwrap();
}
async fn closed(client: NodeClient, mut peer: MemoryTransport) {
    drop(client);
    assert!(phase("no fallback request", peer.next()).await.is_none());
}
#[tokio::test]
async fn diagnostics_supported_reports_are_correlated_and_check_the_welcomed_identity() {
    vessel_testkit::within(SAFETY, async {
        let mut versions = vec![
            DIAGNOSTICS_PROTOCOL.unwrap(),
            vessel_proto::PROTOCOL_VERSION,
        ];
        versions.dedup();
        for version in versions {
            for reported in [None, Some(NodeId::random())] {
                let (client, mut peer) = open(version).await;
                let expected = client.node().id;
                let id = reported.unwrap_or(expected);
                let responder = async {
                    let request = request(&mut peer).await;
                    answer(&mut peer, request, report(id)).await;
                };
                let (result, ()) = tokio::join!(client.diagnostics(), responder);
                match reported {
                    None => assert_eq!(result.unwrap(), report(expected)),
                    Some(reported) => assert!(matches!(result,
                        Err(DiagnosticError::Identity { expected: got, reported: other })
                            if got == expected && other == reported)),
                }
                closed(client, peer).await;
            }
        }
    })
    .await
    .unwrap();
}
#[tokio::test(start_paused = true)]
async fn diagnostics_timeout_is_the_named_client_budget_and_cancels_only_its_request() {
    vessel_testkit::within(SAFETY, async {
        let (client, mut peer) = open(DIAGNOSTICS_PROTOCOL.unwrap()).await;
        let pending = tokio::spawn({ let client = client.clone(); async move { client.diagnostics().await } });
        let id = request(&mut peer).await;
        tokio::time::advance(DIAGNOSTIC_REQUEST_TIMEOUT).await;
        assert!(matches!(phase("client timeout", pending).await.unwrap(), Err(DiagnosticError::Timeout(_))));
        let text = phase("cancel timed out request", peer.next()).await.unwrap().unwrap();
        assert!(matches!(serde_json::from_str::<ClientFrame>(&text).unwrap(), ClientFrame::Cancel { id: got } if got == id));
        // An actual late report cannot become the following call's answer.
        answer(&mut peer, id, report(NodeId::random())).await;
        let responder = async { let next = request(&mut peer).await; assert_ne!(next, id);
            answer(&mut peer, next, report(client.node().id)).await; };
        let (next, ()) = tokio::join!(client.diagnostics(), responder);
        assert_eq!(next.unwrap().fields().node.id, client.node().id);
        closed(client, peer).await;
    }).await.unwrap();
}
#[tokio::test]
async fn diagnostics_disconnect_preserves_the_typed_call_failure_without_fallback() {
    vessel_testkit::within(SAFETY, async {
        let (client, mut peer) = open(DIAGNOSTICS_PROTOCOL.unwrap()).await;
        let responder = async {
            let _id = request(&mut peer).await;
            drop(peer);
        };
        let (result, ()) = tokio::join!(client.diagnostics(), responder);
        assert!(matches!(
            result,
            Err(DiagnosticError::Call(CallError::Disconnected))
        ));
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn diagnostics_caller_abort_sends_the_matching_cancel_and_ignores_a_late_report() {
    vessel_testkit::within(SAFETY, async {
        let (client, mut peer) = open(DIAGNOSTICS_PROTOCOL.unwrap()).await;
        let pending = tokio::spawn({
            let client = client.clone();
            async move { client.diagnostics().await }
        });
        let id = request(&mut peer).await;
        pending.abort();
        assert!(phase("caller joined after abort", pending).await.unwrap_err().is_cancelled());
        let text = phase("cancel abandoned request", peer.next()).await.unwrap().unwrap();
        assert!(matches!(serde_json::from_str::<ClientFrame>(&text).unwrap(), ClientFrame::Cancel { id: got } if got == id));
        answer(&mut peer, id, report(NodeId::random())).await;
        let responder = async {
            let next = request(&mut peer).await;
            assert_ne!(next, id);
            answer(&mut peer, next, report(client.node().id)).await;
        };
        let (next, ()) = tokio::join!(client.diagnostics(), responder);
        assert_eq!(next.unwrap().fields().node.id, client.node().id);
        closed(client, peer).await;
    }).await.unwrap();
}
