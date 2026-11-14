//! New inbox methods must not enter a connection that cannot decode them.
use super::*;
use vessel_proto::inbox::{GetInboxEntry, InboxFetch, InboxKey, InboxRead, ListInbox, MarkInbox};
use vessel_proto::push::{
    PushActivity, PushAuth, PushFocus, PushPublicKey, PushSubscription, ReportPushPresence,
};
use vessel_proto::{Deliver, Forward, Method, Origin, OriginBy, RelayCall, Request as _};
use vessel_rpc::{CallError, UnaryCall};

const OLD: ProtocolVersion = ProtocolVersion::new(18);
const NEEDS: ProtocolVersion = ProtocolVersion::new(19);

fn key() -> InboxKey {
    InboxKey {
        session: session_id(),
        seq: EventSeq::ZERO,
    }
}

fn get() -> GetInboxEntry {
    GetInboxEntry {
        key: key(),
        fetch: InboxFetch::LiveRequest,
    }
}

fn mark() -> MarkInbox {
    MarkInbox {
        key: key(),
        read: InboxRead::Read,
    }
}

async fn old_node() -> (NodeClient, NodeId, tokio::task::JoinHandle<Option<String>>) {
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let hop = node();
    let target = hop.id;
    let task = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        node_end
            .send(
                serde_json::to_string(&ServerFrame::Welcome(Welcome {
                    protocol: OLD,
                    node: hop,
                }))
                .unwrap(),
            )
            .await
            .unwrap();
        // Return and close on the first unwanted frame, so a broken gate
        // fails the exact error assertion instead of waiting for a reply.
        node_end.next().await.map(Result::unwrap)
    });
    let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
        .await
        .unwrap();
    (client, target, task)
}

fn refused(error: &CallError, expected: Method) {
    let CallError::NotInProtocol {
        method,
        needs,
        negotiated,
    } = error
    else {
        panic!("expected a local version refusal, got {error:?}");
    };
    assert_eq!(*method, expected);
    assert_eq!(*needs, NEEDS);
    assert_eq!(*negotiated, OLD);
}

#[tokio::test]
async fn inbox_and_push_public_calls_send_no_frame_to_protocol_eighteen() {
    let (client, _, task) = old_node().await;
    let mut point = [7; 65];
    point[0] = 4;
    let subscription = PushSubscription {
        endpoint: "https://push.example/capability".parse().unwrap(),
        p256dh: PushPublicKey::from_bytes(&point).unwrap(),
        auth: PushAuth::from_bytes([9; 16]),
    };
    for (result, method) in [
        (
            client.inbox(ListInbox::default()).await.map(|_| ()),
            Method::ListInbox,
        ),
        (
            client.inbox_entry(key()).await.map(|_| ()),
            Method::GetInboxEntry,
        ),
        (
            client.inbox_request(key()).await.map(|_| ()),
            Method::GetInboxEntry,
        ),
        (
            client.mark_inbox(key(), InboxRead::Read).await,
            Method::MarkInbox,
        ),
        (client.push_key().await.map(|_| ()), Method::GetPushKey),
        (
            client.subscribe_push(subscription).await.map(|_| ()),
            Method::SubscribePush,
        ),
        (client.unsubscribe_push().await, Method::UnsubscribePush),
        (
            client
                .report_push_presence(ReportPushPresence {
                    focus: PushFocus::Background,
                    activity: PushActivity::Unchanged,
                })
                .await,
            Method::ReportPushPresence,
        ),
    ] {
        refused(&result.unwrap_err(), method);
    }
    drop(client);
    assert!(task.await.unwrap().is_none(), "no post-welcome frame");
}

#[tokio::test]
async fn inbox_call_on_the_welcomed_node_sends_no_frame_to_protocol_eighteen() {
    let (client, target, task) = old_node().await;
    for result in [
        client
            .call_on(target, ListInbox::default())
            .await
            .map(|_| ()),
        client.call_on(target, get()).await.map(|_| ()),
        client.call_on(target, mark()).await.map(|_| ()),
    ] {
        refused(&result.unwrap_err(), Method::Forward);
    }
    drop(client);
    assert!(task.await.unwrap().is_none(), "self-forward sent no frame");
}

#[tokio::test]
async fn all_inbox_envelopes_send_no_frame_to_protocol_eighteen() {
    let (client, target, task) = old_node().await;
    let origin = Origin {
        node: target,
        by: OriginBy::Owner,
        scopes: Vec::new(),
    };
    for inner in [
        ListInbox::default().into_call(),
        get().into_call(),
        mark().into_call(),
    ] {
        for call in [
            Forward {
                node: target,
                call: Box::new(inner.clone()),
            }
            .into_call(),
            RelayCall {
                node: target,
                call: Box::new(inner.clone()),
                origin: origin.clone(),
            }
            .into_call(),
            Deliver {
                call: Box::new(inner.clone()),
                origin: origin.clone(),
            }
            .into_call(),
        ] {
            let method = call.method();
            refused(
                &client
                    .call_any(UnaryCall::try_from(call).unwrap())
                    .await
                    .unwrap_err(),
                method,
            );
        }
    }
    drop(client);
    assert!(task.await.unwrap().is_none(), "no typed envelope was sent");
}
