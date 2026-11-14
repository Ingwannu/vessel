//! Forced old frames must fail before even the fake's dispatch recorder.
use super::*;
use vessel_proto::inbox::{GetInboxEntry, InboxFetch, InboxKey, InboxRead, ListInbox, MarkInbox};
use vessel_proto::push::{
    GetPushKey, PushActivity, PushAuth, PushFocus, PushPublicKey, PushSubscription,
    ReportPushPresence, SubscribePush, UnsubscribePush,
};
use vessel_proto::{Deliver, Forward, Origin, OriginBy, RelayCall};

fn inbox_calls() -> [Call; 3] {
    let key = InboxKey {
        session: session(1),
        seq: EventSeq::ZERO,
    };
    [
        ListInbox::default().into_call(),
        GetInboxEntry {
            key,
            fetch: InboxFetch::LiveRequest,
        }
        .into_call(),
        MarkInbox {
            key,
            read: InboxRead::Read,
        }
        .into_call(),
    ]
}

fn envelopes(call: Call, node: NodeId) -> [Call; 4] {
    let origin = Origin {
        node,
        by: OriginBy::Owner,
        scopes: Vec::new(),
    };
    [
        call.clone(),
        Forward {
            node,
            call: Box::new(call.clone()),
        }
        .into_call(),
        RelayCall {
            node,
            call: Box::new(call.clone()),
            origin: origin.clone(),
        }
        .into_call(),
        Deliver {
            call: Box::new(call),
            origin,
        }
        .into_call(),
    ]
}

async fn response(client: &mut MemoryTransport, id: u64, call: Call) -> Outcome {
    let text = call_raw(client, id, call).await;
    let ServerFrame::Response {
        id: answered,
        outcome,
    } = serde_json::from_str(&text).unwrap()
    else {
        panic!("expected a response, got {text}");
    };
    assert_eq!(answered, RequestId::new(id));
    outcome
}

#[tokio::test]
async fn old_inbox_envelopes_are_refused_before_dispatch_even_when_addressed_here() {
    let (mut client, mut served, node) = raw_client_with_node(18).await;
    let mut id = 0;
    for inner in inbox_calls() {
        for call in envelopes(inner, node) {
            id += 1;
            let method = call.method();
            assert_eq!(
                response(&mut client, id, call).await,
                Outcome::Err(WireError::new(
                    ErrorCode::Unsupported,
                    format!(
                        "`{}` as sent needs protocol 19; this connection speaks 18",
                        method.as_str()
                    ),
                ))
            );
            assert!(
                served.seen.try_recv().is_err(),
                "new call reached the handler"
            );
        }
    }
    // The same connection still admits an old method, inside every envelope.
    for call in envelopes(GetNode.into_call(), node) {
        id += 1;
        let outcome = response(&mut client, id, call.clone()).await;
        if call.method() == GetNode::METHOD {
            assert!(matches!(outcome, Outcome::Ok(_)));
        } else {
            assert_eq!(
                outcome,
                Outcome::Err(WireError::new(ErrorCode::Unsupported, "not in this fake"))
            );
        }
        assert_eq!(served.seen.recv().await.unwrap(), call);
    }
    assert!(served.seen.try_recv().is_err());
}

#[tokio::test]
async fn supported_inbox_envelopes_reach_the_handler_unchanged() {
    let (mut client, mut served, node) = raw_client_with_node(19).await;
    let mut id = 0;
    for inner in inbox_calls() {
        for call in envelopes(inner, node) {
            id += 1;
            assert_eq!(
                response(&mut client, id, call.clone()).await,
                Outcome::Err(WireError::new(ErrorCode::Unsupported, "not in this fake"))
            );
            assert_eq!(served.seen.recv().await.unwrap(), call);
        }
    }
    assert!(served.seen.try_recv().is_err());
}

#[tokio::test]
async fn old_direct_push_calls_are_refused_before_dispatch() {
    let (mut client, mut served) = raw_client(18).await;
    let mut point = [7; 65];
    point[0] = 4;
    let calls = [
        GetPushKey.into_call(),
        SubscribePush {
            subscription: PushSubscription {
                endpoint: "https://push.example/capability".parse().unwrap(),
                p256dh: PushPublicKey::from_bytes(&point).unwrap(),
                auth: PushAuth::from_bytes([9; 16]),
            },
        }
        .into_call(),
        UnsubscribePush.into_call(),
        ReportPushPresence {
            focus: PushFocus::Background,
            activity: PushActivity::Unchanged,
        }
        .into_call(),
    ];
    for (id, call) in calls.into_iter().enumerate() {
        let method = call.method();
        assert_eq!(
            response(&mut client, u64::try_from(id).unwrap(), call).await,
            Outcome::Err(WireError::new(
                ErrorCode::Unsupported,
                format!(
                    "`{}` as sent needs protocol 19; this connection speaks 18",
                    method.as_str()
                )
            ))
        );
        assert!(served.seen.try_recv().is_err());
    }
}
