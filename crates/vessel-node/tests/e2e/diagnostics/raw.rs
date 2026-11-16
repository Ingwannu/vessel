//! Real authenticated websocket correlation at assigned and previous versions.
use crate::support::{Fixture, client, step, url, within};
use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest as _, http::header::AUTHORIZATION},
};
use vessel_proto::diagnostics::{DIAGNOSTICS_PROTOCOL, GetDiagnostics};
use vessel_proto::{
    ClientFrame, ErrorCode, GetNode, Hello, Outcome, ProtocolRange, Reply, Request as _, RequestId,
    ServerFrame,
};
type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
async fn send(socket: &mut Socket, frame: &ClientFrame) {
    step(
        "raw diagnostics send",
        socket.send(Message::text(serde_json::to_string(frame).unwrap())),
    )
    .await
    .unwrap();
}
async fn receive(socket: &mut Socket) -> ServerFrame {
    let Some(Ok(Message::Text(text))) = step("raw diagnostics response", socket.next()).await
    else {
        panic!("expected a text response")
    };
    serde_json::from_str(text.as_str()).unwrap()
}
async fn raw(
    fixture: &Fixture,
    node: &vessel_node::Node,
    version: vessel_proto::ProtocolVersion,
) -> Socket {
    let mut request = url(node).as_uri().clone().into_client_request().unwrap();
    let bearer = format!("Bearer {}", fixture.token().expose());
    let _previous = request
        .headers_mut()
        .insert(AUTHORIZATION, bearer.parse().unwrap());
    let (mut socket, _response) = step(
        "raw diagnostic authenticated upgrade",
        tokio_tungstenite::connect_async(request),
    )
    .await
    .unwrap();
    send(
        &mut socket,
        &ClientFrame::Hello(Hello {
            protocol: ProtocolRange {
                min: version,
                max: version,
            },
            client: client(),
        }),
    )
    .await;
    let ServerFrame::Welcome(welcome) = receive(&mut socket).await else {
        panic!("expected welcome")
    };
    assert_eq!(welcome.protocol, version);
    assert_eq!(welcome.node.id, node.info().id);
    socket
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn diagnostics_real_raw_wire_preserves_correlation_and_refuses_unknown_request_fields() {
    within(async {
        let fixture = Fixture::new();
        let node = step("raw diagnostic node", fixture.start()).await;
        let needs = DIAGNOSTICS_PROTOCOL.unwrap();
        let previous = vessel_proto::ProtocolVersion::new(needs.get().checked_sub(1).unwrap());
        let mut versions = vec![previous, needs, vessel_proto::PROTOCOL_VERSION];
        versions.dedup();
        for version in versions {
            let mut socket = raw(&fixture, &node, version).await;
            let id = RequestId::new(41);
            send(&mut socket, &ClientFrame::Request { id, call: GetDiagnostics {}.into_call() }).await;
            let ServerFrame::Response { id: got, outcome } = receive(&mut socket).await else { panic!("expected report response") };
            assert_eq!(got, id);
            if version < needs {
                assert!(matches!(outcome, Outcome::Err(error) if error.code == ErrorCode::Unsupported));
            } else {
                let Outcome::Ok(Reply::Diagnostics(report)) = outcome else { panic!("expected assigned real report") };
                assert_eq!(report.fields().node.id, node.info().id);
                let private = "DIAGNOSTIC-UNKNOWN-FIELD-CANARY";
                let invalid = format!(r#"{{"type":"request","id":42,"call":{{"method":"node.diagnostics","params":{{"{private}":"{private}"}}}}}}"#);
                step("send unknown diagnostic field", socket.send(Message::text(invalid))).await.unwrap();
                let ServerFrame::Response { id: got, outcome: Outcome::Err(error) } = receive(&mut socket).await else { panic!("expected correlated decode refusal") };
                assert_eq!(got, RequestId::new(42));
                assert_eq!(error.code, ErrorCode::InvalidRequest);
                assert!(!error.message.contains(private));
            }
            let id = RequestId::new(43);
            send(&mut socket, &ClientFrame::Request { id, call: GetNode.into_call() }).await;
            assert!(matches!(receive(&mut socket).await, ServerFrame::Response { id: got, outcome: Outcome::Ok(Reply::NodeInfo(info)) } if got == id && info.id == node.info().id));
            step("close raw diagnostic connection", socket.close(None)).await.unwrap();
        }
        step("stop raw diagnostic node", node.stop()).await.unwrap();
    }).await;
}
