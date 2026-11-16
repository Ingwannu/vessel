//! Calls: every method in both directions, cancellation, bad requests.

use super::*;

#[tokio::test]
async fn every_unary_method_round_trips() {
    let mut running = start(ClientConfig::default()).await;
    let client = &running.client;
    assert_eq!(client.welcome().protocol, vessel_proto::PROTOCOL_VERSION);

    let input = SendInput {
        session: session(1),
        input: SessionInput::Terminal {
            input: TerminalInput::Data {
                data: TerminalBytes::new(bytes::Bytes::from_static(&[0xED, 0x95, 0x9C])),
            },
        },
    };
    let read = ReadEvents {
        session: session(1),
        from: EventSeq::new(3),
        limit: 10,
        wait: None,
    };
    let create = CreateSession { spec: spec() };

    assert_eq!(client.call(GetNode).await.unwrap().name, "test-node");
    assert_eq!(client.call(ListHarnesses).await.unwrap().len(), 1);
    assert_eq!(
        client.call(ListSessions::default()).await.unwrap(),
        vec![info(session(1), spec())]
    );
    assert_eq!(
        client
            .call(GetSession {
                session: session(1)
            })
            .await
            .unwrap(),
        info(session(1), spec())
    );
    assert_eq!(
        client.call(create.clone()).await.unwrap(),
        info(session(2), spec())
    );
    assert_eq!(client.call(input.clone()).await.unwrap(), Ack);
    assert_eq!(
        client
            .call(StopNode {
                when: StopWhen::WhenIdle
            })
            .await
            .unwrap(),
        Ack
    );
    let device = vessel_proto::DeviceId::random();
    assert_eq!(client.call(ListDevices).await.unwrap(), Vec::new());
    assert_eq!(client.call(RevokeDevice { device }).await.unwrap(), Ack);
    let invite = InviteDevice {
        label: Some("laptop".to_owned()),
        ttl_secs: 600,
        scopes: None,
    };
    assert_eq!(
        client.call(invite.clone()).await.unwrap_err().code(),
        Some(ErrorCode::Unsupported)
    );
    let page = client.call(read).await.unwrap();
    assert_eq!(page.records, vec![record(3)]);
    assert_eq!(page.next, EventSeq::new(4));
    // `session.unsubscribe` for an unknown id surfaces the node's error code.
    let unknown = Unsubscribe {
        subscription: vessel_proto::SubscriptionId::new(77),
    };
    assert_eq!(
        client.call(unknown).await.unwrap_err().code(),
        Some(ErrorCode::NotFound)
    );
    let missing = GetSession {
        session: session(3),
    };
    assert_eq!(
        client.call(missing).await.unwrap_err().code(),
        Some(ErrorCode::NotFound)
    );

    // The node received exactly what was sent, in order.
    let expected = [
        GetNode.into_call(),
        ListHarnesses.into_call(),
        ListSessions::default().into_call(),
        GetSession {
            session: session(1),
        }
        .into_call(),
        create.into_call(),
        input.into_call(),
        StopNode {
            when: StopWhen::WhenIdle,
        }
        .into_call(),
        ListDevices.into_call(),
        RevokeDevice { device }.into_call(),
        invite.into_call(),
        read.into_call(),
        unknown.into_call(),
        missing.into_call(),
    ];
    for call in expected {
        assert_eq!(running.seen.recv().await.unwrap(), call);
    }
}

#[tokio::test]
async fn dropping_a_call_aborts_it_on_the_node() {
    let running = start(ClientConfig::default()).await;
    let client = running.client.clone();
    let call = tokio::spawn(async move {
        client
            .call(KillSession {
                session: session(1),
            })
            .await
    });
    running.probe.started.notified().await;
    call.abort();
    running.probe.aborted.notified().await;
    // The connection is still usable.
    assert_eq!(
        running.client.call(GetNode).await.unwrap().name,
        "test-node"
    );
}

#[tokio::test]
async fn unknown_methods_are_answered_by_id() {
    let (mut client_end, server_end) = pair();
    let _served = serve(server_end);
    let hello = ClientFrame::Hello(vessel_proto::Hello {
        protocol: ProtocolRange::CURRENT,
        client: client_info(),
    });
    client_end
        .send(serde_json::to_string(&hello).unwrap())
        .await
        .unwrap();
    let _welcome = client_end.next().await.unwrap().unwrap();
    let call = r#"{"type":"request","id":5,"call":{"method":"session.frobnicate","params":null}}"#;
    client_end.send(call.to_owned()).await.unwrap();
    let answer = client_end.next().await.unwrap().unwrap();
    let ServerFrame::Response {
        id,
        outcome: Outcome::Err(error),
    } = serde_json::from_str(&answer).unwrap()
    else {
        panic!("expected an error response, got {answer}");
    };
    assert_eq!(id, RequestId::new(5));
    assert_eq!(error.code, ErrorCode::InvalidRequest);
}

