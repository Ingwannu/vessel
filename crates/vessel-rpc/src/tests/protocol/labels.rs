//! The shared labels fixture is fictional wire input. Decoding stays here;
//! the SDK and forced-frame assertions exercise the same typed requests.

use super::*;

fn calls() -> Vec<Call> {
    let direct: Vec<Call> = serde_json::from_str(include_str!(
        "../../../../vessel-proto/tests/fixtures/labelled-requests.json"
    ))
    .unwrap();
    let node = NodeId::random();
    let origin = vessel_proto::Origin {
        node,
        by: vessel_proto::OriginBy::Owner,
        scopes: Vec::new(),
    };
    let mut calls = Vec::new();
    for call in direct {
        calls.push(call.clone());
        let mut nested = call.clone();
        for method in [Method::Forward, Method::Relay, Method::Deliver] {
            // These are decoding-edge fixtures, not a second method registry.
            let envelope = |inner: &Call| {
                serde_json::from_value(serde_json::json!({
                    "method": method.as_str(),
                    "params": {"node": node, "origin": origin, "call": inner}
                }))
                .unwrap()
            };
            calls.push(envelope(&call));
            nested = envelope(&nested);
        }
        calls.push(nested);
    }
    calls
}

#[tokio::test]
async fn raw_labelled_specs_and_envelopes_are_refused_before_dispatch() {
    let (mut client, mut served) = raw_client(vessel_proto::PROTOCOL_VERSION.get()).await;
    for call in calls() {
        let method = call.method();
        let error = refusal(&call_raw(&mut client, 1, call).await);
        assert_eq!(error.code, ErrorCode::Unsupported);
        assert_eq!(
            error.message,
            CallError::UnassignedProtocol { method }.to_string()
        );
    }
    let text = call_raw(&mut client, 2, GetNode.into_call()).await;
    assert!(matches!(
        serde_json::from_str::<ServerFrame>(&text).unwrap(),
        ServerFrame::Response {
            outcome: Outcome::Ok(_),
            ..
        }
    ));
    assert_eq!(served.seen.recv().await.unwrap().method(), GetNode::METHOD);
    assert!(served.seen.try_recv().is_err(), "labels reached dispatch");
}

async fn connected() -> (Connection, MemoryTransport) {
    let (client_end, mut server_end) = pair();
    let config = ClientConfig::default();
    let handshake = async move {
        let frame = server_end.next().await.unwrap().unwrap();
        assert!(matches!(
            serde_json::from_str::<ClientFrame>(&frame).unwrap(),
            ClientFrame::Hello(_)
        ));
        let welcome = ServerFrame::Welcome(vessel_proto::Welcome {
            protocol: vessel_proto::PROTOCOL_VERSION,
            node: node(),
        });
        server_end
            .send(serde_json::to_string(&welcome).unwrap())
            .await
            .unwrap();
        server_end
    };
    let (client, server) = tokio::join!(
        Connection::connect(client_end, client_info(), &config),
        handshake
    );
    (client.unwrap(), server)
}

#[tokio::test]
async fn sdk_labelled_specs_and_envelopes_never_send_a_frame_or_consume_an_id() {
    let (client, mut server) = connected().await;
    for call in calls() {
        let method = call.method();
        let error = client
            .call_any(UnaryCall::try_from(call).unwrap())
            .await
            .unwrap_err();
        assert_eq!(error, CallError::UnassignedProtocol { method });
        assert_eq!(error.code(), Some(ErrorCode::Unsupported));
    }
    // A valid request is a writer-queue barrier: it must be the first frame,
    // with the first id, rather than merely observing an empty queue early.
    let barrier = async {
        let frame = server.next().await.unwrap().unwrap();
        let ClientFrame::Request { id, call } = serde_json::from_str(&frame).unwrap() else {
            panic!("expected a request");
        };
        assert_eq!(id, RequestId::new(0));
        assert_eq!(call.method(), GetNode::METHOD);
        let response = ServerFrame::Response {
            id,
            outcome: Outcome::Ok(Reply::NodeInfo(node())),
        };
        server
            .send(serde_json::to_string(&response).unwrap())
            .await
            .unwrap();
    };
    let (result, ()) = tokio::join!(client.call(GetNode), barrier);
    let _node = result.unwrap();
}
