//! Synchronous admission can precede side effects without consuming the send path.

use super::*;

async fn old_connection() -> (Connection, MemoryTransport) {
    let (client_end, mut peer) = pair();
    let handshake = async move {
        let hello = peer.next().await.unwrap().unwrap();
        assert!(matches!(
            serde_json::from_str::<ClientFrame>(&hello).unwrap(),
            ClientFrame::Hello(_)
        ));
        let welcome = ServerFrame::Welcome(vessel_proto::Welcome {
            protocol: vessel_proto::MIN_PROTOCOL_VERSION,
            node: node(),
        });
        peer.send(serde_json::to_string(&welcome).unwrap())
            .await
            .unwrap();
        peer
    };
    let config = ClientConfig::default();
    let (client, peer) = tokio::join!(
        Connection::connect(client_end, client_info(), &config),
        handshake,
    );
    (client.unwrap(), peer)
}

#[tokio::test]
async fn preflight_refuses_exactly_without_consuming_an_id_or_writing_a_frame() {
    let (client, mut peer) = old_connection().await;
    let unsupported = ListProfiles { harness: None }.into_call();
    assert_eq!(
        client.check_call(&unsupported),
        Err(CallError::NotInProtocol {
            method: unsupported.method(),
            needs: unsupported.required_protocol().unwrap(),
            negotiated: vessel_proto::MIN_PROTOCOL_VERSION,
        })
    );
    assert_eq!(client.check_call(&GetNode.into_call()), Ok(()));
    // A subsequent request is a writer barrier, rather than an early empty-queue observation.
    let first_frame = async {
        let text = peer.next().await.unwrap().unwrap();
        let ClientFrame::Request { id, call } = serde_json::from_str(&text).unwrap() else {
            panic!("expected the first real call")
        };
        assert_eq!(id, RequestId::new(0));
        assert_eq!(call, GetNode.into_call());
        peer.send(
            serde_json::to_string(&ServerFrame::Response {
                id,
                outcome: Outcome::Ok(vessel_proto::Reply::NodeInfo(node())),
            })
            .unwrap(),
        )
        .await
        .unwrap();
    };
    let (result, ()) = tokio::join!(client.call(GetNode), first_frame);
    assert_eq!(result.unwrap().name, "test-node");
}
