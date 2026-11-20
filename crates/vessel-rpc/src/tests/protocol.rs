//! Requests against older connections: each peer offers one version,
//! so negotiation cannot hide an unsupported field or method.

use vessel_proto::{CreateSession, GetManifest, Hello};

use super::*;

/// A raw client speaking only `version`, past the welcome.
async fn raw_client(version: u32) -> (MemoryTransport, Served) {
    let (mut client_end, server_end) = pair();
    let served = serve(server_end);
    let version = ProtocolVersion::new(version);
    let hello = ClientFrame::Hello(Hello {
        protocol: ProtocolRange {
            min: version,
            max: version,
        },
        client: client_info(),
    });
    client_end
        .send(serde_json::to_string(&hello).unwrap())
        .await
        .unwrap();
    let welcome = client_end.next().await.unwrap().unwrap();
    let Ok(ServerFrame::Welcome(welcome)) = serde_json::from_str::<ServerFrame>(&welcome) else {
        panic!("expected a welcome, got {welcome}");
    };
    assert_eq!(welcome.protocol, version);
    (client_end, served)
}

/// Send one call and return the text of the next frame.
async fn call_raw(client: &mut MemoryTransport, id: u64, call: Call) -> String {
    let request = ClientFrame::Request {
        id: RequestId::new(id),
        call,
    };
    client
        .send(serde_json::to_string(&request).unwrap())
        .await
        .unwrap();
    client.next().await.unwrap().unwrap()
}

/// Negotiate a client with a node that cannot raise its advertised version.
async fn older_node(version: ProtocolVersion) -> (Connection, MemoryTransport) {
    let (client_end, mut server_end) = pair();
    let older_node = async move {
        let hello = server_end.next().await.unwrap().unwrap();
        let _hello: ClientFrame = serde_json::from_str(&hello).unwrap();
        let welcome = ServerFrame::Welcome(vessel_proto::Welcome {
            protocol: version,
            node: node(),
        });
        server_end
            .send(serde_json::to_string(&welcome).unwrap())
            .await
            .unwrap();
        server_end
    };
    let config = ClientConfig::default();
    let (client, server_end) = tokio::join!(
        Connection::connect(client_end, client_info(), &config),
        older_node
    );
    (client.unwrap(), server_end)
}

fn refusal(text: &str) -> WireError {
    let Ok(ServerFrame::Response {
        outcome: Outcome::Err(error),
        ..
    }) = serde_json::from_str::<ServerFrame>(text)
    else {
        panic!("expected an error response, got {text}");
    };
    error
}