/// Protocol 10: an attachment's name is one file name, checked where the
/// frame is decoded, so a name that would leave its folder never reaches
/// the node's handler.
#[tokio::test]
async fn an_attachment_name_that_leaves_its_folder_never_reaches_the_node() {
    let (mut client_end, server_end) = pair();
    let mut served = serve(server_end);
    let hello = ClientFrame::Hello(vessel_proto::Hello {
        protocol: ProtocolRange::CURRENT,
        client: client_info(),
    });
    client_end
        .send(serde_json::to_string(&hello).unwrap())
        .await
        .unwrap();
    let _welcome = client_end.next().await.unwrap().unwrap();
    let session = session(1);
    for (id, bad) in [
        (1_u64, "../token"),
        (2, "a/b"),
        (3, "a\\\\b"),
        (4, ".."),
        (5, ""),
    ] {
        let call = format!(
            r#"{{"type":"request","id":{id},"call":{{"method":"attachment.begin","params":{{"session":"{session}","name":"{bad}","mime":"text/plain","size":1}}}}}}"#
        );
        client_end.send(call).await.unwrap();
        let answer = client_end.next().await.unwrap().unwrap();
        let ServerFrame::Response {
            outcome: Outcome::Err(error),
            ..
        } = serde_json::from_str(&answer).unwrap()
        else {
            panic!("expected an error response for {bad:?}, got {answer}");
        };
        assert_eq!(error.code, ErrorCode::InvalidRequest, "{bad:?}");
    }
    assert!(served.seen.try_recv().is_err(), "no call reached the node");
    // The same frame with a file name does.
    let call = format!(
        r#"{{"type":"request","id":9,"call":{{"method":"attachment.begin","params":{{"session":"{session}","name":"notes.txt","mime":"text/plain","size":1}}}}}}"#
    );
    client_end.send(call).await.unwrap();
    let _answer = client_end.next().await.unwrap().unwrap();
    assert_eq!(
        served.seen.recv().await.unwrap().method(),
        <vessel_proto::conversation::BeginUpload as Request>::METHOD
    );
}

#[tokio::test]
async fn calls_over_the_limit_are_refused_while_cancel_still_gets_through() {
    let (client_end, server_end) = pair();
    let config = ServerConfig {
        max_in_flight: NonZeroUsize::new(1).unwrap(),
        ..ServerConfig::default()
    };
    let served = serve_with(server_end, config);
    let client = Connection::connect(client_end, client_info(), &ClientConfig::default())
        .await
        .unwrap();
    let blocked = client.clone();
    let stuck = tokio::spawn(async move {
        blocked
            .call(KillSession {
                session: session(1),
            })
            .await
    });
    served.probe.started.notified().await;
    assert_eq!(
        client.call(GetNode).await.unwrap_err().code(),
        Some(ErrorCode::Unavailable)
    );
    // The connection is still read: the cancel arrives and frees the slot.
    stuck.abort();
    served.probe.aborted.notified().await;
    assert_eq!(client.call(GetNode).await.unwrap().name, "test-node");
}

#[tokio::test]
async fn callers_never_wait_on_a_node_that_stopped_reading_and_hung_up() {
    let (client_end, mut node_end) = memory::pair(NonZeroUsize::new(1).unwrap());
    let node_side = async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        let welcome = ServerFrame::Welcome(vessel_proto::Welcome {
            protocol: ProtocolVersion::new(2),
            node: node(),
        });
        node_end
            .send(serde_json::to_string(&welcome).unwrap())
            .await
            .unwrap();
        node_end
    };
    let config = ClientConfig {
        command_capacity: NonZeroUsize::new(1).unwrap(),
        ..ClientConfig::default()
    };
    let (client, mut node_end) = tokio::join!(
        async {
            Connection::connect(client_end, client_info(), &config)
                .await
                .unwrap()
        },
        node_side
    );
    // Far more calls than the transport and the command queue hold; the
    // node reads none of them.
    let calls: Vec<_> = (0..10)
        .map(|_| {
            let client = client.clone();
            tokio::spawn(async move { client.call(GetNode).await })
        })
        .collect();
    tokio::task::yield_now().await;
    node_end.close_sending();
    for call in calls {
        assert_eq!(call.await.unwrap(), Err(CallError::Disconnected));
    }
    drop(node_end);
}

/// `call_any`: a call built elsewhere (a browser's, through `vessel ui`)
/// goes as it is and its reply comes back untouched.
#[tokio::test]
async fn an_already_built_call_round_trips_untouched() {
    let running = start(ClientConfig::default()).await;
    let client = &running.client;
    let call = UnaryCall::try_from(
        GetSession {
            session: session(1),
        }
        .into_call(),
    )
    .unwrap();
    assert_eq!(call.method(), vessel_proto::Method::GetSession);
    let reply = client.call_any(call).await.unwrap();
    assert_eq!(
        reply,
        vessel_proto::Reply::GetSession(info(session(1), spec()))
    );
    // The node's error comes back as for a typed call.
    let missing = UnaryCall::try_from(
        GetSession {
            session: session(404),
        }
        .into_call(),
    )
    .unwrap();
    assert!(matches!(
        client.call_any(missing).await,
        Err(CallError::Remote(error)) if error.code == ErrorCode::NotFound
    ));
}

/// `session.subscribe` opens a stream, so it is not a one-reply call.
#[test]
fn a_subscription_is_not_a_one_reply_call() {
    let subscribe = Subscribe {
        session: session(1),
        from: EventSeq::ZERO,
    };
    assert_eq!(
        UnaryCall::try_from(subscribe.into_call()),
        Err(crate::NotUnary(vessel_proto::Method::Subscribe))
    );
}
