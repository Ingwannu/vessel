//! Protocol 5 against older connections: a raw client that offers only one
//! version, so the server cannot negotiate up.

use vessel_proto::{CreateSession, GetManifest, Hello, Method, Reply};

use super::*;

/// A raw client speaking only `version`, past the welcome.
async fn raw_client(version: u32) -> (MemoryTransport, Served) {
    let (client, served, _) = raw_client_with_node(version).await;
    (client, served)
}

async fn raw_client_with_node(version: u32) -> (MemoryTransport, Served, NodeId) {
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
    (client_end, served, welcome.node.id)
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
    let (client_end, mut server_end) = pair();
    // A node that speaks protocol 4 only.
    let older_node = async move {
        let hello = server_end.next().await.unwrap().unwrap();
        let _hello: ClientFrame = serde_json::from_str(&hello).unwrap();
        let welcome = ServerFrame::Welcome(vessel_proto::Welcome {
            protocol: ProtocolVersion::new(4),
            node: node(),
        });
        server_end
            .send(serde_json::to_string(&welcome).unwrap())
            .await
            .unwrap();
        server_end
    };
    let config = ClientConfig::default();
    let (client, _server_end) = tokio::join!(
        Connection::connect(client_end, client_info(), &config),
        older_node
    );
    let client = client.unwrap();
    let call = UnaryCall::try_from(ListProfiles { harness: None }.into_call()).unwrap();
    assert!(matches!(
        client.call_any(call).await,
        Err(CallError::NotInProtocol {
            method: Method::ListProfiles,
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
    let previous = GetDeviceSelf::METHOD
        .since()
        .unwrap()
        .get()
        .checked_sub(1)
        .unwrap();
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

#[tokio::test]
async fn usage_is_refused_before_dispatch_on_a_pre_usage_connection() {
    use vessel_proto::usage::{QueryUsage, UsagePeriod, UsageScope};
    let previous = QueryUsage::METHOD.since().unwrap().get().saturating_sub(1);
    let (mut client, mut served, target) = raw_client_with_node(previous).await;
    let call = QueryUsage {
        scope: UsageScope::Machine,
        period: UsagePeriod::Day,
        after: None,
    }
    .into_call();
    for wrapped in usage_envelopes(call, target) {
        let expected = WireError::new(
            ErrorCode::Unsupported,
            format!(
                "`{}` as sent needs protocol {}; this connection speaks {}",
                wrapped.method().as_str(),
                QueryUsage::METHOD.since().unwrap(),
                previous
            ),
        );
        assert_eq!(refusal(&call_raw(&mut client, 1, wrapped).await), expected);
        assert!(
            served.seen.try_recv().is_err(),
            "no unsupported envelope reached dispatch"
        );
    }
    // Supported envelopes reach the handler unchanged on the same older
    // connection. This fake deliberately refuses their payloads, recording
    // them first; a version refusal must never substitute for that dispatch.
    for supported in usage_envelopes(GetNode.into_call(), target)
        .into_iter()
        .skip(1)
    {
        let actual = refusal(&call_raw(&mut client, 1, supported.clone()).await);
        assert_eq!(
            actual,
            WireError::new(ErrorCode::Unsupported, "not in this fake")
        );
        assert_eq!(served.seen.recv().await.unwrap(), supported);
    }
    let response = call_raw(&mut client, 2, GetNode.into_call()).await;
    assert!(matches!(
        serde_json::from_str::<ServerFrame>(&response).unwrap(),
        ServerFrame::Response {
            outcome: Outcome::Ok(Reply::NodeInfo(_)),
            ..
        }
    ));
    assert_eq!(served.seen.recv().await.unwrap().method(), Method::NodeInfo);
    assert!(
        served.seen.try_recv().is_err(),
        "usage never reached the handler"
    );
}

fn usage_envelopes(call: Call, node: NodeId) -> [Call; 4] {
    let origin = vessel_proto::Origin {
        node,
        by: vessel_proto::OriginBy::Owner,
        scopes: vec![],
    };
    [
        call.clone(),
        vessel_proto::Forward {
            node,
            call: Box::new(call.clone()),
        }
        .into_call(),
        vessel_proto::RelayCall {
            node,
            call: Box::new(call.clone()),
            origin: origin.clone(),
        }
        .into_call(),
        vessel_proto::Deliver {
            call: Box::new(call),
            origin,
        }
        .into_call(),
    ]
}

#[tokio::test]
async fn forced_limit_policy_and_reads_are_refused_before_old_server_dispatch() {
    use vessel_proto::limits::*;
    let needed = GetUsageLimits::METHOD.since().unwrap();
    assert_eq!(Some(needed), SetUsageLookup::METHOD.since());
    let previous = needed.get().checked_sub(1).unwrap();
    let (mut client, mut served, target) = raw_client_with_node(previous).await;
    let query = GetUsageLimits {
        scope: LimitsScope::Account {
            account: LimitAccount {
                harness: HarnessKind::Codex.into(),
                profile: None,
            },
        },
        refresh: LimitRefresh::Refresh,
    };
    for inner in [
        query.into_call(),
        SetUsageLookup {
            policy: ExternalLookup::On,
        }
        .into_call(),
    ] {
        for wrapped in usage_envelopes(inner, target) {
            let expected = WireError::new(
                ErrorCode::Unsupported,
                format!(
                    "`{}` as sent needs protocol {}; this connection speaks {}",
                    wrapped.method().as_str(),
                    needed,
                    previous
                ),
            );
            assert_eq!(refusal(&call_raw(&mut client, 1, wrapped).await), expected);
            assert!(
                served.seen.try_recv().is_err(),
                "an unsupported limit call reached dispatch"
            );
        }
    }
    for supported in usage_envelopes(GetNode.into_call(), target)
        .into_iter()
        .skip(1)
    {
        assert_eq!(
            refusal(&call_raw(&mut client, 1, supported.clone()).await),
            WireError::new(ErrorCode::Unsupported, "not in this fake")
        );
        assert_eq!(served.seen.recv().await.unwrap(), supported);
    }
}

#[tokio::test]
async fn malformed_creation_labels_are_refused_before_dispatch() {
    let (mut raw, mut served) = raw_client(vessel_proto::PROTOCOL_VERSION.get()).await;
    let too_many: std::collections::BTreeMap<_, _> = (0..vessel_proto::SessionLabels::MAX
        .saturating_add(1))
        .map(|index| (format!("key{index}"), "value"))
        .collect();
    let maps = [
        serde_json::json!({"": "v"}),
        serde_json::json!({"vessel.reserved": "v"}),
        serde_json::json!({"a=b": "v"}),
        serde_json::json!({"\u{0085}": "v"}),
        serde_json::json!({"a": "\u{0085}"}),
        serde_json::json!({"a": "x".repeat(vessel_proto::LabelValue::MAX_CHARS.saturating_add(1))}),
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

mod decode_privacy;
mod inbox;

mod labels;
