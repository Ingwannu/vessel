//! Real selected-node exports plus a typed previous-protocol responder.
use crate::support::{Fixture, within};
use futures_util::{SinkExt as _, StreamExt as _};
use std::{future::Future, time::Duration};
use vessel_proto::diagnostics::{DIAGNOSTICS_PROTOCOL, DiagnosticReport};
use vessel_proto::{ClientFrame, ProtocolVersion, ServerFrame, Welcome};
const PHASE: Duration = Duration::from_secs(20);
async fn phase<F: Future>(name: &str, work: F) -> F::Output {
    tokio::time::timeout(PHASE, Box::pin(work))
        .await
        .unwrap_or_else(|error| panic!("diagnostic CLI {name}: {error}"))
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn diagnostics_named_selection_text_and_json_belong_to_each_selected_node() {
    within(async {
        let a = phase("first node", Fixture::start()).await;
        let b = phase("second node", Fixture::start()).await;
        for (name, selected) in [("first", &a), ("second", &b)] {
            let url = format!("ws://{}", selected.node_address());
            let token = selected.data().join("token");
            let added = phase(
                "add named selection",
                a.vessel_book(
                    a.data(),
                    &[
                        "nodes",
                        "add",
                        name,
                        &url,
                        "--token-file",
                        token.to_str().unwrap(),
                    ],
                    &[],
                ),
            )
            .await;
            assert!(added.ok, "{}", added.stderr);
            let args = ["--node", name, "node", "diagnostics"];
            let text = phase("selected text", a.vessel_book(a.data(), &args, &[])).await;
            assert!(text.ok, "{}", text.stderr);
            assert!(
                text.stdout
                    .contains(&format!("Node: {}", selected.node().info().id))
            );
            let args = ["--node", name, "node", "diagnostics", "--json"];
            let json = phase("selected JSON", a.vessel_book(a.data(), &args, &[])).await;
            assert!(json.ok, "{}", json.stderr);
            let report: DiagnosticReport = serde_json::from_str(&json.stdout).unwrap();
            assert_eq!(report.fields().node.id, selected.node().info().id);
            assert!(!json.stdout.contains(selected.data().to_str().unwrap()));
        }
        assert_ne!(a.node().info().id, b.node().info().id);
        phase("stop first node", a.stop()).await;
        phase("stop second node", b.stop()).await;
    })
    .await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn diagnostics_older_cli_peer_refuses_without_sending_a_request_or_local_fallback() {
    within(async {
        let fixture = phase("CLI temporary home", Fixture::start()).await;
        let needs = DIAGNOSTICS_PROTOCOL.unwrap();
        let previous = ProtocolVersion::new(needs.get().checked_sub(1).unwrap());
        for args in [
            vec!["node", "diagnostics"],
            vec!["node", "diagnostics", "--json"],
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap().to_string();
            let mut info = fixture.node().info();
            info.protocol.max = previous;
            let peer = tokio::spawn(async move {
                let (socket, _address) =
                    phase("old peer accepts", listener.accept()).await.unwrap();
                let mut ws = phase(
                    "old peer websocket",
                    tokio_tungstenite::accept_async(socket),
                )
                .await
                .unwrap();
                let hello = phase("old peer hello", ws.next()).await.unwrap().unwrap();
                assert!(matches!(
                    serde_json::from_str::<ClientFrame>(hello.to_text().unwrap()).unwrap(),
                    ClientFrame::Hello(_)
                ));
                phase(
                    "old peer welcome",
                    ws.send(tokio_tungstenite::tungstenite::Message::Text(
                        serde_json::to_string(&ServerFrame::Welcome(Welcome {
                            protocol: previous,
                            node: info,
                        }))
                        .unwrap()
                        .into(),
                    )),
                )
                .await
                .unwrap();
                // A websocket close/EOF is allowed; an RPC request is not.
                if let Some(Ok(message)) = phase("old peer sees no call", ws.next()).await {
                    assert!(message.is_close());
                }
            });
            let result = phase(
                "old peer CLI refusal",
                fixture.vessel_at(&address, fixture.data(), &args, b""),
            )
            .await;
            assert!(!result.ok);
            assert!(result.stdout.is_empty());
            assert!(result.stderr.contains("this peer uses an older protocol"));
            assert!(!result.stderr.contains(fixture.data().to_str().unwrap()));
            phase("old responder exits", peer).await.unwrap();
        }
        phase("stop old-peer fixture", fixture.stop()).await;
    })
    .await;
}
