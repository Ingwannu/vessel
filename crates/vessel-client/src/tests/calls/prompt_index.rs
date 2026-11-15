//! Pending prompt reads are refused before any direct or routed frame is sent.
use super::*;
use vessel_proto::prompt_index::{ListSessionPrompts, PROMPT_INDEX_PROTOCOL, PromptIndexLimit};
use vessel_proto::{Deliver, Forward, Method, Origin, OriginBy, RelayCall, Request as _};
use vessel_rpc::{CallError, UnaryCall};

#[tokio::test]
async fn pending_prompt_index_never_enters_a_direct_or_enveloped_transport() {
    let negotiated = PROMPT_INDEX_PROTOCOL.map_or(vessel_proto::PROTOCOL_VERSION, |version| {
        ProtocolVersion::new(version.get().checked_sub(1).unwrap())
    });
    let hop = node();
    let target = hop.id;
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let task = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        node_end
            .send(
                serde_json::to_string(&ServerFrame::Welcome(Welcome {
                    protocol: negotiated,
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
        limit: PromptIndexLimit::default(),
    };
    assert_eq!(ListSessionPrompts::METHOD.since(), PROMPT_INDEX_PROTOCOL);
    assert_eq!(
        ListSessionPrompts::METHOD.scope(),
        Some(vessel_proto::Scope::Read)
    );
    assert_eq!(
        ListSessionPrompts::METHOD.shape(),
        vessel_proto::MethodShape::Unary
    );
    refused(
        &client.call(request.clone()).await.unwrap_err(),
        Method::ListSessionPrompts,
        negotiated,
    );
    refused(
        &client.call_on(target, request.clone()).await.unwrap_err(),
        Method::Forward,
        negotiated,
    );
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
        assert_eq!(call.since(), PROMPT_INDEX_PROTOCOL);
        let error = client
            .call_any(UnaryCall::try_from(call).unwrap())
            .await
            .unwrap_err();
        refused(&error, expected, negotiated);
    }
    drop(client);
    assert!(
        task.await.unwrap().is_none(),
        "no request followed the welcome"
    );
}

// Assignment changes the assertion from pending refusal to old-peer refusal;
// the same no-frame evidence remains required after the integrator assigns a number.
fn refused(error: &CallError, expected: Method, negotiated: ProtocolVersion) {
    match PROMPT_INDEX_PROTOCOL {
        None => assert_eq!(error, &CallError::UnassignedProtocol { method: expected }),
        Some(needs) => assert_eq!(
            error,
            &CallError::NotInProtocol {
                method: expected,
                needs,
                negotiated
            }
        ),
    }
}
