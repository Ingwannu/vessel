//! Pending prompt reads are refused before any direct or routed frame is sent.
use super::*;
use vessel_proto::prompt_index::ListSessionPrompts;
use vessel_proto::{Deliver, Forward, Method, Origin, OriginBy, RelayCall, Request as _};
use vessel_rpc::{CallError, UnaryCall};

#[tokio::test]
async fn pending_prompt_index_never_enters_a_direct_or_enveloped_transport() {
    let hop = node();
    let target = hop.id;
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let task = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        node_end
            .send(
                serde_json::to_string(&ServerFrame::Welcome(Welcome {
                    protocol: vessel_proto::PROTOCOL_VERSION,
                    node: hop,
                }))
                .unwrap(),
            )
            .await
            .unwrap();
        node_end.next().await
    });
    let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
        .await
        .unwrap();
    let request = ListSessionPrompts {
        session: session_id(),
        after: None,
        epoch: None,
        limit: Default::default(),
    };
    assert_eq!(ListSessionPrompts::METHOD.since(), None);
    assert_eq!(
        ListSessionPrompts::METHOD.scope(),
        Some(vessel_proto::Scope::Read)
    );
    assert_eq!(
        ListSessionPrompts::METHOD.shape(),
        vessel_proto::MethodShape::Unary
    );
    assert!(matches!(
        client.call(request.clone()).await,
        Err(CallError::UnassignedProtocol {
            method: Method::ListSessionPrompts
        })
    ));
    assert!(matches!(
        client.call_on(target, request.clone()).await,
        Err(CallError::UnassignedProtocol {
            method: Method::Forward
        })
    ));
    let origin = Origin {
        node: target,
        by: OriginBy::Owner,
        scopes: vec![],
    };
    for call in [
        Forward {
            node: target,
            call: Box::new(request.clone().into_call()),
        }
        .into_call(),
        RelayCall {
            node: target,
            call: Box::new(request.clone().into_call()),
            origin: origin.clone(),
        }
        .into_call(),
        Deliver {
            call: Box::new(request.into_call()),
            origin,
        }
        .into_call(),
    ] {
        let expected = call.method();
        assert_eq!(call.since(), None);
        let error = client
            .call_any(UnaryCall::try_from(call).unwrap())
            .await
            .unwrap_err();
        assert!(matches!(error, CallError::UnassignedProtocol { method } if method == expected));
    }
    drop(client);
    assert!(
        task.await.unwrap().is_none(),
        "no request followed the welcome"
    );
}