/// Neither a method from a later version nor a field from a later version
/// reaches the handler on an older connection.
#[tokio::test]
async fn an_older_connection_cannot_use_newer_methods_or_fields() {
    let (mut client, mut served) = raw_client(4).await;

    let error = refusal(&call_raw(&mut client, 1, GetManifest.into_call()).await);
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert!(
        error.message.contains("needs protocol 5"),
        "{}",
        error.message
    );

    let spec = spec().with_profile("alt".parse().unwrap());
    let error = refusal(&call_raw(&mut client, 2, CreateSession { spec }.into_call()).await);
    assert_eq!(error.code, ErrorCode::Unsupported);

    // A call this version has still works on the same connection.
    let text = call_raw(&mut client, 3, GetNode.into_call()).await;
    assert!(text.contains(r#""outcome":{"ok""#), "{text}");
    assert_eq!(served.seen.recv().await.unwrap().method(), GetNode::METHOD);
    assert!(served.seen.try_recv().is_err());
}

/// An event from a later version reaches an older connection as its
/// stand-in, at the same position; a current connection gets the event.
#[tokio::test]
async fn an_older_connection_reads_a_stand_in_for_a_newer_event() {
    for (version, present, absent) in [
        (
            4,
            "vessel: profile changed from default to alt",
            "profile_changed",
        ),
        (5, "profile_changed", "harness_log"),
    ] {
        let (mut client, _served) = raw_client(version).await;
        let read = ReadEvents {
            session: moved(),
            from: EventSeq::new(2),
            limit: 10,
            wait: None,
        };
        let text = call_raw(&mut client, 1, read.into_call()).await;
        assert!(text.contains(present), "protocol {version}: {text}");
        assert!(!text.contains(absent), "protocol {version}: {text}");
        assert!(text.contains(r#""seq":2"#), "protocol {version}: {text}");
    }
}

/// `call_any` holds an already-built call to the negotiated version, as
/// typed calls are: one newer than the connection is not sent.
#[tokio::test]
async fn an_already_built_call_newer_than_the_connection_is_not_sent() {
    let (client, _server) = older_node(ProtocolVersion::new(4)).await;
    let call = UnaryCall::try_from(ListProfiles { harness: None }.into_call()).unwrap();
    assert!(matches!(
        client.call_any(call).await,
        Err(CallError::NotInProtocol {
            method: vessel_proto::Method::ListProfiles,
            ..
        })
    ));
}

#[tokio::test]
async fn a_protocol_12_connection_refuses_preview_before_dispatch() {
    let (mut client, mut served) = raw_client(12).await;
    let request = vessel_proto::automation::PreviewSchedule {
        cadence: vessel_proto::automation::Cadence::Every {
            seconds: vessel_proto::automation::EverySeconds::try_from(60).unwrap(),
            anchor: None,
        },
        after: None,
        count: vessel_proto::automation::PreviewCount::default(),
    };
    let error = refusal(&call_raw(&mut client, 1, request.into_call()).await);
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert!(served.seen.try_recv().is_err());
}

#[tokio::test]
async fn an_old_connection_refuses_direct_and_wrapped_self_before_dispatch() {
    use vessel_proto::{Deliver, Forward, GetDeviceSelf, Origin, OriginBy, RelayCall};
    let previous = GetDeviceSelf::METHOD.since().get().checked_sub(1).unwrap();
    let (mut client, mut served) = raw_client(previous).await;
    let query = GetDeviceSelf {}.into_call();
    let origin = Origin {
        node: NodeId::random(),
        by: OriginBy::Owner,
        scopes: vec![],
    };
    for call in [
        query.clone(),
        Forward {
            node: NodeId::random(),
            call: Box::new(query.clone()),
        }
        .into_call(),
        RelayCall {
            node: NodeId::random(),
            call: Box::new(query.clone()),
            origin: origin.clone(),
        }
        .into_call(),
        Deliver {
            call: Box::new(query),
            origin,
        }
        .into_call(),
    ] {
        assert_eq!(
            refusal(&call_raw(&mut client, 1, call).await).code,
            ErrorCode::Unsupported
        );
        assert!(served.seen.try_recv().is_err());
    }
}

fn labeled_creation_calls() -> [Call; 2] {
    let labels = serde_json::from_str(r#"{"env":"prod"}"#).unwrap();
    let call = CreateSession {
        spec: spec().with_labels(labels),
    }
    .into_call();
    [
        call.clone(),
        vessel_proto::Forward {
            node: NodeId::random(),
            call: Box::new(call),
        }
        .into_call(),
    ]
}

#[tokio::test]
async fn creation_labels_are_refused_before_an_older_server_dispatches() {
    let previous = vessel_proto::lifecycle_version::SINCE
        .checked_sub(1)
        .unwrap();
    let (mut raw, mut served) = raw_client(previous).await;
    for (index, call) in labeled_creation_calls().into_iter().enumerate() {
        let response = call_raw(&mut raw, u64::try_from(index).unwrap(), call).await;
        assert_eq!(refusal(&response).code, ErrorCode::Unsupported);
    }
    assert!(served.seen.try_recv().is_err());
}

#[tokio::test]
async fn creation_labels_are_never_sent_to_an_older_node() {
    let previous = vessel_proto::lifecycle_version::SINCE
        .checked_sub(1)
        .unwrap();
    let (client, mut server_end) = older_node(ProtocolVersion::new(previous)).await;
    for call in labeled_creation_calls() {
        let call = UnaryCall::try_from(call).unwrap();
        assert!(matches!(
            client.call_any(call).await,
            Err(CallError::NotInProtocol { .. })
        ));
    }
    // A permitted call is a barrier: it must be the first frame after Hello.
    let permitted = async {
        let request: ClientFrame =
            serde_json::from_str(&server_end.next().await.unwrap().unwrap()).unwrap();
        let ClientFrame::Request { id, call } = request else {
            panic!("expected node info");
        };
        assert_eq!(call, GetNode.into_call());
        let response = ServerFrame::Response {
            id,
            outcome: Outcome::Ok(vessel_proto::Reply::NodeInfo(node())),
        };
        server_end
            .send(serde_json::to_string(&response).unwrap())
            .await
            .unwrap();
    };
    let (reply, ()) = tokio::join!(client.call(GetNode), permitted);
    let _node = reply.unwrap();
}

#[tokio::test]
async fn malformed_creation_labels_are_refused_before_dispatch() {
    let (mut raw, mut served) = raw_client(vessel_proto::lifecycle_version::SINCE).await;
    let too_many: std::collections::BTreeMap<_, _> = (0..33)
        .map(|index| (format!("key{index}"), "value"))
        .collect();
    let maps = [
        serde_json::json!({"": "v"}),
        serde_json::json!({"vessel.reserved": "v"}),
        serde_json::json!({"a=b": "v"}),
        serde_json::json!({"\u{0085}": "v"}),
        serde_json::json!({"a": "\u{0085}"}),
        serde_json::json!({"a": "x".repeat(257)}),
        serde_json::json!({"a": null}),
        serde_json::json!(too_many),
    ];
    for (index, labels) in maps.into_iter().enumerate() {
        let request = serde_json::json!({"type":"request", "id":index, "call":{
            "method":"session.create", "params":{"spec":{
                "harness":"claude-code", "mode":"structured", "labels":labels
            }}
        }});
        raw.send(request.to_string()).await.unwrap();
        let response = raw.next().await.unwrap().unwrap();
        assert_eq!(refusal(&response).code, ErrorCode::InvalidRequest);
    }
    assert!(
        served.seen.try_recv().is_err(),
        "no malformed input reached the node handler"
    );
}
